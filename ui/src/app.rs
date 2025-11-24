#![allow(non_snake_case)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use crate::components::control_bar::ControlBar;
use crate::processes::{run_encoder, run_recorder};
use crate::state::{AppMode, AppState};
use dioxus::desktop::use_window;
use dioxus::prelude::*;
use mandygif_protocol::RecorderEvent;
use tokio::sync::mpsc;

pub fn App() -> Element {
    use_context_provider(AppState::new);
    let mut state = use_context::<AppState>();
    let window = use_window();

    let start_recording = move |()| {
        if *state.mode.read() == AppMode::Recording {
            return;
        }

        let region = state.region.read().clone();

        state.mode.set(AppMode::Recording);
        state.duration_ms.set(0);

        let (tx, mut rx) = mpsc::unbounded_channel();
        let (stop_tx, mut stop_rx) = mpsc::unbounded_channel();

        state.stop_tx.set(Some(stop_tx));

        spawn(async move {
            if let Err(e) = run_recorder(tx, &mut stop_rx, region).await {
                tracing::error!("Recorder failed: {e}");
            }
        });

        spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    RecorderEvent::Progress { pts_ms } => {
                        state.duration_ms.set(pts_ms as i32);
                    }
                    RecorderEvent::Stopped { duration_ms, path } => {
                        state.mode.set(AppMode::Idle);
                        state.duration_ms.set(duration_ms as i32);
                        state.rec_path.set(Some(path));
                        state.stop_tx.set(None);
                    }
                    RecorderEvent::Error { hint, .. } => {
                        tracing::error!("Recorder Error: {hint}");
                        state.mode.set(AppMode::Idle);
                        state.stop_tx.set(None);
                    }
                    RecorderEvent::Started { .. } => {}
                }
            }
        });
    };

    let stop_recording = move |()| {
        if let Some(tx) = state.stop_tx.take() {
            let _ = tx.send(());
        }
    };

    let start_export = move |()| {
        let Some(path) = state.rec_path.read().clone() else {
            return;
        };

        let fmt = state.export_format.read().clone();
        let fps = *state.export_fps.read();
        let scale = *state.export_scale.read();
        let dur = *state.duration_ms.read() as u64;

        state.mode.set(AppMode::Exporting);

        spawn(async move {
            if let Err(e) = run_encoder(path, &fmt, fps, (0, dur), scale).await {
                tracing::error!("Encoder failed: {e}");
            }
            state.mode.set(AppMode::Idle);
        });
    };

    let current_mode = *state.mode.read();
    let border_color = if current_mode == AppMode::Recording {
        "#ff0000"
    } else {
        "#00ff00"
    };

    let drag_window = window.clone();
    let close_window = window.clone();

    rsx! {
        style { dangerous_inner_html: include_str!("style.css") }

        div {
            class: "overlay-container",
            style: "width: 100vw; height: 100vh; border: 4px solid {border_color}; box-sizing: border-box; position: relative;",

            div {
                class: "header",
                onmousedown: move |_| drag_window.drag(),
                style: "background: rgba(0,0,0,0.8); padding: 8px 12px; display: flex; justify-content: space-between; align-items: center; border-bottom-right-radius: 8px; width: fit-content;",

                span { style: "color: white; font-weight: bold; margin-right: 20px;", "MandyGIF" }
                button {
                    onclick: move |_| close_window.close(),
                    style: "background: #ff4444; color: white; border: none; border-radius: 4px; padding: 2px 8px; cursor: pointer; font-weight: bold;",
                    "✕"
                }
            }

            div {
                style: "position: absolute; bottom: 20px; left: 20px;",
                ControlBar {
                    on_record: start_recording,
                    on_stop: stop_recording,
                    on_export: start_export
                }
            }
        }
    }
}
