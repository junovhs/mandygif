//! MandyGIF - Unified overlay interface with integrated controls

use anyhow::{Context, Result};
use mandygif_protocol::*;
use slint::ComponentHandle;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{info, error, debug};

slint::include_modules!();

struct AppStateData {
    recording_path: Option<PathBuf>,
    recording_duration_ms: u64,
    stop_tx: Option<mpsc::UnboundedSender<()>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    info!("MandyGIF starting - unified overlay mode");

    let ui = UnifiedOverlay::new()?;
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
        
        // Check if already recording
        if state_clone.lock().unwrap().stop_tx.is_some() {
            info!("Already recording, ignoring start request");
            return;
        }
        
        let region = CaptureRegion {
            x: ui.get_sel_x() as i32,
            y: ui.get_sel_y() as i32,
            width: ui.get_sel_width() as u32,
            height: ui.get_sel_height() as u32,
        };
        
        info!("Starting recording: {}x{} at {},{}", 
            region.width, region.height, region.x, region.y);
        
        ui.set_recording(true);
        ui.set_recording_duration_ms(0);
        
        // Create channel for progress updates
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel();
        
        // Spawn recorder
        let state_inner = state_clone.clone();
        tokio::spawn(async move {
            if let Err(e) = run_recorder(progress_tx, state_inner, region).await {
                error!("Recorder failed: {:#}", e);
            }
        });
        
        // Spawn UI update task
        let ui_weak_update = ui.as_weak();
        let state_for_update = state_clone.clone();
        tokio::spawn(async move {
            while let Some(event) = progress_rx.recv().await {
                let ui_clone = ui_weak_update.clone();
                let state_clone = state_for_update.clone();
                slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_clone.upgrade() {
                        match event {
                            RecorderEvent::Progress { pts_ms } => {
                                ui.set_recording_duration_ms(pts_ms as i32);
                            }
                            RecorderEvent::Stopped { duration_ms, .. } => {
                                ui.set_recording(false);
                                ui.set_recording_duration_ms(duration_ms as i32);
                                // Clear stop channel since recording is done
                                state_clone.lock().unwrap().stop_tx = None;
                            }
                            RecorderEvent::Error { hint, .. } => {
                                ui.set_recording(false);
                                error!("Recording error: {}", hint);
                                // Clear stop channel since recording failed
                                state_clone.lock().unwrap().stop_tx = None;
                            }
                            _ => {}
                        }
                    }
                }).unwrap();
            }
        });
    });

    // Stop recording callback
    let state_clone = state.clone();
    ui.on_stop_recording(move || {
        info!("Stop recording requested");
        
        // Send stop signal via channel
        if let Some(tx) = state_clone.lock().unwrap().stop_tx.as_ref() {
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
        
        let format = ui.get_export_format();
        let fps = ui.get_export_fps();
        let scale = ui.get_scale_width();
        let duration_ms = ui.get_recording_duration_ms() as u64;
        
        info!("Exporting: format={}, fps={}, scale={}", format, fps, scale);
        
        let input = recording_path.unwrap();
        tokio::spawn(async move {
            if let Err(e) = run_encoder(
                input, 
                format.to_string(), 
                fps, 
                0, 
                duration_ms, 
                scale
            ).await {
                error!("Encoder failed: {:#}", e);
            }
        });
    });

    // Cancel callback - quit the app
    ui.on_cancel(|| {
        info!("User canceled - exiting");
        slint::quit_event_loop().ok();
    });

    ui.run()?;
    Ok(())
}

async fn run_recorder(
    event_tx: mpsc::UnboundedSender<RecorderEvent>,
    state: Arc<Mutex<AppStateData>>,
    region: CaptureRegion,
) -> Result<()> {
    let output_path = PathBuf::from("/tmp/mandygif_recording.mp4");
    
    let cmd = RecorderCommand::Start {
        region,
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
    
    // Create stop channel using mpsc (unbounded) instead of oneshot
    let (stop_tx, mut stop_rx) = mpsc::unbounded_channel();
    state.lock().unwrap().stop_tx = Some(stop_tx);
    
    // Send start command
    let start_json = to_jsonl(&cmd)?;
    stdin.write_all(start_json.as_bytes()).await?;
    stdin.flush().await?;
    
    // Read events loop
    loop {
        tokio::select! {
            // Check for stop signal
            _ = stop_rx.recv() => {
                info!("Stop signal received, sending stop command to recorder");
                let stop_cmd = RecorderCommand::Stop;
                let stop_json = to_jsonl(&stop_cmd)?;
                stdin.write_all(stop_json.as_bytes()).await?;
                stdin.flush().await?;
                // Continue reading to get the stopped event
            }
            
            // Read recorder events
            line = reader.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if !line.starts_with('{') {
                            continue;
                        }
                        
                        match parse_recorder_event(&line) {
                            Ok(RecorderEvent::Started { .. }) => {
                                info!("Recording started");
                                let _ = event_tx.send(RecorderEvent::Started { pts_ms: 0 });
                            }
                            Ok(RecorderEvent::Progress { pts_ms }) => {
                                let _ = event_tx.send(RecorderEvent::Progress { pts_ms });
                            }
                            Ok(RecorderEvent::Stopped { duration_ms, path }) => {
                                info!("Recording stopped: {}ms, saved to {}", 
                                    duration_ms, path.display());
                                state.lock().unwrap().recording_path = Some(path.clone());
                                state.lock().unwrap().recording_duration_ms = duration_ms;
                                let _ = event_tx.send(RecorderEvent::Stopped { 
                                    duration_ms, 
                                    path 
                                });
                                break;
                            }
                            Ok(event @ RecorderEvent::Error { .. }) => {
                                error!("Recorder error");
                                let _ = event_tx.send(event);
                                break;
                            }
                            Err(e) => {
                                error!("Failed to parse recorder event: {}", e);
                            }
                        }
                    }
                    Ok(None) => break,
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
                break;
            }
            Ok(EncoderEvent::Error { kind, hint }) => {
                error!("Encoder error: {:?} - {}", kind, hint);
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