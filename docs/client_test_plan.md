# Client Test Plan (AI Voice Client - Rust)

This document outlines the test strategy to ensure the Rust-based AI voice client daemon and interception plugin function correctly on the Linux desktop environment, handling native `evdev` inputs, GTK4 UI overlays, and Wayland integrations.

## 1. Automated Mock Testing (No User Needed)
- [ ] **JSON Contract Tests**: Write `#[test]` cases in `network.rs` to verify that `serde_json` successfully deserializes the standard `{"text": "...", "is_final": true}` payload and the dynamic download `{"status": "downloading", "progress_pct": 45.2}` payloads.
- [ ] **Daemon Graceful Reconnection**: Implement a mock WebSocket server on `localhost` in the tests to drop the connection intentionally. The daemon should gracefully log the error without crashing, and successfully reconnect on the next hotkey press.

## 2. Low-Level Input Interception (`/dev/input`)
**Manual Validation Strategy**: Because the `interception_plugin` requires `root` privileges to read raw input devices, this must be tested manually.
- [ ] **Hotkey Consumption**: Open a text editor. Hold `CTRL` and press `SPACE`. Verify that no "space" character appears in the editor, proving the plugin successfully intercepted and consumed the raw event from the OS.
- [ ] **Early Modifier Release Recovery**: Hold `CTRL`, hold `SPACE`, release `CTRL`, then release `SPACE`. The daemon must still log a UDP `RELEASE` signal to prevent the client from getting permanently stuck in the recording state.

## 3. Audio Capture & Pipewire
- [ ] **pw-record Execution**: Trigger the hotkey. Verify via `htop` or logs that the `tokio` process correctly spawns the `pw-record --format s16 --rate 16000 --channels 1 -` subprocess, and successfully kills it upon hotkey release.
- [ ] **Format Consistency**: Verify that the binary chunks piped over the WebSocket accurately decode as 16kHz 16-bit PCM (e.g. by dumping a local payload to a `.wav` header for manual inspection).

## 4. UI & Text Injection (GTK4 / Wayland)
- [ ] **OSD Anchoring & Visibility**: Trigger the hotkey. A GTK4 overlay reading "🎙️ Recording..." must appear instantly, anchored securely to the bottom-center of the screen, staying above all active windows without stealing keyboard focus. It must vanish immediately upon hotkey release.
- [ ] **Dynamic Download Feedback**: Send a simulated WebSocket `{"status": "downloading", "progress_pct": 45.2}` packet to the daemon. The GTK4 OSD must automatically pop up and update the label to `"📥 Downloading Model... 45.2%"`.
- [ ] **Ydotool Injection**: Simulate a successful server transcription payload (`{"text": "Hello world"}`). Focus a terminal or browser window. Ensure that `ydotool type` correctly injects the string directly into the active UI element.
