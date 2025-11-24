#![allow(clippy::cast_possible_wrap)] // GStreamer expects i32 for coordinates

use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use mandygif_protocol::CaptureRegion;
use std::path::Path;
use std::time::Instant;
use tracing::info;

/// The handle for the in-process recorder
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
        // Validate dimensions to prevent GStreamer crashes
        if region.width == 0 || region.height == 0 {
            return Err(anyhow::anyhow!("Invalid dimensions"));
        }

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

    /// Get current recording duration in milliseconds.
    #[must_use]
    pub fn duration_ms(&self) -> u64 {
        u64::try_from(self.start_time.elapsed().as_millis()).unwrap_or(0)
    }

    /// Stop recording and finalize file.
    ///
    /// # Errors
    /// Returns error if pipeline state change fails.
    pub fn stop(self) -> Result<u64> {
        info!("Stopping pipeline...");
        let duration = self.duration_ms();

        // Send EOS to properly close the MP4 container
        self.pipeline.send_event(gst::event::Eos::new());

        if let Some(bus) = self.pipeline.bus() {
            // Wait briefly for EOS to process
            let _ = bus.timed_pop_filtered(
                gst::ClockTime::from_seconds(2),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            );
        }

        self.pipeline.set_state(gst::State::Null)?;
        Ok(duration)
    }
}
