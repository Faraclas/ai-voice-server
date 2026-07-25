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

## Manual Building

```bash
cargo build --release
```
