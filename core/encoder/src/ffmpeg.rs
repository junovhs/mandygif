#![allow(clippy::cast_precision_loss)]
#![allow(clippy::uninlined_format_args)]

use anyhow::{bail, Context, Result};
use mandygif_captions::chain_filters_expr;
use mandygif_protocol::Caption;
use std::process::Command;
use tracing::info;

/// Check if ffmpeg is available.
pub fn check_ffmpeg() -> Result<()> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .context("ffmpeg not found - please install ffmpeg")?;

    if !output.status.success() {
        bail!("ffmpeg exists but returned error");
    }

    let version = String::from_utf8_lossy(&output.stdout);
    let first_line = version.lines().next().unwrap_or("unknown");
    info!("Using {}", first_line);
    Ok(())
}

/// Build ffmpeg video filter string (fps, scale, captions).
///
/// # Errors
/// Returns error if caption filter generation fails.
pub fn build_filter(fps: u32, scale: Option<u32>, caps: &[Caption]) -> Result<String> {
    let mut filters = vec![format!("fps={}", fps)];

    if let Some(width) = scale {
        filters.push(format!("scale={width}:-1:flags=lanczos"));
    }

    // Add caption filters after scaling
    if !caps.is_empty() {
        filters.push(chain_filters_expr(caps)?);
    }

    Ok(filters.join(","))
}

/// Convert milliseconds to seconds string.
#[must_use]
pub fn ms_to_sec(ms: u64) -> String {
    format!("{:.3}", (ms as f64) / 1000.0)
}
