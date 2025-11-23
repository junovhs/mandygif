#![allow(clippy::wildcard_imports)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::uninlined_format_args)]

use anyhow::Result;
use mandygif_protocol::*;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Spawn recorder process and handle communication.
pub async fn run_recorder(
    tx: mpsc::UnboundedSender<RecorderEvent>,
    stop_rx: &mut mpsc::UnboundedReceiver<()>,
    region: CaptureRegion,
) -> Result<()> {
    let bin = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("recorder-linux");
    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    let cmd = RecorderCommand::Start {
        region,
        fps: 30,
        cursor: false,
        out: PathBuf::from("/tmp/mandygif_recording.mp4"),
    };

    let json = to_jsonl(&cmd)?;
    stdin.write_all(json.as_bytes()).await?;

    loop {
        tokio::select! {
            _ = stop_rx.recv() => {
                let stop_cmd = to_jsonl(&RecorderCommand::Stop)?;
                let _ = stdin.write_all(stop_cmd.as_bytes()).await;
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(l)) => handle_line(&l, &tx),
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }
    }

    child.wait().await?;
    Ok(())
}

fn handle_line(line: &str, tx: &mpsc::UnboundedSender<RecorderEvent>) {
    if !line.starts_with('{') {
        return;
    }

    if let Ok(event) = parse_recorder_event(line) {
        let _ = tx.send(event);
    }
}

/// Spawn encoder process.
pub async fn run_encoder(
    input: PathBuf,
    fmt: &str,
    fps: u32,
    trim: (u64, u64),
    scale: u32,
) -> Result<()> {
    let bin = std::env::current_exe()?.parent().unwrap().join("encoder");
    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let cmd = build_encode_cmd(input, fmt, fps, trim, scale);

    if let Some(mut stdin) = child.stdin.take() {
        let json = to_jsonl(&cmd)?;
        stdin.write_all(json.as_bytes()).await?;
    }

    let stdout = child.stdout.take().unwrap();
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
