# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Deferred to 0.3.0 / Future Releases
- [ ] **Historical Price Backfill Engine**: Retroactive OHLCV gap-filling via GeckoTerminal for multi-day macro indicators.
- [ ] **Native Android App (APK)**: Standalone native Android client (currently served via responsive mobile web cockpit).
- [ ] **Multi-Pool Dynamic Ingestion**: Discovering and tracking secondary AMM pools on Base.
- [ ] **Coinbase Staked-ETH Connector**: CDP API integration to monitor staked ETH balances.
- [ ] **Urbit Native (%nocknock Desk)**: Hoon data structures and Gall agent.

## [0.2.1] - 2026-10-12

### Fixed — 0.2.x Triage Cycle (task 000664, live-data audit)
- **Zero-mock**: ETH/USD price lookup in the claim-fees gas valuation no longer falls back to a hardcoded $2,500; when the price RPC fails, gas USD degrades to "unavailable" instead of a fabricated valuation.
- **Zero-mock**: removed the synthetic Aerodrome tick bracket (0.00000 → 999.00000) from `tick_bracket_hypervisor`; the chart now shows only the live tick-derived V4 bracket (Tranche 2 liquidity remains visible in the stack/deviation panels).
- **Zero-mock**: MCP formatters (`nocknock-mcp` stdio + `/v1/mcp` HTTP) surface the live gas break-even from the snapshot ("unavailable" when absent) instead of a hardcoded ~$0.005 gas and a stale $5.00 execution trigger.
- **Tax honesty**: harvest `confirmed_at` now uses the on-chain block timestamp instead of the recording wall clock, keeping Koinly export dates correct across pending-broadcast retries.
- Gas break-even no longer silently clamped at $0.05 (documented $0.002 L1-data buffer retained).
- Dashboard: deviation matrix legend no longer shows a phantom black swatch for "Actual %" (the per-status color key below the chart documents the real cell colors); Floor Health keeps its "%" unit; tick-bracket subtitle derives IN/OUT OF RANGE from live state; residual hardcoded fallbacks (floor 100000, bracket strings, $0.00 price) replaced with honest placeholders; 60/40 dials no longer draw a phantom sliver at zero.
- Lite Cockpit: EXECUTE actions hidden while WalletConnect is unconfigured; an idempotent record-harvest retry (409 ALREADY_RECORDED) now completes as recorded instead of showing a failure; API errors surface the backend's human `detail` with the HTTP status preserved.
- Golden-path handlers return a clean 503 when no tracked wallet is configured instead of panicking on the empty config index.
- Makefile tmux targets use `$(CURDIR)` instead of a hardcoded macOS path.

### Tests
- Worker: 33 unit tests (+2: claim-simulation gas honesty, no synthetic bracket).
- Frontend: 25 tests (+5: error mapping detail/status, record-harvest classifier).

## [0.2.0] - 2026-10-12

### Added — Golden Path: Web Cockpit & Mobile Wallet Execution
- **WalletConnect v2 Integration (`@walletconnect/ethereum-provider`)**:
  - Instant QR pairing modal and deep links connecting the cockpit to the Uniswap mobile app on Android/iOS.
  - Active session auto-reconnects across refreshes with zero private key custody; mismatched wallet addresses are strictly rejected before signing.
  - Requires `VITE_WALLETCONNECT_PROJECT_ID` (Reown project ID), gracefully displaying setup hints when unconfigured.
- **Sequential Harvest Execution Engine (`GET /v1/execute/claim-fees/payload`)**:
  - Dynamically constructs unsigned Aerodrome `claimFees()` and Uniswap V4 PositionManager collect transactions (`modifyLiquidities` decrease 0 + `takePair`) from live on-chain snapshots.
  - Calldata verified against `@uniswap/v4-sdk` planner specifications and pre-simulated on Base via Alchemy `eth_call`.
  - Dispatched directly via WalletConnect for biometric confirmation (Fingerprint / Face ID); no transaction aggregation or proxying (preserving Aerodrome `msg.sender` invariant).
- **Receipt-Verified Harvest Ledger (`POST /v1/execute/record-harvest`)**:
  - Queries Base transaction receipts, extracts ERC-20 Transfers for USDC and NOCK paid to the sovereign wallet, and records verified harvest events idempotently.
  - Resilient broadcast retry handling absorbs Uniswap mobile app delayed broadcast latency without false errors.
- **Virtual 60/40 Sub-Ledger (`sovereign_ledger`)**:
  - Maintains internal accounting allocation within the single sovereign wallet driven by `tax_reserve_ratio` (default 40%).
  - Splits verified harvest revenues into a strictly protected Virtual Tax Reserve Buffer (40%) and Liquid Growth Dry Powder (60%).
  - Tax guardrail suppresses buy recommendations (`SWEEP TO STABLE / ACCUMULATE DRY POWDER (Tax Guardrail)`) when tax reserves exceed stable cash reserves.
- **Koinly-Ready Tax Export Engine (`GET /v1/tax/export?format=koinly`)**:
  - Generates standardized CSV exports adhering strictly to Koinly custom import specifications.
  - Fee harvests tagged unambiguously as `income` with verified Base transaction hashes, ensuring CPAs and tax software distinguish yield income from capital dispositions.
- **Anti-Top-Buying Guardrail & Regime Detection**:
  - Wilder RSI(14) calculated over historical NOCK spot price series; surfaces `regime` and `rsi_14` in GlobalStatus.
  - Overextension Cooldown suppresses FOMO buy directives when RSI ≥ 70 (`SWEEP TO STABLE / ACCUMULATE DRY POWDER (Overextension Cooldown)`).
- **Directive-Transition Notifications**:
  - Dispatches transition alerts via Telegram, Pushover, and generic HTTP webhooks when active directives shift, with silent first-boot and deep links to cockpit.
- **Lite Mobile Cockpit & 1-Click View Switcher**:
  - Dedicated mobile-friendly cockpit at `/?view=lite` rendering floor health, uncollected fee valuation, and Golden Path execution buttons.
  - 1-click view switcher pills in header (`⚡ LITE COCKPIT` ↔ `📊 FULL STATE ROOM`).
- **Sovereign 60/40 Ledger Panel**:
  - Dual visual gauges on desktop dashboard plotting protected tax reserves vs deployable dry powder, alongside live P/L vs high-water mark.

## [0.1.4] - 2026-10-12

### Fixed — Worker Data Integrity
- **Snapshot deduplication actually fires** (`fix(worker): canonical state hash`): the
  state hash embedded wall-clock timestamps (`updated_at`, `high_water_mark_date`, series
  `time_points`, fabricated channel timestamps), so it changed every ingestion cycle and
  `store_snapshot` inserted a new row every 5 minutes even with unchanged on-chain state —
  growing the table unboundedly and filling history charts with duplicate flat points.
  The hash now strips volatile time fields; identical state no longer inserts rows while
  `/v1/snapshot/latest` still serves fresh timestamps from the in-memory cache.
- **Strategy Counterfactual uses recent history**: the backfill query selected the
  OLDEST 100 snapshots (`ORDER BY generated_at ASC LIMIT 100`), so the chart stopped
  reflecting recent history as the table grew. It now selects the most recent 100,
  oldest-first, as paired `(timestamp, portfolio, native)` tuples so the two series can
  never misalign, and carries real snapshot timestamps instead of fabricated 5-minute
  spacing (structural channels likewise).
- **Bollinger window off-by-one**: the final SMA window now includes the newest sample.

### Fixed — Zero-Mock Invariant
- Failing ETH-price or gas-price RPC queries now **fail the ingestion cycle** instead of
  silently substituting hardcoded `2500.0` USD / `0.005` USD / `0.0043` USD fallbacks into
  snapshots (gas break-even, efficiency ratios, advice text).
- `recommended_action` is computed from claimable yield vs gas break-even
  (`claim_now` at ≥2× gas, else `claim_recommended` / `accumulate`) — the dashboard no
  longer shows "CLAIM NOW" at $0 claimable.
- Uniswap V4 bracket display prices, `in_range`, and `distance_bps` are derived from the
  on-chain ticks (verified `0.012366` / `2.02788`) instead of duplicated hardcoded
  strings; the RANGE EXIT advice threshold uses the same derived bound.

### Fixed — Frontend
- **Dashboard survives transient fetch failures**: `useFreshness` no longer tears down and
  recreates its polling interval (with an extra foreground fetch) on every render, and a
  failed fetch keeps last-known data — the full-screen error state appears only when no
  snapshot has ever loaded. History fetch retries every 60s while empty.
- **Deep, bounded chart timeline** (~24h instead of ~4h): charts request 288 snapshots;
  historical responses truncate embedded multi-point series the charts never read,
  bounding the payload to ~1 MB.
- **Honest stat labels**: the "Alpha Generated vs passive hold" stat is relabeled to what
  it actually measures — "Stable Reserve Delta (Total − Native-Only)"; the counterfactual
  panel is titled "Portfolio Composition: Total (Native + USDC) vs Native-Only".
- The tick-bracket chart shows the true upper price (removed hardcoded $2.50 clip).
- New 7th panel renders the backend Bollinger structural channel (SMA ± 1σ/2σ bands)
  when band data exists.

### Fixed — API Contract
- `/v1/snapshot/historical` implements `before`/`after` RFC3339 cursor filters, clamps
  `limit` to 1–1000, and returns 400 for the never-implemented `interval` parameter
  (removed from `specs/core/api.yaml`).
- `client.ts` `request()` no longer lets caller headers clobber `Content-Type`.
- Removed dead code: `fetchIfFresh`, `useFreshnessPoll`.

### Tests
- Worker: 18 unit tests (was 11) — hash time-invariance/value-sensitivity, counterfactual
  alignment & real timestamps, Bollinger window, bracket-price round-trip, bracket
  distance, claim-action thresholds.
- Frontend: 20 contract/logic tests green; tsc build clean.

## [0.1.3] - 2026-10-12

### Fixed
- **Uniswap V4 Live Fee Querying via StateView (`0xa3c0...7a71`)**:
  - Implemented live on-chain queries against official Base Uniswap V4 `StateView` contract using `getPositionInfo` (`0xdacf1d2f`) and `getFeeGrowthInside` (`0x53e9c1fb`).
  - Added deterministic zero-dependency `U256` math (`wrapping_sub` and `mul_div_q128`) to calculate accurate uncollected fees matching on-chain balances and the Uniswap mobile app.
  - Eliminated silent mock fallbacks and static baseline numbers, preserving the zero-mock invariant. Uncollected fees disappear and accrete dynamically upon fee collections.
- **Tiered Freshness Status & Grace Allowance**:
  - Added sync duration measurement (`sync_duration_ms`) via `Instant::now()` exposed on `/health` and `/v1/health` alongside `sync_window_seconds`.
  - Introduced 3-tier sync status in `useFreshness` and App header (`LIVE ON-CHAIN` green, `SYNC IN PROGRESS` cyan, `SYNC DELAYED` amber) with a 90-second grace allowance (`syncGraceSeconds = 90`) on top of the 300s window.
  - Eliminated false-alarm "Data is five minutes old" / "Data is six minutes old" warnings during normal scheduled background sync cycles.
  - Switched `/v1/snapshot/latest` HTTP caching to `Cache-Control: no-cache, must-revalidate` and added `{ cache: 'no-cache' }` fetch headers to prevent browsers from trapping stale snapshots.
- **Contract & Route Aliases**:
  - Mounted `/v1/snapshot` and `/snapshot` route aliases in Axum router alongside `/v1/snapshot/latest`.
  - Added contract tests in `freshness.contract.test.ts` validating tiered sync status, grace allowance thresholds, and `/v1/health` JSON payload contract.

### Added
- **Model Context Protocol (MCP) Stdio Configuration**:
  - Added repository `.mcp.json` configuring `nocknock-mcp` in stdio mode for Gemini Antigravity, Claude Desktop, and Pi Harness (`pi-mcp-adapter`).
  - Documented MCP configuration, environment variables, and verification commands in `OPERATOR_GUIDANCE.md`.

## [0.1.2] - 2026-10-11

### Fixed
- **API Routing Contract & Health Probe**:
  - Mounted `/v1/health` in Axum router alongside root `/health`, adhering to `specs/core/api.yaml` OpenAPI server path (`/v1`) and frontend `api.health()` client expectations ([#16](https://github.com/urmanac/nocknock/issues/16)).
  - Eliminated `.replace('/v1', '')` workaround in `frontend/src/api/freshness.contract.test.ts`.
  - Added dedicated contract test validating `/v1/health` JSON schema response structure.
- **Zero-Mock Data Invariant in Derived Metrics**:
  - Eliminated hardcoded fallback floats (`649976.0`, `24237793.0`, `4004.31`, `23.63`, `5655.59`, `156.64`, `96004.52`, `0.23`, `0.0268`) in `compute_derived_metrics` ([#16](https://github.com/urmanac/nocknock/issues/16)).
  - Ensured dip reinvestment advice requires positive spot price (`spot_price > 0.0 && spot_price < 0.020`) to prevent uninitialized zero values from masquerading as market dips.

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
