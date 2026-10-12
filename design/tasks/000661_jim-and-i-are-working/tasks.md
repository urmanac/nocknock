# Implementation Tasks: NockNock v0.2.0 Golden Path — Android Claim MVP & Sovereign Cockpit

> Ordered by dependency. Each phase is one or more PRs against `nocknock` (branch off `main`, PR lands via `unset GITHUB_TOKEN && gh pr create --fill`). Pre-flight per PR: `make test-brain && make test-worker && make test-frontend && make test-mcp && make dist`.

## Phase 1 — Golden Path payload engine (worker)

- [ ] Smoke test: same-device Reown Sign pairing with the Uniswap app on a real phone, signing an Aerodrome `claimFees()` contract-interaction tx — **requires operator's device (sandbox has no phone); folded into the Phase 7 playtest**
- [~] Add `OperatorProfile` fields: `tax_reserve_ratio` (default 0.40) to `config.yaml.example` + worker config struct with test
- [ ] Implement `GET /v1/execute/claim-fees/payload`: build Aerodrome `claimFees()` (`0xd294f093` → `0x85f1aa3a…d626`) and Uniswap V4 PositionManager `modifyLiquidities` collect (`DECREASE_LIQUIDITY` 0-delta + `TAKE_PAIR` → sovereign wallet) unsigned txs from the live snapshot, with pre-dispatch `eth_call` simulation and explicit failure
- [ ] Add unit tests for the V4 posm multicall encoder (action byte layout, deadline, recipient padding) mirroring the existing `U256`/hex-parsing test style
- [ ] Extend `specs/core/api.yaml` with the payload endpoint and run `make test-mcp` to confirm MCP/HTTP surfaces still pass

## Phase 2 — Minimal Android claim app

- [ ] Create `android/` module: single-activity Kotlin app, dark aesthetic tokens, no wallet/credential UI
- [ ] Integrate Reown/WalletConnect Sign SDK; implement pairing (on-device deep link + QR to Uniswap app) and persisted session reconnection
- [ ] Handle the Uniswap-app broadcast quirk: surface an "approved, awaiting wallet broadcast" state and mark a tx complete only when its receipt is visible on-chain
- [ ] Keep the app stateless about money: persist only the server URL + WC session; add a `/v1/health` version/capability probe that warns if required payload endpoints are missing
- [ ] Implement `NockApi.kt`: fetch `/v1/snapshot/latest` and `/v1/execute/claim-fees/payload`; explicit degraded state when the worker is unreachable
- [ ] Implement "EXECUTE BATCH HARVEST": simulate-first via worker, then queue the two txs sequentially as `eth_sendTransaction` WC requests; show per-tx status + Base tx hash
- [ ] Post confirmed tx hashes to `POST /v1/execute/record-harvest`; display ledger confirmation
- [ ] Build debug APK locally (`gradle assembleDebug`), verify it contains no secrets (no Alchemy key/tokens), and document sideload install in README/DEPLOYMENT

## Phase 3 — Harvest ledger, 60/40 sub-ledger, tax export

- [ ] Add `harvest_events` table (init-db.sql migration) + `POST /v1/execute/record-harvest` that verifies the Base receipt before recording (reject unverifiable hashes)
- [ ] Compute virtual 60/40 split (40% tax reserve / 60% dry powder) + `tax_health` in `compute_derived_metrics`; extend `specs/core/snapshot.yaml` and the frontend contract tests
- [ ] Brain guardrail: block dip-deployment advice that would breach the tax reserve or `nock_floor_target`; brain test for the invariant (brain stays pure math)
- [ ] Implement `GET /v1/tax/export?format=koinly` (CSV first, JSON secondary) tagging fee claims as income and rebalances as transfers, each row with its Base tx hash
- [ ] Add Koinly tie-out test suite to `make test-worker`: ledger ↔ on-chain receipt ↔ export consistency for a harvested-fee scenario

## Phase 4 — Anti-top-buying guardrails + notifications

- [ ] Worker: compute RSI / upper-band extension series from historical snapshots (reuse structural-channels pipeline) and feed regime inputs to the brain (issue #29)
- [ ] Brain: emit `SWEEP TO STABLE / ACCUMULATE DRY POWDER` + cooldown directive when overextension is detected; suppress buy advice during it (issue #29)
- [ ] Worker: detect directive transitions between cycles and dispatch notifications (issue #28): Telegram bot, Pushover, generic webhook adapters configured under `[notifications]` in `config.yaml`
- [ ] Notification payload: trigger, gross claimable, estimated gas, net accretion, deep link to lite cockpit; log (never swallow) delivery failures

## Phase 5 — Web cockpit: lite mobile view + WalletConnect

- [ ] Integrate Reown AppKit/Wagmi in the React app; persist/reconnect WC session across refreshes (issue #31, shares Phase 1 payloads)
- [ ] Build the lite mobile cockpit view: floor health + buffer delta, uncollected fees + gas-to-fee ROI, "EXECUTE BATCH HARVEST" button (issue #31)
- [ ] Desktop dashboard: WalletConnect session banner with paired-wallet indicator

## Phase 6 — Visual overhaul, discovery, Coinbase, CI

- [ ] Dual-dial gauges (40% tax reserve vs 60% dry powder) + tax-health indicator + at-a-glance P/L dial computed from local recorded data only (high-water marks, spot history, verified harvests — no external APIs in 0.2.0); retire the dormant `structural_channels` panel per TRIAGE item 12
- [ ] Expand graph canvas beyond the original 7 panels for pool registry, multi-vault trajectories (follow dynamic-Y-domain and contrast rules in `OPERATOR_GUIDANCE.md` §8)
- [ ] Config-driven multi-AMM pool registry: add ETH-related Base pools to tracking, unified NAV aggregation, historical snapshots (read-only)
- [ ] Coinbase read-only connector: record staked ETH/cbETH balance snapshots (public endpoints or key — per open question 3); explicit degraded state on failure
- [ ] Add `Swatinem/rust-cache@v2` to `build-brain`/`build-worker` CI jobs (leverage the CI changes pattern from PR #27's Makefile/CI work)
- [ ] Optional: CI job to build + attach debug-signed APK as a 5th release artifact
- [ ] Multi-tenant schema blueprint only: parameterized floors/split ratios noted in `specs/core/snapshot.yaml` comments + Urbit `%nocknock` desk notes updated in HANDOFF.md (no runtime multi-tenancy)

## Phase 7 — Release readiness

- [ ] Update `CHANGELOG.md` `[0.2.0]` section from `[ ]` to landed items; keep `## [Unreleased]` for post-0.2.0
- [ ] Update `HANDOFF.md` (Golden Path now executed end-to-end), `OPERATOR_GUIDANCE.md` (Android app install + pairing runbook), `TRIAGE.md` (close related registry items)
- [ ] Document the WASM brain code-reuse gap (Android consumes REST-computed values for 0.2.0; encoders are Rust-only) in `ARCHITECTURE.md` so the long-term reuse goal isn't silently lost
- [ ] Playtest the full loop on the real phone: notification → lite cockpit/Android app → pair → batch harvest → biometric sign → ledger + Koinly row appear
- [ ] Verify release packaging (`make dist`) including the APK if included; then, after all CI green on `main`, tag `v0.2.0` **only with explicit operator authorization** (standing order)
