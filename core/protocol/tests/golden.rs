//! Golden file tests for protocol stability
//! 
//! When messages change, these fixtures must be updated deliberately.

use mandygif_protocol::*;

#[test]
fn recorder_start_golden() {
    let json = r#"{"cmd":"start","region":{"x":128,"y":96,"width":640,"height":360},"fps":30,"cursor":true,"out":"/tmp/clip.mp4"}"#;
    let cmd = parse_recorder_command(json).expect("parse failed");
    
    match cmd {
        RecorderCommand::Start { region, fps, cursor, out } => {
            assert_eq!(region.x, 128);
            assert_eq!(region.width, 640);
            assert_eq!(fps, 30);
            assert!(cursor);
            assert_eq!(out.to_str().unwrap(), "/tmp/clip.mp4");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn recorder_event_golden() {
    let json = r#"{"event":"stopped","duration_ms":10333,"path":"/tmp/clip.mp4"}"#;
    let event = parse_recorder_event(json).expect("parse failed");
    
    match event {
        RecorderEvent::Stopped { duration_ms, path } => {
            assert_eq!(duration_ms, 10333);
            assert_eq!(path.to_str().unwrap(), "/tmp/clip.mp4");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn encoder_gif_golden() {
    let json = r#"{"cmd":"gif","in":"/tmp/clip.mp4","trim":{"start_ms":200,"end_ms":5200},"fps":15,"scale_px":480,"loop":"pingpong","captions":[],"out":"/tmp/out.gif"}"#;
    let cmd = parse_encoder_command(json).expect("parse failed");
    
    match cmd {
        EncoderCommand::Gif { input, trim, fps, scale_px, loop_mode, captions, out } => {
            assert_eq!(trim.start_ms, 200);
            assert_eq!(fps, 15);
            assert_eq!(scale_px, Some(480));
            assert_eq!(loop_mode, LoopMode::Pingpong);
            assert!(captions.is_empty());
        }
        _ => panic!("wrong variant"),
    }
}