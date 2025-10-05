//! Cross-platform encoder: GIF, MP4, WebP
//! 
//! GIF: gifski (best quality dithering)
//! MP4: ffmpeg (hardware encode when available, x264 fallback)
//! WebP: ffmpeg (lossy + lossless support)

use anyhow::{Context, Result, bail};
use mandygif_protocol::*;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{info, error, debug, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    info!("encoder starting (protocol v{})", PROTOCOL_VERSION);
    
    // Check ffmpeg availability
    check_ffmpeg()?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        let line = line?;
        debug!("Received: {}", line);
        
        match parse_encoder_command(&line) {
            Ok(EncoderCommand::Gif { input, trim, fps, scale_px, loop_mode, captions, out }) => {
                info!("Encoding GIF: {} -> {} (fps={}, scale={:?})", 
                    input.display(), out.display(), fps, scale_px);
                
                match encode_gif(&input, &trim, fps, scale_px, &loop_mode, &captions, &out).await {
                    Ok(()) => {
                        info!("GIF encoded successfully: {}", out.display());
                        let event = EncoderEvent::Done { path: out };
                        write_event(&mut stdout, &event)?;
                    }
                    Err(e) => {
                        error!("GIF encoding failed: {:#}", e);
                        let event = EncoderEvent::Error {
                            kind: ErrorKind::EncodingFailed,
                            hint: e.to_string(),
                        };
                        write_event(&mut stdout, &event)?;
                    }
                }
            }
            
            Ok(EncoderCommand::Mp4 { input, trim, fps, scale_px, quality, captions, out }) => {
                info!("Encoding MP4: {} -> {} (fps={}, quality={}, scale={:?})", 
                    input.display(), out.display(), fps, quality, scale_px);
                
                match encode_mp4(&input, &trim, fps, scale_px, quality, &captions, &out).await {
                    Ok(()) => {
                        info!("MP4 encoded successfully: {}", out.display());
                        let event = EncoderEvent::Done { path: out };
                        write_event(&mut stdout, &event)?;
                    }
                    Err(e) => {
                        error!("MP4 encoding failed: {:#}", e);
                        let event = EncoderEvent::Error {
                            kind: ErrorKind::EncodingFailed,
                            hint: e.to_string(),
                        };
                        write_event(&mut stdout, &event)?;
                    }
                }
            }
            
            Ok(EncoderCommand::Webp { input, trim, fps, scale_px, quality, lossless, captions, out }) => {
                info!("Encoding WebP: {} -> {} (fps={}, quality={}, lossless={}, scale={:?})", 
                    input.display(), out.display(), fps, quality, lossless, scale_px);
                
                match encode_webp(&input, &trim, fps, scale_px, quality, lossless, &captions, &out).await {
                    Ok(()) => {
                        info!("WebP encoded successfully: {}", out.display());
                        let event = EncoderEvent::Done { path: out };
                        write_event(&mut stdout, &event)?;
                    }
                    Err(e) => {
                        error!("WebP encoding failed: {:#}", e);
                        let event = EncoderEvent::Error {
                            kind: ErrorKind::EncodingFailed,
                            hint: e.to_string(),
                        };
                        write_event(&mut stdout, &event)?;
                    }
                }
            }
            
            Err(e) => {
                error!("Invalid command: {}", e);
                let event = EncoderEvent::Error {
                    kind: ErrorKind::InvalidInput,
                    hint: format!("Could not parse command: {}", e),
                };
                write_event(&mut stdout, &event)?;
            }
        }
    }
    
    info!("Encoder shutting down");
    Ok(())
}

/// Check if ffmpeg is available on the system
fn check_ffmpeg() -> Result<()> {
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

/// Encode GIF using ffmpeg palettegen (faster, reliable)
async fn encode_gif(
    input: &Path,
    trim: &TrimRange,
    fps: u32,
    scale_px: Option<u32>,
    loop_mode: &LoopMode,
    captions: &[Caption],
    output: &Path,
) -> Result<()> {
    let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
    let palette_path = temp_dir.path().join("palette.png");
    
    let video_filter = build_video_filter(fps, scale_px, captions);
    
    // Step 1: Generate palette
    debug!("Generating palette for GIF");
    let mut palette_cmd = Command::new("ffmpeg");
    palette_cmd
        .arg("-i").arg(input)
        .arg("-ss").arg(format!("{}ms", trim.start_ms))
        .arg("-to").arg(format!("{}ms", trim.end_ms))
        .arg("-vf").arg(format!("{},palettegen", video_filter))
        .arg("-y")
        .arg(&palette_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    
    let palette_output = palette_cmd.output().context("Palette generation failed")?;
    if !palette_output.status.success() {
        bail!("ffmpeg palette generation failed: {}", String::from_utf8_lossy(&palette_output.stderr));
    }
    
    // Step 2: Generate GIF with palette
    debug!("Encoding GIF with palette");
    let mut gif_cmd = Command::new("ffmpeg");
    gif_cmd
        .arg("-i").arg(input)
        .arg("-i").arg(&palette_path)
        .arg("-ss").arg(format!("{}ms", trim.start_ms))
        .arg("-to").arg(format!("{}ms", trim.end_ms))
        .arg("-lavfi").arg(format!("{} [x]; [x][1:v] paletteuse", video_filter));
    
    // Handle loop mode
    match loop_mode {
        LoopMode::Once => gif_cmd.arg("-loop").arg("1"),
        _ => gif_cmd.arg("-loop").arg("0"),
    };
    
    gif_cmd
        .arg("-y")
        .arg(output)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    
    let gif_output = gif_cmd.output().context("GIF encoding failed")?;
    if !gif_output.status.success() {
        bail!("ffmpeg GIF encoding failed: {}", String::from_utf8_lossy(&gif_output.stderr));
    }
    
    // TODO: Handle ping-pong loop mode (needs frame reversal)
    if matches!(loop_mode, LoopMode::Pingpong) {
        warn!("Ping-pong loop mode not yet implemented for GIF");
    }
    
    Ok(())
}
/// Encode MP4 using ffmpeg (with quality mapping to CRF)
async fn encode_mp4(
    input: &Path,
    trim: &TrimRange,
    fps: u32,
    scale_px: Option<u32>,
    quality: f32,
    captions: &[Caption],
    output: &Path,
) -> Result<()> {
    // Map quality (0.0-1.0) to CRF (51-18, lower is better)
    let crf = (51.0 - (quality * 33.0)).round() as u32;
    
    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg
        .arg("-i").arg(input)
        .arg("-ss").arg(format!("{}ms", trim.start_ms))
        .arg("-to").arg(format!("{}ms", trim.end_ms))
        .arg("-vf").arg(build_video_filter(fps, scale_px, captions))
        .arg("-c:v").arg("libx264")
        .arg("-preset").arg("medium")
        .arg("-crf").arg(crf.to_string())
        .arg("-pix_fmt").arg("yuv420p")
        .arg("-movflags").arg("+faststart")
        .arg("-y")
        .arg(output)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    
    debug!("Running: {:?}", ffmpeg);
    let output_result = ffmpeg.output().context("ffmpeg MP4 encoding failed")?;
    
    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        bail!("ffmpeg MP4 encoding failed: {}", stderr);
    }
    
    Ok(())
}

/// Encode WebP using ffmpeg
async fn encode_webp(
    input: &Path,
    trim: &TrimRange,
    fps: u32,
    scale_px: Option<u32>,
    quality: f32,
    lossless: bool,
    captions: &[Caption],
    output: &Path,
) -> Result<()> {
    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg
        .arg("-i").arg(input)
        .arg("-ss").arg(format!("{}ms", trim.start_ms))
        .arg("-to").arg(format!("{}ms", trim.end_ms))
        .arg("-vf").arg(build_video_filter(fps, scale_px, captions));
    
    if lossless {
        ffmpeg.arg("-lossless").arg("1");
    } else {
        ffmpeg.arg("-quality").arg((quality * 100.0).round().to_string());
    }
    
    ffmpeg
        .arg("-loop").arg("0") // Infinite loop
        .arg("-y")
        .arg(output)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    
    debug!("Running: {:?}", ffmpeg);
    let output_result = ffmpeg.output().context("ffmpeg WebP encoding failed")?;
    
    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        bail!("ffmpeg WebP encoding failed: {}", stderr);
    }
    
    Ok(())
}

/// Build ffmpeg video filter string (fps, scale, captions)
fn build_video_filter(fps: u32, scale_px: Option<u32>, captions: &[Caption]) -> String {
    let mut filters = vec![format!("fps={}", fps)];
    
    if let Some(width) = scale_px {
        filters.push(format!("scale={}:-1:flags=lanczos", width));
    }
    
    // Add caption filters (Phase 1: drawtext)
    if !captions.is_empty() {
        warn!("Caption rendering not yet implemented - captions will be ignored");
        // TODO: Use mandygif-captions to generate drawtext filters
    }
    
    filters.join(",")
}

fn write_event(stdout: &mut io::Stdout, event: &EncoderEvent) -> Result<()> {
    let json = to_jsonl(event).context("Failed to serialize event")?;
    stdout.write_all(json.as_bytes()).context("Failed to write to stdout")?;
    stdout.flush().context("Failed to flush stdout")?;
    Ok(())
}