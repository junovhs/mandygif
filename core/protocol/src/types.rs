use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum RecorderEvent {
    Started { pts_ms: u64 },
    Progress { pts_ms: u64 },
    Stopped { duration_ms: u64, path: PathBuf },
    Error { kind: ErrorKind, hint: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

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
        quality: f32,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event", rename_all = "lowercase")]
pub enum EncoderEvent {
    Progress { percent: u32 },
    Done { path: PathBuf },
    Error { kind: ErrorKind, hint: String },
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
    pub color: String,
    pub stroke: String,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CaptionRect {
    pub x: f32,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    PermissionDenied,
    InvalidInput,
    EncodingFailed,
    IoError,
    UnsupportedPlatform,
}
