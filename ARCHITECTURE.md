# Architecture & Progressive Degradation Blueprint

## 1. Overview & Strategic Vision

NockNock is designed according to the **Mecris Progressive Degradation Model** (modeled after [`moon-phase-clock`](file:///Users/yebyen/w/mecris/tools/moon-phase-clock)).

The system delivers real-time monitoring and tactical shot advice across multiple environments—from high-availability cloud servers to local agent desktops and edge Android mobile devices—without hard failure when external infrastructure becomes unavailable.

```
+-------------------------------------------------------------------------+
|                  MECRIS PROGRESSIVE DEGRADATION SPECTRUM                |
+-------------------------------------------------------------------------+
|                                                                         |
|  Tier 1: Full Cloud / Server Mode (Current Web Portal)                  |
|  [PostgreSQL] <-> [Axum Ingestion Worker] <-> [React Web Dashboard]     |
|         ^                                                               |
|         |-- (Network / Cloud Partition)                                 |
|         v                                                               |
|  Tier 2: Agent Desktop / MCP Gateway Mode                               |
|  [AI Agent / Antigravity] <-> [NockNock MCP Server] <-> [Direct DB/RPC] |
|         ^                                                               |
|         |-- (Database & Server Down)                                    |
|         v                                                               |
|  Tier 3: Edge Mobile / Android Moon Clock Model                         |
|  [Android Jetpack App] <-> [Embedded WASM Brain] <-> [Direct RPC / Base]|
|                                                                         |
+-------------------------------------------------------------------------+
```

---

## 2. The Three Operational Tiers

### Tier 1: Sovereign Server & Web Room (Current Core)
* **Components**:
  * **PostgreSQL (Port 5432)**: Append-only persistent ledger storing snapshots and deterministic SHA-256 state hashes.
  * **Ingestion Worker (`ingestion-worker/`)**: Rust/Axum host service querying Base RPC / Alchemy, evaluating the 5-tier shot advice decision tree, and ratcheting the High-Water Mark.
  * **Frontend Portal (`frontend/`)**: React 18, Vite, and Recharts monitoring room served over HTTP `:3000` (or Nginx in production).
* **Network Requirements**: Alchemy API key, live Base mainnet connection, local or remote PostgreSQL.

---

### Tier 2: Agent Desktop & Model Context Protocol (MCP) Mode
* **Rationale**: In an AI-assisted or restricted execution environment, an agent or desktop client (e.g. Claude Code, Gemini Antigravity, Mecris daemon) may not have direct access or credentials to the backend database.
* **Architecture (`nocknock-mcp`)**:
  * An MCP server tool layer communicating via `stdio` JSON-RPC 2.0.
  * Standardized MCP Tools:
    * `get_portfolio_state`: Returns current total sovereign portfolio value (USD), NOCK floor buffer delta, native token balance, and tranche 1/2 allocations.
    * `get_shot_advice`: Returns the active tactical directive (`HOLD & ACCRETE`, `BATCH HARVEST SHOT`, `DIP REINVESTMENT SHOT`, `DEFENSIVE LOCK`, etc.) along with current gas cost and accrued fee ROI.
    * `simulate_claim_fees`: Simulates gas expenditure vs net yield accrued in Aerodrome and Uniswap V4 tranches.
  * **Zero-DB Dual Fallback**:
    1. Primary: Queries local Axum API endpoint `http://127.0.0.1:8081/v1/snapshot/latest`.
    2. Fallback: Directly queries Base mainnet RPC (`eth_call`) without requiring PostgreSQL or server worker.
* **Degradation Trigger**: Ingestion worker is running locally or remotely, but direct SQL ports are closed or firewalled.

---

### On-Chain Dynamic Gas Spot Pricing
* **On-Chain Oracle**: Rather than relying on static ETH price assumptions (e.g. $2,500) or off-chain API dependencies, the system queries the Aerodrome `vAMM-WETH/USDC` pool (`0x88A43bbDF9D098eEC7bCEda4e2494615dfD9bB9C`) using read-only `getReserves()` (`0x0902f1ac`).
* **Formula**:
  $$\text{ETH Price (USD)} = \frac{r_1 (\text{USDC}) / 10^6}{r_0 (\text{WETH}) / 10^{18}}$$
* **Resilience**: The query is wrapped with exponential backoff and jitter to protect against RPC rate limits (HTTP 429) or transient network spikes. If RPC is completely unreachable, it degrades safely to the conservative $2,500 peg.

---

### Tier 3: Edge Mobile & Moon Clock Android Model (`moon-phase-clock`)
* **v0.2.0 status**: deferred. The web `/?view=lite` cockpit + WalletConnect QR flow ships instead; the native Android app remains planned future work consuming the same worker payload endpoints. **Code-reuse gap (documented, deliberate)**: for 0.2.0 clients consume REST-computed values and the V4/Aerodrome claim encoders are Rust-only in the worker — the brain-wasm-in-Android-assets reuse pattern is not wired yet; `android/app/src/main/assets/` remains the reserved embedding path.
* **Rationale**: Mirroring `moon-phase-clock`, the Android mobile app provides calm, sovereign glancability directly on a handset or widget.
* **Two-Brain Separation**:
  1. **Server Brain (`ingestion-worker/src/main.rs`)**: Focuses on high-volume historical analytics, database deduplication, cron ingestion, and multi-day charting.
  2. **Edge WASM Brain (`brain/target/wasm32-wasip1/release/nocknock_brain.wasm`)**: Embedded directly inside Android assets (`android/app/src/main/assets/`). Provides pure, deterministic offline mathematics:
     * Uniswap V4 tick-to-price conversions and liquidity scaling ratio.
     * Aerodrome pool invariant $k = x \cdot y$ and LP reserve share computation.
     * 100,000 NOCK hard floor delta and defensive lock triggers.
     * State hashing and snapshot envelope verification.
* **Progressive RPC Fallback Chain**:
  * **Preferred**: Query the NockNock REST API (`/v1/snapshot/latest`).
  * **Fallback A (API down)**: Android app queries Alchemy Base JSON-RPC directly using client credentials.
  * **Fallback B (No Alchemy key / Rate limited)**: Android app queries a public decentralized Base RPC node (`https://mainnet.base.org`) directly for raw `eth_call` contract states (`0xd294f093` claimFees, `0x1efeed33` getPositionLiquidity), piping raw bytes into the embedded WASM brain for local evaluation.

---

## 3. The WASM Brain Maintenance Invariant

Even while active feature development occurs in the host worker (`ingestion-worker`), the WASM Brain (`brain/`) **must never be discarded or allowed to rot**:
* It is compiled to `wasm32-wasip1` in every CI build.
* Unit tests in `brain/` run on every PR and commit.
* Releases automatically package `nocknock_brain.wasm` alongside binary artifacts so future Android or embedded targets can consume the verified math without rebuilding the host toolchain.

---

## 4. Release Machinery Flow

1. Developer updates [`CHANGELOG.md`](CHANGELOG.md) under the target version section.
2. Developer tags the commit: `git tag v0.1.0`.
3. Push to GitHub triggers `.github/workflows/ci.yml`:
   * Job 1: Compiles and tests `brain` (`wasm32-wasip1`).
   * Job 2: Compiles and tests `ingestion-worker` (`SQLX_OFFLINE=true cargo build --release`).
   * Job 3: Tests (`vitest`) and bundles `frontend` (`vite build`).
   * Job 4: Publishes a GitHub Release attaching all build artifacts with release notes extracted from `CHANGELOG.md`.
