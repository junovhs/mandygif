//! MandyGIF UI - Slint-based cross-platform interface
//! 
//! Transparent overlay with draggable capture region, controls, and timeline.

use anyhow::Result;
use mandygif_protocol::*;
use std::process::{Command, Stdio};
use std::io::Write;
use tracing::info;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let ui = AppWindow::new()?;
    
    // Handle start recording
    let ui_weak = ui.as_weak();
    ui.on_start_recording(move || {
        let ui = ui_weak.unwrap();
        info!("Start recording clicked");
        
        // TODO: Spawn recorder process and send start command via stdin
        // For now, just update UI state
        ui.set_recording(true);
    });
    
    // Handle stop recording
    let ui_weak = ui.as_weak();
    ui.on_stop_recording(move || {
        let ui = ui_weak.unwrap();
        info!("Stop recording clicked");
        
        // TODO: Send stop command to recorder
        ui.set_recording(false);
    });
    
    ui.run()?;
    Ok(())
}