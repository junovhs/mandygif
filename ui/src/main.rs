//! MandyGIF UI - Slint interface with process spawning for recorder/encoder

use anyhow::{Context, Result};
use mandygif_protocol::*;
use slint::ComponentHandle;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;
use tracing::{info, error, debug};

slint::include_modules!();

struct AppStateData {
    recording_path: Option<PathBuf>,
    recording_duration_ms: u64,
    stop_tx: Option<oneshot::Sender<()>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    info!("MandyGIF UI starting");

    let ui = AppWindow::new()?;
    let state = Arc::new(Mutex::new(AppStateData {
        recording_path: None,
        recording_duration_ms: 0,
        stop_tx: None,
    }));

    // Start recording callback
    let ui_weak = ui.as_weak();
    let state_clone = state.clone();
    ui.on_start_recording(move || {
        let ui = ui_weak.unwrap();
        let region = ui.get_capture_region();
        
        info!("Starting recording: {}x{} at {},{}", 
            region.width, region.height, region.x, region.y);
        
        ui.set_state(AppState::Recording);
        ui.set_status_text("Recording...".into());
        
        // Spawn recorder with stop channel
        let ui_weak_inner = ui.as_weak();
        let state_inner = state_clone.clone();
        tokio::spawn(async move {
            if let Err(e) = run_recorder(ui_weak_inner, state_inner, region).await {
                error!("Recorder failed: {:#}", e);
            }
        });
    });

    // Stop recording callback
    let ui_weak = ui.as_weak();
    let state_clone = state.clone();
    ui.on_stop_recording(move || {
        let ui = ui_weak.unwrap();
        info!("Stop recording requested");
        ui.set_status_text("Stopping...".into());
        
        // Send stop signal via channel
        if let Some(tx) = state_clone.lock().unwrap().stop_tx.take() {
            let _ = tx.send(());
        }
    });

    // Export callback
    let ui_weak = ui.as_weak();
    let state_clone = state.clone();
    ui.on_start_export(move || {
        let ui = ui_weak.unwrap();
        
        let recording_path = state_clone.lock().unwrap().recording_path.clone();
        if recording_path.is_none() {
            error!("No recording to export");
            return;
        }
        
        ui.set_state(AppState::Exporting);
        ui.set_status_text("Exporting...".into());
        
        let format = ui.get_export_format();
        let fps = ui.get_export_fps();
        let trim_start = ui.get_trim_start_ms();
        let trim_end = ui.get_trim_end_ms();
        let scale = ui.get_scale_width();
        
        info!("Exporting: format={}, fps={}, trim={}..{}, scale={}", 
            format, fps, trim_start, trim_end, scale);
        
        let ui_weak_inner = ui.as_weak();
        let input = recording_path.unwrap();
        tokio::spawn(async move {
            if let Err(e) = run_encoder(ui_weak_inner, input, format.to_string(), 
                fps, trim_start as u64, trim_end as u64, scale).await {
                error!("Encoder failed: {:#}", e);
            }
        });
    });

    // Region selector placeholder
    ui.on_show_region_selector(|| {
        info!("Region selector not yet implemented - using default region");
    });

    ui.run()?;
    Ok(())
}

async fn run_recorder(
    ui: slint::Weak<AppWindow>,
    state: Arc<Mutex<AppStateData>>,
    region: Region,
) -> Result<()> {
    let output_path = PathBuf::from("/tmp/mandygif_recording.mp4");
    
    let cmd = RecorderCommand::Start {
        region: CaptureRegion {
            x: region.x,
            y: region.y,
            width: region.width as u32,
            height: region.height as u32,
        },
        fps: 30,
        cursor: false,
        out: output_path.clone(),
    };
    
    let recorder_bin = std::env::current_exe()?
        .parent().unwrap()
        .join("recorder-linux");
    
    let mut child = Command::new(&recorder_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("Failed to spawn recorder")?;
    
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();
    
    // Create stop channel
    let (stop_tx, mut stop_rx) = oneshot::channel();
    state.lock().unwrap().stop_tx = Some(stop_tx);
    
    // Send start command
    let start_json = to_jsonl(&cmd)?;
    stdin.write_all(start_json.as_bytes()).await?;
    stdin.flush().await?;
    
    // Read events loop
    eprintln!("UI: Starting event read loop");
    let ui_clone = ui.clone();
    loop {
        tokio::select! {
            // Check for stop signal
            _ = &mut stop_rx => {
                info!("Stop signal received, sending stop command");
                let stop_cmd = RecorderCommand::Stop;
                let stop_json = to_jsonl(&stop_cmd)?;
                stdin.write_all(stop_json.as_bytes()).await?;
                stdin.flush().await?;
                drop(stdin); // Close stdin to signal we're done
                break;
            }
            
            // Read recorder events
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        eprintln!("UI: Got line from recorder: {:?}", line);
                        
                        // Skip non-JSON lines (GStreamer debug, etc)
                        if !line.starts_with('{') {
                            eprintln!("UI: Skipping non-JSON line");
                            continue;
                        }
                        
                        match parse_recorder_event(&line) {
                            Ok(RecorderEvent::Started { .. }) => {
                                info!("Recording started");
                                if let Some(ui) = ui_clone.upgrade() {
                                    ui.set_status_text("⏺ Recording...".into());
                                }
                            }
                            Ok(RecorderEvent::Progress { pts_ms }) => {
                                eprintln!("UI: Progress event parsed: {}ms", pts_ms);
                                if let Some(ui) = ui_clone.upgrade() {
                                    eprintln!("UI: Calling set_recording_duration_ms({})", pts_ms);
                                    ui.set_recording_duration_ms(pts_ms as i32);
                                    eprintln!("UI: Timer should now show: {}ms", pts_ms);
                                } else {
                                    eprintln!("UI: FAILED to upgrade ui weak reference!");
                                }
                            }
                            Ok(RecorderEvent::Stopped { duration_ms, path }) => {
                                info!("Recording stopped: {}ms, saved to {}", duration_ms, path.display());
                                
                                state.lock().unwrap().recording_path = Some(path);
                                state.lock().unwrap().recording_duration_ms = duration_ms;
                                
                                if let Some(ui) = ui_clone.upgrade() {
                                    ui.set_state(AppState::Editing);
                                    ui.set_status_text("Ready to export".into());
                                    ui.set_trim_start_ms(0);
                                    ui.set_trim_end_ms(duration_ms as i32);
                                }
                                break;
                            }
                            Ok(RecorderEvent::Error { kind, hint }) => {
                                error!("Recorder error: {:?} - {}", kind, hint);
                                if let Some(ui) = ui_clone.upgrade() {
                                    ui.set_state(AppState::Idle);
                                    ui.set_status_text(format!("Error: {}", hint).into());
                                }
                                break;
                            }
                            Err(e) => {
                                error!("Failed to parse recorder event: {}", e);
                            }
                        }
                    }
                    Ok(None) => break, // EOF
                    Err(e) => {
                        error!("Error reading from recorder: {}", e);
                        break;
                    }
                }
            }
        }
    }
    
    let _ = child.wait().await;
    Ok(())
}

async fn run_encoder(
    ui: slint::Weak<AppWindow>,
    input: PathBuf,
    format: String,
    fps: i32,
    trim_start_ms: u64,
    trim_end_ms: u64,
    scale_px: i32,
) -> Result<()> {
    let output_path = PathBuf::from(format!("/tmp/mandygif_export.{}", format));
    
    let cmd = match format.as_str() {
        "gif" => EncoderCommand::Gif {
            input: input.clone(),
            trim: TrimRange { start_ms: trim_start_ms, end_ms: trim_end_ms },
            fps: fps as u32,
            scale_px: Some(scale_px as u32),
            loop_mode: LoopMode::Normal,
            captions: vec![],
            out: output_path.clone(),
        },
        "webp" => EncoderCommand::Webp {
            input: input.clone(),
            trim: TrimRange { start_ms: trim_start_ms, end_ms: trim_end_ms },
            fps: fps as u32,
            scale_px: Some(scale_px as u32),
            quality: 0.85,
            lossless: false,
            captions: vec![],
            out: output_path.clone(),
        },
        _ => EncoderCommand::Mp4 {
            input: input.clone(),
            trim: TrimRange { start_ms: trim_start_ms, end_ms: trim_end_ms },
            fps: fps as u32,
            scale_px: Some(scale_px as u32),
            quality: 0.8,
            captions: vec![],
            out: output_path.clone(),
        },
    };
    
    let encoder_bin = std::env::current_exe()?
        .parent().unwrap()
        .join("encoder");
    
    let mut child = Command::new(&encoder_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("Failed to spawn encoder")?;
    
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();
    
    // Send encode command
    let cmd_json = to_jsonl(&cmd)?;
    stdin.write_all(cmd_json.as_bytes()).await?;
    drop(stdin);
    
    // Read events
    while let Some(line) = reader.next_line().await? {
        debug!("Encoder: {}", line);
        
        if !line.starts_with('{') {
            continue;
        }
        
        match parse_encoder_event(&line) {
            Ok(EncoderEvent::Progress { percent }) => {
                info!("Encoding progress: {}%", percent);
            }
            Ok(EncoderEvent::Done { path }) => {
                info!("Export complete: {}", path.display());
                
                if let Some(ui) = ui.upgrade() {
                    ui.set_state(AppState::Idle);
                    ui.set_status_text(format!("Exported to {}", path.display()).into());
                }
                break;
            }
            Ok(EncoderEvent::Error { kind, hint }) => {
                error!("Encoder error: {:?} - {}", kind, hint);
                if let Some(ui) = ui.upgrade() {
                    ui.set_state(AppState::Editing);
                    ui.set_status_text(format!("Export failed: {}", hint).into());
                }
                break;
            }
            Err(e) => {
                error!("Failed to parse encoder event: {}", e);
            }
        }
    }
    
    let _ = child.wait().await;
    Ok(())
}