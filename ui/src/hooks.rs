#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use crate::processes::{run_encoder, run_recorder};
use crate::state::{use_app_state, AppMode};
use dioxus::desktop::tao::dpi::PhysicalPosition;
use dioxus::desktop::use_window;
use dioxus::prelude::*;
use mandygif_protocol::RecorderEvent;
use tokio::sync::mpsc;

pub struct RecorderController {
    pub start: Callback<()>,
    pub stop: Callback<()>,
    pub export: Callback<()>,
}

pub fn use_recorder() -> RecorderController {
    let mut state = use_app_state();
    let window = use_window();
    let rec_window = window.clone();

    // FIX: Wrap closure in Callback::new()
    let start = Callback::new(move |()| {
        if *state.mode.read() == AppMode::Recording {
            return;
        }

        let outer_pos = rec_window
            .outer_position()
            .unwrap_or(PhysicalPosition::new(0, 0));
        let outer_size = rec_window.outer_size();

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
    });

    // FIX: Wrap closure in Callback::new()
    let stop = Callback::new(move |()| {
        if let Some(tx) = state.stop_tx.take() {
            let _ = tx.send(());
        }
    });

    // FIX: Wrap closure in Callback::new()
    let export = Callback::new(move |()| {
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
    });

    RecorderController {
        start,
        stop,
        export,
    }
}
