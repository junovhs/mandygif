//! Cross-platform encoder: GIF, MP4, WebP

#![allow(clippy::wildcard_imports)]

mod ffmpeg;
mod gif;
mod video;

use anyhow::{Context, Result};
use mandygif_protocol::*;
use std::io::{self, BufRead, Write};
use tracing::{error, info};

fn main() -> Result<()> {
    // FIX: Force logs to stderr
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter("info")
        .init();

    info!("encoder starting (protocol v{})", PROTOCOL_VERSION);
    ffmpeg::check_ffmpeg()?;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if let Err(e) = handle_command(&line, &mut stdout) {
            error!("Command failed: {:#}", e);
            send_error(&mut stdout, ErrorKind::EncodingFailed, e.to_string())?;
        }
    }

    Ok(())
}

fn handle_command(line: &str, out: &mut io::Stdout) -> Result<()> {
    match parse_encoder_command(line)? {
        EncoderCommand::Gif {
            input,
            trim,
            fps,
            scale_px,
            loop_mode,
            captions,
            out: path,
        } => {
            gif::encode_gif(&input, &trim, fps, scale_px, &loop_mode, &captions, &path)?;
            send_done(out, path)?;
        }
        EncoderCommand::Mp4 {
            input,
            trim,
            fps,
            scale_px,
            quality,
            captions,
            out: path,
        } => {
            video::encode_mp4(&input, &trim, fps, scale_px, quality, &captions, &path)?;
            send_done(out, path)?;
        }
        EncoderCommand::Webp {
            input,
            trim,
            fps,
            scale_px,
            quality,
            lossless,
            captions,
            out: path,
        } => {
            video::encode_webp(
                &input, &trim, fps, scale_px, quality, lossless, &captions, &path,
            )?;
            send_done(out, path)?;
        }
    }
    Ok(())
}

fn send_done(stdout: &mut io::Stdout, path: std::path::PathBuf) -> Result<()> {
    let event = EncoderEvent::Done { path };
    let json = to_jsonl(&event)?;
    stdout.write_all(json.as_bytes())?;
    stdout.flush()?;
    Ok(())
}

fn send_error(stdout: &mut io::Stdout, kind: ErrorKind, hint: String) -> Result<()> {
    let event = EncoderEvent::Error { kind, hint };
    let json = to_jsonl(&event)?;
    stdout.write_all(json.as_bytes())?;
    stdout.flush().context("Failed to flush stdout")?;
    Ok(())
}
