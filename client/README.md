# Voice-to-Text Client

This is the push-to-talk client script for the AI Voice Dictation system. 
It records your voice, sends it to the Gentoo server, and copies the transcribed text to your Wayland clipboard.

## Basic Setup on GNOME Wayland
1. Install dependencies: `sudo emerge -av wl-clipboard jq alsa-utils libnotify`
2. Make script executable: `chmod +x dictate.sh`
3. Map `dictate.sh` to a global hotkey in GNOME Settings -> Keyboard -> Custom Shortcuts.

---

## Enabling Auto-Typing (Phase 4)
By default, the script places the transcribed text into your clipboard and sends a notification, requiring you to manually press `Ctrl+V` to paste. 

To have the script automatically type the text for you (simulating keystrokes), you must use a tool called `ydotool`. Traditional key-simulators like `xdotool` do not work on GNOME Wayland due to strict security protocols. `ydotool` bypasses this by creating a virtual hardware keyboard at the kernel level.

### Auto-Typing Setup Instructions:

1. **Install ydotool**:
   ```bash
   sudo emerge -av ydotool
   ```

2. **Start the background daemon**:
   Because it interacts with `/dev/uinput` at the kernel level, it requires a daemon. For a quick test, simply run:
   ```bash
   sudo ydotoold &
   ```
   *(For a permanent setup, add your user to the `input` system group and enable `ydotoold` as an OpenRC/systemd service so it runs on boot without requiring `sudo`).*

3. **Update the Script**:
   Open `dictate.sh` and uncomment the `ydotool` line near the bottom of the file:
   ```bash
   ydotool key 29:1 47:1 47:0 29:0
   ```
   *(Note: This sends hardware scan-codes simulating pressing and releasing `Left Ctrl` (29) and `v` (47)).*

Once completed, pressing your hotkey a second time to stop recording will instantly paste the transcribed text directly into whatever window your cursor is currently focused on!
