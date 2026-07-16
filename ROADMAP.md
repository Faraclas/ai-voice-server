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

The client currently relies on standard GNOME desktop notifications and desktop environment shortcuts, which causes clutter and workflow interruptions, especially when using VMs.

### 1. Low-Level Hotkey Interception (`interception-tools`)
- **Problem:** GNOME keyboard shortcuts are captured by KVM virtual machines, requiring the user to manually click out of the VM to trigger dictation, and click back in to paste.
- **Goal:** Bypass the desktop environment's shortcut manager entirely so the dictation hotkey works globally with zero latency.
- **Tasks:**
  - Utilize `sys-apps/interception-tools` (a fast, production-ready C daemon) to grab the keyboard at the kernel level (`/dev/input`).
  - Configure an exec plugin (like `interception-tools-exec`) to watch for the dictation hotkey, swallowing it before it reaches KVM, and triggering the `dictate.sh` bash script.

### 2. Audio Feedback (Replacing GNOME Notifications)
- **Problem:** Standard notifications clutter the tray, don't show up reliably across all workspaces, and trigger in awkward orders. Wayland makes cross-workspace global visual overlays extremely difficult to implement reliably.
- **Goal:** Provide immediate, non-intrusive feedback without visual clutter.
- **Tasks:**
  - Replace `notify-send` with short, subtle audio cues (e.g., a 'click' or 'beep' using `paplay` or `aplay`) that trigger instantly when recording starts, and when it successfully pastes.
  
### 3. Auto-Pasting (`ydotool`)
- **Problem:** Manually pasting requires clicking back into the target window.
- **Goal:** Automatically inject the returned text into the active window.
- **Tasks:**
  - Integrate `ydotool` into the `dictate.sh` script to simulate the `Ctrl+V` keystrokes after updating the Wayland clipboard via `wl-copy`.

---

## Proposed Next Steps
1. **Server Stabilization:** (Done) The systemd unit with graceful NVIDIA hardware detection is written.
2. **Client Hotkeys:** Configure `interception-tools` to handle the global kernel-level hotkey.
3. **Client UI & Pasting:** Update `dictate.sh` to use audio cues and `ydotool`.
4. **Future Work**
- Add the LLM formatting engine mentioned in the original README.
- **Performance Benchmarking (CUDA vs Vulkan):** While the Vulkan backend compiles smoothly and performs well natively, we need to revisit and benchmark a proper CUDA compile (`--features nvidia`) once the CUDA toolkit is installed. The goal is to rigorously compare latency and throughput to see if Vulkan truly matches CUDA on the RTX 3060 Ti for `whisper.cpp` workloads.

## Packaging (Gentoo Ebuilds)
- **Goal:** Package the entire project into the user's personal Gentoo overlay for native package management.
- **Tasks:**
  - Create an ebuild with `client` and `server` USE flags.
  - Move the repository to the final overlay location once the structure is finalized.
