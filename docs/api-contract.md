# AI Voice Server: API Contract

This document defines the interface between the Client (workstation) and the
Server (GPU machine). By establishing this contract first, both client and
server development can proceed independently.

## Base URL

All endpoints are relative to the server's base URL and port (e.g.,
`http://<server-ip>:3000`).

---

## 1. Real-Time Transcription Stream

**Endpoint:** `WebSocket /stream` **URL Scheme:** `ws://<server-ip>:3000/stream`

Establishes a persistent, full-duplex connection for real-time audio
transcription.

### Client -> Server (Messages)

The client streams audio data as it is recorded.

- **Binary Messages:** The client sends uncompressed **16 kHz, mono, 16-bit PCM (s16le)** audio chunks continuously as binary frames over the WebSocket.
- **Text Messages (JSON):** The client can optionally send JSON messages to
  configure the stream (e.g., `{"action": "set_model", "model": "medium.en"}`)
  or signal the end of a recording burst (e.g., `{"action": "end_stream"}`).

### Server -> Client (Messages)

The server processes the audio chunks in the Job Queue and streams the
transcribed text back as soon as it's ready.

- **Text Messages (JSON):**

```json
{
  "text": "This is the transcribed text.",
  "is_final": true,
  "processing_time_ms": 150
}
```

_Note: `is_final: false` can be used in the future if we implement partial
word-by-word streaming._

---

## 2. Server Status (Healthcheck)

**Endpoint:** `GET /health`

Used by the client to quickly verify that the server is online and ready to
accept transcription requests before attempting to record.

### Response (Success - 200 OK)

- **Content-Type:** `application/json`

```json
{
  "status": "ready",
  "gpu_active": true,
  "loaded_model": "medium.en"
}
```

---

## 3. Set Model (Admin/Scripting)

**Endpoint:** `POST /set_model`

Allows external scripts or tools to dynamically swap the active Whisper model without opening a WebSocket connection.

### Request Body

- **Content-Type:** `application/json`

```json
{
  "model": "large-v3"
}
```

### Response (Success - 200 OK)

```json
{
  "status": "success",
  "loaded_model": "large-v3"
}
```
