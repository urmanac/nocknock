# Operating the Pi Agent Harness with Strata & NockNock MCP

> **Operational Playbook for Urmanac Autonomous Edge Agents**  
> *Hardware-verified on Apple M2 (24 GB RAM) running Strata + Pi Coding Agent*

---

## 1. Overview & Architecture

This playbook documents how to run the **Pi Agent Harness** (`pi`) as the interactive agent and tool-execution boundary for a local **Strata** inference engine, augmented with **NockNock's Model Context Protocol (MCP)** server.

```
┌────────────────────────────────────────────────────────┐
│                   Pi Coding Agent CLI                  │
│  - Interactive TUI & Scripted CLI                      │
│  - Built-in Tools: read, bash, edit, write             │
│  - MCP Client: executes ./dist/nocknock-mcp via stdio  │
└──────────────────────────┬─────────────────────────────┘
                           │ OpenAI API (/v1/chat/completions)
                           │ Host: 127.0.0.1:8080
                           ▼
┌────────────────────────────────────────────────────────┐
│             Strata Sovereign LLM Engine                │
│  - Model: Qwen3.8-Flash-Next (GSQ-RCO-Q2_0)            │
│  - Backend: Apple Metal MoE Streaming                  │
│  - Emits function calls: nocknock_get_portfolio_state  │
└────────────────────────────────────────────────────────┘
```

By allowing Pi to serve as the external harness, the LLM is relieved of sandbox emulation and focuses solely on high-order reasoning. Pi provides real execution boundaries and feeds real `stdout` into the context loop.

---

## 2. Prerequisites & Pre-Flight Checks

Before launching Pi, ensure the three underlying components are active:

### 1. Strata Engine on Port 8080
The local Strata server must be running and healthy:
```bash
# Foreground
cd ~/w/Strata && ./run-q2_0.sh

# Or background daemon
cd ~/w/Strata && nohup ./run-q2_0.sh > strata-server.out 2>&1 &
```
Verify the health endpoint:
```bash
curl -s http://127.0.0.1:8080/health | jq .
# Expected: {"status": "ok", "loaded": true, "model": "qwen3.8-flash-next-q2_0", ...}
```

### 2. NockNock MCP Binary
Ensure the MCP server binary is compiled:
```bash
cd ~/w/nocknock
ls -la ./dist/nocknock-mcp
```
*(If missing, run `make dist` or `cargo build --release -p nocknock-mcp && cp target/release/nocknock-mcp dist/`).*

### 3. Pi CLI on PATH
Verify Pi is installed and callable:
```bash
which pi
# Output: /opt/homebrew/bin/pi
```

---

## 3. Configuration Setup

The configuration files connect Pi to Strata and NockNock MCP:

### A. Provider Configuration: `~/.pi/agent/models.json`
Add the `strata` provider entry to your global Pi models configuration:
```json
{
  "providers": {
    "strata": {
      "baseUrl": "http://127.0.0.1:8080/v1",
      "api": "openai-completions",
      "apiKey": "sk-strata-local",
      "authHeader": false,
      "models": [
        {
          "id": "qwen3.8-flash-next-q2_0",
          "name": "qwen3.8-flash-next-q2_0",
          "reasoning": true,
          "input": ["text"],
          "cost": {
            "input": 0,
            "output": 0,
            "cacheRead": 0,
            "cacheWrite": 0
          },
          "contextWindow": 32768,
          "maxTokens": 4096
        }
      ]
    }
  }
}
```

Verify that Pi detects the model:
```bash
pi --list-models strata
```

### B. Project MCP Configuration: `~/w/nocknock/.mcp.json`
Ensure `~/w/nocknock/.mcp.json` points to the local binary:
```json
{
  "mcpServers": {
    "nocknock": {
      "command": "./dist/nocknock-mcp",
      "directTools": true
    }
  }
}
```

---

## 4. Usage Patterns

### Option A: Interactive TUI (Full Agent Session)
Launch an interactive pairing session inside the NockNock workspace:
```bash
cd ~/w/nocknock
pi --provider strata --model qwen3.8-flash-next-q2_0
```
Inside the interactive session, you can prompt naturally:
```text
pi> What is our current sovereign portfolio balance and shot advice?
```

### Option B: Non-Interactive / Scripted Prompt (`-p`)
Execute a one-shot directive from shell scripts or automation:
```bash
cd ~/w/nocknock
pi --provider strata --model qwen3.8-flash-next-q2_0 -p "Call nocknock_get_portfolio_state and summarize current directives."
```

### Option C: Verifying Available Tools
To confirm which tools Pi has bound into the model's schema:
```bash
cd ~/w/nocknock
pi --provider strata --model qwen3.8-flash-next-q2_0 -p "What tools do you have available? List their names."
```
Expected output:
* **Core Tools:** `read`, `bash`, `edit`, `write`
* **NockNock Tools:** `nocknock_get_portfolio_state`, `nocknock_get_shot_advice`, `nocknock_simulate_claim_fees`

---

## 5. Performance Nuances & Prompt Caching

When running on an Apple M2 with 24 GB of RAM, keep these performance characteristics in mind:

1. **The First Request (~45–55 Seconds):**  
   Pi sends a comprehensive agent system prompt along with full JSON Schema definitions for all available tools (~5,000 tokens). On first run, Strata must stream the prompt through its matrix multiplication pipeline in chunks of 1024 tokens.
2. **Subsequent Multi-Turn Requests (~3–5 Seconds):**  
   Strata features **prefix KV caching**. On subsequent turns within the same session or context prefix, Strata reuses the 5,000 cached tokens instantly, processing the prompt in under 3 seconds before beginning generation.
3. **Generation Speed (~5.0 to 6.2 tok/s):**  
   Token generation proceeds smoothly at 5+ tok/s using Apple Metal cooperative tensors and GPU expert caching.

---

## 6. (Optional) Setting Strata as Your Default Pi Model

If you want to invoke `pi` without specifying `--provider strata --model ...` every time, update `~/.pi/agent/settings.json`:

```json
{
  "defaultProvider": "strata",
  "defaultModel": "qwen3.8-flash-next-q2_0",
  "defaultThinkingLevel": "medium"
}
```

Once set, typing `pi` in `~/w/nocknock` automatically boots with your sovereign local LLM and NockNock MCP tools.
