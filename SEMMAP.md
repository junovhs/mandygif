# project -- Semantic Map

**Purpose:** screen recorder

## Legend

`[ENTRY]` Application entry point

`[CORE]` Core business logic

`[TYPE]` Data structures and types

`[UTIL]` Utility functions

## Layer 0 -- Config

`Cargo.toml`
Workspace configuration.

`core/captions/Cargo.toml`
Crate configuration.

`core/encoder/Cargo.toml`
Crate configuration.

`core/protocol/Cargo.toml`
Crate configuration.

`core/recorder-linux/Cargo.toml`
Crate configuration.

`core/recorder-mac/Cargo.toml`
Crate configuration.

`core/recorder-win/Cargo.toml`
Crate configuration.

`neti.toml`
Configuration for neti.

`ui/Cargo.toml`
Crate configuration.

## Layer 1 -- Core

`core/captions/src/lib.rs`
Caption rendering module  Phase 1: Generates ffmpeg drawtext filter strings.
→ Exports: chain_filters_expr, ffmpeg_text_expr, chain_filters, ffmpeg_text

`core/encoder/src/main.rs`
Cross-platform encoder: GIF, MP4, WebP.

`core/protocol/src/lib.rs`
JSONL protocol for IPC between UI, recorder, and encoder processes.

`core/recorder-linux/src/lib.rs`
Implements duration ms.
→ Exports: duration_ms, Recorder, start, stop

`core/recorder-mac/src/main.rs`
macOS screen recorder using `ScreenCaptureKit` + `VideoToolbox`  TODO: Implement `SCStream` capture with `VTCompressionSession` encoding.

`core/recorder-win/src/main.rs`
Windows screen recorder using `Windows.Graphics.Capture` + Media Foundation  TODO: Implement WGC + MF H.264 encoding.

`ui/src/main.rs`
MandyGIF` - Dioxus UI.

## Layer 2 -- Domain

`core/encoder/src/ffmpeg.rs`
Validates ffmpeg.
→ Exports: ms_to_sec, build_filter, check_ffmpeg

`core/encoder/src/gif.rs`
Implements encode gif.
→ Exports: encode_gif

`core/encoder/src/video.rs`
Implements encode webp.
→ Exports: encode_mp4, encode_webp

`core/protocol/src/parsing.rs`
Parses recorder command.
→ Exports: to_jsonl, parse_encoder_command, parse_encoder_event, parse_recorder_command

`core/protocol/src/types.rs`
Implements caption animation.
→ Exports: CaptureRegion, ErrorKind, LoopMode, TrimRange

`ui/src/app.rs`
Implements app functionality.
→ Exports: App

`ui/src/components.rs`
Implements components functionality.

`ui/src/components/control_bar.rs`
Implements control bar.
→ Exports: ControlBar

`ui/src/components/icons.rs`
Implements icon export.
→ Exports: IconExport, IconStop

`ui/src/components/resize_handle.rs`
Implements resize handles.
→ Exports: ResizeHandles

`ui/src/hooks.rs`
Implements use recorder.
→ Exports: RecorderController, use_recorder

`ui/src/processes.rs`
Orchestrates `anyhow`, `mandygif_protocol`, `mandygif_recorder_linux`.

`ui/src/state.rs`
Implements app state.
→ Exports: use_app_state, AppMode, AppState

## Layer 4 -- Tests

`core/protocol/tests/golden.rs`
Orchestrates `mandygif_protocol`.

