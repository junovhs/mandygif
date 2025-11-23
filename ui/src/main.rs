//! `MandyGIF` - Unified overlay interface

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

mod processes;

use anyhow::Result;
use mandygif_protocol::{CaptureRegion, RecorderEvent};
use processes::{run_encoder, run_recorder};
use slint::ComponentHandle;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{error, info};

slint::include_modules!();

struct AppState {
    rec_path: Option<PathBuf>,
    stop_tx: Option<mpsc::UnboundedSender<()>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let ui = UnifiedOverlay::new()?;
    let state = Arc::new(Mutex::new(AppState {
        rec_path: None,
        stop_tx: None,
    }));

    setup_callbacks(&ui, &state);

    ui.run()?;
    Ok(())
}

fn setup_callbacks(ui: &UnifiedOverlay, state: &Arc<Mutex<AppState>>) {
    let w = ui.as_weak();
    let s = state.clone();

    ui.on_start_recording(move || {
        let Some(ui) = w.upgrade() else { return };

        // Fix: Removed `mut` here as guard is only read from
        let Ok(guard) = s.lock() else {
            error!("Lock poisoned");
            return;
        };

        if guard.stop_tx.is_some() {
            return;
        }

        let region = CaptureRegion {
            x: ui.get_sel_x() as i32,
            y: ui.get_sel_y() as i32,
            width: ui.get_sel_width() as u32,
            height: ui.get_sel_height() as u32,
        };

        ui.set_recording(true);
        // Release lock before spawning
        drop(guard);

        start_rec_task(w.clone(), s.clone(), region);
    });

    let s2 = state.clone();
    ui.on_stop_recording(move || {
        if let Ok(guard) = s2.lock() {
            if let Some(tx) = guard.stop_tx.as_ref() {
                let _ = tx.send(());
            }
        }
    });

    let w3 = ui.as_weak();
    let s3 = state.clone();
    ui.on_start_export(move || {
        let Some(ui) = w3.upgrade() else { return };

        let path_opt = s3.lock().map(|g| g.rec_path.clone()).unwrap_or(None);

        if let Some(path) = path_opt {
            let fmt = ui.get_export_format().to_string();
            let fps = ui.get_export_fps() as u32;
            let scale = ui.get_scale_width() as u32;
            let dur = ui.get_recording_duration_ms() as u64;

            tokio::spawn(async move {
                let _ = run_encoder(path, &fmt, fps, (0, dur), scale).await;
            });
        }
    });

    ui.on_cancel(|| std::process::exit(0));
}

fn start_rec_task(
    ui_weak: slint::Weak<UnifiedOverlay>,
    state: Arc<Mutex<AppState>>,
    region: CaptureRegion,
) {
    let (ptx, mut prx) = mpsc::unbounded_channel();
    let (stx, mut srx) = mpsc::unbounded_channel();

    if let Ok(mut guard) = state.lock() {
        guard.stop_tx = Some(stx);
    }

    tokio::spawn(async move {
        let _ = run_recorder(ptx, &mut srx, region).await;
    });

    tokio::spawn(async move {
        while let Some(event) = prx.recv().await {
            let w = ui_weak.clone();
            let s = state.clone();

            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = w.upgrade() {
                    handle_event(&ui, &s, event);
                }
            });
        }
    });
}

fn handle_event(ui: &UnifiedOverlay, state: &Arc<Mutex<AppState>>, ev: RecorderEvent) {
    match ev {
        RecorderEvent::Started { .. } => {
            info!("Recording started");
        }
        RecorderEvent::Progress { pts_ms } => {
            // Safety: pts_ms (u64) fits in i32 for video duration < 596 hours
            // We are ok with truncating if it exceeds that.
            ui.set_recording_duration_ms(pts_ms as i32);
        }
        RecorderEvent::Stopped { duration_ms, path } => {
            ui.set_recording(false);
            ui.set_recording_duration_ms(duration_ms as i32);
            if let Ok(mut g) = state.lock() {
                g.rec_path = Some(path);
                g.stop_tx = None;
            }
        }
        RecorderEvent::Error { hint, .. } => {
            ui.set_recording(false);
            error!("Error: {}", hint);
            if let Ok(mut g) = state.lock() {
                g.stop_tx = None;
            }
        }
    }
}
