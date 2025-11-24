#![allow(clippy::cast_possible_wrap)]

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use mandygif_protocol::CaptureRegion;
use std::path::Path;
use std::time::Instant;
use tracing::{error, info};

pub struct Recorder {
    pipeline: gst::Pipeline,
    start_time: Instant,
}

impl Recorder {
    /// Initialize `GStreamer`.
    ///
    /// # Errors
    /// Returns error if `GStreamer` cannot be initialized.
    pub fn init() -> Result<()> {
        gst::init().context("Failed to initialize GStreamer")
    }

    /// Start recording a region to a file.
    ///
    /// # Errors
    /// Returns error if dimensions are invalid or pipeline cannot be constructed/started.
    pub fn start(region: &CaptureRegion, fps: u32, cursor: bool, out: &Path) -> Result<Self> {
        if region.width == 0 || region.height == 0 {
            return Err(anyhow::anyhow!("Invalid dimensions"));
        }

        // Note: qtmux needs to be sent EOS to write the moov atom (file footer)
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

        info!("Pipeline: {}", desc);

        let pipeline = gst::parse::launch(&desc)?
            .dynamic_cast::<gst::Pipeline>()
            .map_err(|_| anyhow::anyhow!("Not a pipeline"))?;

        pipeline.set_state(gst::State::Playing)?;

        Ok(Self {
            pipeline,
            start_time: Instant::now(),
        })
    }

    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        u64::try_from(self.start_time.elapsed().as_millis()).unwrap_or(0)
    }

    /// Stop recording and finalize file.
    ///
    /// # Errors
    /// Returns error if pipeline state change fails.
    pub fn stop(self) -> Result<u64> {
        info!("Sending EOS to pipeline...");
        let duration = self.duration_ms();

        // 1. Send EOS event to the pipeline. This tells qtmux to finish the file.
        let eos_sent = self.pipeline.send_event(gst::event::Eos::new());
        if !eos_sent {
            error!("Failed to send EOS event");
        }

        // 2. Wait for the EOS message on the bus.
        // This is CRITICAL. If we stop before this, the MP4 is corrupt (no moov atom).
        if let Some(bus) = self.pipeline.bus() {
            info!("Waiting for EOS...");
            let msg = bus.timed_pop_filtered(
                gst::ClockTime::from_seconds(5), // Wait up to 5s
                &[gst::MessageType::Eos, gst::MessageType::Error],
            );

            if let Some(msg) = msg {
                match msg.view() {
                    gst::MessageView::Eos(_) => info!("EOS received, file finalized."),
                    gst::MessageView::Error(err) => {
                        error!("Pipeline error during stop: {}", err.error());
                    }
                    _ => (),
                }
            } else {
                error!("Timed out waiting for EOS - file might be corrupt");
            }
        }

        // 3. Now it is safe to set NULL
        self.pipeline.set_state(gst::State::Null)?;

        Ok(duration)
    }
}
