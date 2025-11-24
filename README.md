
# MandyGIF

**MandyGIF** is a high-performance, native screen recording and GIF creation tool written in Rust. It provides a modern, transparent overlay interface designed for capturing specific regions of the screen with pixel-perfect precision.

Unlike traditional screen recorders that use heavy Electron wrappers or separate selection windows, MandyGIF leverages a transparent Dioxus window that acts as both the viewport and the controller, offering a seamless "what you see is what you get" recording experience.

---

## Technical Architecture

MandyGIF is built as a Rust workspace comprising a unified UI and specialized backend cores.

### 1. Frontend (UI)
*   **Framework:** [Dioxus](https://dioxuslabs.com/) (Desktop).
*   **Rendering:** WebKit/WebView via `wry` and `tao`.
*   **Window Management:** Custom implementation for transparent, undecorated, always-on-top windows with pass-through input handling and native resizing logic.
*   **State Management:** Dioxus Signals and Context API.

### 2. Recording Core (`mandygif-recorder-linux`)
*   **Engine:** GStreamer.
*   **Integration:** Directly linked as a Rust library (no IPC latency).
*   **Pipeline:**
    *   **Source:** `ximagesrc` (X11 capture) with zero-copy pointers where supported.
    *   **Encoding:** Hardware-accelerated H.264 (via `x264enc`) wrapped in MP4 containers.
    *   **Synchronization:** Real-time PTS (Presentation Time Stamp) tracking for accurate duration UI updates.

### 3. Encoding Core (`mandygif-encoder`)
*   **Engine:** FFmpeg (spawned subprocess).
*   **Isolation:** Runs independently to prevent UI freezing during heavy rendering tasks.
*   **Capabilities:**
    *   **GIF:** 2-pass encoding with `palettegen` and `paletteuse` for high-quality dithering.
    *   **MP4:** Re-encoding with CRF (Constant Rate Factor) for optimization.
    *   **WebP:** Support for both lossy and lossless compression.
*   **Protocol:** Communicates via a strongly-typed JSONL protocol over `stdin`/`stdout`.

---

## Prerequisites (Linux)

MandyGIF currently targets Linux (X11). Ensure the following dependencies are installed:

### System Libraries
```bash
# Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    libssl-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.0-dev \
    libxdo-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-base \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly \
    ffmpeg
```

*Note: `gstreamer1.0-plugins-ugly` is required for the `x264` encoder.*

---

## Build & Run

The project is managed via Cargo.

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/junovhs/mandygif.git
    cd mandygif
    ```

2.  **Build the project:**
    ```bash
    cargo build --release
    ```

3.  **Run the application:**
    ```bash
    cargo run --bin mandygif
    ```

---

## Usage

1.  **Position:** Launch the app. Drag the window via the "MandyGIF" header or resize the edges to frame the content you wish to record.
2.  **Record:** Click the **RECORD** button. The border will turn red.
3.  **Stop:** Click **STOP** to finish capturing. The raw footage is saved temporarily to `/tmp/`.
4.  **Export:**
    *   Select your desired output format (GIF, MP4, or WebP).
    *   Click **EXPORT**.
    *   The final file will be generated in `/tmp/export.[ext]`.

---

## Project Status

| Component | Status | Notes |
| :--- | :--- | :--- |
| **UI** | ✅ Beta | Resizing, dragging, and state management fully functional. |
| **Linux Recorder** | ✅ Beta | X11 support verified. Wayland support pending. |
| **Encoder** | ✅ Stable | Supports High-Quality GIF, MP4, WebP. |
| **Windows Recorder** | 🚧 Planned | WGC (Windows Graphics Capture) implementation pending. |
| **macOS Recorder** | 🚧 Planned | ScreenCaptureKit implementation pending. |

## License

MIT / Apache-2.0
