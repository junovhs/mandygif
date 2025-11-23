use crate::types::{EncoderCommand, EncoderEvent, RecorderCommand, RecorderEvent};
use serde::Serialize;
use serde_json::Error;

/// Parse a JSONL message into a `RecorderCommand`.
///
/// # Errors
/// Returns `serde_json::Error` if the string is not valid JSON or doesn't match the schema.
pub fn parse_recorder_command(line: &str) -> Result<RecorderCommand, Error> {
    serde_json::from_str(line)
}

/// Parse a JSONL message into a `RecorderEvent`.
///
/// # Errors
/// Returns `serde_json::Error` if the string is not valid JSON or doesn't match the schema.
pub fn parse_recorder_event(line: &str) -> Result<RecorderEvent, Error> {
    serde_json::from_str(line)
}

/// Parse a JSONL message into a `EncoderCommand`.
///
/// # Errors
/// Returns `serde_json::Error` if the string is not valid JSON or doesn't match the schema.
pub fn parse_encoder_command(line: &str) -> Result<EncoderCommand, Error> {
    serde_json::from_str(line)
}

/// Parse a JSONL message into a `EncoderEvent`.
///
/// # Errors
/// Returns `serde_json::Error` if the string is not valid JSON or doesn't match the schema.
pub fn parse_encoder_event(line: &str) -> Result<EncoderEvent, Error> {
    serde_json::from_str(line)
}

/// Serialize a command or event to a JSONL string (with newline).
///
/// # Errors
/// Returns `serde_json::Error` if serialization fails.
pub fn to_jsonl<T: Serialize>(msg: &T) -> Result<String, Error> {
    let mut json = serde_json::to_string(msg)?;
    json.push('\n');
    Ok(json)
}
