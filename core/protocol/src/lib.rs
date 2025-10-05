//! JSONL protocol for IPC between UI, recorder, and encoder processes.
//! 
//! Version: 1
//! All messages are newline-delimited JSON for easy parsing and logging.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Protocol version - increment when breaking changes occur
pub const PROTOCOL_VERSION: u32 = 1;

// ============================================================================
// RECORDER PROTOCOL
// ============================================================================

/// Commands sent to recorder process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum RecorderCommand {
    Start {
        region: CaptureRegion,
        fps: u32,
        #[serde(default)]
        cursor: bool,
        out: PathBuf,
    },
    Stop,
}

/// Events emitted by recorder process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum RecorderEvent {
    Started {
        pts_ms: u64,
    },
    Progress {
        pts_ms: u64,
    },
    Stopped {
        duration_ms: u64,
        path: PathBuf,
    },
    Error {
        kind: ErrorKind,
        hint: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// ============================================================================
// ENCODER PROTOCOL
// ============================================================================

/// Commands sent to encoder process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "cmd", rename_all = "lowercase")]
pub enum EncoderCommand {
    Gif {
        #[serde(rename = "in")]
        input: PathBuf,
        trim: TrimRange,
        fps: u32,
        scale_px: Option<u32>,
        #[serde(rename = "loop")]
        loop_mode: LoopMode,
        captions: Vec<Caption>,
        out: PathBuf,
    },
    Mp4 {
        #[serde(rename = "in")]
        input: PathBuf,
        trim: TrimRange,
        fps: u32,
        scale_px: Option<u32>,
        quality: f32, // 0.0 - 1.0, maps to CRF
        captions: Vec<Caption>,
        out: PathBuf,
    },
    Webp {
        #[serde(rename = "in")]
        input: PathBuf,
        trim: TrimRange,
        fps: u32,
        scale_px: Option<u32>,
        quality: f32,
        lossless: bool,
        captions: Vec<Caption>,
        out: PathBuf,
    },
}

/// Events emitted by encoder process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum EncoderEvent {
    Progress {
        percent: u32, // 0-100
    },
    Done {
        path: PathBuf,
    },
    Error {
        kind: ErrorKind,
        hint: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrimRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LoopMode {
    Normal,
    Pingpong,
    Once,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Caption {
    pub text: String,
    pub font: String,
    pub style: CaptionStyle,
    pub rect: CaptionRect,
    pub start_ms: u64,
    pub end_ms: u64,
    #[serde(default)]
    pub animation: CaptionAnimation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptionStyle {
    pub color: String,      // hex: "#ffffff"
    pub stroke: String,     // hex with alpha: "#0008"
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptionRect {
    pub x: f32,      // 0.0 - 1.0 (proportional)
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptionAnimation {
    #[default]
    None,
    Fade,
}

// ============================================================================
// SHARED TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    PermissionDenied,
    InvalidInput,
    EncodingFailed,
    IoError,
    UnsupportedPlatform,
}

// ============================================================================
// PARSING HELPERS
// ============================================================================

/// Parse a JSONL message into a typed command/event
pub fn parse_recorder_command(line: &str) -> Result<RecorderCommand, serde_json::Error> {
    serde_json::from_str(line)
}

pub fn parse_recorder_event(line: &str) -> Result<RecorderEvent, serde_json::Error> {
    serde_json::from_str(line)
}

pub fn parse_encoder_command(line: &str) -> Result<EncoderCommand, serde_json::Error> {
    serde_json::from_str(line)
}

pub fn parse_encoder_event(line: &str) -> Result<EncoderEvent, serde_json::Error> {
    serde_json::from_str(line)
}

/// Serialize a command/event to JSONL format
pub fn to_jsonl<T: Serialize>(msg: &T) -> Result<String, serde_json::Error> {
    let mut json = serde_json::to_string(msg)?;
    json.push('\n');
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_start_roundtrip() {
        let cmd = RecorderCommand::Start {
            region: CaptureRegion { x: 128, y: 96, width: 640, height: 360 },
            fps: 30,
            cursor: true,
            out: PathBuf::from("/tmp/clip.mp4"),
        };
        
        let jsonl = to_jsonl(&cmd).unwrap();
        let parsed = parse_recorder_command(&jsonl).unwrap();
        assert_eq!(cmd, parsed);
    }

    #[test]
    fn test_encoder_gif_roundtrip() {
        let cmd = EncoderCommand::Gif {
            input: PathBuf::from("/tmp/clip.mp4"),
            trim: TrimRange { start_ms: 200, end_ms: 5200 },
            fps: 15,
            scale_px: Some(480),
            loop_mode: LoopMode::Pingpong,
            captions: vec![],
            out: PathBuf::from("/tmp/out.gif"),
        };
        
        let jsonl = to_jsonl(&cmd).unwrap();
        let parsed = parse_encoder_command(&jsonl).unwrap();
        assert_eq!(cmd, parsed);
    }
}