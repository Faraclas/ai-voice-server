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
- **Client:** A push-to-talk bash script (`python-prototype/client/dictate.sh`) designed to be bound to a global system hotkey on a Wayland desktop. It records audio, handles the API request, and auto-pastes the result.

## Setup
- **Server (v2):** Configure via `.env` for local testing, or `/etc/conf.d/ai-voice-server` for production systemd deployments.
- **Client:** See `python-prototype/client/README.md` for detailed instructions on setting up the dictation hotkey and kernel-level auto-typing.

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
