#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use crate::ffmpeg::{build_filter, ms_to_sec};
use anyhow::{bail, Context, Result};
use mandygif_protocol::{Caption, TrimRange};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::debug;

/// Encode MP4 using ffmpeg.
pub fn encode_mp4(
    input: &Path,
    trim: &TrimRange,
    fps: u32,
    scale: Option<u32>,
    qual: f32,
    caps: &[Caption],
    out: &Path,
) -> Result<()> {
    let crf = (51.0 - (qual * 33.0)).round() as u32;
    let start = ms_to_sec(trim.start_ms);
    let dur = ms_to_sec(trim.end_ms.saturating_sub(trim.start_ms));
    let filter = build_filter(fps, scale, caps)?;

    debug!("Encoding MP4 (CRF {})", crf);
    let status = Command::new("ffmpeg")
        .args(["-ss", &start, "-t", &dur])
        .arg("-i")
        .arg(input)
        .arg("-vf")
        .arg(filter)
        .args(["-c:v", "libx264", "-preset", "medium"])
        .arg("-crf")
        .arg(crf.to_string())
        .args(["-pix_fmt", "yuv420p", "-movflags", "+faststart"])
        .arg("-y")
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .context("ffmpeg MP4 encoding failed")?;

    if !status.success() {
        bail!("ffmpeg MP4 encoding failed");
    }

    Ok(())
}

/// Encode WebP using ffmpeg.
#[allow(clippy::too_many_arguments)]
pub fn encode_webp(
    input: &Path,
    trim: &TrimRange,
    fps: u32,
    scale: Option<u32>,
    qual: f32,
    lossless: bool,
    caps: &[Caption],
    out: &Path,
) -> Result<()> {
    let start = ms_to_sec(trim.start_ms);
    let dur = ms_to_sec(trim.end_ms.saturating_sub(trim.start_ms));
    let filter = build_filter(fps, scale, caps)?;

    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-ss", &start, "-t", &dur])
        .arg("-i")
        .arg(input)
        .arg("-vf")
        .arg(filter);

    if lossless {
        cmd.args(["-lossless", "1"]);
    } else {
        cmd.arg("-quality").arg((qual * 100.0).round().to_string());
    }

    debug!("Encoding WebP");
    let status = cmd
        .args(["-loop", "0", "-y"])
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .context("ffmpeg WebP encoding failed")?;

    if !status.success() {
        bail!("ffmpeg WebP encoding failed");
    }

    Ok(())
}
