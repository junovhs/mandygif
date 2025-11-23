//! macOS screen recorder using `ScreenCaptureKit` + `VideoToolbox`
//!
//! TODO: Implement `SCStream` capture with `VTCompressionSession` encoding

#![allow(clippy::wildcard_imports)]
#![allow(clippy::doc_markdown)]

use anyhow::Result;
#[cfg(target_os = "macos")]
use mandygif_protocol::*;
#[cfg(target_os = "macos")]
use std::io::{self, BufRead, Write};
#[cfg(target_os = "macos")]
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    #[cfg(not(target_os = "macos"))]
    {
        eprintln!("recorder-mac only runs on macOS");
        std::process::exit(1);
    }

    #[cfg(target_os = "macos")]
    {
        info!(
            "recorder-mac starting (STUB), protocol v{}",
            PROTOCOL_VERSION
        );

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;

            if let Ok(RecorderCommand::Start { .. }) = parse_recorder_command(&line) {
                let err_event = RecorderEvent::Error {
                    kind: ErrorKind::UnsupportedPlatform,
                    hint: "macOS recorder not yet implemented".into(),
                };
                let json = to_jsonl(&err_event)?;
                stdout.write_all(json.as_bytes())?;
                stdout.flush()?;
                break;
            }
        }
        Ok(())
    }
}
