# AI Voice Server - Rust Client

This directory contains the native Rust client daemon and kernel hotkey
plugin for the AI Voice Dictation system.

---

## Dependencies

When installing via Portage (`emerge -av app-misc/ai-voice-server`), these
dependencies are managed automatically by the ebuild. For manual source builds,
ensure the following system packages are installed:

1. **`gui-libs/gtk4-layer-shell`**
   - **Purpose:** Draws the Wayland-native GTK OSD overlay. Required for GTK
     layer shell bindings.
2. **`app-misc/interception-tools`**
   - **Purpose:** Low-level `evdev` event manipulation framework. Runs the
     `interception_plugin` binary under `udevmon` for global hotkey grabbing.
3. **`x11-misc/ydotool`**
   - **Purpose:** Kernel-level uinput keystroke simulator for auto-typing text
     into active applications on Wayland.

---

## Running & Services

When installed via Portage, the client components run as background services:

```bash
# Start input interception daemon (root)
sudo systemctl enable --now udevmon

# Start ydotool virtual keyboard daemon (user)
systemctl --user enable --now ydotool

# Start AI Voice Client overlay daemon (user)
systemctl --user enable --now ai-voice-client
```

---

## Features & System Tray Indicator

The client daemon exposes a GNOME-compatible StatusNotifierItem system tray app
indicator (`ksni`) featuring live color-coded status states:

- ⚪ **`microphone-sensitivity-high-symbolic`:** Idle / Ready (Server Online).
- 🔴 **`media-record`:** Active audio recording burst.
- 🟡 **`dialog-warning`:** Warm WebSocket stream open (60-second idle keep-alive).
- ❌ **`dialog-error`:** Server disconnected or unreachable.

### System Tray Actions
- **🌐 Open Server Web Page:** Opens the server Web Dashboard (`/admin`).
- **⚙️ Edit Client Config:** Opens `~/.config/ai-voice-server/client.env` in text editor.
- **🔄 Restart Local Client Service:** Restarts `ai-voice-client` user service via systemctl.
- **⚡ Restart Remote Server Service:** Authenticated `POST /restart` to remote server.

---

## Configuration

Configured via `~/.config/ai-voice-server/client.env` (or legacy `client.conf`):

- `AI_VOICE_SERVER_WS_URL`: Full WebSocket URL (e.g. `ws://192.168.0.205:3000/stream`).
- `SERVER_HOST`: Server IP/hostname (e.g. `192.168.0.205`).
- `SERVER_PORT`: Server port (e.g. `3000`).
- `AI_VOICE_ADMIN_API_KEY`: Secret key for remote server restarts & model swapping.
- `AI_VOICE_OUTPUT_MODE`: `type` (simulated keystrokes) or `clipboard` (`wl-copy`).
- `AI_VOICE_TYPING_DELAY`: Keystroke delay in ms.
- `AI_VOICE_TYPING_HOLD`: Keystroke hold duration in ms.

---

## Manual Building

```bash
cargo build --release
```
