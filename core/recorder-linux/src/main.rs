//! Linux screen recorder using GStreamer + X11/XWayland capture
//! 
//! Uses ximagesrc for screen capture, x264 encoding to MP4.
//! Excludes overlay window from capture.

use anyhow::{Context, Result};
use mandygif_protocol::*;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, error, debug};

/// Shared state between command handler and async tasks
struct RecorderState {
    pipeline: Option<gst::Pipeline>,
    output_path: Option<PathBuf>,
    start_time: Option<std::time::Instant>,
}

impl RecorderState {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            pipeline: None,
            output_path: None,
            start_time: None,
        }))
    }
}

/// Validate recording parameters - Rule 5: assertions
fn validate_region(region: &CaptureRegion) -> Result<()> {
    if region.width == 0 || region.height == 0 {
        return Err(anyhow::anyhow!("Invalid region dimensions"));
    }
    if region.width > 3840 || region.height > 2160 {
        return Err(anyhow::anyhow!("Region too large"));
    }
    if region.x < 0 || region.y < 0 {
        return Err(anyhow::anyhow!("Negative coordinates not allowed"));
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    const MAX_ITERATIONS: u32 = 10000;  // Rule 2: bounded loop
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    gst::init().context("Failed to initialize GStreamer")?;
    
    info!("recorder-linux starting (protocol v{})", PROTOCOL_VERSION);
    info!("Using GStreamer version {}", gst::version_string());

    let state = RecorderState::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut iteration_count = 0u32;
    
    // Rule 2: bounded loop with explicit counter
    for line in stdin.lock().lines() {
        iteration_count += 1;
        if iteration_count >= MAX_ITERATIONS {
            error!("Maximum iterations reached, exiting");
            break;
        }
        
        let line = line.context("Failed to read stdin")?;
        debug!("Received: {}", line);
        
        match parse_recorder_command(&line) {
            Ok(RecorderCommand::Start { region, fps, cursor, out }) => {
                info!(
                    "Starting capture: region={}x{}+{}+{}, fps={}, cursor={}, output={}",
                    region.width, region.height, region.x, region.y, fps, cursor, out.display()
                );
                
                // Rule 5: validate inputs
                if let Err(e) = validate_region(&region) {
                    let event = RecorderEvent::Error {
                        kind: ErrorKind::InvalidInput,
                        hint: format!("Invalid region: {}", e),
                    };
                    write_event(&mut stdout, &event)?;
                    continue;
                }
                
                match start_recording(&state, region, fps, cursor, out.clone()) {
                    Ok(()) => {
                        let event = RecorderEvent::Started { pts_ms: 0 };
                        write_event(&mut stdout, &event)?;
                        
                        // Start progress reporter with shared state
                        let progress_state = state.clone();
                        std::thread::spawn(move || {
                            spawn_progress_reporter(progress_state);
                        });
                    }
                    Err(e) => {
                        error!("Failed to start recording: {:#}", e);
                        let event = RecorderEvent::Error {
                            kind: ErrorKind::EncodingFailed,
                            hint: format!("Could not start GStreamer pipeline: {}", e),
                        };
                        write_event(&mut stdout, &event)?;
                    }
                }
            }
            
            Ok(RecorderCommand::Stop) => {
                info!("Stop command received");
                
                match stop_recording(&state) {
                    Ok((duration_ms, path)) => {
                        info!("Recording stopped: duration={}ms, saved to {}", 
                            duration_ms, path.display());
                        let event = RecorderEvent::Stopped { duration_ms, path };
                        write_event(&mut stdout, &event)?;
                    }
                    Err(e) => {
                        error!("Failed to stop recording: {:#}", e);
                        let event = RecorderEvent::Error {
                            kind: ErrorKind::EncodingFailed,
                            hint: format!("Error stopping pipeline: {}", e),
                        };
                        write_event(&mut stdout, &event)?;
                    }
                }
                
                break;
            }
            
            Err(e) => {
                error!("Invalid command: {} (error: {})", line, e);
                let event = RecorderEvent::Error {
                    kind: ErrorKind::InvalidInput,
                    hint: format!("Could not parse command: {}", e),
                };
                write_event(&mut stdout, &event)?;
            }
        }
    }
    
    info!("Recorder shutting down");
    Ok(())
}

fn start_recording(
    state: &Arc<Mutex<RecorderState>>,
    region: CaptureRegion,
    fps: u32,
    cursor: bool,
    output_path: PathBuf,
) -> Result<()> {
    // Rule 7: validate FPS
    let fps = if fps > 0 && fps <= 60 { fps } else { 30 };
    
    // Build pipeline with proper MP4 muxing
    let pipeline_desc = format!(
        "ximagesrc startx={} starty={} endx={} endy={} use-damage=false show-pointer={} ! \
         video/x-raw,framerate={}/1 ! \
         videoconvert ! \
         video/x-raw,format=I420 ! \
         x264enc speed-preset=ultrafast tune=zerolatency ! \
         h264parse ! \
         mp4mux ! \
         filesink location={} name=sink",
        region.x,
        region.y,
        region.x + region.width as i32 - 1,
        region.y + region.height as i32 - 1,
        cursor,
        fps,
        output_path.display()
    );
    
    debug!("GStreamer pipeline: {}", pipeline_desc);
    
    let pipeline = gst::parse::launch(&pipeline_desc)
        .context("Failed to create GStreamer pipeline")?
        .dynamic_cast::<gst::Pipeline>()
        .map_err(|_| anyhow::anyhow!("Pipeline is not a gst::Pipeline"))?;
    
    // Set pipeline to playing state
    pipeline.set_state(gst::State::Playing)
        .context("Failed to set pipeline to PLAYING state")?;
    
    let mut state_guard = state.lock().unwrap();
    state_guard.pipeline = Some(pipeline);
    state_guard.output_path = Some(output_path);
    state_guard.start_time = Some(std::time::Instant::now());
    
    Ok(())
}

fn spawn_progress_reporter(state: Arc<Mutex<RecorderState>>) {
    const MAX_REPORTS: u32 = 7200;  // Rule 2: max 1 hour at 500ms intervals
    
    let mut report_count = 0u32;
    
    // Rule 2: bounded loop
    while report_count < MAX_REPORTS {
        report_count += 1;
        std::thread::sleep(Duration::from_millis(500));
        
        let state_guard = state.lock().unwrap();
        
        // Calculate duration from start time
        let duration_ms = if let Some(start_time) = state_guard.start_time {
            start_time.elapsed().as_millis() as u64
        } else {
            0
        };
        
        // Check if pipeline still exists
        if state_guard.pipeline.is_none() {
            break;
        }
        
        drop(state_guard);
        
        let event = RecorderEvent::Progress {
            pts_ms: duration_ms,
        };
        
        // Rule 7: check write result - write to stdout directly
        if let Ok(json) = to_jsonl(&event) {
            use std::io::Write;
            let mut stdout = io::stdout();
            if stdout.write_all(json.as_bytes()).is_err() {
                break;
            }
            if stdout.flush().is_err() {
                break;
            }
        }
    }
}

fn stop_recording(state: &Arc<Mutex<RecorderState>>) -> Result<(u64, PathBuf)> {
    let mut state_guard = state.lock().unwrap();
    
    let pipeline = state_guard.pipeline.take()
        .context("No active recording to stop")?;
    
    let output_path = state_guard.output_path.take()
        .context("No output path stored")?;
    
    // Calculate final duration
    let duration_ms = if let Some(start_time) = state_guard.start_time {
        start_time.elapsed().as_millis() as u64
    } else {
        0
    };
    
    // Clear start time to stop progress reporter
    state_guard.start_time = None;
    drop(state_guard);
    
    debug!("Sending EOS to finalize MP4 file");
    pipeline.send_event(gst::event::Eos::new());
    
    // Wait for EOS with proper timeout
    if let Some(bus) = pipeline.bus() {
        // Wait up to 5 seconds for EOS
        let timeout = gst::ClockTime::from_seconds(5);
        
        loop {
            if let Some(msg) = bus.timed_pop_filtered(
                timeout,
                &[gst::MessageType::Eos, gst::MessageType::Error]
            ) {
                match msg.view() {
                    gst::MessageView::Eos(_) => {
                        debug!("EOS received, MP4 finalized");
                        break;
                    }
                    gst::MessageView::Error(err) => {
                        error!("Pipeline error: {} ({:?})", err.error(), err.debug());
                        break;
                    }
                    _ => {}
                }
            } else {
                error!("Timeout waiting for EOS");
                break;
            }
        }
    }
    
    // Ensure pipeline is stopped
    pipeline.set_state(gst::State::Null)
        .context("Failed to set pipeline to NULL state")?;
    
    // Give filesystem time to sync
    std::thread::sleep(Duration::from_millis(100));
    
    // Rule 5: validate output
    if duration_ms < 500 {
        error!("Warning: Recording very short ({}ms), file may be corrupt", duration_ms);
    }
    
    Ok((duration_ms, output_path))
}

fn write_event(stdout: &mut io::Stdout, event: &RecorderEvent) -> Result<()> {
    let json = to_jsonl(event).context("Failed to serialize event")?;
    stdout.write_all(json.as_bytes()).context("Failed to write to stdout")?;
    stdout.flush().context("Failed to flush stdout")?;
    Ok(())
}