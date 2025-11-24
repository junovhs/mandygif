#![allow(clippy::wildcard_imports)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::uninlined_format_args)]

use anyhow::{Context, Result};
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
    let exe = std::env::current_exe()?;
    let bin_dir = exe
        .parent()
        .context("Failed to determine executable directory")?;
    let bin = bin_dir.join("recorder-linux");

    let mut child = Command::new(&bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped()) // Capture stderr
        .spawn()
        .context("Failed to spawn recorder process")?;

    let mut stdin = child
        .stdin
        .take()
        .context("Failed to open recorder stdin")?;
    let stdout = child
        .stdout
        .take()
        .context("Failed to open recorder stdout")?;
    let stderr = child
        .stderr
        .take()
        .context("Failed to open recorder stderr")?;

    let mut reader = BufReader::new(stdout).lines();
    let mut err_reader = BufReader::new(stderr).lines();

    // Spawn a task to log stderr so we don't miss GStreamer errors
    tokio::spawn(async move {
        while let Ok(Some(line)) = err_reader.next_line().await {
            error!("[recorder-linux stderr] {}", line);
        }
    });

    let cmd = RecorderCommand::Start {
        region,
        fps: 30,
        cursor: false,
        out: PathBuf::from("/tmp/mandygif_recording.mp4"),
    };

    let json = to_jsonl(&cmd)?;
    stdin.write_all(json.as_bytes()).await?;

    let mut exit_reason = None;

    loop {
        tokio::select! {
            msg = stop_rx.recv() => {
                match msg {
                    Some(()) => {
                        let stop_cmd = to_jsonl(&RecorderCommand::Stop)?;
                        // If write fails, child is likely dead
                        if let Err(e) = stdin.write_all(stop_cmd.as_bytes()).await {
                            error!("Failed to send stop command: {}", e);
                            break;
                        }
                    }
                    None => break, // Channel closed
                }
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        if handle_line(&l, &tx) {
                            exit_reason = Some("normal_stop");
                            break;
                        }
                    },
                    Ok(None) => {
                        error!("Recorder stdout closed unexpectedly");
                        break;
                    },
                    Err(e) => {
                        error!("Error reading recorder output: {}", e);
                        break;
                    },
                }
            }
        }
    }

    // Safety Net: If we broke the loop without a proper stop event, tell the UI
    if exit_reason.is_none() {
        let _ = tx.send(RecorderEvent::Error {
            kind: ErrorKind::IoError,
            hint: "Recorder process died unexpectedly. Check logs.".into(),
        });
    }

    drop(stdin);
    let _ = child.wait().await;
    Ok(())
}

/// Returns true if the loop should exit (Stopped or Error event)
fn handle_line(line: &str, tx: &mpsc::UnboundedSender<RecorderEvent>) -> bool {
    if !line.starts_with('{') {
        // Log non-JSON output (debugging)
        info!("[recorder] {}", line);
        return false;
    }

    if let Ok(event) = parse_recorder_event(line) {
        let should_exit = matches!(
            event,
            RecorderEvent::Stopped { .. } | RecorderEvent::Error { .. }
        );
        let _ = tx.send(event);
        return should_exit;
    }
    false
}

// ... run_encoder (unchanged from previous valid state) ...
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
