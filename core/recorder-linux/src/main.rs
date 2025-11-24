//! Linux screen recorder using `GStreamer` + X11

#![allow(clippy::wildcard_imports)]

mod pipeline;
mod state;

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*; // Import traits for bus handling
use mandygif_protocol::*;
use pipeline::{spawn_reporter, start_pipeline, stop_pipeline};
use state::RecorderState;
use std::io::{self, BufRead, Write};
use tracing::{error, info};

fn main() -> Result<()> {
    // FIX: Force logs to stderr so they don't break the JSON protocol on stdout
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter("info")
        .init();

    gst::init()?;

    let state = RecorderState::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Spawn a thread to monitor the pipeline for async errors (like X11 crashes)
    let state_monitor = state.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let guard = state_monitor.lock().unwrap();
            if let Some(pipeline) = &guard.pipeline {
                if let Some(bus) = pipeline.bus() {
                    // Check for messages without blocking
                    while let Some(msg) = bus.pop() {
                        if let gst::MessageView::Error(err) = msg.view() {
                            error!("GStreamer Async Error: {} ({:?})", err.error(), err.debug());
                            // We can't easily write to stdout here safely due to locking,
                            // but the error log will show up in the UI console now.
                        }
                    }
                }
            }
        }
    });

    for line in stdin.lock().lines() {
        let line = line?;
        if let Err(e) = handle_cmd(&line, &mut stdout, &state) {
            error!("Command handler error: {}", e);
            send_error(&mut stdout, ErrorKind::IoError, e.to_string())?;
        }
    }
    Ok(())
}

fn handle_cmd(
    line: &str,
    out: &mut io::Stdout,
    state: &std::sync::Arc<std::sync::Mutex<RecorderState>>,
) -> Result<()> {
    match parse_recorder_command(line)? {
        RecorderCommand::Start {
            region,
            fps,
            cursor,
            out: path,
        } => {
            if region.width == 0 || region.height == 0 {
                return send_error(out, ErrorKind::InvalidInput, "Bad dimensions".into());
            }

            info!("Starting recording to {:?}", path);
            start_pipeline(state, &region, fps, cursor, &path)?;

            let event = RecorderEvent::Started { pts_ms: 0 };
            let json = to_jsonl(&event)?;
            out.write_all(json.as_bytes())?;
            out.flush()?;

            spawn_reporter(state.clone());
        }
        RecorderCommand::Stop => {
            info!("Stopping recording...");
            let (ms, path) = stop_pipeline(state)?;
            info!("Stopped. Duration: {}ms", ms);

            let event = RecorderEvent::Stopped {
                duration_ms: ms,
                path,
            };
            let json = to_jsonl(&event)?;
            out.write_all(json.as_bytes())?;
            out.flush()?;
        }
    }
    Ok(())
}

fn send_error(out: &mut io::Stdout, kind: ErrorKind, hint: String) -> Result<()> {
    let event = RecorderEvent::Error { kind, hint };
    let json = to_jsonl(&event)?;
    out.write_all(json.as_bytes())?;
    out.flush().context("Flush failed")?;
    Ok(())
}
