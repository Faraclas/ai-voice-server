# AI Voice Server: Project Roadmap

This roadmap outlines planned future features, enhancements, and upcoming
milestones for the AI Voice Server project. Completed features (such as the
Rust v2 client/server architecture, systemd services, and Portage ebuild) are
documented in the primary [README.md](README.md) and [docs/](docs/).

---

## 1. Audio Feedback Cues

- **Goal:** Provide subtle, non-intrusive auditory confirmation during push-to-talk dictation.
- **Details:** 
  - Play a lightweight sound cue (e.g. via PipeWire / `paplay` or native Rust
    audio buffer) when recording starts, stops, and successfully completes
    text injection.
  - Allows users to dictate confidently without needing to watch the GTK OSD.

---

## 2. LLM Formatting Engine (Voice Commands)

- **Goal:** Enable natural voice commands for text formatting (e.g., "new
  paragraph", "bulleted list", "capitalize next word").
- **Proposed Architecture:**
  - Whisper converts raw audio into unformatted text.
  - The server passes the transcribed text to a local LLM (e.g., Llama 3 or
    Gemma via Ollama/llama.cpp) with a strict system prompt.
  - The LLM applies formatting instructions without altering the original speech
    wording before returning it to the client.

---

## 3. Performance Benchmarking (CUDA vs. Vulkan)

- **Goal:** Conduct rigorous latency and throughput benchmarks between compute
  backends on the RTX 3060 Ti.
- **Details:**
  - Compare Vulkan (`--features vulkan`) vs. CUDA (`--features nvidia`) across
    various Whisper model sizes (`small.en`, `medium.en`, `large-v3`).
  - Measure VRAM allocation efficiency, warm-path inference latency, and cold
    model load times.

---

## 4. AMD / ROCm Hardware Validation & eGPU Hot-Plugging

- **Goal:** Physically test and validate ROCm builds on AMD GPU hardware.
- **Details:**
  - Verify that the `rocm` build backend (`hipblas`) compiles cleanly inside
    the Gentoo Portage sandbox.
  - Benchmark inference speed on AMD GPUs.
  - Test and verify `udev` rules for AMD eGPUs to ensure seamless auto-restart
    upon hot-plugging, matching the existing NVIDIA eGPU behavior.

---

## 5. Background Service System Tray Icon & Pop-up Menu

- **Goal:** Implement a system tray icon (`ksni` / `libappindicator` / GTK tray)
  that remains active in the system panel while `ai-voice-client` runs as a
  background systemd user service.
- **Tray Menu Options:**
  - **Status & Compute Header:** Display real-time connection status, active
    server URL, current compute backend (`CUDA`, `Vulkan`, `ROCm`, `CPU`), and
    loaded Whisper model.
  - **Output Mode Switcher:** A menu toggle to switch between Auto-Typing
    (`ydotool`) and Clipboard Copy (`wl-copy`), staying in sync with the
    `Right Ctrl + Space` hotkey.
  - **Settings & Config Editor:** Menu option to view and edit parameters (such
    as typing speeds, target WebSocket URL, and default model) in
    `~/.config/ai-voice-server/client.env`.
  - **Remote Server Restart Command:** Authenticated tray action sending
    `POST /restart` to trigger a server service restart/re-probe when booted on CPU.
  - **Server Web Interface & Admin Menu:** Menu entry to launch the server's web
    interface/documentation in the browser, plus an authenticated sub-menu to
    trigger model hot-swaps (`set_model` to `small.en`, `large-v3`, etc.)
    directly via API key.

---

## 6. Word-by-Word Interim Streaming Transcripts

- **Goal:** Stream partial transcription updates to the client in real-time as
  speech is being processed (`is_final: false`).
- **Details:**
  - Explore partial segment decoding in `whisper-rs`.
  - Update the client OSD to display live streaming words prior to final
    keystroke injection.
