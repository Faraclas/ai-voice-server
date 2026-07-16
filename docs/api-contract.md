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

---

## 4. Connection Lifecycle & Keep-Alive

To ensure ultra-low latency while minimizing server resource usage, the WebSocket connection for `/stream` adheres to a strict keep-alive lifecycle:

### Client Responsibilities
1. **Connect on Demand:** The client opens the WebSocket connection on the very first hotkey press (or pre-warms it if desired).
2. **Persistent Reuse:** After sending an `{"action": "end_stream"}` message and receiving the transcription JSON, the client **keeps the connection open**. Subsequent dictation bursts are sent over this exact same connection.
3. **Idle Timer (Graceful Close):** The client maintains an internal idle timer (e.g., 60 seconds). If no dictation occurs within this window, the client gracefully closes the WebSocket connection to free up server resources.

### Server Responsibilities
1. **Stay Open:** The server **must not** close the WebSocket connection after sending the transcription JSON back to the client. It must return to listening for the next incoming binary audio chunk on the same socket.
2. **Fail-Safe Timeout:** The server **must** implement a hard timeout (e.g., 5 minutes of no incoming audio frames) to forcefully drop the connection. This protects the server from zombie connections if a client application crashes or loses network connectivity without sending a graceful close signal.
