#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]

use crate::state::RecorderState;
use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use mandygif_protocol::{to_jsonl, CaptureRegion, RecorderEvent};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::debug;

pub fn start_pipeline(
    state: &Arc<Mutex<RecorderState>>,
    region: &CaptureRegion,
    fps: u32,
    cursor: bool,
    out: &Path,
) -> Result<()> {
    let desc = format!(
        "ximagesrc startx={} starty={} endx={} endy={} use-damage=false show-pointer={} ! \
         video/x-raw,framerate={}/1 ! \
         videoconvert ! \
         video/x-raw,format=I420 ! \
         x264enc speed-preset=ultrafast tune=zerolatency ! \
         h264parse ! \
         qtmux name=mux ! \
         filesink location={} sync=false",
        region.x,
        region.y,
        region.x + region.width as i32 - 1,
        region.y + region.height as i32 - 1,
        cursor,
        fps,
        out.display()
    );

    let pipeline = gst::parse::launch(&desc)?
        .dynamic_cast::<gst::Pipeline>()
        .map_err(|_| anyhow::anyhow!("Not a pipeline"))?;

    pipeline.set_state(gst::State::Playing)?;

    state
        .lock()
        .map_err(|_| anyhow::anyhow!("Lock poisoned"))?
        .set_running(pipeline, out.to_path_buf());

    Ok(())
}

pub fn stop_pipeline(state: &Arc<Mutex<RecorderState>>) -> Result<(u64, PathBuf)> {
    let mut guard = state.lock().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
    let pipe = guard.pipeline.take().context("No active recording")?;
    let (dur, path) = guard.stop()?;
    drop(guard);

    debug!("Sending EOS");
    pipe.send_event(gst::event::Eos::new());

    if let Some(bus) = pipe.bus() {
        let _ = bus.timed_pop_filtered(
            gst::ClockTime::from_seconds(5),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        );
    }

    pipe.set_state(gst::State::Null)?;
    std::thread::sleep(Duration::from_millis(100));

    Ok((dur, path))
}

pub fn spawn_reporter(state: Arc<Mutex<RecorderState>>) {
    std::thread::spawn(move || {
        let mut count = 0;
        while count < 7200 {
            // 1 hour max
            std::thread::sleep(Duration::from_millis(500));
            count += 1;

            if let Ok(guard) = state.lock() {
                if guard.pipeline.is_none() {
                    break;
                }

                let ms = guard
                    .start_time
                    .map_or(0, |t| u64::try_from(t.elapsed().as_millis()).unwrap_or(0));

                if let Ok(json) = to_jsonl(&RecorderEvent::Progress { pts_ms: ms }) {
                    let _ = std::io::Write::write_all(&mut std::io::stdout(), json.as_bytes());
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            } else {
                break;
            }
        }
    });
}
