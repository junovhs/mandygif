#![allow(clippy::cast_possible_truncation)]

use anyhow::Result;
use gstreamer as gst;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Shared state for the recorder.
pub struct RecorderState {
    pub pipeline: Option<gst::Pipeline>,
    pub output_path: Option<PathBuf>,
    pub start_time: Option<Instant>,
}

impl RecorderState {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            pipeline: None,
            output_path: None,
            start_time: None,
        }))
    }

    /// Update state with a running pipeline.
    pub fn set_running(&mut self, pipe: gst::Pipeline, path: PathBuf) {
        self.pipeline = Some(pipe);
        self.output_path = Some(path);
        self.start_time = Some(Instant::now());
    }

    /// Clear state and return previous duration/path.
    pub fn stop(&mut self) -> Result<(u64, PathBuf)> {
        let duration = self
            .start_time
            .map_or(0, |t| u64::try_from(t.elapsed().as_millis()).unwrap_or(0));

        let path = self
            .output_path
            .take()
            .ok_or_else(|| anyhow::anyhow!("No active path"))?;

        self.start_time = None;
        Ok((duration, path))
    }
}
