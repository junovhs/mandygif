#![allow(clippy::wildcard_imports)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::uninlined_format_args)]

use anyhow::{Context, Result};
use mandygif_protocol::*;
use mandygif_recorder_linux::Recorder;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Run recorder directly in this process.
pub async fn run_recorder(
    tx: mpsc::UnboundedSender<RecorderEvent>,
    stop_rx: &mut mpsc::UnboundedReceiver<()>,
    region: CaptureRegion,
) -> Result<()> {
    Recorder::init()?;

    let out_path = PathBuf::from("/tmp/mandygif_recording.mp4");

    // Pass references to match library signature
    let recorder = Recorder::start(&region, 30, false, &out_path)?;

    let _ = tx.send(RecorderEvent::Started { pts_ms: 0 });

    loop {
        tokio::select! {
            _ = stop_rx.recv() => {
                info!("Stop signal received");
                break;
            }
            // FIX: Explicitly match the unit result from sleep
            () = tokio::time::sleep(Duration::from_millis(100)) => {
                let ms = recorder.duration_ms();
                let _ = tx.send(RecorderEvent::Progress { pts_ms: ms });
            }
        }
    }

    let duration = recorder.stop()?;

    let _ = tx.send(RecorderEvent::Stopped {
        duration_ms: duration,
        path: out_path,
    });

    Ok(())
}

pub async fn run_encoder(
    input: PathBuf,
    fmt: &str,
    fps: u32,
    trim: (u64, u64),
    scale: u32,
) -> Result<()> {
    let exe = std::env::current_exe()?;
    let bin_dir = exe
        .parent()
        .context("Failed to determine executable directory")?;
    let bin = bin_dir.join("encoder");

    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn encoder process")?;

    let cmd = build_encode_cmd(input, fmt, fps, trim, scale);

    if let Some(mut stdin) = child.stdin.take() {
        let json = to_jsonl(&cmd)?;
        stdin.write_all(json.as_bytes()).await?;
    }

    let stdout = child
        .stdout
        .take()
        .context("Failed to open encoder stdout")?;
    let mut reader = BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        if let Ok(event) = parse_encoder_event(&line) {
            match event {
                EncoderEvent::Done { path } => info!("Export done: {:?}", path),
                EncoderEvent::Error { hint, .. } => error!("Export error: {}", hint),
                EncoderEvent::Progress { .. } => {}
            }
        }
    }

    child.wait().await?;
    Ok(())
}

fn build_encode_cmd(
    input: PathBuf,
    fmt: &str,
    fps: u32,
    trim: (u64, u64),
    scale: u32,
) -> EncoderCommand {
    let tr = TrimRange {
        start_ms: trim.0,
        end_ms: trim.1,
    };
    let out = PathBuf::from(format!("/tmp/export.{}", fmt));

    match fmt {
        "gif" => EncoderCommand::Gif {
            input,
            trim: tr,
            fps,
            scale_px: Some(scale),
            loop_mode: LoopMode::Normal,
            captions: vec![],
            out,
        },
        "webp" => EncoderCommand::Webp {
            input,
            trim: tr,
            fps,
            scale_px: Some(scale),
            quality: 0.8,
            lossless: false,
            captions: vec![],
            out,
        },
        _ => EncoderCommand::Mp4 {
            input,
            trim: tr,
            fps,
            scale_px: Some(scale),
            quality: 0.8,
            captions: vec![],
            out,
        },
    }
}
