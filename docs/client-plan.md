# AI Voice Server: Client Implementation Plan & AppIndicator Design

This document defines the architecture, requirements, and design specification
for the native Rust client daemon (`ai-voice-client`), low-level input
plugin (`interceptor`), and desktop AppIndicator system tray service.

---

## 1. System Architecture & Boundaries

The client consists of two binaries to preserve root security boundaries:

- **`interception_plugin` (Root / `udevmon`):** A lightweight `evdev` event
  grabber running under `sys-apps/interception-tools`. Intercepts raw keyboard
  events (`/dev/input`) to support global hotkeys across KVM virtual machines,
  sending UDP signals (`PRESS`, `RELEASE`, `MODIFIER_UP`) to local loopback.
- **`daemon` (User / Systemd User Service):** The main GTK4 and Tokio application
  running as `ai-voice-client.service`. Manages audio recording via `pw-record`,
  WebSocket streaming to the server, `ydotool` keystroke injection, GTK layer-shell
  OSD overlay, and the system tray AppIndicator.

---

## 2. AppIndicator / System Tray Design (`ksni` Pure Rust)

### A. Technology Selection

- **Framework:** Pure Rust [`ksni`](https://crates.io/crates/ksni) crate.
- **Protocol:** `org.kde.StatusNotifierItem` D-Bus protocol (native Linux tray
  standard, supported out-of-the-box by GNOME via `gnome-shell-extension-appindicator`
  and KDE/Sway/Hyprland).
- **Rationale:** Pure Rust execution over D-Bus via `zbus`. Requires zero C-library
  dependencies (`libappindicator3` unnecessary), integrates directly into the
  daemon's existing Tokio runtime, and shares in-memory state (`Arc<RwLock<Config>>`).

---

### B. Authentication & Admin Security Model

All server-side configuration changes (such as model hot-swapping, remote
server parameter editing, and remote service restarts) require authentication
via `ADMIN_API_KEY`.

1. **Config Key Storage:** The client looks for `AI_VOICE_ADMIN_API_KEY` in the
   user's local config file (`~/.config/ai-voice-server/client.env`).
2. **Interactive Auth Prompt:** If `AI_VOICE_ADMIN_API_KEY` is missing or invalid
   when the user clicks a protected admin action (e.g. model swap, server config
   editor, or restart), the client pops up a GTK password/key prompt asking the
   user to enter the `ADMIN_API_KEY`.
3. **Modal Guard:** The client maintains an `is_auth_modal_open` atomic boolean to
   prevent duplicate GTK auth dialogs if a user clicks multiple protected tray items.
4. **Optional Persistence & File Security:** The auth prompt dialog includes a checkbox:
   `[ ] Remember key in ~/.config/ai-voice-server/client.env`.
   - **If Checked:** The key is saved into `client.env` with strict `0600` user-only
     file permissions.
   - **If Unchecked:** The key is held only in memory for the current session,
     allowing visiting administrators on shared workstations to execute actions
     without leaving their secret key stored on disk.
5. **Unauthorized (401) Recovery:** If an API call fails with `401 Unauthorized`,
   the client clears any cached in-memory key and automatically re-prompts.

---

### C. Tray Menu Functionality & Layout

The AppIndicator tray menu provides full operational control over the client
daemon and remote server interactions:

```text
┌─────────────────────────────────────────────────────────────┐
│ 🎙️ AI Voice Client                                           │
│ Server: Connected (ws://127.0.0.1:3000)                     │
│ Compute: CUDA (NVIDIA RTX 3060 Ti)                          │
│ Model: small.en                                             │
├─────────────────────────────────────────────────────────────┤
│ Output Mode                                                │
│   (*) Auto-Typing (ydotool)                                 │
│   ( ) Clipboard Copy (wl-copy)                             │
├─────────────────────────────────────────────────────────────┤
│ Typing Speed Profile                                        │
│   ( ) Fast (2ms delay / 2ms hold)                           │
│   (*) Normal (5ms delay / 5ms hold)                         │
│   ( ) VM Safe (15ms delay / 10ms hold)                      │
├─────────────────────────────────────────────────────────────┤
│ Remote Model Hot-Swap (Requires Admin Key)                 │
│   - small.en                                                │
│   - medium.en                                               │
│   - large-v3                                                │
├─────────────────────────────────────────────────────────────┤
│ Remote Server Configuration (Requires Admin Key)            │
│   - Hardware Mode Policy (auto / require)                   │
│   - Device Priority (cuda, vulkan, cpu)                     │
│   - Max Queue Depth (10)                                    │
├─────────────────────────────────────────────────────────────┤
│ 🌐 Open Server Web Page / Docs                              │
│ ⚡ Restart Remote Server Service (Requires Admin Key)        │
│ ⚙️ Edit Local Client Config (~/.config/...)                 │
│ 🔄 Test Server Connection                                   │
├─────────────────────────────────────────────────────────────┤
│ Quit Client                                                 │
└─────────────────────────────────────────────────────────────┘
```

---

### D. Feature Specifications

#### 1. Server Heartbeat & Visual Status Indicators

- **Background Heartbeat:** The client runs a lightweight background polling
  loop in Tokio that queries `GET /health` once every **30 seconds** (or when a
  reconnect/restart is triggered).
- **Non-Interference Guarantee:**
  - `GET /health` runs on an isolated async task and never blocks or interacts
    with active WebSocket streaming or GTK UI rendering.
  - If a WebSocket stream is currently active (e.g. during dictation), the HTTP
    heartbeat poll is automatically skipped, since active streaming already
    proves connection health. Reset interval timer after stream completion.
- **Tray Icon Visual States:**
  - **Connected / Ready (Green/Active Icon):** `GET /health` returns `200 OK`
    and `status: "ready"`. Tooltip shows host, active compute device (`CUDA`/`CPU`),
    and model size.
  - **Disconnected / Offline (Yellow/Warning Icon):** Server is unreachable or
    `GET /health` failed.
- **Offline Hotkey Warning & Stale Fallback:**
  - **Normal Online Toggle:** Press 1 starts recording (`🎙️ Recording...`),
    Press 2 stops recording and transcribes.
  - **Offline Behavior:** Press 1 checks server state. If offline, audio capture
    is bypassed and the GTK OSD pops up with `⚠️ Server Unavailable`, auto-hiding
    after 1.5 seconds.
  - **Implementation Simplicity:** A 3-line `if/else` check at the start of the GTK
    hotkey handler (`main.rs`). Requires zero state machinery changes and completely
    avoids forcing the user to press the hotkey a 2nd time just to clear an error.

#### 2. Remote Service Restart & Status Recovery

- **Restart Trigger:** Clicking `⚡ Restart Remote Server Service` sends an
  authenticated `POST /restart` request using `AI_VOICE_ADMIN_API_KEY`.
- **Status Transition:**
  1. The client immediately marks connection status as `Restarting...` and sets
     the tray icon to the yellow warning state.
  2. The server exits and systemd restarts it, re-probing GPU hardware.
  3. The client's background heartbeat polls `GET /health` every 1–2 seconds during
     the restart window.
  4. As soon as the server comes back online, the client parses the updated
     `active_device` payload (e.g. `CUDA`) and updates the tray icon back to green,
     showing a desktop notification: `Server Online! Compute: CUDA (NVIDIA)`.

#### 3. Remote Server Parameter Editor (Authenticated)

- **Config Sub-menu:** Allows remote admins to view and modify static server
  parameters (`GPU_MODE`, `DEVICE_PRIORITY`, `MAX_QUEUE_DEPTH`) via `GET /config`
  and `POST /config`.
- **Apply & Restart Workflow:** Updating parameters saves them to the server
  configuration (`/etc/conf.d/ai-voice-server` or `.env`) and offers an immediate
  option to trigger `POST /restart` to apply the changes.

#### 4. Output Mode Selector

- Radio check items toggling between `Auto-Typing` (`ydotool`) and `Clipboard Copy` (`wl-copy`).
- Synchronized dynamically with the secondary hotkey (`Right Ctrl + Space`). Changing
  the mode in the menu updates the daemon in-memory state and notifies the OSD.

#### 5. Typing Speed Presets

- Allows switching `ydotool` key delay and hold speeds on the fly without
  restarting the service.
- Options for Fast (2ms), Default (5ms), and VM Compatibility (15ms delay for slow
  guest OS input queues).

#### 6. Remote Model Hot-Swapping (Authenticated)

- Sends an authenticated JSON command (`POST /set_model` or WebSocket message with
  `Authorization: Bearer <ADMIN_API_KEY>`) to change the active VRAM model on the
  fly (`small.en`, `medium.en`, `large-v3`).
- Prompts for the admin key if missing from `client.env`, with optional persistence.
- Displays download progress in the GTK OSD if the server needs to fetch missing
  GGUF weights.

#### 7. External Actions & Zero-Overhead Config Loading

- **Open Server Web Page:** Launches `http://<server-host>:<port>` in the default
  browser using `xdg-open` / `open` crate.
- **Edit Local Client Config:** Opens `~/.config/ai-voice-server/client.env` in the user's
  preferred text editor (`$EDITOR` or `xdg-open`). Re-reads config settings on-demand when
  editing or changing options—no background file watcher required, ensuring zero CPU/kernel overhead.
- **Test Server Connection:** Sends a `GET /health` probe to the server and triggers a
  desktop notification with latency, active compute device, and model status.

---

## 3. Technical Implementation Roadmap

1. **Add `ksni` Dependency:** Add `ksni = "0.2"` to `src/client/daemon/Cargo.toml`.
2. **Implement `tray.rs`:** Create a `ksni::Tray` struct bound to shared `Config`
  and status channels.
3. **Heartbeat Loop:** Implement 30s Tokio interval polling `GET /health` (skipping
  during active streams) and updating icon/tooltip state.
4. **Auth Prompt GUI:** Implement interactive GTK input dialog for missing `AI_VOICE_ADMIN_API_KEY`
  with modal guard and optional `0600` persistence.
5. **Spawn in Tokio:** Launch `ksni::TrayService` inside the Tokio async runtime in `main.rs`.
6. **State Synchronization:** Connect hotkey events, `GET /health` compute status, and D-Bus
  tray actions to update shared atomic state.
