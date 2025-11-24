#![allow(non_snake_case)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use crate::components::control_bar::ControlBar;
use crate::components::resize_handle::ResizeHandles;
use crate::processes::{run_encoder, run_recorder};
use crate::state::{AppMode, AppState};
use dioxus::desktop::tao::dpi::{LogicalSize, PhysicalPosition};
use dioxus::desktop::use_window;
use dioxus::prelude::*;
use mandygif_protocol::RecorderEvent;
use tokio::sync::mpsc;

pub fn App() -> Element {
    use_context_provider(AppState::new);
    let mut state = use_context::<AppState>();
    let window = use_window();

    let win_startup = window.clone();
    use_hook(move || {
        // FIX: Use LogicalSize here too
        let _ = win_startup.set_inner_size(LogicalSize::new(800.0, 600.0));
    });

    // ... (rest of the file remains identical, just pasting the relevant change above)
    let win_rec = window.clone();
    let start_recording = move |()| {
        if *state.mode.read() == AppMode::Recording {
            return;
        }

        // Capture region logic remains physical (correct for recording)
        let outer_pos = win_rec
            .outer_position()
            .unwrap_or(PhysicalPosition::new(0, 0));
        let outer_size = win_rec.outer_size();

        let region = mandygif_protocol::CaptureRegion {
            x: outer_pos.x + 1,
            y: outer_pos.y + 1,
            width: outer_size.width - 2,
            height: outer_size.height - 2,
        };

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
                        state.mode.set(AppMode::Review);
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
    let border_col = if current_mode == AppMode::Recording {
        "#ff0000"
    } else {
        "#00ff00"
    };

    let drag_win = window.clone();

    #[allow(dependency_on_unit_never_type_fallback)]
    let close_handler = move |_: MouseEvent| {
        std::process::exit(0);
    };

    rsx! {
        style { dangerous_inner_html: include_str!("style.css") }

        div {
            class: "app-frame",
            style: "border: 1px solid {border_col};",

            if current_mode == AppMode::Idle || current_mode == AppMode::Review {
                ResizeHandles {}
            }

            div {
                class: "header",
                style: "background: rgba(0,0,0,0.8); padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; border-bottom-right-radius: 6px; width: fit-content;",

                span {
                    onmousedown: move |_| drag_win.drag(),
                    style: "color: white; font-weight: 700; font-size: 14px; margin-right: 15px; cursor: move; user-select: none; letter-spacing: 0.5px;",
                    "MandyGIF"
                }

                button {
                    onclick: close_handler,
                    style: "background: #ff4444; color: white; border: none; border-radius: 3px; padding: 2px 6px; cursor: pointer; font-weight: 800; font-size: 12px;",
                    "✕"
                }
            }

            div {
                style: "position: absolute; bottom: 15px; left: 15px;",
                ControlBar {
                    on_record: start_recording,
                    on_stop: stop_recording,
                    on_export: start_export
                }
            }
        }
    }
}
