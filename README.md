# MandyGIF

**MandyGIF** is a high-performance, native screen recording and GIF creation tool written in Rust. It brings the polished, glassmorphic aesthetic of modern macOS tools to Linux.

Unlike traditional recorders that use heavy Electron wrappers or separate selection windows, MandyGIF uses a transparent **Dioxus** overlay. The window *is* the viewport.

![MandyGIF UI](https://via.placeholder.com/800x600?text=MandyGIF+Glass+UI)

## ✨ Features

*   **WYSIWYG Recording:** The window frame defines exactly what gets captured.
*   **Modern UI:** Glassmorphism, floating control pill, and reactive resize handles.
*   **High-Performance Backend:**
    *   **Recording:** Zero-copy X11 capture via **GStreamer** (`ximagesrc` → `x264`).
    *   **Encoding:** High-quality **FFmpeg** pipeline for GIF (palette generation), WebP, and MP4.
*   **Sandboxed Architecture:** The UI, Recorder, and Encoder run in separate processes to ensure the UI never freezes during rendering.

## 🛠 Prerequisites (Linux)

MandyGIF currently targets **Linux (X11)**. Ensure the following system dependencies are installed:

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

## 🚀 Quick Start

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/junovhs/mandygif.git
    cd mandygif
    ```

2.  **Run the application:**
    ```bash
    cargo run --bin mandygif
    ```

## 🎮 How to Use

### 1. Position & Resize
*   **Move:** Click and drag the **"MandyGIF" bar** at the top of the window.
*   **Resize:** Hover over any edge or corner. You will see **white corner brackets** appear. Click and drag to define your capture region.
*   The interior will have a slight tint to show you exactly what will be recorded.

### 2. Record
*   Click the **Red Circle** in the floating control pill at the bottom.
*   The window border will turn **Red**.
*   The top drag bar will disappear to ensure a clean recording.

### 3. Stop
*   Click the **Stop (Square)** button in the pill.
*   The border will turn **Green** (Review Mode).

### 4. Export
*   Select your format: **GIF**, **MP4**, or **WebP**.
*   Click **Export**.
*   The file will be saved to `/tmp/export.[ext]`.

## 🏗 Architecture

The project is organized as a Rust Workspace:

| Crate | Description |
| :--- | :--- |
| **`ui`** | The Dioxus frontend. Handles the transparent window, state management, and spawns backend processes. |
| **`core/recorder-linux`** | GStreamer implementation. Captures screen coordinates to raw H.264/MP4. |
| **`core/encoder`** | FFmpeg implementation. Handles palette generation for GIFs and compression for WebP. |
| **`core/protocol`** | Shared JSONL types for Inter-Process Communication (IPC). |

## 🗺 Roadmap

- [x] **Linux (X11) Recorder**
- [x] **Glassmorphic UI**
- [x] **High-Quality GIF Engine**
- [ ] **Wayland Support** (via PipeWire)
- [ ] **macOS Support** (via ScreenCaptureKit)
- [ ] **Windows Support** (via Windows Graphics Capture)
- [ ] **Audio Capture** (PulseAudio/PipeWire)

## License

MIT / Apache-2.0
