use dioxus::prelude::*;
use mandygif_protocol::CaptureRegion; // Removed RecorderEvent
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppMode {
    Idle,
    Recording,
    Exporting,
}

#[derive(Clone, Copy, Debug)]
pub struct AppState {
    pub mode: Signal<AppMode>,
    pub region: Signal<CaptureRegion>,
    pub duration_ms: Signal<i32>,
    pub rec_path: Signal<Option<PathBuf>>,
    pub stop_tx: Signal<Option<UnboundedSender<()>>>,
    pub export_format: Signal<String>,
    pub export_fps: Signal<u32>,
    pub export_scale: Signal<u32>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: Signal::new(AppMode::Idle),
            region: Signal::new(CaptureRegion {
                x: 100,
                y: 100,
                width: 800,
                height: 600,
            }),
            duration_ms: Signal::new(0),
            rec_path: Signal::new(None),
            stop_tx: Signal::new(None),
            export_format: Signal::new("gif".to_string()),
            export_fps: Signal::new(15),
            export_scale: Signal::new(480),
        }
    }
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>()
}
