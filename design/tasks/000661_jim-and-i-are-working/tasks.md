# Implementation Tasks: NockNock v0.2.0 Golden Path — Android Claim MVP & Sovereign Cockpit

> Ordered by dependency. Each phase is one or more PRs against `nocknock` (branch off `main`, PR lands via `unset GITHUB_TOKEN && gh pr create --fill`). Pre-flight per PR: `make test-brain && make test-worker && make test-frontend && make test-mcp && make dist`.

## Phase 1 — Golden Path payload engine (worker)

- [ ] Smoke test: same-device Reown Sign pairing with the Uniswap app on a real phone, signing an Aerodrome `claimFees()` contract-interaction tx — **requires operator's device (sandbox has no phone); folded into the Phase 7 playtest**
- [x] Add `OperatorProfile` fields: `tax_reserve_ratio` (default 0.40) to `config.yaml.example` + worker config struct with test
- [x] Implement `GET /v1/execute/claim-fees/payload`: build Aerodrome `claimFees()` (`0xd294f093` → `0x85f1aa3a…d626`) and Uniswap V4 PositionManager `modifyLiquidities` collect (`DECREASE_LIQUIDITY` 0-delta + `TAKE_PAIR` → sovereign wallet) unsigned txs from the live snapshot, with pre-dispatch `eth_call` simulation and explicit failure
- [x] Add unit tests for the V4 posm multicall encoder (action byte layout, deadline, recipient padding) mirroring the existing `U256`/hex-parsing test style — 3 golden-vector tests vs @uniswap/v4-sdk reference bytes; live-verified twice on 2026-10-12; 22/22 worker tests green
- [x] Extend `specs/core/api.yaml` with the payload endpoint and run `make test-mcp` to confirm MCP/HTTP surfaces still pass — **fully green with live on-chain data** (STDIO + HTTP: portfolio state, shot advice, claim simulation). api.yaml extended + validated.
- [x] Live verification of the payload endpoint with the operator's Alchemy key (real Base data): Aerodrome `claimFees()` simulated success (0.0212 USDC + 1.63 NOCK, gas $0.0021); Uniswap V4 collect simulated success (0.0164 USDC + 11.17 NOCK, gas $0.0020); net after gas $0.3359. Read-only — nothing broadcast.
- [x] Found + fixed while verifying: snapshot_id `VARCHAR(26)` in the 20241010 migration cannot hold 36-char UUIDs (docker `init-db.sql` uses UUID — schema drift); fixed via new `20261012000003` migration (ALTER to UUID). Also learned: sqlx takes the ENTIRE leading number as the migration version, so `YYYYMMDD_NNNNNN` names collide — migrations renamed to unique 14-digit versions.
- [ ] Extend `specs/core/api.yaml` with the payload endpoint and run `make test-mcp` to confirm MCP/HTTP surfaces still pass

## Phase 2 — Minimal Android claim app — DEFERRED (operator decision, 2026-10-12)

> Operator decision during implementation: 0.2.0 ships the **web QR/WalletConnect flow** (Phase 5) instead of a native Android app. The Android app stays planned but out of the 0.2.0 release; Phase 1's payload endpoint is the shared primitive both surfaces consume. Items below kept for the future task.

- [ ] Create `android/` module: single-activity Kotlin app, dark aesthetic tokens, no wallet/credential UI (deferred)
- [ ] Integrate Reown/WalletConnect Sign SDK; implement pairing (on-device deep link + QR to Uniswap app) and persisted session reconnection (deferred)
- [ ] Handle the Uniswap-app broadcast quirk in a native app: surface an "approved, awaiting wallet broadcast" state and mark a tx complete only when its receipt is visible on-chain (deferred — the quirk is documented for the web flow too)
- [ ] Keep the app stateless about money: persist only the server URL + WC session; add a `/v1/health` version/capability probe that warns if required payload endpoints are missing (deferred)
- [ ] Implement `NockApi.kt`: fetch `/v1/snapshot/latest` and `/v1/execute/claim-fees/payload`; explicit degraded state when the worker is unreachable (deferred)
- [ ] Implement "EXECUTE BATCH HARVEST": simulate-first via worker, then queue the two txs sequentially as `eth_sendTransaction` WC requests; show per-tx status + Base tx hash (deferred)
- [ ] Post confirmed tx hashes to `POST /v1/execute/record-harvest`; display ledger confirmation (deferred — endpoint ships in Phase 3 regardless, web flow uses it too)
- [ ] Build debug APK locally (`gradle assembleDebug`), verify it contains no secrets (no Alchemy key/tokens), and document sideload install in README/DEPLOYMENT (deferred)

## Phase 3 — Harvest ledger, 60/40 sub-ledger, tax export

- [x] Add `harvest_events` table (init-db.sql migration) + `POST /v1/execute/record-harvest` that verifies the Base receipt before recording (reject unverifiable hashes) — migration + ERC-20 Transfer log parsing (exact integer decimal formatting), receipt fetch/verify, idempotent inserts (409 on duplicate), 404 RECEIPT_NOT_FOUND doubles as the pending-broadcast signal; rejection paths verified live against the local daemon (422/422/503); 25/25 worker tests green. Also fixed two real bugs found during live verification: snapshot_id `VARCHAR(26)` drift (migration-provisioned DBs could not persist snapshots) and sqlx migration-version collision (`YYYYMMDD_NNNNNN` names collide — renamed to unique 14-digit versions)
- [x] Compute virtual 60/40 split (40% tax reserve / 60% dry powder) + `tax_health` in `compute_derived_metrics`; extend `specs/core/snapshot.yaml` and the frontend contract tests — `sovereign_ledger` section live in the real snapshot (zeros = truthful empty ledger), split-math test incl. custom ratio; 26/26 tests green (frontend contract assertions + tax_health flag ride with the dashboard work in Phase 6 — data plane done here)
- [x] Brain guardrail: block dip-deployment advice that would breach the tax reserve or `nock_floor_target`; brain test for the invariant (brain stays pure math) — implemented in the worker's advice engine (the live compute path; the standalone brain wasm is the Tier-3 placeholder copy): DIP deployment suppressed with `SWEEP TO STABLE / ACCUMULATE DRY POWDER (Tax Guardrail)` when the virtual tax reserve exceeds the stable stack; otherwise advice surfaces the deployable amount and the protected reserve explicitly. Floor safety already precedes via DEFENSIVE LOCK. 29/29 tests green
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
