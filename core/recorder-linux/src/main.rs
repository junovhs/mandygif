//! Linux screen recorder using GStreamer + X11/XWayland capture
//! 
//! Uses ximagesrc for screen capture, x264 encoding to MP4.
//! TODO: Upgrade to PipeWire portal for native Wayland support.

use anyhow::{Context, Result};
use mandygif_protocol::*;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::{info, error, debug};

/// Shared state between command handler and async tasks
struct RecorderState {
    pipeline: Option<gst::Pipeline>,
    output_path: Option<PathBuf>,
}

impl RecorderState {
    fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            pipeline: None,
            output_path: None,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
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
    
    for line in stdin.lock().lines() {
        let line = line.context("Failed to read stdin")?;
        debug!("Received: {}", line);
        
        match parse_recorder_command(&line) {
            Ok(RecorderCommand::Start { region, fps, cursor, out }) => {
                info!(
                    "Starting capture: region={}x{}+{}+{}, fps={}, cursor={}, output={}",
                    region.width, region.height, region.x, region.y, fps, cursor, out.display()
                );
                
                match start_recording(&state, region, fps, out.clone()) {
                    Ok(()) => {
                        let event = RecorderEvent::Started { pts_ms: 0 };
                        write_event(&mut stdout, &event)?;
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
                        info!("Recording stopped: duration={}ms, saved to {}", duration_ms, path.display());
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
    output_path: PathBuf,
) -> Result<()> {
    let pipeline_desc = format!(
        "ximagesrc startx={} starty={} endx={} endy={} use-damage=false ! \
         video/x-raw,framerate={}/1 ! \
         videoconvert ! \
         video/x-raw,format=I420 ! \
         x264enc speed-preset=ultrafast tune=zerolatency bitrate=4000 key-int-max={} pass=qual quantizer=23 ! \
         video/x-h264,profile=baseline ! \
         mp4mux ! \
         filesink location={} name=sink",
        region.x,
        region.y,
        region.x + region.width as i32 - 1,
        region.y + region.height as i32 - 1,
        fps,
        fps * 2,
        output_path.display()
    );
    
    debug!("GStreamer pipeline: {}", pipeline_desc);
    
    let pipeline = gst::parse::launch(&pipeline_desc)
        .context("Failed to create GStreamer pipeline")?
        .dynamic_cast::<gst::Pipeline>()
        .map_err(|_| anyhow::anyhow!("Pipeline is not a gst::Pipeline"))?;
    
    pipeline.set_state(gst::State::Playing)
        .context("Failed to set pipeline to PLAYING state")?;
    
    let mut state_guard = state.lock().unwrap();
    state_guard.pipeline = Some(pipeline.clone());
    state_guard.output_path = Some(output_path);
    drop(state_guard);
    
    // Spawn progress reporter thread (uses sync GStreamer queries)
    let pipe_weak = pipeline.downgrade();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            if let Some(pipe) = pipe_weak.upgrade() {
                if let Some(pos) = pipe.query_position::<gst::ClockTime>() {
                    let event = RecorderEvent::Progress {
                        pts_ms: pos.mseconds(),
                    };
                    if let Ok(json) = to_jsonl(&event) {
                        use std::io::Write;
                        let stdout = std::io::stdout();
                        let mut handle = stdout.lock();
                        let _ = handle.write_all(json.as_bytes());
                        let _ = handle.flush();
                        drop(handle);
                    }
                }
            } else {
                break;
            }
        }
    });

    Ok(())
}

fn stop_recording(state: &Arc<Mutex<RecorderState>>) -> Result<(u64, PathBuf)> {
    let mut state_guard = state.lock().unwrap();
    
    let pipeline = state_guard.pipeline.take()
        .context("No active recording to stop")?;
    
    let output_path = state_guard.output_path.take()
        .context("No output path stored")?;
    
    drop(state_guard); // Release lock before blocking operations
    
    // Query duration before stopping
    let duration_ms = pipeline.query_position::<gst::ClockTime>()
        .map(|t| t.mseconds())
        .unwrap_or(0);
    
    debug!("Sending EOS to finalize MP4 file");
    pipeline.send_event(gst::event::Eos::new());
    
    // Wait for EOS to complete (max 5 seconds)
    if let Some(bus) = pipeline.bus() {
        let timeout = gst::ClockTime::from_seconds(5);
        if let Some(msg) = bus.timed_pop_filtered(timeout, &[gst::MessageType::Eos, gst::MessageType::Error]) {
            match msg.view() {
                gst::MessageView::Error(err) => {
                    error!("Pipeline error during EOS: {} (debug: {:?})", err.error(), err.debug());
                }
                gst::MessageView::Eos(_) => {
                    debug!("EOS processed successfully");
                }
                _ => {}
            }
        }
    }
    
    pipeline.set_state(gst::State::Null)
        .context("Failed to set pipeline to NULL state")?;
    
    Ok((duration_ms, output_path))
}

fn write_event(stdout: &mut io::Stdout, event: &RecorderEvent) -> Result<()> {
    let json = to_jsonl(event).context("Failed to serialize event")?;
    stdout.write_all(json.as_bytes()).context("Failed to write to stdout")?;
    stdout.flush().context("Failed to flush stdout")?;
    Ok(())
}