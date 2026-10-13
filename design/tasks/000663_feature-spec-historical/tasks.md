# Implementation Tasks: Historical Price Backfill & Multi-Timeframe Regime Engine

## Phase A — Schema & Contemporaneous Recording

- [x] Add migration `ingestion-worker/migrations/20261012000004_create_market_price_history.up.sql` per design Phase A (PK `timestamp`, `price_usd`, nullable OHLCV, `source`, `is_backfilled`, `backfilled_at`, DESC index, all `IF NOT EXISTS`)
- [x] In `run_ingestion_cycle`, insert the live spot price as a contemporaneous row (`source='aerodrome_rpc'`, `is_backfilled=false`) via `INSERT ... ON CONFLICT (timestamp) DO NOTHING`, placed before the `state_hash` dedup short-circuit, skipped when `live_spot <= 0`

## Phase B — Backfill Engine

- [x] Add GeckoTerminal client: serde structs for `data.attributes.ohlcv_list` entries `[open_time, open, high, low, close, volume]` (newest-first → reverse), plus DEXScreener `pairs[0].priceUsd` parser
- [x] Add pure `plan_backfill` decision function (gap > 15 min, depth < 14 days, empty-table boot case, fallback seeding) per design Phase B
- [x] Implement `ensure_price_history(&state)` fail-open: boot call (after migrations, before first cycle) + call at start of every `run_ingestion_cycle`; all errors logged and swallowed; ≤3 HTTP calls per check; HTTP 429 → log and stop
- [x] Implement idempotent upsert of candles (`is_backfilled=true`, `source='geckoterminal'`, `backfilled_at=now()`), skipping candles ≥ latest contemporaneous point
- [x] Implement DEXScreener single-point fallback (timestamp = observation time, `source='dexscreener'`, never backdated) used only when GeckoTerminal fails and a gap exists

## Phase C — Macro Regime & Provenance

- [x] Add macro daily-close query (`DISTINCT ON (date_trunc('day', timestamp))`, last price per UTC day, ascending) from `market_price_history`; append live spot as current partial-day close; feed existing `compute_rsi_series(&closes, 14)`
- [x] Extend `MarketRegime` with macro RSI (per reviewer, industry-standard naming: `rsi_1d` internal / `rsi_14_1d` API field — "daily RSI(14)"); key `overextended` on macro RSI; update Overextension Cooldown directive text in `compute_derived_metrics` to cite the daily series
- [x] Add `rsi_14_1d` to `GlobalStatus` (`"{:.1}"` formatting like `rsi_14`); keep `rsi_14` as the unchanged micro (5-minute) series
- [x] Compute and inject top-level `data_provenance: { last_backfill_at, backfilled_points_count }` into `GET /v1/snapshot` + `/v1/snapshot/latest` responses (computed live; not in `SnapshotEnvelope`, not in `state_hash`; 304 path untouched)

## Phase D — Config, Contracts, Tests, Release

- [x] Add optional serde-defaulted `market_data` config section (`AppConfig` + `config.yaml.example` block: base URLs, pool address, gap/depth thresholds, lookbacks, `backfill_enabled`)
- [x] Keep new queries in runtime `sqlx::query` form (no `query!` macros) so `.sqlx/` cache is unchanged and `SQLX_OFFLINE=true cargo build --release` stays green — verified
- [x] Update `specs/core/snapshot.yaml` (add `rsi_14_1d`, clarify `rsi_14` = 5-minute series, `regime` keyed on daily RSI, add `DataProvenance` schema + envelope field) — Redocly reports the same 3 pre-existing errors + 13 warnings as `origin/main` (api.yaml `ClaimFeesPayload`/`ClaimFeeAction` struct shape, missing `info.license`); my additions introduce zero new lint problems
- [x] Unit tests in `main.rs` test module: GeckoTerminal/DEXScreener parsing (live-verified fixtures), `plan_backfill` cases, macro day-bucketing, RSI order/gap invariance, and the core regression (micro-overbought + macro-normal → no Overextension Cooldown; macro-overbought → fires) — 43/43 pass
- [x] Add DB-gated integration tests (skip gracefully when `DATABASE_URL` unset; verified against the live sandbox Postgres): idempotent re-upsert → 1 row; backfill cannot overwrite contemporaneous row
- [x] Bump `ingestion-worker/Cargo.toml` to `0.3.0` and add `CHANGELOG.md` `[Unreleased] → Added — Market Price Backfill & Multi-Timeframe Regime Engine (v0.3.0)` section (release chore PR later promotes it to a `## [0.3.0]` heading per repo convention)
- [x] Pre-flight: `make test-brain` (3/3), `make test-worker` (43/43), `make test-frontend` (25/25), `SQLX_OFFLINE=true cargo build --release` — all green; branch merged from current `main` (v0.2.1), pushed as `feature/000663-historical-price`

## E2E Verification (live, sandbox Postgres + real providers)

- [x] Boot on empty `market_price_history`: backfilled **29 daily + 161 hourly real candles** from live GeckoTerminal (in-progress candles correctly excluded)
- [x] Immediate second check: idempotent no-op (0 new rows)
- [x] Fresh DB cycle: contemporaneous `aerodrome_rpc` spot row recorded; `rsi_14_1d = 38.3` computed from real daily closes; `regime: normal`; `rsi_14: None` (fresh DB < 15 micro samples → fail-open)
- [x] `GET /v1/snapshot` returns `data_provenance: { last_backfill_at, backfilled_points_count: 190 }`; DB provenance split verified: 190 `geckoterminal`/backfilled vs contemporaneous `aerodrome_rpc` rows
- [x] Release binary boot with fresh (< 15 min) data: silent no-op backfill, snapshot cycle 280ms

## Review Follow-up (pre-merge)

- [x] Frontend observability: render regime badge (NORMAL green / OVEREXTENDED amber), `RSI(14D)` + `RSI(5M)` readouts, and a backfilled-candles provenance indicator in the top Tactical Shot Advice banner (`frontend/src/types/snapshot.ts`, `frontend/src/App.tsx`; envelope field also added to the hook + api-client mirrors). Backend `data_provenance` extended with a truthful `sources` breakdown so the UI names the feed from data instead of hardcoding it. Verified visually against the live worker+vite stack: `REGIME: NORMAL`, `RSI(14D): 41.1`, `RSI(5M): —` (fresh DB fail-open), `✓ 190 backfilled candles (GeckoTerminal)` — screenshot `screenshots/01-dashboard-regime-rsi-provenance.png`
- [x] Lockstep version bump: `brain/Cargo.toml`, `frontend/package.json`, `frontend/package-lock.json` → 0.3.0
- [x] Eliminate `build_v4_collect_unlock_data_decrease_only` dead-code warning — annotated `#[allow(dead_code)]` (kept: test-exercised golden vector); release build now emits zero crate warnings (only the pre-existing sqlx-postgres 0.7.4 future-incompat advisory from the dependency remains, present on main)
- [x] Record v0.4.0 Koinly direction (Issue #56: on-chain DeFi transaction classifier; NO manual CSV download buttons/tooling — direct Base wallet sync only, duplicate-transaction + budget hazard) — documented in CHANGELOG deferred + design.md notes; nothing CSV-related built

## Release Prep (post-approval, CI green on PR #57)

- [x] Verify PR #57 state via GitHub API: head `0fc88f4`, `mergeable_state: clean`, PR body synced from pull_request.md
- [x] Promote CHANGELOG `[Unreleased] → Added` to `## [0.3.0] - 2026-10-13` (tag precondition 3: version section populated); `make release-notes VERSION=0.3.0` parses exactly the Added section; Deferred items stay under `[Unreleased]`; pushed as `3830291` (PR #57 updates, CI re-runs)
- [x] MCP playtest (standing order: never release untested components): STDIO + HTTP all checks passed against the live worker — provenance `sources: {geckoterminal: 190}`, `rsi_14_1d: 41.0`, `regime: normal`
- [ ] OPERATOR: merge PR #57, wait for CI green on `main`, then tag `v0.3.0` — **explicit operator authorization required before tagging per OPERATOR_GUIDANCE.md §1.3** (release flow: tag push triggers `create-release` job which attaches the 4 artifacts and uses `make release-notes` output)
