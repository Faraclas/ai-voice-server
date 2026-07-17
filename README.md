# AI Voice Server (Local Dictation Pipeline)

A completely self-hosted, highly accurate, and GPU-accelerated voice dictation pipeline. 
This project allows a user to dictate on a client machine and have the audio processed by a dedicated AI server (Gentoo Linux with an NVIDIA GPU), returning the text directly to the client's clipboard or auto-typing it into the currently focused window.

## Quick Start (Running the Server)

**Production Version (v2 - Rust):**
The new system-level Rust server is located in `src/server`. It uses `dotenvy` for local configuration.
```bash
cd ~/code/ai-voice-server/src/server
cp .env.example .env
# Edit .env to set BIND_ADDR=0.0.0.0 if connecting from a remote client
cargo run --release
```
*(By default, this loads the `small.en` Whisper model into GPU VRAM and listens on port `3000`)*

**Proof of Concept (v1 - Python):**
To start the legacy Python PoC, navigate to `python-prototype` and run the script:
```bash
cd ~/code/ai-voice-server/python-prototype
./server/start.sh
```

## Architecture
- **Server (v2):** A native Rust application (`src/server/`) using `axum` and `whisper-rs`. It is highly concurrent, gracefully degrades to CPU if the GPU is missing, and protects VRAM via a single-worker job queue.
- **Server (v1 PoC):** A FastAPI application (`python-prototype/server/server.py`) running `faster-whisper`.
- **Client:** A robust toggle-based dictation client (press and release hotkey to start, press and release again to stop) designed to run as a background Wayland daemon. It records audio, handles the WebSocket connection, and auto-types or copies the result to the clipboard.
  - **Dynamic Output Modes:** Switch between auto-typing and clipboard-copy modes on the fly using a secondary hotkey (default: Right Ctrl + Space).
  - **Auto-Typing Safety:** Automatically waits for physical modifier key releases before typing to prevent accidental shortcut triggers, supporting custom propagation delays for VMs.
  - **Rich Configuration:** Automatically generates a user config file at `~/.config/ai-voice-server/client.env` for customizing typing speeds, hotkeys, and server addresses. Hotkeys are defined using intuitive string names (e.g., `KEY_RIGHTCTRL`) rather than raw keycodes.
  - **Long-form Dictation:** Supports recording up to 10 minutes of audio in a single buffer, complete with visual "transcribing" UI states via layer-shell.

## Setup
- **Server (v2):** Configure via `.env` for local testing, or `/etc/conf.d/ai-voice-server` for production systemd deployments.
- **Client (v2):** See `src/client/README.md` for the Rust client dependencies (`interception-tools`, `ydotool`, `gtk4-layer-shell`). Client settings (like typing speed, output mode, and target server) can be customized via `~/.config/ai-voice-server/client.env`, which is automatically generated on first run.
- **Client (v1 PoC):** See `python-prototype/client/README.md` for detailed instructions on setting up the dictation hotkey and kernel-level auto-typing.

---

## Future Work

### LLM Formatting Engine (Voice Commands)
Currently, `faster-whisper` is a pure transcription model. It automatically adds proper punctuation (commas, periods, question marks, capitalization) based on speech inflection, but it does **not** natively understand explicit formatting commands like *"new paragraph"* or *"make a bulleted list"*.

**Proposed Architecture Upgrade:**
To add powerful voice formatting commands, we can introduce a lightweight Local LLM (Large Language Model) into the server pipeline:

1. **Transcription:** Whisper converts the raw audio to raw text.
2. **LLM Filtering:** The FastAPI server instantly passes that raw text to a local LLM (e.g., Llama 3 or Gemma running via Ollama on the same Gentoo GPU).
3. **Prompting:** The LLM is given a strict system prompt: *"You are a formatting assistant. Apply any formatting commands the user dictates. If they say 'new paragraph', insert `\n\n`. If they say 'make a bulleted list', format it appropriately. Do not change the original wording."*
4. **Delivery:** The perfectly formatted text is returned to the client and auto-typed.

### AMD GPU (ROCm) Support & Testing
While the server is designed to compile with AMD ROCm support (via the `rocm` Gentoo USE flag), we have not yet tested this on actual AMD hardware. The `ai-voice-server.sh` wrapper script and `ai-voice-server-9999.ebuild` currently include logic for ROCm detection and fallback, but a physical test on an AMD GPU is still needed to ensure the `whisper.cpp` ROCm backend compiles correctly inside the Portage sandbox and runs stably in a production environment.
