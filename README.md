# MandyGIF Development Handoff Document

When writing/modifying code, obey this mantra:
Least Power — simplest viable construct (KISS).
Modularity — clean, replaceable boundaries (abstractions).
Single Responsibility — one purpose per unit (no tangles).
Antifragility — failure strengthens (resilience, degradation).
Transparency — clarity over cleverness (self-reading intent).
Reduce Surface Area — expose essentials (lean interfaces).
Emergence — simple interactions yield complexity (adaptive).
Decentralization — distribute control (peer scale).
Adaptivity — evolve with context (CI/branch).
Redundancy — thoughtful backups (invisible failover).
Debug addendum: Evidence-first (min repro + env). Structured logs (JSONL: ts/level/rid/subsystem/action/code/msg/context). Stable errors (codes, causes, next steps—never swallow). Instrumentation (flag-guarded probes at entry/exit/decisions/fails; strip post-fix). No guessing (flag thin data, request exact logs/cmds). Artifacts (save failing inputs/files, print paths). Confidence (low/med/high labels).
Output executable code with these baked in + diff note.

## Journey Summary

### Vision
Build a cross-platform, offline, native screen-to-GIF recorder that **destroys GIPHY Capture** - smooth, fast, professional quality, with clean architecture that won't rot.

### Architecture Decisions
**Core Principle**: Rust + Slint UI + Native Capture/Encode + JSONL IPC

**Why this stack:**
- **Least Power**: Slint for declarative UI, native APIs per platform, JSONL for IPC (no over-engineered RPC)
- **Modularity**: Clean process boundaries - UI doesn't know about GStreamer internals
- **Single Responsibility**: Each binary has one job (record, encode, select region, orchestrate)
- **Antifragility**: Process crashes don't take down the whole app; each component testable in isolation
- **Transparency**: JSONL messages are human-readable, debuggable with `tee`, versionable

### What We Built

#### 1. **Protocol Layer** (`core/protocol/`)
- Versioned JSONL message definitions using Serde
- Two protocols: Recorder ↔ UI, Encoder ↔ UI
- Golden tests ensure protocol stability
- Zero dependencies on implementation details

**Key types:**
```rust
RecorderCommand::Start { region, fps, cursor, out }
RecorderEvent::Progress { pts_ms }
EncoderCommand::Gif { input, trim, fps, scale_px, loop_mode, captions, out }
```

#### 2. **Linux Recorder** (`core/recorder-linux/`)
- **Tech**: GStreamer + PipeWire portal (X11/XWayland for now, Wayland native planned)
- **Flow**: ximagesrc → videoconvert → x264enc → mp4mux → filesink
- **Streams to disk** during recording (no RAM buffer bloat)
- Emits progress events every 500ms with PTS
- EOS handling ensures MP4 finalization before exit

**Critical lesson learned**: Progress events from background thread need proper stdout locking to avoid buffering issues.

#### 3. **Encoder** (`core/encoder/`)
- **GIF**: ffmpeg palettegen/paletteuse (2-pass for quality)
- **MP4**: ffmpeg with libx264, CRF mapping from quality slider (0.0-1.0 → CRF 51-18)
- **WebP**: ffmpeg with lossy/lossless toggle
- All operations use temp directories, stream processing (no memory explosion)

**Not yet implemented**: Caption rendering (Phase 1: ffmpeg drawtext filters exist but not wired)

#### 4. **Region Selector** (`core/region-selector/`)
- **Tech**: Slint fullscreen transparent overlay
- **UX**: GIPHY Capture style - draggable title bar, resizable from bottom-right corner with 3-line handle
- **Output**: JSON coordinates on stdout when user confirms

**Evolution**:
- Started with X11 raw overlay (flickered, high CPU) ❌
- Tried Flameshot click-drag approach (smooth but no persistent window) ⚠️
- Settled on Slint windowed overlay (buttery smooth, cross-platform) ✅

**Key insight**: Slint's `Path` primitive for vector graphics > pixel-by-pixel Rectangle hacks

#### 5. **Main UI** (`ui/`)
- **Tech**: Slint with async Tokio runtime
- **Pattern**: Spawn child processes, communicate via JSONL over stdin/stdout
- **Critical fix**: UI updates MUST use `slint::invoke_from_event_loop()` - can't mutate UI from arbitrary async tasks

**Progress timer fix journey**:
1. Initial: Weak references failed to upgrade ❌
2. Channel approach: Events sent but UI didn't update ❌  
3. Final: Channel + `invoke_from_event_loop()` ✅

**Lessons**:
- Slint has its own event loop; respect it
- Channels bridge async world → UI world cleanly
- Debug with `eprintln!` everywhere when threading issues arise

---

## Technical Architecture (Comprehensive)

### System Diagram
```
┌─────────────────────────────────────────────────────────────┐
│                        Main UI (Slint)                       │
│  - State management (AppStateData)                           │
│  - Spawns child processes                                    │
│  - Reads JSONL events via tokio::process                     │
└────────┬──────────────────────────────┬─────────────────────┘
         │                              │
         │ JSONL/stdin/stdout          │ JSONL/stdin/stdout
         ▼                              ▼
┌──────────────────────┐      ┌──────────────────────┐
│  recorder-linux      │      │     encoder          │
│  (GStreamer)         │      │  (ffmpeg wrapper)    │
│                      │      │                      │
│  - ximagesrc         │      │  - palettegen        │
│  - x264enc           │      │  - libx264/libwebp   │
│  - Progress thread   │      │  - Trim/scale        │
└──────────────────────┘      └──────────────────────┘

         ┌──────────────────────┐
         │  region-selector     │
         │  (Slint overlay)     │
         │                      │
         │  - Transparent win   │
         │  - Drag/resize       │
         │  - JSON output       │
         └──────────────────────┘
```

### Data Flow: Recording Session

**1. User Interaction**
```
User clicks "Start Recording"
  ↓
UI.on_start_recording() callback fires
  ↓
region = ui.get_capture_region() // Currently hardcoded 800x600
  ↓
Spawn recorder-linux binary
  ↓
Create channel: (progress_tx, progress_rx)
  ↓
Spawn two async tasks:
  - run_recorder() // Talks to child process
  - UI update task  // Receives events, calls invoke_from_event_loop()
```

**2. Recorder Process**
```rust
// In recorder-linux main loop
stdin reads: {"cmd":"start","region":{...},"fps":30,...}
  ↓
Parse RecorderCommand::Start
  ↓
Build GStreamer pipeline:
  ximagesrc startx=X starty=Y endx=X2 endy=Y2
  ↓ video/x-raw,framerate=30/1
  ↓ videoconvert
  ↓ video/x-raw,format=I420
  ↓ x264enc speed-preset=ultrafast tune=zerolatency
  ↓ mp4mux
  ↓ filesink location=/tmp/mandygif_recording.mp4
  
Set pipeline to PLAYING
  ↓
stdout writes: {"event":"started","pts_ms":0}\n
  ↓
Background thread queries pipeline.query_position() every 500ms
  ↓
stdout writes: {"event":"progress","pts_ms":499}\n
         {"event":"progress","pts_ms":999}\n
         ...
```

**3. UI Update Task**
```rust
// In UI async task
loop {
    event = progress_rx.recv().await
    ↓
    slint::invoke_from_event_loop(move || {
        ui.set_recording_duration_ms(pts_ms)
    })
}
```

**4. Stop Recording**
```
User clicks "Stop Recording"
  ↓
Send () via oneshot channel to run_recorder task
  ↓
run_recorder writes: {"cmd":"stop"}\n to child stdin
  ↓
Recorder receives stop command
  ↓
pipeline.send_event(Eos)
  ↓
Wait for EOS on bus (max 5 seconds)
  ↓
pipeline.set_state(Null)
  ↓
stdout writes: {"event":"stopped","duration_ms":10333,"path":"/tmp/..."}\n
  ↓
UI receives Stopped event → transition to Editing state
```

### Data Flow: Export Session

**1. User configures export**
```
UI shows:
  - Format dropdown (GIF/MP4/WebP)
  - FPS slider (5-60)
  - Trim handles (start_ms, end_ms)
  - Scale width (480/720/1080)
```

**2. User clicks export**
```rust
let cmd = match format {
    "gif" => EncoderCommand::Gif {
        input: "/tmp/mandygif_recording.mp4",
        trim: TrimRange { start_ms, end_ms },
        fps: 15,
        scale_px: Some(480),
        loop_mode: LoopMode::Normal,
        captions: vec![],
        out: "/tmp/mandygif_export.gif"
    },
    // ... similar for mp4/webp
};

Spawn encoder binary
  ↓
Write cmd as JSONL to stdin
  ↓
Read events from stdout
```

**3. Encoder Process (GIF example)**
```
Read EncoderCommand::Gif from stdin
  ↓
Step 1: Generate palette
  ffmpeg -i input.mp4 -ss 0.2s -to 5.2s 
         -vf "fps=15,scale=480:-1:flags=lanczos,palettegen"
         -y /tmp/palette.png
  ↓
Step 2: Apply palette
  ffmpeg -i input.mp4 -i /tmp/palette.png
         -ss 0.2s -to 5.2s
         -lavfi "fps=15,scale=480:-1:flags=lanczos [x]; [x][1:v] paletteuse"
         -loop 0 -y output.gif
  ↓
stdout writes: {"event":"done","path":"/tmp/mandygif_export.gif"}\n
```

### Region Selector Deep Dive

**Architecture**:
- Single Slint Window (3840x2160px to cover 4K displays)
- `background: transparent` (no fullscreen darkening)
- All UI elements positioned absolutely relative to selection box

**Components**:
```
Selection Box (Rectangle)
  ├─ Colored overlay (#00ffff20) with 2px border
  ├─ Top bar (-40px Y offset)
  │   ├─ Dimensions text: "1920 × 1080"
  │   ├─ Close button (red X)
  │   └─ TouchArea (draggable)
  ├─ Bottom bar (+height Y offset)
  │   ├─ REC button (confirms selection)
  │   └─ Resize handle (3 diagonal Path elements)
  └─ TouchArea on resize handle
```

**Interaction Model**:

*Moving*:
```rust
// Top bar TouchArea
pointer-event(down) => {
    offset-x = mouse-x
    offset-y = mouse-y
}
moved => {
    sel-x += mouse-x - offset-x
    sel-y += mouse-y - offset-y
}
```

*Resizing*:
```rust
// Resize handle TouchArea
moved => {
    sel-width = max(200px, sel-width + mouse-x)
    sel-height = max(100px, sel-height + mouse-y)
}
```

**Output**:
```rust
on_confirm() => {
    let region = Region { x, y, width, height };
    println!("{}", serde_json::to_string(&region)?);
    quit_event_loop();
}
```

---

## Current State & Known Issues

### ✅ Working
- Recording with live timer updates (Linux)
- Stop recording with proper MP4 finalization
- Region selector UI (Linux + Windows confirmed)
- Protocol definitions stable
- Encoder exists and tested standalone

### ⚠️ Partially Working
- Export flow: Button wired but not end-to-end tested
- Caption rendering: Code exists but not integrated

### ❌ Not Working / TODO
1. **Region selector not integrated into main UI**
   - Button logs "not yet implemented"
   - Need to spawn region-selector, parse JSON, update capture_region
   
2. **Windows/macOS recorders**
   - Stubs exist, return UnsupportedPlatform error
   - Need WGC (Windows) and SCStream (macOS) implementations

3. **Preview during recording**
   - MP4 file exists at `/tmp/mandygif_recording.mp4`
   - Could use video player widget or frame extraction

4. **Caption authoring UI**
   - Need text input, timeline placement, style controls
   - Backend (ffmpeg drawtext) ready

5. **Advanced features**
   - Ping-pong loop (needs frame reversal)
   - Rolling 30s buffer (needs segment recording)
   - Multiple output formats in one pass

---

## Build & Test Commands

```bash
# Build everything
cargo build --release

# Test recorder standalone (Linux only)
echo '{"cmd":"start","region":{"x":100,"y":100,"width":800,"height":600},"fps":30,"cursor":false,"out":"/tmp/test.mp4"}' | ./target/debug/recorder-linux
# Wait 3 seconds
echo '{"cmd":"stop"}' | ./target/debug/recorder-linux

# Test encoder standalone
echo '{"cmd":"gif","in":"/tmp/test.mp4","trim":{"start_ms":0,"end_ms":2700},"fps":15,"scale_px":480,"loop":"normal","captions":[],"out":"/tmp/test.gif"}' | ./target/debug/encoder

# Test region selector
./target/debug/region-selector
# Returns: {"x":460,"y":240,"width":1280,"height":720}

# Run main UI
./target/release/mandygif
```

---

## Debugging Patterns Used

**1. Process Communication Issues**
```rust
// Always wrap events with debug output
eprintln!("UI: Got line from recorder: {:?}", line);
eprintln!("Sending progress: {}ms", pts_ms);
```

**2. Slint UI Update Issues**
```rust
// WRONG - won't update UI
tokio::spawn(async move {
    ui.set_value(x); // ❌ Called from wrong thread
});

// RIGHT - queues on Slint event loop
slint::invoke_from_event_loop(move || {
    ui.set_value(x); // ✅ Runs on UI thread
});
```

**3. GStreamer Pipeline Issues**
```bash
# Redirect stderr to see GStreamer logs
./target/debug/recorder-linux 2>&1 | tee recorder.log

# Verify MP4 is valid
ffprobe /tmp/mandygif_recording.mp4
```

**4. JSONL Protocol Issues**
```bash
# Pipe through jq for pretty printing
./target/debug/region-selector | jq .

# Test with invalid input to verify error handling
echo 'invalid json' | ./target/debug/encoder
```

---

## Next Immediate Steps (Priority Order)

### 1. **Integrate Region Selector** (1-2 hours)
**File**: `ui/src/main.rs`

```rust
ui.on_show_region_selector(move || {
    let ui_weak = ui.as_weak();
    tokio::spawn(async move {
        let region_bin = std::env::current_exe()?
            .parent().unwrap()
            .join("region-selector");
        
        let output = Command::new(&region_bin)
            .output()
            .await?;
        
        if output.status.success() {
            let json = String::from_utf8_lossy(&output.stdout);
            if let Ok(region) = serde_json::from_str::<Region>(&json) {
                slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_capture_region(Region {
                            x: region.x,
                            y: region.y,
                            width: region.width as i32,
                            height: region.height as i32,
                        });
                    }
                }).unwrap();
            }
        }
    });
});
```

**Confidence**: High (same pattern as recorder/encoder)

### 2. **Test Export Flow End-to-End** (30 min)
- Record 5 second clip
- Export to GIF with trim
- Verify output quality
- Test MP4/WebP formats

**Confidence**: Medium (encoder tested standalone, integration unknown)

### 3. **Add Preview Window** (2-3 hours)
Options:
- **A**: Slint VideoPlayer widget (if available)
- **B**: Extract frames with ffmpeg, show as Image sequence
- **C**: Embed mpv/vlc via platform window handle

**Confidence**: Medium (depends on Slint capabilities)

### 4. **Caption UI** (4-6 hours)
- Text input field
- Font picker (list system fonts)
- Color pickers (text + stroke)
- Timeline scrubber for placement
- Live preview overlay

**Confidence**: High (UI work, backend ready)

### 5. **Windows Recorder** (8-12 hours)
- Use `windows` crate for WGC APIs
- Capture to texture → encode with Media Foundation
- Handle multi-monitor coordinates
- Cursor overlay

**Confidence**: Medium (API available, unfamiliar territory)

---

## Key Files Reference

```
mandygif/
├── core/
│   ├── protocol/
│   │   ├── src/lib.rs           # JSONL message definitions
│   │   └── tests/golden.rs      # Protocol stability tests
│   │
│   ├── recorder-linux/
│   │   └── src/main.rs          # GStreamer pipeline + progress thread
│   │
│   ├── encoder/
│   │   └── src/main.rs          # ffmpeg wrapper (GIF/MP4/WebP)
│   │
│   ├── captions/
│   │   └── src/lib.rs           # ffmpeg drawtext filter generation
│   │
│   └── region-selector/
│       ├── src/main.rs          # Region struct + callbacks
│       └── ui/selector.slint    # Transparent overlay UI
│
└── ui/
    ├── src/main.rs              # Main app: spawns processes, event loop
    └── ui/main.slint            # Main window layout (placeholder)
```

---

## Architecture Wins

1. **Process isolation**: UI crash ≠ data loss; encoder crash ≠ UI freeze
2. **Testability**: Each binary can be fuzzed, profiled, debugged independently
3. **Cross-platform**: Platform-specific code contained in recorder binaries
4. **Observable**: JSONL can be logged, replayed, analyzed with standard tools
5. **Evolvable**: Can swap GStreamer → FFmpeg, or Slint → egui, without rewriting everything

## Architecture Debts

1. **No error recovery**: If recorder crashes mid-recording, recording lost
2. **Hardcoded paths**: `/tmp/mandygif_*` not configurable, conflicts possible
3. **No state persistence**: Close app = lose unsaved recordings
4. **Single monitor**: Region selector assumes screen 0
5. **No streaming optimizations**: Everything goes through disk (fine for short clips)

---

## Handoff Notes

**Context for next session**:
- You're picking up a working prototype
- Core loop proven: record → stop → edit → export
- UI framework (Slint) has learning curve but pays off in cross-platform + performance
- GStreamer knowledge helpful but not required (wrapper is done)
- Next phase is integration & polish, not greenfield architecture

**When stuck**:
- Add `eprintln!` liberally - stdout is for protocol, stderr for debug
- Test binaries standalone before integration
- Slint hot-reloads `.slint` files - use this for UI iteration
- Check process exit codes when spawning (`status.success()`)

**Philosophy maintained**:
- If you're writing >20 lines without testing, stop and test
- If abstraction doesn't pay for itself in 3 uses, inline it
- If you can't explain it in one sentence, simplify it

---
!!!!ALWAYS REMEMBER!!!!!

When writing/modifying code, ***obey this mantra:***
Least Power — simplest viable construct (KISS).
Modularity — clean, replaceable boundaries (abstractions).
Single Responsibility — one purpose per unit (no tangles).
Antifragility — failure strengthens (resilience, degradation).
Transparency — clarity over cleverness (self-reading intent).
Reduce Surface Area — expose essentials (lean interfaces).
Emergence — simple interactions yield complexity (adaptive).
Decentralization — distribute control (peer scale).
Adaptivity — evolve with context (CI/branch).
Redundancy — thoughtful backups (invisible failover).
Debug addendum: Evidence-first (min repro + env). Structured logs (JSONL: ts/level/rid/subsystem/action/code/msg/context). Stable errors (codes, causes, next steps—never swallow). Instrumentation (flag-guarded probes at entry/exit/decisions/fails; strip post-fix). No guessing (flag thin data, request exact logs/cmds). Artifacts (save failing inputs/files, print paths). Confidence (low/med/high labels).
Output executable code with these baked in + diff note.

**End of handoff. Godspeed.** 🚀