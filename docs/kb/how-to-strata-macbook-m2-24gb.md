# Strata on Apple Silicon: Running 30+ GB MoE Models on a 24 GB M2 Mac

> **A Feature Story & Operational Playbook of Urmanac Sovereign Infrastructure**  
> *Hardware-verified on Apple M2 (arm64, 24 GB Unified Memory, macOS 26 / Sequoia)*

---

## 1. Executive Summary & Architectural Milestone

In sovereign operations, reliance on centralized inference APIs creates surveillance exposure, network vulnerability, and operational recurring costs. The goal of the **Urmanac** architecture is total local autonomy: running frontier-class reasoning and tool-calling models entirely on sovereign, commodity Apple Silicon hardware.

This playbook documents the breakthrough deployment of **Strata** (v0.1.44 / v0.1.44.1) on an **Apple M2 MacBook Air/Pro with only 24 GB of Unified Memory**, running the full 66 GB **Qwen-3.8-Flash-Next** Mixture-of-Experts (MoE) model.

### The Numbers & Hardware Reality
* **Physical RAM:** 24 GB Unified Memory.
* **Model:** `Qwen3.8-Flash-Next-GSQ-RCO-Q2_0` (MoE architecture with a 31.64 GiB streamed expert pack).
* **Storage Requirement:** NVMe SSD capable of ~3 GB/s uncached sequential reads.
* **Decode Throughput:** **3.7 to 4.1 tokens/second** sustained.
* **Capabilities:** Full multi-turn conversational reasoning (`<think>...</think>`), structured tool calling (terminal shell command execution), and OpenAI-compatible API serving.

---

## 2. The Engineering Breakthroughs

Running a model larger than total system RAM requires Strata's native streaming engine (`--mmap-experts`, `STRATA_METAL_STREAM=1`). In standard operation on a 24 GB Mac, two fatal upstream bugs blocked execution until patched:

### Bug 1: The macOS 15+/26 Metal Shader Compiler Crash
* **Defect:** In `metal/kernels/qsa_prompt.metal`, line 234 attempted a copy constructor on Apple Metal cooperative tensors: `auto tot = acc;`.
* **Root Cause:** In macOS 15+ (Sequoia) and macOS 26 SDKs, Apple's `Layout::copy_construct(thread void*, thread void*)` in `MPPTensorOpsMatMul2dImpl.h` takes a non-const source pointer. Passing a `const void*` caused `mtl: compile failed` with exit code 3.
* **Fix:** Replaced the copy with a direct allocation call via the cooperative tensor factory:
  ```metal
  auto tot = opO.template get_destination_cooperative_tensor<__tensor_ops_detail::__remove_addrspace_t<decltype(tPh)>,
                                                             __tensor_ops_detail::__remove_addrspace_t<decltype(rv)>, float>();
  ```
  *(Upstreamed in Strata hotfix `v0.1.44.1` via Issue #2323).*

### Bug 2: Small-Arena Prompt Buffer Allocation Failure
* **Defect:** Strata aborted on the very first user prompt with:
  ```text
  metal prompt: no arena region for the prompt buffers
  ```
* **Root Cause:** On a 24 GB Mac, Strata partitioned the expert memory cache arena into smaller chunks (~0.80 GB each). However, prompt matrix evaluation required a contiguous **1.60 GB** scratch buffer. The engine looped over arena chunks, found that every chunk was smaller than 1.60 GB, and exited with `nullptr`. Upstream's existing standalone `mmap` fallback was trapped *inside* the chunk loop behind an unreachable size check.
* **Fix:** Extracted `ExpertStream::standalone_region(bytes)` to dynamically allocate an independent anonymous `mmap` region whenever no individual arena chunk can fit the prompt's footprint:
  ```text
  strata metal stream: the arena is small: the prompt's 1.60 GB of buffers are their own allocation, not a region of its slots
  ```
  *(Contributed via PR from `kingdonb/Strata` to `Niko1221/Strata`).*

---

## 3. Directory Layout & Artifacts

All operational files live under `~/w/Strata` and `~/w/Strata-data`:

```text
~/w/
├── Strata/                          # Strata engine source & build
│   ├── build-metal/
│   │   └── strata                   # Compiled Metal binary
│   ├── serve/
│   │   └── server.py                # Web server, UI & OpenAI API router
│   ├── strata-q2_0.json             # Engine runtime profile
│   ├── run-q2_0.sh                  # Startup script
│   └── stop-q2_0.sh                 # Graceful termination script
└── Strata-data/
    ├── models/Q2_0/
    │   ├── Qwen3.8-Flash-Next-GSQ-RCO-Q2_0-00001-of-00002.gguf
    │   ├── Qwen3.8-Flash-Next-GSQ-RCO-Q2_0-00002-of-00002.gguf
    │   └── Qwen3.8-Flash-Next-GSQ-RCO-Q2_0-00001-of-00002.gguf.experts  (31.64 GiB)
    └── packs/q2_0/                  # Native Strata pack & tokenizer
```

---

## 4. Configuration: `strata-q2_0.json`

The operational configuration for the 24 GB M2 profile:

```json
{
 "exe": "/Users/yebyen/w/Strata/build-metal/strata",
 "args": [
  "--pack",
  "/Users/yebyen/w/Strata-data/packs/q2_0",
  "--native",
  "/Users/yebyen/w/Strata-data/models/Q2_0/Qwen3.8-Flash-Next-GSQ-RCO-Q2_0-00001-of-00002.gguf",
  "--ple-gguf",
  "/Users/yebyen/w/Strata-data/models/Q2_0/Qwen3.8-Flash-Next-GSQ-RCO-Q2_0-00002-of-00002.gguf",
  "--expert-profile",
  "/Users/yebyen/w/Strata/data/expert-profile.bin",
  "--expert-cache",
  "64",
  "--prefill",
  "auto",
  "--spec",
  "2",
  "--suffix-draft",
  "0",
  "--max-context",
  "32768",
  "--mmap-experts"
 ],
 "cwd": "/Users/yebyen/w/Strata",
 "tokenizer": "/Users/yebyen/w/Strata-data/packs/q2_0/tokenizer",
 "model_name": "qwen3.8-flash-next-q2_0",
 "log": "/Users/yebyen/w/Strata/strata-q2_0.log",
 "lib_dirs": [],
 "port": 8080,
 "backend": "metal",
 "api_monitor": true,
 "env": {
  "STRATA_METAL_STREAM": "1"
 }
}
```

---

## 5. Day-to-Day Operations

### Starting the Server

#### Option A: Foreground (Interactive Development)
```bash
cd ~/w/Strata && ./run-q2_0.sh
```
* Displays live token throughput in the terminal.
* Automatically launches the web UI in default browser.
* Terminate with `Ctrl + C`.

#### Option B: Background Daemon (Sovereign Service)
```bash
cd ~/w/Strata && nohup ./run-q2_0.sh > strata-server.out 2>&1 &
```
* Runs detached in the background.
* Monitor live token generation and thoughts with:
  ```bash
  tail -f ~/w/Strata/strata-server.out
  ```

### Stopping the Server
```bash
cd ~/w/Strata && ./stop-q2_0.sh
```
*(Or run `pkill -f "serve/server.py.*strata"`).*

---

## 6. Access Endpoints

| Service | URL | Purpose |
|---|---|---|
| **Web Chat Interface** | `http://127.0.0.1:8080/` | Interactive reasoning chat with thinking drop-downs & tools |
| **API & GPU Monitor** | `http://127.0.0.1:8080/api-monitor` | Live latency, cache hits, token rate, and SSD read bandwidth |
| **OpenAI Compatible API** | `http://127.0.0.1:8080/v1` | Headless API for Aider, Claude Code, Continue, and custom agents |

### Using with External Agent Tooling (e.g., Aider, Continue, MCP)
Point any standard OpenAI-compatible client to:
* **Base URL:** `http://127.0.0.1:8080/v1`
* **API Key:** Any arbitrary string (e.g. `sk-sovereign`)
* **Model:** `qwen3.8-flash-next-q2_0`

---

## 7. Strategic Significance to NockNock & Urmanac

NockNock orchestrates autonomous Uniswap v4 liquidity defense and Ames P2P syndication without custodial intermediaries. By hosting local frontier-class reasoning via Strata on edge hardware:
1. **Zero RPC Leakage:** Autonomous agent reasoning logs and strategy deliberation never leave the local machine.
2. **Deterministic Economics:** No token billing or rate limits from external cloud providers.
3. **Resilience:** Liquidity management continues unimpeded during upstream cloud or network partitions.
