# Server Implementation Plan (v2)

Status: **Implemented.** This document plans the production **v2** server, a native
Rust rewrite of the Python PoC in `python-prototype/server/`. Per `ROADMAP.md`,
the PoC stays untouched as a working baseline; v2 is built alongside it in
`src/server/`.

## 1. Scope & Goals

- Replace the Python/`faster-whisper` runtime with a compiled Rust binary for
  lower latency, lower memory overhead, and single-binary deployment.
- Keep the AI model resident in GPU VRAM for sub-second, warm-path
  transcription.
- Serve multiple clients safely without exhausting VRAM (concurrency job queue).
- Run headlessly as a resilient `systemd` service with GPU hardware detection.
- Match the wire protocol defined in `docs/api-contract.md` so client work can
  proceed independently.

**Distribution-agnostic boundary (important):** this repository is portable,
upstream source code that must not embed distro-specific assumptions. It exposes
_mechanisms_ only — Cargo feature flags for backends, runtime configuration via
environment/config file, and no hardcoded distro paths. All Gentoo-specific
_packaging_ (the ebuild, USE flags, `REQUIRED_USE`, `RDEPEND`, conf.d/systemd
install paths) lives **downstream** in the `adaptive-overlay` repo, not here.
Wherever this plan mentions USE flags, it is naming the overlay-side policy that
maps onto the upstream Cargo features documented below. See §9 for the exact
split of responsibilities.

Non-goals for v2.0 (explicitly deferred):

- The LLM formatting engine (README "Future Work"). Leave a clean seam for it.
- Word-by-word partial/interim transcripts (`is_final: false`). Reserve the
  field now, implement later.
- Any Gentoo/ebuild artifacts in this repo (they belong in `adaptive-overlay`).

## 2. Lessons Carried Over From the PoC

The PoC (`server.py`) is the reference for behavior we want to preserve:

- **Warm model in VRAM** via app lifespan; load once at startup, reuse per
  request.
- **CUDA-first with CPU fallback** (`get_whisper_model`): try GPU, degrade
  gracefully.
- **Dynamic model switching at runtime** (`/set_model`) without a restart —
  loading a new model replaces the old one and frees its VRAM.
- **Model selected via env var** (`WHISPER_MODEL`, default `small.en`).
- **CUDA math libs on the library path** (see `start.sh` `LD_LIBRARY_PATH`). The
  Rust build linking `whisper.cpp` with CUDA will have an analogous runtime
  linkage concern.
- Return **timing metadata** with each transcription (the PoC returns
  `processing_time_seconds`; the contract uses `processing_time_ms`).

## 3. Technical Stack

- **Language:** Rust (stable).
- **Web framework:** Axum (Tokio-based) — WebSocket + HTTP in one server.
- **Async runtime:** Tokio (multi-threaded).
- **Inference:** `whisper.cpp` via the `whisper-rs` bindings (pin ~`0.14.x`).
  Stack, bottom-up:
  - **`ggml`** — low-level C tensor/math library that owns the compute
    _backends_ (CPU, CUDA, HIP/ROCm, Vulkan, SYCL, …).
  - **`whisper.cpp`** — C/C++ implementation of the Whisper model built on
    `ggml`.
  - **`whisper-rs`** — Rust FFI bindings that _vendor_ current `whisper.cpp`
    (hence current `ggml`) and build it via CMake. Each backend is a Cargo
    feature that flips a `GGML_*` CMake define, which is what lets us drive
    backend selection from Gentoo USE flags. GPU controls (`use_gpu`,
    `gpu_device`) are exposed for runtime device choice. The GPU backend(s) are
    a **compile-time** choice (see §6.6 and the ebuild in §9); the CPU backend
    is always compiled in.
- **Audio decode/resample:** candidate crates `symphonia` (decode) + `rubato`
  (resample) to normalize client audio to whisper's required 16 kHz mono `f32`
  PCM. See Open Question O3.
- **Serialization:** `serde` / `serde_json` for the JSON control + result
  messages.

## 4. Proposed Project Structure

```text
src/server/
├── Cargo.toml
├── build.rs                 # if needed for whisper.cpp/CUDA build flags
└── src/
    ├── main.rs              # startup, config load, GPU check, bind, run
    ├── config.rs            # env/conf.d parsing (port, model, queue depth…)
    ├── routes/
    │   ├── mod.rs
    │   ├── health.rs        # GET /health
    │   └── stream.rs        # WebSocket /stream handler
    ├── transcribe/
    │   ├── mod.rs
    │   ├── engine.rs        # whisper-rs wrapper, model load/switch, GPU/CPU
    │   └── queue.rs         # single-worker GPU job queue
    └── audio.rs             # decode + resample to 16kHz mono f32
```

## 5. API Surface (aligned to `docs/api-contract.md`)

### 5.1 `GET /health`

Returns readiness + GPU/model state. Response shape per contract:

```json
{ "status": "ready", "gpu_active": true, "loaded_model": "medium.en" }
```

### 5.2 `WebSocket /stream`

- **Client → Server:** binary frames of audio chunks; optional JSON control
  messages (`{"action":"set_model","model":"medium.en"}`,
  `{"action":"end_stream"}`).
- **Server → Client:** JSON result:

```json
{ "text": "…", "is_final": true, "processing_time_ms": 150 }
```

**v2.0 processing model (decision):** accumulate the binary chunks per
connection into a buffer; on `end_stream`, decode/resample the full buffer,
enqueue one transcription job, and return a single `is_final: true` result. This
delivers the contract's semantics without the complexity of streaming partial
decodes. Interim results are a later enhancement (Open Question O2).

## 6. Internal Design

### 6.1 Connection lifecycle (`/stream`)

1. Accept upgrade; create a per-connection buffer and (optional) model override.
2. On binary frame → append to buffer.
3. On JSON `set_model` → validate + request a model switch (see 6.3).
4. On JSON `end_stream` (or socket close after data) → finalize: decode →
   resample → enqueue job → await result → send JSON result frame.
5. Handle client disconnect mid-stream by dropping the buffer (no job enqueued).
6. **Keepalive:** send WebSocket ping/pong and avoid an aggressive idle timeout
   so the socket stays open while a job runs. This matters most on the CPU path
   (§6.6), where a transcription can take noticeably longer than on GPU.

### 6.2 GPU job queue (VRAM safety)

- A **single dedicated inference worker** owns the loaded model and is the only
  code that touches the GPU. Requests reach it over an `mpsc` channel; each job
  carries the PCM samples and a `oneshot` sender for the result.
- Serializing inference through one worker prevents concurrent decodes from
  multiplying VRAM use and is the simplest correct answer to the "job queue"
  requirement. Multiple clients can connect concurrently; their finalize steps
  queue and are served in order.
- Bound the queue depth (config) and reject/park excess with a clear error so a
  burst of clients can't grow memory unboundedly.

### 6.3 Model management

- Load the default model at startup from config (mirrors PoC lifespan).
- Runtime switching is handled **inside the worker** so it can't race with an
  in-flight transcription: a `SetModel` command replaces the resident model
  (freeing the old one) between jobs.
- Contract currently exposes model switching only via a `/stream` JSON message;
  the PoC also had a dedicated HTTP `/set_model`. See Open Question O4.

### 6.4 Inference engine (`whisper-rs`)

- **GPU vs CPU is a runtime flag, decided per policy (§6.6):** in a GPU-enabled
  build, `whisper-rs` still chooses GPU or CPU per context via `use_gpu` (+
  `gpu_device`). Detect the GPU first and set the flag accordingly, rather than
  requesting the GPU blindly (an absent device can make the GPU backend error
  instead of degrading).
- Reuse a whisper state/context across jobs; feed 16 kHz mono `f32` samples.
- Return `text` (trimmed, segments joined) plus elapsed ms.

### 6.5 Audio handling

- whisper.cpp expects 16 kHz mono `f32`. The contract lets clients send
  "PCM/FLAC".
- Normalize on the server: detect/decode the incoming format, downmix to mono,
  resample to 16 kHz. Nailing down the exact accepted formats is Open Question
  O3 (ideally the client sends a known fixed format to keep the server path
  simple).

### 6.6 GPU backends, build combinations & no-GPU policy

Three independent concerns: (a) which backends are compiled in, (b) which one is
used at runtime, (c) what happens when no GPU is present.

**(a) Compile-time backends — multiple allowed, with limits.** `ggml` supports
several backends compiled into one binary. Each maps to a `whisper-rs` Cargo
feature / `GGML_*` define (the overlay maps its USE flags onto these, §9). The
CPU backend is always present. Confirmed build compatibility (from the
`whisper-rs-sys` build script):

| Combination               | Buildable | Notes                                                                   |
| ------------------------- | --------- | ----------------------------------------------------------------------- |
| `nvidia` + `vulkan` + cpu | Yes       | no compiler conflict; both link cleanly (laptop target)                 |
| `rocm` + `vulkan` + cpu   | Likely    | AMD's `hipcc` also compiles the Vulkan backend — validate by build (O6) |
| `nvidia` + `rocm`         | **No**    | CUDA can't build under `hipcc`; forbidden by `REQUIRED_USE`             |
| `openblas` (CPU accel)    | Yes       | accelerates the CPU path; combinable with any of the above              |

Rationale: `rocm` (`hipblas`) and `intel-sycl` each _override the C/C++
compiler_ (`hipcc` / `icx`), so no two **native vendor** backends can coexist.
`vulkan` and `openblas` do not, so they combine freely with a single native
backend.

**Backend → detection tool:** `nvidia` → `nvidia-smi`, `rocm` → `rocm-smi`,
`vulkan` → `vulkaninfo`.

**(b) Runtime device selection.** With multiple backends compiled in, `ggml`'s
device registry enumerates available devices; whisper.cpp selects via
`use_gpu` + `gpu_device`. The server tries devices in the operator-defined order
from `DEVICE_PRIORITY` (§7), e.g. `cuda,vulkan,cpu` on the docked laptop — use
CUDA when the eGPU is present, fall to Vulkan/CPU when it isn't. Exact
preference when several GPUs are visible at once must be validated by test (O6).

**(c) No-GPU policy (`GPU_MODE`, §7).** When no device from `DEVICE_PRIORITY`
(other than CPU) is usable:

- `auto` → fall back to CPU (PoC parity: slower but "workable in a pinch").
- `require` → refuse to start / fail health, so the user knows to attach the
  GPU.

**Linkage constraint:** a binary linked against a vendor backend still needs
that vendor's userspace libraries present to load at all, even when running
CPU-only. On Gentoo this is a non-issue — a host only sets `USE=nvidia` if it
already has the NVIDIA libraries, etc. — which is exactly why backends are a
build-time (USE-flag) decision.

**Reserved for the future — `intel-sycl` (native Intel).** `whisper-rs` exposes
an `intel-sycl` feature for Intel iGPUs/Arc, but it needs Intel's oneAPI
compiler (`icx`) toolchain to build and we have no hardware to test it. It is
**intentionally not exposed as a USE flag yet.** Vulkan already covers Intel
iGPUs at lower toolchain cost. Nothing in this design precludes adding an
`intel-sycl` USE flag later (it slots into the same "one native vendor backend"
slot as `nvidia`/`rocm`).

## 7. Configuration

Following the PoC + ROADMAP, config comes from the environment / Gentoo
`conf.d`. The server also uses `dotenvy` to automatically load a `.env` file 
from the working directory if one exists, simplifying local testing.

- `WHISPER_MODEL` (default `small.en`) — default model to load.
- `PORT` (default `3000`) — listen port.
- `BIND_ADDR` (default `127.0.0.1`) — listen address.
- `MODEL_DIR` — where whisper.cpp `.bin` (GGUF) model files live.
- `MAX_QUEUE_DEPTH` — job queue bound.
- `GPU_MODE` (`auto` | `require`, default `auto`) — when no usable GPU is
  detected, `auto` falls back to CPU (PoC parity) and `require` fails instead of
  degrading (§6.6). Independent of the compile-time backend chosen via USE flags
  (§9); a CPU-only build ignores it and always runs on CPU.
- `DEVICE_PRIORITY` (ordered list, e.g. `cuda,vulkan,cpu`) — the order in which
  the server attempts to use compiled-in backends at runtime (§6.6b). First
  usable device wins; `cpu` is the implicit last resort. Entries for backends
  not compiled in are skipped. Lets one binary prefer the discrete eGPU when
  docked and gracefully step down to Vulkan/iGPU/CPU when not.

## 8. systemd & Hardware Detection

- The Rust binary must be **service-manager friendly but not systemd-specific**:
  run in the foreground, log to stdout/stderr, read config from the environment,
  and take no action that assumes a particular init system. This keeps the
  source portable.
- Upstream **may** ship a _reference_ `systemd` unit as documentation/example
  (systemd itself is cross-distro), but the **installed** unit — with Gentoo
  paths, `conf.d` wiring, and USE-driven `ExecCondition` — is produced
  downstream by the overlay (§9). (The existing
  `python-prototype/.../ai-voice-server.service` targets the PoC and stays
  as-is.)
- Reference-unit patterns worth carrying (the overlay will finalize them):
  - `EnvironmentFile=-/etc/conf.d/ai-voice-server` for config.
  - `Restart=on-failure`.
- **Hardware detection depends on `GPU_MODE` (§7) and the compiled backend
  (§6.6):**
  - `GPU_MODE=require`: gate startup with a backend-appropriate `ExecCondition`
    (`nvidia-smi` for `nvidia`, `rocm-smi` for `rocm`, `vulkaninfo` for
    `vulkan`) so the service **skips** (not fails) when the GPU is absent — the
    ROADMAP "graceful degradation" behavior. Because the right condition depends
    on the enabled USE flags, this belongs in the ebuild, not the source.
  - `GPU_MODE=auto`: no `ExecCondition`; the service starts and runs CPU-only
    when the GPU is missing.
- `ExecStart` points at the compiled binary; no `LD_LIBRARY_PATH` hack needed —
  GPU libs are resolved at build/link time and declared as overlay `RDEPEND`s.

## 9. Packaging & the Upstream/Downstream Boundary

This repo is **distribution-agnostic source**; Gentoo packaging lives in the
separate `adaptive-overlay` repo. Responsibilities split as follows.

### 9.1 Upstream (this repo) must expose the _mechanisms_

- **Cargo features** that pass through to the corresponding `whisper-rs`
  features: `nvidia` → `cuda`, `rocm` → `hipblas`, `vulkan` → `vulkan`,
  `openblas` → `openblas`. Default feature set = none (plain CPU build).
- **Runtime configuration via environment / config file** (§7) with sane
  defaults and no hardcoded distro paths, so a packager can point it at
  `/etc/conf.d/...` without code changes.
- **Foreground, log-to-stdout** behavior so any service manager can supervise
  it.
- Optional: a `client`/`server` split (workspace crates or bins) so the overlay
  can offer matching USE flags per `ROADMAP.md`.

### 9.2 Downstream (`adaptive-overlay`) owns the _policy_

All of the following live in the overlay ebuild (e.g. a
`media-sound/ai-voice-server` package), **not** in this repo — modeled on the
existing `x11-terms/boxxy` ebuild (`inherit cargo git-r3`, `cargo_src_*`):

- **USE flags → Cargo features:** `nvidia`, `rocm`, `vulkan`, `openblas` (+
  `client`/`server`).
- **`REQUIRED_USE`:** only native vendors are mutually exclusive —
  `?? ( nvidia rocm )` (reserves the slot for a future `intel-sycl`).
  `vulkan`/`openblas` combine freely; CPU is always built.
- **`RDEPEND` per flag:** CUDA runtime (`nvidia`), ROCm/HIP (`rocm`), Vulkan
  loader (`vulkan`), `sci-libs/openblas` (`openblas`).
- **Install** the systemd unit (with the USE-driven `ExecCondition`) and a
  `conf.d` sample documenting `GPU_MODE`, `DEVICE_PRIORITY`, and the other §7
  variables; ship `metadata.xml` describing the flags.

### 9.3 Per-host build examples (the Gentoo model — each host builds for its own hardware)

- **Laptop (eGPU + Intel iGPU):** `USE="nvidia vulkan openblas"` — CUDA when
  docked, Vulkan for the Intel iGPU when mobile, OpenBLAS-accelerated CPU as
  final fallback. Never `rocm` (no AMD card).
- **Big AMD desktop (if it serves):** `USE="rocm vulkan openblas"` — native
  ROCm, Vulkan fallback, fast CPU. Never `nvidia` (no NVIDIA drivers installed).

**Self-hosting note:** server and client can co-reside on one machine over
loopback. An undocked laptop can run the server _and_ be its own client; the
desktop can serve itself if the laptop is unavailable. No special design needed
— both `client` and `server` USE flags enabled and the client pointed at
`ws://127.0.0.1:<port>/stream`.

## 10. Open Questions / Decisions Needed

*(All open questions have been resolved)*

### Resolved decisions

- **O1 — Port (resolved):** Default to `3000` (configurable via `PORT` environment variable).
- **O2 — Streaming granularity (resolved):** Single final result on `end_stream`. Continuous interim results require repeated GPU decoding (high overhead) and complex Wayland backspace-simulation on the client. Batching the burst and decoding once at the end guarantees the lowest possible latency and maximum stability.
- **O3 — Audio format (resolved):** The client must send raw uncompressed **16 kHz mono s16le PCM**. This eliminates server-side decode complexity entirely, and bandwidth is negligible (~32 KB/s).
- **O4 — Model-switch API (resolved):** Both WebSocket message AND HTTP `POST /set_model` are supported.
- **O5 — Model format (resolved):** We will use the modern `GGUF` format models for `whisper.cpp`.
- **Packaging separation (resolved):** this repo is distro-agnostic source
  exposing Cargo features + config; all Gentoo ebuild/USE-flag/`RDEPEND` policy
  lives in `adaptive-overlay` (§9).
- **No-GPU behavior (resolved):** `GPU_MODE` config option (`auto` = CPU
  fallback, `require` = fail) rather than hard-coded. See §7 / §6.6.
- **Backend selection (resolved):** Cargo features
  `nvidia`/`rocm`/`vulkan`/`openblas` (overlay USE flags); native
  `nvidia`↔`rocm` mutually exclusive, `vulkan`/`openblas` combine freely, CPU
  always built. See §9 / §6.6.
- **Runtime device order (resolved):** operator-set via `DEVICE_PRIORITY` (§7).
- **AMD support (resolved):** yes — via `rocm` (native HIP) or `vulkan`
  (portable).
- **CPU acceleration (resolved):** optional `openblas` feature/USE flag
  (recommended on any host doing CPU inference, e.g. the undocked laptop); not
  mandatory so minimal builds can skip the BLAS dependency.
- **`intel-sycl` (deferred, not blocked):** not exposed as a feature/USE flag
  now (no test hardware, heavy oneAPI toolchain); the design reserves a
  native-backend slot so it can be added later without rework. See §6.6.

## 11. Milestones

1. **Scaffold:** `src/server/` Cargo project; Axum server; `GET /health`
   returning static readiness. Resolve O1 (port).
2. **Engine:** integrate `whisper-rs`; wire the `GPU_MODE` policy (GPU detect →
   `use_gpu` flag, `auto` vs `require`); load default model at startup;
   transcribe a fixed WAV to prove the GPU path (reuse `server/test_audio.wav`).
3. **Queue:** single-worker GPU job queue with `mpsc` + `oneshot`; bounded
   depth.
4. **Stream:** implement `/stream` (buffer → `end_stream` → job → JSON result)
   with WS keepalive. Wire `/health` to real model/GPU state.
5. **Model switching:** `set_model` via the worker (and O4's HTTP endpoint if
   chosen).
6. **Audio:** decode/resample per O3.
7. **Deploy:** ship the distro-agnostic bits from this repo (Cargo features,
   sample config, optional reference systemd unit). The Gentoo ebuild —
   USE→feature mapping, `REQUIRED_USE`, `RDEPEND`, `ExecCondition`, `conf.d`
   install — is authored separately in `adaptive-overlay` (§9).

## 12. Reference: PoC → v2 Endpoint Mapping

| PoC (Python, :8000)       | v2 (Rust, contract)              | Notes                          |
| ------------------------- | -------------------------------- | ------------------------------ |
| `POST /transcribe` (file) | `WebSocket /stream`              | batch upload → streamed buffer |
| `GET /status`             | `GET /health`                    | richer readiness/GPU payload   |
| `POST /set_model`         | `POST /set_model` or WS msg      | supported via HTTP and WS      |
