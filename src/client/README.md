# AI Voice Server - Rust Client (v2)

This is the next-generation native Rust client for the AI Voice Dictation system.

## Dependencies

To build and run this client (especially on a Gentoo Linux host), you will need to install the following system dependencies:

1. **`gtk4-layer-shell`**
   - **Purpose:** Used to draw the Wayland native overlay GUI. It is required for the Rust client to compile.
   - **Gentoo Package:** `gui-libs/gtk4-layer-shell`
   - **Install:** `sudo emerge -av gui-libs/gtk4-layer-shell`

2. **`interception-tools`**
   - **Purpose:** A low-level `udev`/`evdev` manipulation framework. The client uses a custom interception plugin to capture the dictation hotkey at the hardware level (allowing it to work across VM boundaries) and to safely inject the transcribed text back into the input stream.
   - **Gentoo Package:** `app-misc/interception-tools`
   - **Install:** `sudo emerge -av app-misc/interception-tools`

3. **`ydotool`**
   - **Purpose:** Provides kernel-level auto-typing of the transcribed text (bypassing strict Wayland security policies where tools like `xdotool` fail). It may be used alongside or as an alternative to the interception plugin for text injection.
   - **Gentoo Package:** `x11-misc/ydotool`
   - **Install:** `sudo emerge -av x11-misc/ydotool`

## Building

```bash
cargo build --release
```
