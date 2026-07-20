# AI Voice Server: Project Roadmap

**Current State: Stable Proof of Concept (PoC)**
The current bash and Python scripts in this repository work exceptionally well as a lightweight, functional Proof of Concept. To preserve a working baseline, we will **not** aggressively edit or mutate these existing scripts. 

Instead, this roadmap outlines the plan to use the lessons learned from the PoC to build a robust, system-level **Production Version (v2)** alongside it.

---

## The Production Vision (v2)

## Server Improvements

Currently, the server is run manually via SSH and requires an active session (or Tmux) to stay alive. It also assumes the external GPU is always present.

### 1. Systemd Integration
- **Goal:** Convert the server script into a background `systemd` service so it starts automatically and runs headlessly without an active SSH session.
- **Tasks:**
  - Create a `ai-voice-server.service` file.
  - Implement a configuration file (e.g., in `/etc/default/ai-voice-server` or a local `.env`) to easily change variables like the **model size** (base, small, medium) directly through systemd.

### 2. Hardware Detection (Graceful Degradation)
- **Goal:** Ensure the service doesn't crash or cause issues if the external NVIDIA GPU is not plugged in during boot.
- **Tasks:**
  - Add a pre-start check in the server script (or systemd `ExecStartPre`) to detect the NVIDIA GPU (e.g., using `nvidia-smi` or checking PCI devices).
  - If the GPU is not found, gracefully exit the service or fall back to a CPU-only mode (if desired).

### 3. Remote Administration API (Model Hot-Swapping)
- **Goal:** Allow remote clients to change the active model (e.g., switch to `large-v3`) dynamically without needing to SSH into the server to edit config files or restart the systemd service.
- **Tasks:**
  - Build authenticated endpoints into the Rust server (e.g. `POST /admin/model/swap`) protected by an `ADMIN_API_KEY` defined in the config.
  - Implement auto-downloading logic in the Rust server so it can automatically fetch missing `.bin` models from HuggingFace.
  - Implement seamless hot-swapping in the Rust server to drop the old model from GPU memory and load the new one on the fly.
  - Add admin CLI commands to the client-side Rust application to securely communicate with this new endpoint.

---

## Client Improvements

The original client relied on bash scripts (`dictate.sh`), standard GNOME desktop notifications, and desktop environment shortcuts, which caused clutter, workflow interruptions, and VM capture issues. We have replaced this with a native, robust Rust daemon.

### 1. Low-Level Hotkey Interception (`interception-tools`)
- **Problem:** GNOME keyboard shortcuts are captured by KVM virtual machines, requiring the user to manually click out of the VM to trigger dictation, and click back in to paste.
- **Goal:** Bypass the desktop environment's shortcut manager entirely so the dictation hotkey works globally with zero latency.
- **Status (Done):** Built a native, blazing-fast Rust plugin (`interceptor`) for `sys-apps/interception-tools` to grab the keyboard at the kernel level (`/dev/input`). It implements a true toggle logic (e.g. `Left_Ctrl + Space`), swallowing the hotkey before KVM sees it and triggering the daemon via UDP.

### 2. UI and Audio Feedback
- **Problem:** Standard `notify-send` notifications clutter the tray, don't show up reliably across all workspaces, and trigger in awkward orders.
- **Status (Partially Done):** Implemented a lightweight, native GTK OSD overlay directly in the Rust daemon using `gtk4-layer-shell` (with graceful fallback for vanilla GNOME Wayland).
- **Goal (Pending):** Provide immediate, non-intrusive auditory feedback. Replace the need for visual confirmation with short, subtle audio cues (e.g., a 'click' or 'beep' using PipeWire/`paplay`) that trigger instantly when recording starts, stops, and successfully pastes.

### 3. Auto-Pasting (`ydotool`)
- **Problem:** `dictate.sh` originally relied on `wl-copy`, which required clicking back into the target window to paste.
- **Goal:** Automatically inject the returned text into the active window identically across all OS contexts.
- **Status (Done):** Natively integrated `ydotool` directly into the Rust daemon's Tokio async runtime to instantaneously type the transcribed text with zero delay, bypassing all clipboard inconsistencies.

---

## Proposed Next Steps
1. **Server Stabilization:** (Done) The systemd unit, fail-safe timeouts, and graceful hardware checks are written.
2. **Client Foundation:** (Done) The Rust GTK daemon, Tokio networking, and `ydotool` auto-pasting are complete.
3. **Client Hotkeys:** (Done) The native `interceptor` plugin is integrated and functioning perfectly as a toggle.
4. **Audio Cues:** (Pending) Add `paplay` sound effects to the daemon.
5. **Future Work:**
- Add the LLM formatting engine mentioned in the original README.
- **Performance Benchmarking (CUDA vs Vulkan):** While the Vulkan backend compiles smoothly and performs well natively, we need to revisit and benchmark a proper CUDA compile (`--features nvidia`) once the CUDA toolkit is installed. The goal is to rigorously compare latency and throughput to see if Vulkan truly matches CUDA on the RTX 3060 Ti for `whisper.cpp` workloads.
- **AMD / ROCm Support & eGPU Hot-plugging:** The `rocm` build backend has not yet been physically tested on AMD hardware. Crucially, we have not tested or designed udev rules for AMD eGPUs. When we eventually test AMD support, we will need to identify the correct udev attributes (e.g., AMD vendor IDs) to ensure the service gracefully auto-restarts upon hot-plugging, just like it currently does for NVIDIA.

## Packaging (Gentoo Ebuilds)
- **Goal:** Package the entire project into the user's personal Gentoo overlay for native package management.
- **Tasks:**
  - Create an ebuild with `client` and `server` USE flags.
    - **Note:** Ensure that the ebuild explicitly requires `x11-misc/ydotool` as a dependency if `USE=client` is set.
    - **Note:** Ensure that the ebuild explicitly requires `app-misc/interception-tools` as a dependency if `USE=client` is set.
    - **Note:** Explicitly enforce that `systemd`, `wayland`, and `pipewire` are REQUIRED environment configurations. OpenRC, X11, and raw ALSA/PulseAudio are unsupported.
  - Move the repository to the final overlay location once the structure is finalized.
