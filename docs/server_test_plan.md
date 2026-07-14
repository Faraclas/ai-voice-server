# Server Test Plan (AI Voice Server - Rust)

This document outlines the test strategy to ensure the Rust-based AI voice server is functioning correctly, handling hardware acceleration gracefully, and responding appropriately to the client agent.

## 1. Startup & Model Initialization
- [ ] **Missing Model Handling**: Start the server without the `.gguf` files. Ensure the internal worker thread logs the error cleanly (which we know it does) AND that a subsequent request to `/health` reports `status: "error"`. 
- [ ] **Native Model Downloading**: The server will natively handle downloading missing models. Test that when `/set_model` is called with a model that isn't on disk, the server downloads it from HuggingFace, saves it to `./models`, streams progress to the client via WebSockets, and successfully initializes the `WhisperEngine` upon completion.
- [ ] **Hardware Probing**: Ensure `nvidia-smi` is detected and the server assigns `active_device: "cuda"` and `use_gpu: true`. 
- [ ] **Graceful GPU Fallback**: Mock the absence of CUDA (e.g., rename `nvidia-smi`), and verify the server skips CUDA and gracefully falls back to Vulkan without panicking.
- [ ] **GPU Requirement Override**: Set `GPU_MODE=require` in the `.env` file and mock the absence of ALL GPUs (CUDA, ROCm, Vulkan). Only in this scenario should the server hard panic and refuse to start.

## 2. Health & REST Endpoints
- [ ] **GET `/health` (Ready State)**: Should return `{"status": "ready", "gpu_active": true, "loaded_model": "small.en"}`.
- [ ] **GET `/health` (Error State)**: Should return `{"status": "error", "gpu_active": false, "loaded_model": "small.en"}` when the model fails to load.
- [ ] **POST `/set_model`**: Send `{"model_size": "medium.en"}` and ensure the internal `JobQueue` thread swaps out the engine, freeing the previous VRAM block and allocating the new one.

## 3. WebSocket Streaming (`/stream`)
**Automated Server Validation Strategy**: We will write an isolated, automated Python/Bash test script to validate the WebSocket behavior directly using our existing `test_audio.wav`. This removes the UI client from the equation, eliminating the "user in the loop" and preventing us from debugging two unknowns simultaneously.
- [ ] **Binary Audio Accumulation**: The automated test script will connect via WebSocket and send multiple binary frames containing raw 16kHz, 16-bit PCM audio. Ensure the server accumulates these chunks without crashing.
- [ ] **OOM Prevention cap**: Send more than 60 seconds worth of audio (1,920,000 bytes) and verify the server forces an end-of-stream event to prevent OOM.
- [ ] **End Stream Trigger**: Send a JSON text frame: `{"action": "end_stream"}`.
- [ ] **Transcription Response**: Verify the server runs inference and returns a JSON text frame: `{"text": "Hello world...", "is_final": true, "processing_time_ms": 450}`.

## 4. Concurrent Testing
- [ ] **Queue Depth Check**: Send simultaneous requests to the `/stream` WebSocket and `/set_model` and ensure they are processed sequentially by the `JobQueue` without race conditions or memory corruption.
