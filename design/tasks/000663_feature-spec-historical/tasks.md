# Implementation Tasks: Historical Price Backfill & Multi-Timeframe Regime Engine

## Phase A — Schema & Contemporaneous Recording

- [x] Add migration `ingestion-worker/migrations/20261012000004_create_market_price_history.up.sql` per design Phase A (PK `timestamp`, `price_usd`, nullable OHLCV, `source`, `is_backfilled`, `backfilled_at`, DESC index, all `IF NOT EXISTS`)
- [x] In `run_ingestion_cycle`, insert the live spot price as a contemporaneous row (`source='aerodrome_rpc'`, `is_backfilled=false`) via `INSERT ... ON CONFLICT (timestamp) DO NOTHING`, placed before the `state_hash` dedup short-circuit, skipped when `live_spot <= 0`

## Phase B — Backfill Engine

- [~] Add GeckoTerminal client: serde structs for `data.attributes.ohlcv_list` entries `[open_time, open, high, low, close, volume]` (newest-first → reverse), plus DEXScreener `pairs[0].priceUsd` parser
- [~] Add pure `plan_backfill` decision function (gap > 15 min, depth < 14 days, empty-table boot case, fallback seeding) per design Phase B
- [~] Implement `ensure_price_history(&state)` fail-open: boot call (after migrations, before first cycle) + call at start of every `run_ingestion_cycle`; all errors logged and swallowed; ≤3 HTTP calls per check; HTTP 429 → log and stop
- [~] Implement idempotent upsert of candles (`is_backfilled=true`, `source='geckoterminal'`, `backfilled_at=now()`), skipping candles ≥ latest contemporaneous point
- [~] Implement DEXScreener single-point fallback (timestamp = observation time, `source='dexscreener'`, never backdated) used only when GeckoTerminal fails and a gap exists

## Phase C — Macro Regime & Provenance

- [~] Add macro daily-close query (`DISTINCT ON (date_trunc('day', timestamp))`, last price per UTC day, ascending) from `market_price_history`; append live spot as current partial-day close; feed existing `compute_rsi_series(&closes, 14)`
- [~] Extend `MarketRegime` with `rsi_macro: Option<f64>`; key `overextended` on macro RSI; update Overextension Cooldown directive text in `compute_derived_metrics` to cite the macro value ("Macro RSI(14) ...")
- [~] Add `rsi_macro_14` to `GlobalStatus` (`"{:.1}"` formatting like `rsi_14`); keep `rsi_14` as the unchanged micro series
- [~] Compute and inject top-level `data_provenance: { last_backfill_at, backfilled_points_count }` into `GET /v1/snapshot` + `/v1/snapshot/latest` responses (computed live; not in `SnapshotEnvelope`, not in `state_hash`; 304 path untouched)

## Phase D — Config, Contracts, Tests, Release

- [ ] Add optional serde-defaulted `market_data` config section (`AppConfig` + `config.yaml.example` block: base URLs, pool address, gap/depth thresholds, lookbacks, `backfill_enabled`)
- [ ] Regenerate and commit `.sqlx/` query cache entries for any new macro queries (or keep new queries runtime-form so `SQLX_OFFLINE=true cargo build --release` stays green)
- [ ] Update `specs/core/snapshot.yaml` (add `rsi_macro_14`, clarify `rsi_14` = micro, `regime` keyed on macro) and `specs/core/api.yaml` for `data_provenance`; run `make validate-specs`
- [ ] Unit tests in `main.rs` test module: GeckoTerminal/DEXScreener parsing (live-verified fixtures), `plan_backfill` cases, macro day-bucketing, RSI order/gap invariance, and the core regression (micro-overbought + macro-normal → no Overextension Cooldown; macro-overbought → fires)
- [ ] Add DB-gated integration tests (skip gracefully when `DATABASE_URL` unset): idempotent re-upsert → 1 row; backfill cannot overwrite contemporaneous row
- [ ] Bump `ingestion-worker/Cargo.toml` to `0.3.0` and add `CHANGELOG.md` `## [0.3.0]` section (backfill engine, macro/micro RSI split, `data_provenance`)
- [ ] Pre-flight: `make test-brain`, `make test-worker`, `make test-frontend`, `SQLX_OFFLINE=true cargo build --release` (worker), `make validate-specs` all green; then branch off `main`, PR, and merge per branch-protection protocol (no tag until merged + green CI + CHANGELOG populated)
