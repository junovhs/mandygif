#![allow(clippy::uninlined_format_args)]

use crate::ffmpeg::{build_filter, ms_to_sec};
use anyhow::{bail, Context, Result};
use mandygif_protocol::{Caption, LoopMode, TrimRange};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::{debug, warn};

/// Encode GIF using ffmpeg palettegen.
pub fn encode_gif(
    input: &Path,
    trim: &TrimRange,
    fps: u32,
    scale: Option<u32>,
    loop_mode: &LoopMode,
    caps: &[Caption],
    out: &Path,
) -> Result<()> {
    let temp = tempfile::tempdir().context("Failed to create temp dir")?;
    let palette = temp.path().join("palette.png");

    let filter = build_filter(fps, scale, caps)?;
    let start = ms_to_sec(trim.start_ms);
    let dur = ms_to_sec(trim.end_ms.saturating_sub(trim.start_ms));

    // Step 1: Generate palette
    debug!("Generating palette for GIF");
    let status = Command::new("ffmpeg")
        .args(["-ss", &start, "-t", &dur])
        .arg("-i")
        .arg(input)
        .arg("-vf")
        .arg(format!("{filter},palettegen"))
        .arg("-y")
        .arg(&palette)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .context("Palette generation failed")?;

    if !status.success() {
        bail!("ffmpeg palette generation failed");
    }

    // Step 2: Generate GIF
    debug!("Encoding GIF with palette");
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-ss", &start, "-t", &dur])
        .arg("-i")
        .arg(input)
        .arg("-i")
        .arg(&palette)
        .arg("-lavfi")
        .arg(format!("{filter} [x]; [x][1:v] paletteuse"));

    match loop_mode {
        LoopMode::Once => {
            cmd.arg("-loop").arg("-1");
        }
        _ => {
            cmd.arg("-loop").arg("0");
        }
    }

    if matches!(loop_mode, LoopMode::Pingpong) {
        warn!("Ping-pong loop mode not yet implemented for GIF");
    }

    let status = cmd
        .arg("-y")
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .context("GIF encoding failed")?;

    if !status.success() {
        bail!("ffmpeg GIF encoding failed");
    }

    Ok(())
}
