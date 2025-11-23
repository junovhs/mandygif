//! Windows screen recorder using `Windows.Graphics.Capture` + Media Foundation
//!
//! TODO: Implement WGC + MF H.264 encoding

#![allow(clippy::wildcard_imports)]

use anyhow::Result;
#[cfg(target_os = "windows")]
use mandygif_protocol::*;
#[cfg(target_os = "windows")]
use std::io::{self, BufRead, Write};
#[cfg(target_os = "windows")]
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("recorder-win only runs on Windows");
        std::process::exit(1);
    }

    #[cfg(target_os = "windows")]
    {
        info!(
            "recorder-win starting (STUB), protocol v{}",
            PROTOCOL_VERSION
        );

        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;

            if let Ok(RecorderCommand::Start { .. }) = parse_recorder_command(&line) {
                let err_event = RecorderEvent::Error {
                    kind: ErrorKind::UnsupportedPlatform,
                    hint: "Windows recorder not yet implemented".into(),
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
