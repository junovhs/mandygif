//! JSONL protocol for IPC between UI, recorder, and encoder processes.
//!
//! Version: 1
//! All messages are newline-delimited JSON for easy parsing and logging.

mod parsing;
mod types;

pub use parsing::*;
pub use types::*;

/// Protocol version - increment when breaking changes occur
pub const PROTOCOL_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_start_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let cmd = RecorderCommand::Start {
            region: CaptureRegion {
                x: 128,
                y: 96,
                width: 640,
                height: 360,
            },
            fps: 30,
            cursor: true,
            out: PathBuf::from("/tmp/clip.mp4"),
        };

        let jsonl = to_jsonl(&cmd)?;
        let parsed = parse_recorder_command(&jsonl)?;
        assert_eq!(cmd, parsed);
        Ok(())
    }

    #[test]
    fn test_gif_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let cmd = EncoderCommand::Gif {
            input: PathBuf::from("/tmp/clip.mp4"),
            trim: TrimRange {
                start_ms: 200,
                end_ms: 5200,
            },
            fps: 15,
            scale_px: Some(480),
            loop_mode: LoopMode::Pingpong,
            captions: vec![],
            out: PathBuf::from("/tmp/out.gif"),
        };

        let jsonl = to_jsonl(&cmd)?;
        let parsed = parse_encoder_command(&jsonl)?;
        assert_eq!(cmd, parsed);
        Ok(())
    }
}
