# AI Voice Server (Local Dictation Pipeline)

A completely self-hosted, highly accurate, and GPU-accelerated voice dictation
pipeline for Linux (Wayland). 

This system allows a user to dictate on a client machine via global hotkeys and
have audio processed in real-time by a dedicated AI server (Gentoo Linux with an
NVIDIA/AMD GPU or CPU fallback), returning transcribed text directly into the
focused window or Wayland clipboard.

---

## Features

- **Low-Latency Push-to-Talk:** Kernel-level input capture via
  `interception-tools` bypasses virtual machines and desktop environment limits.
- **Hardware Acceleration:** Native C++ Whisper inference via `whisper-rs` with
  support for CUDA, Vulkan, ROCm, and CPU fallback.
- **Universal Text Injection:** Simulates hardware keystrokes via `ydotool` to
  type text natively into any Wayland or X11 application.
- **On-Demand Streaming:** Audio streams over WebSockets directly to a
  VRAM-managed job queue with an automatic 60-second idle connection timeout.
- **Wayland GTK OSD:** Ephemeral visual overlay built with `gtk4-layer-shell`.
- **Gentoo Integration:** Distributed as a native Portage ebuild with systemd
  user and system service integration.

---

## System Architecture

- **Server (`src/server/`):** Async Rust service built on `axum` and
  `whisper-rs`. Features model auto-downloading from HuggingFace, dynamic
  hot-swapping via WebSocket/HTTP, and VRAM protection through a single-worker
  queue.
- **Client (`src/client/`):** Split-binary Rust architecture:
  - `interception_plugin`: Lightweight kernel input event grabber running under
    `udevmon`.
  - `daemon`: User-level Wayland GTK application managing audio, UI overlays,
    `ydotool` injection, and WebSocket streaming.
- **Packaging (`packaging/` & Gentoo Overlay):** Systemd units, udev rules, and
  Portage ebuild (`app-misc/ai-voice-server`) in `adaptive-overlay`.

---

## Quick Start & Installation

### Option A: Gentoo Overlay (Recommended)

1. Enable the system overlay and install the package:
   ```bash
   sudo emerge -av app-misc/ai-voice-server
   ```
2. Enable and start the required services:
   ```bash
   # Server (on GPU host)
   sudo systemctl enable --now ai-voice-server

   # Client (on workstation)
   sudo systemctl enable --now udevmon
   systemctl --user enable --now ydotool
   systemctl --user enable --now ai-voice-client
   ```

### Option B: Building From Source

```bash
# Build client and server binaries
cargo build --release --workspace

# Run the server locally
cd src/server && cargo run --release

# Run the client daemon
cd src/client && cargo run --release --bin daemon
```

---

## Configuration

- **Server Config:** Configured via `/etc/conf.d/ai-voice-server` (or `.env` in `src/server`). Options include `WHISPER_MODEL`, `PORT`, `BIND_ADDR`, and `GPU_MODE`.
- **Client Config:** Automatically generates `~/.config/ai-voice-server/client.env` on first launch. Allows customizing `AI_VOICE_SERVER_WS_URL`, `AI_VOICE_OUTPUT_MODE` (`type` or `clipboard`), and `ydotool` typing delays.

---

## Documentation & Roadmap

- **[ROADMAP.md](ROADMAP.md):** Planned upcoming features and enhancements.
- **[docs/architecture.md](docs/architecture.md):** Detailed system architecture design.
- **[docs/api-contract.md](docs/api-contract.md):** WebSocket and HTTP API specification.
- **[docs/server-plan.md](docs/server-plan.md):** Server implementation details.
- **[docs/client-plan.md](docs/client-plan.md):** Client implementation details.
