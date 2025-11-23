use mandygif_protocol::*;
use std::path::PathBuf;

#[test]
fn test_recorder_command() {
    let cmd = RecorderCommand::Start {
        region: CaptureRegion {
            x: 10,
            y: 20,
            width: 800,
            height: 600,
        },
        fps: 60,
        cursor: true,
        out: PathBuf::from("/tmp/test.mp4"),
    };

    let json = to_jsonl(&cmd).expect("serialization failed");
    let parsed = parse_recorder_command(&json).expect("parse failed");

    assert_eq!(cmd, parsed);
}

#[test]
fn test_encoder_command() {
    let cmd = EncoderCommand::Gif {
        input: PathBuf::from("in.mp4"),
        trim: TrimRange {
            start_ms: 0,
            end_ms: 1000,
        },
        fps: 15,
        scale_px: Some(320),
        loop_mode: LoopMode::Normal,
        captions: vec![],
        out: PathBuf::from("out.gif"),
    };

    let json = to_jsonl(&cmd).expect("serialization failed");
    let parsed = parse_encoder_command(&json).expect("parse failed");

    if let EncoderCommand::Gif {
        input,
        trim: _,
        fps,
        scale_px,
        loop_mode: _,
        captions: _,
        out,
    } = parsed
    {
        assert_eq!(input, PathBuf::from("in.mp4"));
        assert_eq!(fps, 15);
        assert_eq!(scale_px, Some(320));
        assert_eq!(out, PathBuf::from("out.gif"));
    } else {
        panic!("Wrong variant parsed");
    }
}

#[test]
fn test_recorder_event_roundtrip() {
    let evt = RecorderEvent::Started { pts_ms: 1234 };
    let json = to_jsonl(&evt).expect("serialization failed");
    let parsed = parse_recorder_event(&json).expect("parse failed");

    if let RecorderEvent::Started { pts_ms } = parsed {
        assert_eq!(pts_ms, 1234);
    } else {
        panic!("Wrong variant");
    }
}
