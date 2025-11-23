//! Linux screen recorder using `GStreamer` + X11

#![allow(clippy::wildcard_imports)]

mod pipeline;
mod state;

use anyhow::{Context, Result};
use gstreamer as gst;
use mandygif_protocol::*;
use pipeline::{spawn_reporter, start_pipeline, stop_pipeline};
use state::RecorderState;
use std::io::{self, BufRead, Write};

fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    gst::init()?;

    let state = RecorderState::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if let Err(e) = handle_cmd(&line, &mut stdout, &state) {
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

            start_pipeline(state, &region, fps, cursor, &path)?;

            let event = RecorderEvent::Started { pts_ms: 0 };
            let json = to_jsonl(&event)?;
            out.write_all(json.as_bytes())?;
            out.flush()?;

            spawn_reporter(state.clone());
        }
        RecorderCommand::Stop => {
            let (ms, path) = stop_pipeline(state)?;
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
