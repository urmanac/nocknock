# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - Unreleased

### Golden Path: Web Cockpit & Uniswap Mobile App Execution
- [ ] **WalletConnect Integration (AppKit / Wagmi / Viem)**:
  - [ ] Embed QR code modal on web dashboard allowing instant pairing with the Uniswap mobile app on Android/iOS.
  - [ ] Auto-reconnect active session across browser refreshes without credential or private key exposure.
- [ ] **1-Click Tactical Multicall Generator**:
  - [ ] Direct multicall encoder on Base: construct Aerodrome and Uniswap fee claim and rebalance calls.
  - [ ] Push transaction requests directly to the paired Uniswap app for biometric (Fingerprint / Face ID) operator confirmation.
  - [ ] Real-time Base gas quote display and execution feedback loop.

### Ethereum & Multi-AMM Pool Discovery & Tracking
- [ ] **Dynamic ETH Pool Discovery (Uniswap & Aerodrome)**:
  - [ ] Discover and optionally track Ethereum-related AMM pools on Base (e.g. WETH/USDC, WETH/NOCK, and secondary liquidity pairs).
  - [ ] Multi-pool registry in backend ingestion worker: put discovered pools on the board and count them as part of the unified portfolio whole.
  - [ ] Support dynamic reserve, tick, and fee queries across heterogeneous AMM architectures.
- [ ] **Multi-Vault Portfolio Aggregation**:
  - [ ] Expand tracking beyond the single primary vault to support multiple disparate vault contracts and liquidity tranches.
  - [ ] Aggregated net asset value (NAV) and unified token balance accounting.

### Coinbase API Integration: Staked Ethereum Tracking
- [ ] **Read-Only Coinbase Connector**:
  - [ ] Secure read-only API integration to observe staked Ethereum (staked ETH / cbETH) balances over time.
  - [ ] Symbiotic Yield Telemetry: track staked ETH baseline growth alongside on-chain Base Nock vault reserves.
  - [ ] Self-Funding Invariant: monitor how on-chain Nock vault yields fund Base L2 operations while staked ETH accumulates off-chain without churn.
  - [ ] Historical snapshot recording for staked ETH balances in worker database.

### Dual Income Streams: Harvested Yield & Capital Gains (P/L)
- [ ] **Low-Hanging P/L Gauge Integration**:
  - [ ] Connect to external wallet/portfolio P/L APIs (Moralis / Alchemy / Uniswap mobile telemetry) to pull token cost basis and net P/L without building a complex cost-basis engine from scratch.
  - [ ] Local Epoch P/L: compute deterministic session delta against high-water marks and recorded spot prices.
  - [ ] Unified Income Model: account for both ordinary yield (fee harvests) and capital gains/losses (P/L) across active positions.

### Sovereign Bookkeeping & 60/40 Balance Sheet Partition
- [ ] **Virtual Tax Reserve Sub-Ledger**:
  - [ ] Maintain the 60/40 Split Rule as an internal accounting allocation within the single sovereign wallet (no separate physical address required).
  - [ ] Virtual Tax Escrow Buffer: earmark 40% of realized fee harvest revenue AND net capital gains to protect the operator's forward tax obligations.
  - [ ] Liquid Growth Dry Powder: track 60% of harvested yield and trading returns available for tactical market dip accumulation.
  - [ ] Over-allocation Guardrail: cockpit alert if a manual or tactical move would dip into the 40% tax reserve buffer or breach the treasury floor invariant.

### Tax Compliance & Koinly Audit Integration
- [ ] **Koinly-Ready Tax Export Engine**:
  - [ ] Generate standardized, pre-formatted CSV and JSON exports adhering strictly to Koinly custom import specifications.
  - [ ] Classify on-chain events unambiguously: tag fee claims as `Income / Staking Reward`, tag pool rebalances as non-taxable liquidity transfers.
  - [ ] Expose `/v1/tax/export?format=koinly` endpoint on Axum backend.
- [ ] **"Tie-Out" Test Suite & Historical Verification**:
  - [ ] Automated accounting validation tests: prove that local ledger state, on-chain balances, Uniswap/indexer P/L gauges, and Koinly-formatted transaction histories mathematically "tie out" without unexplained discrepancies.
  - [ ] Audit preparation package for CPA review, ensuring peace of mind and zero tax surprises.

### Canvas Expansion & State Room Redesign (v0.2.0 vs v0.1.0)
- [ ] **Visual Identity & UI Overhaul**:
  - [ ] Complete UI refresh making it impossible to confuse a v0.1.0 dashboard with a v0.2.0 dashboard.
  - [ ] Expanded Graph Canvas: increase panel count beyond the original 7 charts to plot discovered ETH pools, multi-vault trajectories, and staked ETH accumulation curves.
  - [ ] Dual-Dial Cockpit: distinct visual gauges for the 40% Tax Reserve Buffer and 60% Growth Dry Powder.
  - [ ] Live P/L Performance Dial: visual capital gains tracking alongside yield harvest counters.
  - [ ] WalletConnect Session Banner with live connection status and paired wallet indicator.
  - [ ] Tax Health & Invariant Safeguard indicator (clear visual reassurance that tax obligations are covered).

### CI Performance & Rust Dependency Caching
- [ ] **GitHub Actions Build Optimization**:
  - [ ] Integrate `Swatinem/rust-cache@v2` across `build-brain` and `build-worker` jobs.
  - [ ] Cache cargo registry, git checkouts, and target directories for `SQLX_OFFLINE=true` release compilation to cut CI turnaround times from minutes to seconds.

### Multi-Tenant Architecture & Urbit Roadmap
- [ ] **Parameterized Operator Profiles (Backend & Schema)**:
  - [ ] Migrate single-wallet config to multi-tenant profile schema in `specs/core/snapshot.yaml` and `api.yaml`.
  - [ ] Parameterized invariant floors (custom minimum balance target beyond 100k).
  - [ ] Configurable tranche allocations and custom split ratios (60/40, 80/20, 50/50).
- [ ] **Urbit Native (%nocknock Desk Blueprint)**:
  - [ ] Hoon data structures in `sur/nocknock.hoon`.
  - [ ] Gall agent state skeleton in `app/nocknock.hoon`.

## [0.1.1] - 2026-10-11

### Fixed
- **Freshness Polling Hook & Stale Closure**:
  - Resolved stale snapshot reference in `useFreshness` polling interval with `snapshotRef` and continuous background conditional GET polling ([#7](https://github.com/urmanac/nocknock/issues/7)).
- **Dashboard Metric Calculation & Contrast**:
  - Re-engineered Portfolio Deviation Matrix with high-contrast Sky Blue (`#38bdf8`) Target % and vivid Emerald/Amber/Red Actual % on dark background; bright axis tick labels (`#cbd5e1`/`#e2e8f0`) ([#8](https://github.com/urmanac/nocknock/issues/8)).
  - Replaced hardcoded static percentages (`+6.3%`, `+5.6%`) with live mathematical derivations.
- **Dynamic Chart YAxis Bounds**:
  - Fixed clipping in Sovereign NOCK Stack AreaChart by deriving dynamic domain bounds framing the 100k floor ([#9](https://github.com/urmanac/nocknock/issues/9)).
- **Uniswap V4 Uncollected Yield**:
  - Dynamically calculates accreting V4 fees scaled by position liquidity ratio and config baselines ([#5](https://github.com/urmanac/nocknock/issues/5)).
- **Historical Snapshot Timestamp Integrity**:
  - Preserved original `generated_at` timestamps in PostgreSQL by skipping row mutation when state hash is unchanged ([#10](https://github.com/urmanac/nocknock/issues/10)).
- **Elimination of Silent RPC Mock Fallbacks**:
  - Strict validation on Alchemy RPC calls for pool reserves, token balances, total supply, and V4 liquidity to fail explicitly rather than substituting synthetic constants ([#11](https://github.com/urmanac/nocknock/issues/11)).
- **Canvas Tooltip Date Formatting**:
  - Resolved `Invalid Date` display in the Total Value vs Baseline dual-axis chart and Strategy Counterfactual chart.
  - Eliminated redundant `new Date()` wrapping on pre-formatted snapshot time strings.
- **Chart Tooltip Theme & Visual Contrast**:
  - Aligned all Recharts tooltips with the State Room dark aesthetic (`#161b22` container, `#30363d` border, `#8b949e` label header).
  - Eliminated default stark white tooltip cards that clashed with cockpit canvas panels.

- **Model Context Protocol (MCP) Verification & Stateless HTTP**:
  - Exposed `/v1/mcp` and `/mcp` endpoints on the Axum ingestion worker delivering stateless HTTP MCP services compliant with the 2024-11-05 MCP specification.
  - Added optional Bearer token authentication via `mcp_bearer_token` configuration.
  - Added comprehensive automated test suite (`scripts/playtest-mcp.py` / `make test-mcp`) validating handshake, tool discovery, and tool execution across both STDIO and HTTP transports.
  - Dynamically bound `nocknock-mcp` version to `env!("CARGO_PKG_VERSION")`.
  - Removed silent mock spot price fallback in edge direct-RPC mode, ensuring explicit failure states when Base RPC queries fail.
- **Operator Guidance & Invariants Compilation**:
  - Published [`OPERATOR_GUIDANCE.md`](OPERATOR_GUIDANCE.md) documenting standing orders, branch protection protocols, zero-mock-data rules, and 60/40 virtual sub-ledger accounting.

### Changed
- **CI Build Performance**:
  - Added `Swatinem/rust-cache@v2` across `build-brain` and `build-worker` GitHub Actions jobs, dramatically accelerating offline SQLX compilation times.

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
