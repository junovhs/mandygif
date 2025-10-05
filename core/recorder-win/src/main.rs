//! Windows screen recorder using Windows.Graphics.Capture + Media Foundation
//! 
//! TODO: Implement WGC + MF H.264 encoding

use anyhow::Result;
use mandygif_protocol::*;
use std::io::{self, BufRead, Write};
use tracing::{info, error};

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

    info!("recorder-win starting (STUB), protocol v{}", PROTOCOL_VERSION);

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines() {
        let line = line?;
        
        if let Ok(RecorderCommand::Start { .. }) = parse_recorder_command(&line) {
            let err_event = RecorderEvent::Error {
                kind: ErrorKind::UnsupportedPlatform,
                hint: "Windows recorder not yet implemented".into(),
            };
            stdout.write_all(to_jsonl(&err_event)?.as_bytes())?;
            stdout.flush()?;
            break;
        }
    }
    
    Ok(())
}