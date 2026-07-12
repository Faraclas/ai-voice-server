# AI Voice Server (Local Dictation Pipeline)

A completely self-hosted, highly accurate, and GPU-accelerated voice dictation pipeline. 
This project allows a user to dictate on a client machine and have the audio processed by a dedicated AI server (Gentoo Linux with an NVIDIA GPU), returning the text directly to the client's clipboard or auto-typing it into the currently focused window.

## Quick Start (Running the Server)
To start the AI transcription server on the Gentoo machine, simply navigate to this directory and run the start script:
```bash
cd ~/code/ai-voice-server/python-prototype
./server/start.sh
```
*(By default, this will automatically load the `small.en` Whisper model into your GPU VRAM and start listening on port `8000` across your local network).*

## Architecture
- **Server:** A FastAPI application (`python-prototype/server/server.py`) running `faster-whisper`. It keeps the AI model loaded in VRAM for instant, sub-second transcription.
- **Client:** A push-to-talk bash script (`python-prototype/client/dictate.sh`) designed to be bound to a global system hotkey on a Wayland desktop. It records audio, handles the API request, and auto-pastes the result.

## Setup
- **Server:** Run `./server/start.sh` from the `python-prototype` directory on the Gentoo machine to launch the API.
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
