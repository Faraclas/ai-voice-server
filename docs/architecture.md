# AI Voice Server: System Architecture

## Overview
The AI Voice Server is a split-architecture dictation system designed for high performance, low latency, and deep system integration on Linux (Wayland). 
It consists of a lightweight **Client** running on the user's primary workstation and a dedicated **Server** running on a Gentoo Linux machine equipped with an NVIDIA GPU.

## 1. The Pipeline
1. **Trigger:** The user presses a global hotkey on the Client to toggle recording on.
2. **Record:** The Client intercepts the hotkey, activates the microphone, and begins recording.
3. **Transmit:** The Client streams audio chunks over a WebSocket connection to the Server in real-time.
4. **Stop:** The user presses the hotkey again to toggle recording off, signaling the end of the stream.
5. **Transcribe:** The Server processes the audio chunks through a Whisper AI model loaded in GPU VRAM.
6. **Format (Future):** The raw transcription is passed to a local LLM for formatting (e.g., executing commands like "new paragraph").
7. **Return & Inject:** The Server returns the final text payload to the Client, which immediately injects it into the active window.

## 2. Communication Protocol
The system uses a WebSocket connection for low-latency, real-time streaming.
- **Endpoint:** `WebSocket /stream`
- **Payload:** Raw audio chunks (PCM/FLAC) streamed over binary messages.
- **Response:** JSON messages containing the transcribed text and processing metrics.

## 3. Division of Responsibilities

### Client (Workstation)
- **Low-Level Input:** Must capture keyboard events before the Desktop Environment or VMs using `interception-tools`.
- **Audio Capture:** Efficiently record from the default microphone.
- **Audio Feedback:** Play subtle, non-visual audio cues indicating recording state and completion.
- **Auto-Pasting:** Utilize `ydotool` or Wayland clipboard managers to simulate keyboard input to inject the text into the active window.
- **Language:** Compiled language (Rust or Go) for speed and easy distribution as a single binary.

### Server (GPU Machine)
- **Web Server:** A fast concurrent web server to handle incoming transcription requests.
- **AI Inference Engine:** A compiled binding to `whisper.cpp` (or similar) to execute transcription natively on the NVIDIA GPU without the overhead of the Python runtime.
- **Model Management:** Ability to dynamically load, unload, and switch Whisper models via API.
- **System Service:** Managed via a resilient `systemd` unit with hardware detection.
- **Language:** Compiled language (Rust or Go) for maximum performance and efficient memory usage.

## 4. Development Workflow
- The **Client** development and documentation will take place in `docs/client-plan.md` and the `src/client/` directory.
- The **Server** development and documentation will take place in `docs/server-plan.md` and the `src/server/` directory.
- Both components will ultimately be packaged into Gentoo ebuilds for deployment.
