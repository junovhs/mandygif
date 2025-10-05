# MandyGIF

A cross-platform, offline, native screen-to-GIF recorder that surpasses GIPHY Capture.

## Architecture

- **JSONL protocol** for clean IPC boundaries
- **Native capture** per platform (PipeWire/WGC/ScreenCaptureKit)
- **Hardware encoding** (VAAPI/Media Foundation/VideoToolbox)
- **Streaming to disk** during recording (no RAM buffer bloat)
- **Slint UI** for lightweight, declarative interface

## Structure

```
core/
├── protocol/         # JSONL message definitions
├── recorder-linux/   # PipeWire + GStreamer capture
├── recorder-mac/     # ScreenCaptureKit + VideoToolbox
├── recorder-win/     # WGC + Media Foundation
├── encoder/          # GIF/MP4/WebP encoding
├── captions/         # Text rendering (ffmpeg → skia)
└── ui/               # Slint interface
```

## Building

```bash
cargo build --release
```

## Running

```bash
# Run UI
cargo run --bin mandygif

# Test recorder directly (Linux)
echo '{"cmd":"start","region":{"x":0,"y":0,"width":640,"height":360},"fps":30,"cursor":false,"out":"/tmp/test.mp4"}' | cargo run --bin recorder-linux
```

## Status

- Protocol definitions
- Project structure
- Linux recorder (PipeWire + GStreamer)
- Encoder (gifski/ffmpeg)
- Slint UI with overlay
- macOS recorder
- Windows recorder
- Caption rendering (Phase 2: skia)

## License

MIT OR Apache-2.0

---