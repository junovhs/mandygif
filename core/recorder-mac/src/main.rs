//! macOS screen recorder using ScreenCaptureKit + VideoToolbox
//! 
//! TODO: Implement SCStream capture with VTCompressionSession encoding

use anyhow::Result;
use mandygif_protocol::*;
use std::io::{self, BufRead, Write};
use tracing::{info, error};

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

    info!("recorder-mac starting (STUB), protocol v{}", PROTOCOL_VERSION);

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        let line = line?;
        
        if let Ok(RecorderCommand::Start { .. }) = parse_recorder_command(&line) {
            let err_event = RecorderEvent::Error {
                kind: ErrorKind::UnsupportedPlatform,
                hint: "macOS recorder not yet implemented".into(),
            };
            stdout.write_all(to_jsonl(&err_event)?.as_bytes())?;
            stdout.flush()?;
            break;
        }
    }
    
    Ok(())
}