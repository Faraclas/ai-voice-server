# Client Implementation Plan

(To be filled out by the Client Agent)

## 1. Core Requirements

- Must integrate deeply with the Wayland desktop via `interception-tools` for
  global kernel-level hotkeys (bypassing VMs).
- Must continuously record and stream uncompressed 16 kHz mono 16-bit PCM (s16le) audio chunks over WebSockets to the server
  for batch transcription at the end of the burst. (Note: Audio hardware is ONLY active when
  recording is toggled on. The WebSocket connection uses an internal client-side
  idle timer to stay alive for fast subsequent dictations, shutting down
  gracefully after inactivity).
- Must support a toggle-based hotkey mode (press to start, press to stop) by
  default, freeing up the user's hands while dictating.
- Must provide a dual-element lightweight graphical UI:
  1. **Floating OSD (On-Screen Display):** An ephemeral popup that appears only
     while recording is active to provide clear visual confirmation, and
     vanishes when recording stops.
  2. **System Tray Icon (AppIndicator):** Remains alive in the background for
     the duration of the keep-alive timeout. Clicking it opens a menu to switch
     output modes or access parameters without needing extra hotkeys.
- Must provide subtle audio feedback (beeps/clicks) to confirm recording
  start/stop when the user isn't looking at the UI.
- Must support three configurable text output modes:
  1. **Buffered Clipboard (Default):** Audio is transcribed in the background.
     Stopping the recording copies the text to the Wayland clipboard
     (`wl-clipboard`), allowing the user to manually paste it.
  2. **Buffered Auto-Type:** Audio is transcribed in the background. Stopping
     the recording instantly types the buffered text into the currently active
     window via `ydotool`.

## 2. Technical Stack

- **Language:** Rust
- **Audio Capture:** PipeWire (via `pipewire-rs`)
- **Text Injection:** `ydotool` (simulates a virtual keyboard via `/dev/uinput`
  to universally inject text, natively bypassing GNOME's strict Wayland security
  policies).
- **Hotkey Capture:** `interception-tools` (kernel-level evdev capture, ensuring
  hotkeys are intercepted even when VMs have input focus).

## 3. Implementation Steps

1. Setup project structure and dependencies.
2. Implement WebSocket network client for real-time streaming of audio chunks.
3. Integrate `interception-tools` logic for hotkey capture.
4. Implement audio recording and feedback cues.
5. Implement auto-pasting logic.
6. Create Gentoo ebuild.
