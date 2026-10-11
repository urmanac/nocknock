# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Pre-Release Gates & Hardening (Must-Haves before v0.1.0 tag)
- [x] **Dynamic Live Gas Token Pricing**: Query live ETH/USD price dynamically via Aerodrome vAMM-WETH/USDC (`0x88A43bbDF9D098eEC7bCEda4e2494615dfD9bB9C`) `getReserves()` on Base to replace static gas USD conversion.
- [x] **Dynamic `tick_current` Query**: Derived real-time active tick dynamically from on-chain spot price ratio rather than static fallback tick values.
- [x] **Worker Unit & Integration Test Suite**:
  - [x] Unit test parsing `0xd294f093` (`claimFees`) return data hex buffers.
  - [x] Unit tests for tactical advice engine: batch harvesting, defensive floor lock, hold & accrete.
  - [x] Unit test for deterministic SHA-256 state hashing.
  - [x] Unit test for dynamic on-chain ETH/USD reserve decoding.
- [x] **Database Migration & Disaster Recovery Tooling**:
  - [x] Added `make db-backup` and `make db-restore` targets using PostgreSQL dump tooling.
- [x] **Graceful Shutdown**: Implemented Unix signal handling (`SIGTERM`/`SIGINT`) in Axum server with in-flight worker draining.
- [x] **RPC Rate-Limit & Backoff Resilience**: Added exponential backoff and jitter for Alchemy JSON-RPC requests (`post_rpc_with_retry`) to handle HTTP 429 and transient network timeouts.

### Tier 2: Agent Desktop & Model Context Protocol (MCP) Adapter
- [x] **MCP Server Binary (`nocknock-mcp`)**: Implemented dedicated Model Context Protocol server communicating over `stdio`:
  - `get_portfolio_state`: Returns current live sovereign balance, NOCK floor buffer delta, and tranche allocations.
  - `get_shot_advice`: Returns active tactical advice (Hold & Accrete, Batch Harvest, Dip Reinvestment).
  - `simulate_claim_fees`: Simulates gas vs fee return for active tranches.
- [x] **Zero-DB Mode for Agents**: MCP server queries the local Axum API (`/v1/snapshot/latest`) with seamless fallback to direct Base JSON-RPC when running without PostgreSQL or worker.

### Tier 3: Edge Mobile / Android Moon Clock Model (`moon-phase-clock`)
- [ ] **Android App Scaffold (`android/`)**:
  - Initialize Jetpack Compose Android application mirroring `~/w/mecris/tools/moon-phase-clock`.
  - Configure Gradle with Kotlin Multiplatform or JNI to bundle `nocknock_brain.wasm` into assets.
- [ ] **WASM Edge Runtime**:
  - Wire Extism or Wasmer Android SDK to execute `nocknock_brain.wasm` directly on device.
  - Implement offline deterministic math: tick calculations, LP invariant $k=x \cdot y$, and 100k floor verification.
- [ ] **Decentralized RPC Fallback Pipeline**:
  - Primary: REST API sync (`/v1/snapshot/latest`).
  - Secondary: Alchemy Base JSON-RPC client on Android.
  - Tertiary: Public Base node (`https://mainnet.base.org`) direct `eth_call` queries when no API key is provided.

### Sovereign Automation & Execution Engine (Legendary / Vision)
- [ ] **1-Click Execution via Hardware Signer / Safe**:
  - Integrate Safe (formerly Gnosis Safe) multisig transaction generation for batch harvesting and rebalancing.
  - Support Ledger / Trezor / EIP-1193 wallet connect directly from the dashboard to execute tactical shots without manual contract interaction.
- [ ] **Sovereign Multi-Wallet Aggregator**:
  - Aggregate balances across multiple cold storage addresses, escrow contracts, and timelocks into a unified Sovereign Stack view.
- [ ] **Real-Time Sound & Push Alerts**:
  - Non-intrusive audio cues for Batch Harvest availability ($5.00+ fee threshold).
  - High-priority sovereign sirens if total NOCK stack ever approaches the 100,000 hard floor buffer.

## [0.1.0] - 2026-10-11

### Added
- **Tactical Dual-Growth Engine**:
  - Live on-chain tracking for sovereign asset NOCK (16 decimals) and USDC (6 decimals) on Base.
  - Three distinct strategic tranches:
    - Tranche 1: Uniswap V4 Concentrated LP (`0.01237 - 2.02783`) scaled dynamically via position NFT `#3126000`.
    - Tranche 2: Aerodrome Volatile vAMM LP (`vAMM-USDC/NOCK`) proportional share calculation.
    - Tranche 3: Sovereign Protected Treasury with 100,000 NOCK hard floor invariant.
- **Dynamic On-Chain Fee Harvesting & Pricing**:
  - Direct read-only simulation (`eth_call`) of `claimFees()` (selector `0xd294f093`) on Aerodrome pool (`0x85f1aa3a70fedd1c52705c15baed143e675cd626`).
  - Auto-compounding detection for Uniswap V4 modifying liquidity deltas.
  - Dynamic on-chain ETH/USD spot pricing queried directly from Aerodrome `vAMM-WETH/USDC` (`0x88A43bbDF9D098eEC7bCEda4e2494615dfD9bB9C`).
  - Dynamic `tick_current` derivation from spot price ratios.
  - Live Base gas price query with claim efficiency and ROI calculations.
  - Alchemy RPC resilience helper with exponential backoff and randomized jitter (`post_rpc_with_retry`).
- **Backend Ingestion Worker**:
  - Rust Axum HTTP service with PostgreSQL persistent storage.
  - Persistent High-Water Mark tracking with automatic ratchet upward.
  - SHA-256 state hashing and snapshot deduplication.
  - Offline SQLX query caching (`SQLX_OFFLINE=true`) for reproducible container builds.
  - Graceful Unix signal handling (`SIGTERM`/`SIGINT`) for clean worker shutdown.
  - Robust configuration loading with fallback to `config.yaml.example` and environment variables.
  - 6 unit tests covering fee hex decoding, ETH spot parsing, tactical advice decisions, and state hash determinism.
- **Model Context Protocol (MCP) Adapter (`nocknock-mcp`)**:
  - Dedicated CLI binary communicating over stdio JSON-RPC 2.0.
  - Tools for AI agents: `get_portfolio_state`, `get_shot_advice`, `simulate_claim_fees`.
  - Zero-DB progressive degradation with direct Base decentralized RPC fallback.
- **Monitoring Portal (Frontend)**:
  - 6 KPI stat cards with dual-goal metrics and floor health buffer.
  - Tactical Shot Advice cockpit banner with 5-tier dynamic advice matrix.
  - 7 visual data canvas panels built with Recharts.
  - React `ErrorBoundary` wrapper and `useFreshness` polling hook.
- **Progressive Degradation Architecture**:
  - Extensible WASM Brain (`wasm32-wasip1`) compiled for edge evaluation and Android app embedding.
  - Architectural blueprint for MCP adapter and direct Alchemy/Base RPC fallbacks.
- **Operations & Orchestration**:
  - Persistent background execution via `make tmux` orchestrating worker and frontend.
  - Disaster recovery database tooling via `make db-backup` and `make db-restore`.
  - Multi-stage Dockerfiles and Docker Compose configuration.
  - Complete automated CI and release packaging pipeline for GitHub Actions (`make dist`).
- **Licensing**:
  - Licensed under Apache 2.0, Copyright 2026 Urmanac.
