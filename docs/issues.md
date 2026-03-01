

# issues-0002: Architecture V2 — Research-Informed Redesign

---

## FORMAT (DO NOT MODIFY)

**Status values:** `OPEN`, `IN PROGRESS`, `DONE`, `DESCOPED`

**Issue format:**
```
## [N] Title
**Status:** OPEN
**Files:** list of files to modify

Description of the task.

**Resolution:** (fill when DONE) What was done, any notes.
```

**Instructions:**
- Work issues in order you feel is most important.
- Update status as you go
- Add **Resolution:** when completing
- Don't modify this FORMAT section
- Content below the line is the work. When done, archive in docs/archive and create next issues doc.

---

## Context: Research Findings vs Current Architecture

The research surfaced five fundamental mismatches between our current architecture and what a production screen recorder requires:

| Area | Current | Should Be | Impact |
|------|---------|-----------|--------|
| Encoding | ffmpeg child process via stdin/stdout | `ffmpeg-next` in-process or platform SDK | Every frame copied through CPU twice |
| IPC | JSONL text over pipes | `iceoryx2` shared memory (frames), JSONL retained for commands only | 1080p60 = 370MB/s through a pipe |
| Captions | Post-hoc ffmpeg drawtext at export | Real-time GPU compositing via `wgpu` + `fontdue` | User can't preview captions before export |
| Capture | Linux-only, hard-coded | Per-platform crates behind shared trait | App doesn't compile on 2 of 3 targets |
| Region select | Transparent WebView window | Separate native `tao` window | WebView latency + transparency bugs on Win/Wayland |

The JSONL protocol crate and its types remain valuable — command/event IPC between processes is still the right model for control messages. What changes is that **frame data** never touches a pipe.

### Crate Decisions (from research)

| Role | Crate | Why |
|------|-------|-----|
| Capture (macOS) | `screencapturekit-rs` | Direct IOSurface access, active maintenance |
| Capture (Windows) | `windows-capture` | Smart frame updates, v2.0 released |
| Capture (Linux) | `ashpd` + `pipewire` | Native portal + PipeWire, standard approach |
| Encoding | `ffmpeg-next` | Access to `h264_videotoolbox`, `h264_mf`, `h264_vaapi` without spawning process |
| Frame IPC | `iceoryx2` | True zero-copy shared memory between capture and encoder processes |
| GPU compositing | `wgpu` | Cross-platform GPU access for caption overlay |
| Font rasterization | `fontdue` | CPU-efficient, no system font dependency |
| Packaging | `cargo-packager` | Cross-platform DMG/MSI/DEB |

---

## Order (work top to bottom)

**(Phase 1 — Foundation: make it compile and capture on all platforms)**
1. **[01]** Define `RecorderTrait` in new `core/recorder-trait` crate
2. **[02]** Implement Linux recorder behind the trait (`ashpd` + `pipewire`)
3. **[03]** Implement macOS recorder behind the trait (`screencapturekit-rs`)
4. **[04]** Implement Windows recorder behind the trait (`windows-capture`)
5. **[05]** Platform-gate UI process spawning with cfg + trait objects
6. **[06]** Replace `expect()`/`#[allow]` violations (quick governance sweep)

**(Phase 2 — Encoding pipeline: replace ffmpeg child process)**
7. **[07]** Integrate `ffmpeg-next` as in-process encoder, remove child process architecture
8. **[08]** Design frame transport layer (`iceoryx2` shared memory for capture→encoder)
9. **[09]** Wire hardware-accelerated encoding paths per platform
10. **[10]** Retain JSONL protocol for control messages only, remove frame-over-pipe assumption

**(Phase 3 — Caption system: GPU-based real-time rendering)**
11. **[11]** Replace hard-coded font path with `fontdue` rasterizer
12. **[12]** Build `wgpu` compositing pipeline for caption overlay on captured frames
13. **[13]** Real-time caption preview in UI (user sees captions before export)
14. **[14]** Remove ffmpeg drawtext filter generation (captions crate becomes GPU compositor)

**(Phase 4 — UI architecture: region selection + overlay)**
15. **[15]** Separate region selector into native `tao` window (not WebView)
16. **[16]** Platform-specific transparency fixes (Windows resize hack, Wayland RGBA visual)
17. **[17]** Global hotkey integration for start/stop recording

**(Phase 5 — Distribution: make it installable)**
18. **[18]** macOS: App bundle, entitlements, notarization, Sequoia re-auth handling
19. **[19]** Windows: MSIX packaging and manifest for Graphics Capture capability
20. **[20]** Linux: Flatpak with PipeWire socket access and portal permissions
21. **[21]** CI pipeline: build + sign + package for all three platforms

**(Phase 6 — Type safety and validation: harden the protocol)**
22. **[22]** `EncodeJob` params struct (replace 7-8 arg functions)
23. **[23]** `OutputFormat` enum (replace magic string matching)
24. **[24]** `TrimRange` validation (end > start, sane bounds)
25. **[25]** `NormalizedFloat` newtype for quality (enforce 0.0–1.0)
26. **[26]** Wildcard imports → explicit named imports

**(Phase 7 — Feature completion)**
27. **[27]** Encoder progress reporting (parse ffmpeg progress, emit events)
28. **[28]** Pingpong loop mode (reverse+concat filter)
29. **[29]** Test foundation: unit, integration, mutation across all crates

---

## [01] Define `RecorderTrait` in new `core/recorder-trait` crate
**Status:** OPEN
**Files:** new `core/recorder-trait/Cargo.toml`, new `core/recorder-trait/src/lib.rs`, `Cargo.toml` (workspace members)

Create a new crate that defines the platform-agnostic recorder contract. This is the single most important architectural piece — everything else plugs into it.

```rust
pub trait Recorder: Send + 'static {
    fn start(config: RecordConfig) -> Result<Self, RecorderError> where Self: Sized;
    fn duration_ms(&self) -> u64;
    fn stop(self) -> Result<RecordingResult, RecorderError>;
}

pub struct RecordConfig {
    pub region: CaptureRegion,
    pub fps: u32,
    pub cursor: bool,
    pub out: PathBuf,
}

pub struct RecordingResult {
    pub duration_ms: u64,
    pub path: PathBuf,
}
```

Use `thiserror` for `RecorderError`, not `anyhow`. This is a library crate. Error variants: `PermissionDenied`, `NoDisplay`, `EncoderInit`, `Io`, `UnsupportedPlatform`.

The trait takes ownership of config (not references) because the recorder runs on a background thread/task. `CaptureRegion` is re-exported from `mandygif-protocol` — the trait crate depends on protocol for shared types.

**Resolution:**

---

## [02] Implement Linux recorder behind the trait
**Status:** OPEN
**Files:** `core/recorder-linux/Cargo.toml`, `core/recorder-linux/src/lib.rs`

Rewrite the Linux recorder to implement `RecorderTrait`. Currently it has its own ad-hoc API. The new implementation should:

- Use `ashpd` to request screen capture via XDG Desktop Portal (handles the permission dialog)
- Connect to the PipeWire stream using `pipewire` crate to receive frame buffers
- Store frames to disk via `ffmpeg-next` (initially; [08] will add shared memory later)
- Expose `duration_ms()` by tracking elapsed time from first frame callback
- `stop()` finalizes the file and returns the result

The portal interaction is async (D-Bus). The PipeWire stream runs on its own thread with a `pw::main_loop`. Bridge these with a channel back to the trait's synchronous `stop()`.

Depends on: [01]

**Resolution:**

---

## [03] Implement macOS recorder behind the trait
**Status:** OPEN
**Files:** `core/recorder-mac/Cargo.toml`, `core/recorder-mac/src/lib.rs` (rename from `main.rs`)

Replace the current TODO stub with a real implementation using `screencapturekit-rs`.

- Use `SCShareableContent` to enumerate displays
- Create `SCContentFilter` for the target region/display
- Configure `SCStreamConfiguration` with requested fps, pixel format (BGRA initially, YCbCr later for zero-copy)
- Implement `SCStreamOutput` trait to receive `CMSampleBuffer` frames
- Pass frames to `ffmpeg-next` for encoding to the output file
- The IOSurface backing the CMSampleBuffer enables future zero-copy to VideoToolbox ([09])

This crate should be a library (`lib.rs`), not a binary (`main.rs`). The binary entry point lives in the UI or a platform-specific launcher.

Key gotcha: The app must have a Bundle ID and `NSScreenCaptureUsageDescription` in Info.plist. This crate should expose a `fn check_permission() -> PermissionStatus` that wraps SCKit's permission check.

Depends on: [01]

**Resolution:**

---

## [04] Implement Windows recorder behind the trait
**Status:** OPEN
**Files:** `core/recorder-win/Cargo.toml`, `core/recorder-win/src/lib.rs` (rename from `main.rs`)

Replace the current TODO stub using `windows-capture` crate (v2.0).

- Use `GraphicsCaptureItem` to target the selected monitor/region
- Create `CaptureFramePool` to receive `ID3D11Texture2D` frames
- The `windows-capture` crate provides a callback-based API — implement its `CaptureHandler` trait
- Write frames to disk via `ffmpeg-next`
- Handle the yellow capture border (document it as expected behavior, don't try to hide it)

Requires a running Win32 message loop. Dioxus's `tao` already provides one, but if the recorder runs in a separate process, it needs its own. Document this decision.

Same as macOS: library crate, not binary.

Depends on: [01]

**Resolution:**

---

## [05] Platform-gate UI process spawning
**Status:** OPEN
**Files:** `ui/src/processes.rs`, `ui/Cargo.toml`

Replace the unconditional `use mandygif_recorder_linux::Recorder` with cfg-gated imports and a factory function:

```rust
#[cfg(target_os = "linux")]
use mandygif_recorder_linux::LinuxRecorder as PlatformRecorder;

#[cfg(target_os = "macos")]
use mandygif_recorder_mac::MacRecorder as PlatformRecorder;

#[cfg(target_os = "windows")]
use mandygif_recorder_win::WinRecorder as PlatformRecorder;
```

Gate dependencies in `Cargo.toml`:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
mandygif-recorder-linux = { path = "../core/recorder-linux" }
```

The `run_recorder` function becomes generic over `RecorderTrait` or uses the type alias. Either way, the UI crate compiles on all platforms — it just fails at runtime with `UnsupportedPlatform` if no recorder is available.

This unblocks cross-compilation immediately, even before [03] and [04] are complete.

Depends on: [01]. Partially blocked by [03], [04] for full functionality.

**Resolution:**

---

## [06] Governance sweep: remove `#[allow]` and `expect()`
**Status:** OPEN
**Files:** `core/encoder/src/main.rs`, `core/encoder/src/ffmpeg.rs`, `core/encoder/src/gif.rs`, `core/encoder/src/video.rs`, `core/captions/src/lib.rs`, `ui/src/main.rs`, `ui/src/processes.rs`

Quick pass to fix all governance violations before the architecture changes make these files harder to touch:

- Replace all `#[allow(clippy::wildcard_imports)]` with explicit named imports
- Replace `#[allow(clippy::uninlined_format_args)]` by inlining the format args
- Replace `#[allow(clippy::cast_precision_loss)]` with targeted `#[expect]` + comment on the one function where it's intentional (`ms_to_sec`)
- Clamp values before casting to prevent `cast_possible_truncation` (CRF calculation, pixel coordinates)
- Replace `.expect()` in `ui/src/main.rs` with `eprintln!` + `exit(1)`
- Remove `#[allow(non_snake_case)]` and add `#[component]` attributes to Dioxus components
- Remove `#[allow(clippy::many_single_char_names)]` and rename `r,g,b,a` to full names in `ff_color`

This is housekeeping, not architecture. Do it in one pass, don't spread it across weeks.

**Resolution:**

---

## [07] Integrate `ffmpeg-next` as in-process encoder
**Status:** OPEN
**Files:** `core/encoder/Cargo.toml`, `core/encoder/src/main.rs`, `core/encoder/src/gif.rs`, `core/encoder/src/video.rs`, `core/encoder/src/ffmpeg.rs`

This is the biggest single change in the redesign. Replace the architecture of spawning `ffmpeg` as a child process and communicating via stdin/stdout with direct use of `ffmpeg-next` (Rust bindings to libav*).

Current flow: `UI → JSONL → encoder binary → spawn ffmpeg CLI → pipe bytes → output file`

New flow: `UI → JSONL → encoder binary → ffmpeg-next API calls → output file`

Benefits:
- No serialization/deserialization of frame data through pipes
- Direct access to hardware encoders (`h264_videotoolbox`, `h264_qsv`, `h264_vaapi`)
- Programmatic progress callbacks instead of parsing stderr text
- Error handling via Result, not parsing exit codes

The encoder binary still exists as a separate process (isolates crashes from UI), but it links `ffmpeg-next` directly instead of spawning a subprocess.

Keep `check_ffmpeg()` as a runtime check that the required shared libraries are loadable.

Depends on: nothing (can be done in parallel with [01]-[05])

**Resolution:**

---

## [08] Frame transport layer with `iceoryx2`
**Status:** OPEN
**Files:** new `core/transport/Cargo.toml`, new `core/transport/src/lib.rs`, `core/recorder-trait/src/lib.rs`

Design and implement zero-copy frame transport between the capture process and the encoder process using `iceoryx2` shared memory.

The current architecture sends control messages (start, stop, progress) over JSONL pipes. This is fine for commands — keep it. The problem is frame data. At 1080p BGRA @ 60fps, that's ~370MB/s of pixel data that currently doesn't flow at all (the recorder writes directly to a file, and the encoder reads it after recording stops).

For real-time preview and live caption compositing, frames need to flow from recorder → compositor → encoder continuously:

```rust
pub struct FrameSlot {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub pts_ms: u64,
    pub data: [u8], // variable-length, in shared memory
}
```

`iceoryx2` provides publish/subscribe over shared memory with zero serialization. The recorder publishes frames, the compositor (caption overlay) subscribes, composites, and republishes, and the encoder subscribes to the composited stream.

This is a significant infrastructure piece. Prototype with a simple ring buffer first, then migrate to `iceoryx2` if the ring buffer proves insufficient.

Depends on: [01], [07]

**Resolution:**

---

## [09] Wire hardware-accelerated encoding paths per platform
**Status:** OPEN
**Files:** `core/encoder/src/video.rs`, `core/encoder/src/ffmpeg.rs`

Once `ffmpeg-next` is integrated ([07]), configure it to use platform-native hardware encoders:

| Platform | Encoder | Buffer Source |
|----------|---------|---------------|
| macOS | `h264_videotoolbox` | IOSurface via CVPixelBuffer |
| Windows | `h264_mf` or `h264_qsv` | ID3D11Texture2D via MFCreateDXGISurfaceBuffer |
| Linux | `h264_vaapi` | DMA-BUF imported as VA surface |

Implement a fallback chain: try hardware encoder first, fall back to `libx264` (software) if unavailable. Log which encoder was selected.

The zero-copy path from capture buffer to encoder is the ultimate goal but requires [08] (shared memory transport). Initially, the hardware encoder still gets CPU-copied frames — the win is that encoding itself runs on dedicated silicon (GPU media engine), freeing the CPU.

Depends on: [07]

**Resolution:**

---

## [10] Scope JSONL protocol to control messages only
**Status:** OPEN
**Files:** `core/protocol/src/types.rs`, `core/protocol/src/lib.rs`

The JSONL protocol is well-designed for what it does. But the research clarifies its role: it handles **control plane** messages (start, stop, configure, progress, errors), not **data plane** messages (frames).

Audit the protocol types and add documentation making this boundary explicit:

```rust
//! Control-plane protocol for IPC between UI, recorder, and encoder.
//!
//! Frame data flows through shared memory (see `mandygif-transport`).
//! This protocol handles commands, events, and status only.
```

Remove any assumptions in the codebase that frame data flows through the same channel as commands. If `EncoderCommand::Gif { input: PathBuf }` currently means "read this file from disk," that's fine for Phase 1 — but document that Phase 2 will replace it with a shared memory handle.

Depends on: [08]

**Resolution:**

---

## [11] Replace hard-coded font path with `fontdue`
**Status:** OPEN
**Files:** `core/captions/Cargo.toml`, `core/captions/src/lib.rs`

The current captions crate generates ffmpeg drawtext filter strings containing `/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf`. This path doesn't exist on macOS or Windows, making the entire caption system Linux-only.

Replace with `fontdue` for font rasterization:

- Bundle a default font (e.g., Inter or Noto Sans) as embedded bytes via `include_bytes!`
- Use `fontdue::Font::from_bytes()` to parse at startup
- Rasterize glyphs to bitmaps that can be composited onto frames
- Remove all ffmpeg drawtext generation code (it will be replaced by GPU compositing in [12])

`fontdue` has zero system dependencies — it's pure Rust. This eliminates the platform font path problem entirely.

If [12] (wgpu compositing) isn't ready yet, `fontdue` can rasterize to CPU bitmaps as an intermediate step. The font rasterization is the same either way.

**Resolution:**

---

## [12] Build `wgpu` compositing pipeline for caption overlay
**Status:** OPEN
**Files:** new `core/compositor/Cargo.toml`, new `core/compositor/src/lib.rs`

Create a new crate that composites caption text onto captured video frames using the GPU.

Pipeline:
1. Receive a raw frame (from shared memory or file)
2. Upload to a `wgpu::Texture`
3. Rasterize caption glyphs via `fontdue` → upload glyph atlas texture
4. Run a fragment shader that blends the glyph atlas onto the frame at the correct position, with alpha, color, stroke
5. Read back the composited frame (or pass the texture directly to the encoder if it supports GPU textures)

This replaces the entire ffmpeg drawtext filter approach. Benefits:
- Captions render identically on all platforms
- Preview is possible (same pipeline renders to screen and to encoder)
- Animation (fade in/out) is a shader uniform, not a complex filter expression
- No font path issues — fonts are embedded

Start with a minimal pipeline: single text string, solid color, positioned at (x, y). Expand to styled text, stroke, and animation after the basic pipeline works.

Depends on: [11]

**Resolution:**

---

## [13] Real-time caption preview in UI
**Status:** OPEN
**Files:** `ui/src/app.rs`, `ui/src/components.rs`, new `ui/src/preview.rs`

Once the compositor ([12]) can render captions onto frames, display the result in the Dioxus UI so users can see exactly what their export will look like before they hit export.

Options:
- Render composited frames to a texture, display in the WebView via a `<canvas>` element and base64-encoded image updates (simple but slow)
- Use a separate `wgpu` surface in a native `tao` window for the preview (fast but more complex)
- Use Dioxus's `use_asset` or custom asset server to serve rendered frames

Start with the simplest approach that gives acceptable latency (<100ms update). The preview doesn't need 60fps — 5-10fps is fine for caption positioning.

Depends on: [12]

**Resolution:**

---

## [14] Remove ffmpeg drawtext filter generation
**Status:** OPEN
**Files:** `core/captions/src/lib.rs`, `core/encoder/src/ffmpeg.rs`

Once GPU compositing ([12]) is working and integrated, remove:

- `ffmpeg_text()` and `ffmpeg_text_expr()` — no longer needed
- `chain_filters()` and `chain_filters_expr()` — no longer needed
- The caption filter injection in `build_filter()`
- The `ff_color()` function and all hex color parsing (the compositor uses its own color representation)

The captions crate transforms from "generate ffmpeg filter strings" to "provide the compositor with positioned, styled text regions." Its public API becomes the data types (`Caption`, `CaptionStyle`, `CaptionRect`) and the font loading/rasterization functions.

This is a breaking change to the captions crate's API surface. But since the only consumer is the encoder, it's contained.

Depends on: [12] fully working and tested

**Resolution:**

---

## [15] Separate region selector into native `tao` window
**Status:** OPEN
**Files:** `ui/src/app.rs`, new `ui/src/region_selector.rs`, `ui/src/state.rs`

The research is clear: transparent WebView windows have platform-specific bugs (Windows white flash, Wayland compositor issues). The region selection overlay — a transparent fullscreen window with draggable handles — is exactly the kind of UI that breaks.

Create a dedicated `tao` window (no WebView) for region selection:

- Fullscreen, transparent, always-on-top
- Custom rendering of the selection rectangle and handles (use `tiny-skia` for 2D software rendering, or `wgpu` if it's already a dependency from [12])
- Mouse event handling directly through `tao`'s event loop
- When selection is confirmed, send the `CaptureRegion` back to the main Dioxus app via a channel

The main Dioxus window handles settings, export options, and caption editing. The region selector is a modal interaction — it appears, the user drags, it disappears and returns coordinates.

This also eliminates the `with_transparent(true)` requirement on the main window, which simplifies things considerably.

**Resolution:**

---

## [16] Platform-specific transparency and window fixes
**Status:** OPEN
**Files:** `ui/src/main.rs`, `ui/src/region_selector.rs`

Even with a native region selector ([15]), platform-specific window behavior needs handling:

**Windows:** The documented "forced resize hack" — resize to 1x1 then back to original size on creation to force transparency to initialize. Apply this in the region selector window creation.

**Linux/Wayland:** Request an RGBA visual explicitly. Some compositors (Hyprland, Sway) need explicit damage tracking. Test on GNOME, KDE, and Sway. If transparency is unreliable, fall back to a semi-opaque dark overlay (like OBS does).

**macOS:** Generally works. Ensure the window level is set to `NSScreenSaverWindowLevel` or equivalent to appear above the menu bar and dock.

Each fix should be cfg-gated and documented with the specific compositor/OS version where the issue was observed.

Depends on: [15]

**Resolution:**

---

## [17] Global hotkey integration
**Status:** OPEN
**Files:** `ui/src/main.rs`, `ui/src/hooks.rs`, `ui/src/state.rs`

Screen recording apps need keyboard shortcuts that work even when the app isn't focused. The user is recording their screen — by definition, another app has focus.

Use the `global-hotkey` crate (already compatible with `tao`):

- `Ctrl+Shift+R` (or `Cmd+Shift+5` on macOS to match system convention): start/stop recording
- `Escape`: cancel recording
- `Ctrl+Shift+E`: quick export

Register hotkeys at app startup. Unregister on quit. Handle the case where another app has already registered the same hotkey (log a warning, don't crash).

Connect hotkey events to the existing `AppState` signals so the UI updates when recording starts/stops via hotkey.

**Resolution:**

---

## [18] macOS: App bundle, entitlements, and notarization
**Status:** OPEN
**Files:** new `platform/macos/Info.plist`, new `platform/macos/entitlements.plist`, new `platform/macos/build.sh`, `Cargo.toml`

The research is explicit: macOS requires a proper app bundle with specific entitlements for screen capture. Without notarization, Gatekeeper blocks the app entirely on modern macOS.

Required:

**Info.plist:**
- `CFBundleIdentifier`: `com.mandygif.app`
- `NSScreenCaptureUsageDescription`: "MandyGIF needs screen recording access to capture your selected region."

**entitlements.plist:**
- `com.apple.security.app-sandbox`: false (screen recording requires non-sandboxed)
- `com.apple.security.cs.allow-jit`: true (if WebView needs it)

**Build script:**
- Compile universal binary (x86_64 + aarch64)
- Create `.app` bundle structure
- Sign with Developer ID
- `xcrun notarytool submit` for notarization
- Staple the ticket to the DMG

**Sequoia re-authorization:** Document for users that macOS 15+ will prompt to re-approve screen recording every 30 days. This is Apple policy, not a bug. Consider adding an in-app notification when permission is revoked.

**Resolution:**

---

## [19] Windows: MSIX packaging and manifest
**Status:** OPEN
**Files:** new `platform/windows/Package.appxmanifest`, new `platform/windows/build.ps1`

For Windows Store distribution or clean install/uninstall, package as MSIX.

**Manifest requirements:**
- Declare `graphicsCapture` capability if targeting WinRT APIs
- Set `dpiAwareness` to `perMonitorV2` for correct region coordinates on multi-DPI setups

**Standard distribution (non-Store):**
- MSI via WiX or `cargo-packager`
- Include Visual C++ redistributable if linking dynamically
- Code signing with an EV certificate (removes SmartScreen warning)

**The yellow border:** Document in user-facing help that Windows draws a yellow border around captured regions. This is a security feature and cannot be disabled.

**Resolution:**

---

## [20] Linux: Flatpak with PipeWire portal access
**Status:** OPEN
**Files:** new `platform/linux/com.mandygif.app.yml` (Flatpak manifest), new `platform/linux/com.mandygif.app.desktop`

Flatpak is the recommended distribution format for Linux screen recording apps because it integrates cleanly with XDG portals.

**Flatpak manifest must include:**
- `--socket=pipewire` (access to PipeWire)
- `--talk-name=org.freedesktop.portal.ScreenCast` (portal D-Bus access)
- `--device=dri` (GPU access for hardware encoding)

**Desktop file:**
- Set `X-Flatpak-RenamedFrom` if migrating from a non-Flatpak install
- Include appropriate categories and keywords

**Also provide:** AppImage and `.deb` for users who don't use Flatpak. `cargo-packager` can generate these.

Test on: Ubuntu 24.04 (GNOME/Wayland), Fedora 41 (GNOME/Wayland), Arch (KDE/Wayland), and at least one Sway setup.

**Resolution:**

---

## [22] `EncodeJob` params struct
**Status:** OPEN
**Files:** `core/encoder/src/video.rs`, `core/encoder/src/gif.rs`, `core/encoder/src/main.rs`

`encode_webp` takes 8 arguments. `encode_gif` takes 7. All share 6 common params. Create:

```rust
pub struct EncodeJob<'a> {
    pub input: &'a Path,
    pub trim: &'a TrimRange,
    pub fps: u32,
    pub scale: Option<u32>,
    pub captions: &'a [Caption],
    pub out: &'a Path,
}
```

Functions become:
```rust
pub fn encode_gif(job: &EncodeJob, loop_mode: &LoopMode) -> Result<()>
pub fn encode_mp4(job: &EncodeJob, quality: f32) -> Result<()>
pub fn encode_webp(job: &EncodeJob, quality: f32, lossless: bool) -> Result<()>
```

This is a prerequisite for [07] (ffmpeg-next integration) because the in-process encoder will need even more shared configuration. Better to have the struct now.

**Resolution:**

---

## [23] `OutputFormat` enum
**Status:** OPEN
**Files:** `core/protocol/src/types.rs`, `ui/src/processes.rs`

Replace `fmt: &str` in `build_encode_cmd` with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat { Gif, Mp4, Webp }

impl OutputFormat {
    pub fn extension(&self) -> &str {
        match self { Self::Gif => "gif", Self::Mp4 => "mp4", Self::Webp => "webp" }
    }
}
```

The match in `build_encode_cmd` becomes exhaustive. A typo is a compile error, not a silent wrong export.

**Resolution:**

---

## [24] `TrimRange` validation
**Status:** OPEN
**Files:** `core/protocol/src/types.rs`

`TrimRange { start_ms: 5000, end_ms: 1000 }` currently produces `-t -4.000` passed to ffmpeg, which either errors cryptically or produces a zero-length file.

Add validation:

```rust
impl TrimRange {
    pub fn duration_ms(&self) -> Option<u64> {
        self.end_ms.checked_sub(self.start_ms).filter(|&d| d > 0)
    }
}
```

Validate during deserialization with a custom `Deserialize` impl, or validate in `parse_encoder_command` and return an `InvalidInput` error. The encoder should never receive an invalid trim range.

**Resolution:**

---

## [25] `NormalizedFloat` for quality parameter
**Status:** OPEN
**Files:** `core/protocol/src/types.rs`, `core/encoder/src/video.rs`

`quality: f32` with value 1.5 produces `CRF = 51 - 49.5 = 1.5 → 2` (accidentally great quality). Value 2.0 produces `CRF = 51 - 66 = -15` which wraps via `as u32` to `4294967281`. That's a corrupted output.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "f32")]
pub struct NormalizedFloat(f32);

impl TryFrom<f32> for NormalizedFloat {
    type Error = String;
    fn try_from(v: f32) -> Result<Self, Self::Error> {
        if (0.0..=1.0).contains(&v) { Ok(Self(v)) }
        else { Err(format!("value {v} not in 0.0..=1.0")) }
    }
}
```

The CRF calculation becomes infallible. No clamping needed, no wrapping possible.

**Resolution:**

---

## [26] Wildcard imports → explicit named imports
**Status:** OPEN
**Files:** `core/encoder/src/main.rs`, `ui/src/processes.rs`

Replace `use mandygif_protocol::*` with the specific types used in each file. This is the lowest-priority governance item — do it last or as part of touching each file for other reasons.

**Resolution:**

---

## [27] Encoder progress reporting
**Status:** OPEN
**Files:** `core/encoder/src/gif.rs`, `core/encoder/src/video.rs`, `core/encoder/src/main.rs`

`EncoderEvent::Progress { percent }` exists in the protocol but is never emitted. The UI has no way to show export progress.

With `ffmpeg-next` ([07]), progress comes from the encoding loop directly — you know how many frames you've processed vs total. Emit `Progress` events every N frames or every 500ms.

Without [07] (still using ffmpeg CLI), use `-progress pipe:2` and parse `out_time_us` from stderr to compute percentage.

Either way, the event flows through JSONL stdout to the UI process.

**Resolution:**

---

## [28] Pingpong loop mode
**Status:** OPEN
**Files:** `core/encoder/src/gif.rs`

Currently logs a warning and falls through to normal looping. Implement by:

With ffmpeg CLI: Add `split[a][b];[b]reverse[r];[a][r]concat=n=2:v=1` to the filter graph before palettegen.

With `ffmpeg-next` ([07]): Buffer all decoded frames, then encode them forward followed by reversed. This doubles memory usage for the recording duration — document this tradeoff.

**Resolution:**

---

## [29] Test foundation
**Status:** OPEN
**Files:** multiple new test files across all crates

Create test coverage for the codebase. This is listed last not because it's unimportant, but because the architecture is about to change significantly ([07], [08], [12]). Writing extensive tests for code that will be replaced is waste.

**What to test now (stable interfaces):**
- Protocol types: round-trip serialization, edge cases, validation
- `TrimRange::duration_ms()` after [24]
- `NormalizedFloat` construction after [25]
- `OutputFormat::extension()` after [23]
- Recorder trait contract tests (mock impl)

**What to test after architecture settles:**
- `ffmpeg-next` encoding pipeline
- `wgpu` compositor output
- `iceoryx2` frame transport
- Integration tests: full capture → encode → verify output

Target ≥65% mutation kill rate on protocol and types crates. Use `cargo-mutants`.

**Resolution:**
