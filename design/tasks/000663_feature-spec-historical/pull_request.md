# Add historical price backfill & daily RSI engine

## Summary
Implements the v0.3.0 Historical Price Backfill & Multi-Timeframe Regime Engine: an idempotent, fail-open gap-filling engine for `market_price_history` fed by free unauthenticated OHLCV feeds (GeckoTerminal, DEXScreener fallback), and a regime refactor so the Overextension Cooldown keys on the industry-standard **daily RSI(14)** instead of the ~70-minute pulse produced by the old `LIMIT 50` snapshot window. Laptop-sleep gaps no longer distort macro indicators, and no on-chain state is ever fabricated — every price row carries explicit provenance.

## Changes
- **Schema**: new migration `20261012000004_create_market_price_history.up.sql` (`timestamp` PK, `price_usd`, nullable OHLCV, `source`, `is_backfilled`, `backfilled_at`).
- **Contemporaneous recording**: every ingestion cycle stores the live Aerodrome spot (`aerodrome_rpc`, `is_backfilled=false`) before the `state_hash` dedup short-circuit.
- **Backfill engine (`ensure_price_history`)**: runs at boot and at the start of every cycle; gap > 15 min → hourly candles (limit 168), depth < 14 days → daily candles (limit 30); idempotent `ON CONFLICT DO NOTHING`; in-progress provider candles skipped; DEXScreener fallback seeds at most one point at observation time; all failures log and never block the ingestion loop.
- **Regime refactor**: `MarketRegime` now carries `rsi_5m` and `rsi_1d`; `overextended` = daily RSI(14) ≥ 70; directive text cites "daily RSI(14) N >= 70". API adds `rsi_14_1d` while `rsi_14` keeps its 5-minute semantics (industry naming per review: RSI(14) = 14 bars of the underlying timeframe).
- **Provenance API**: `GET /v1/snapshot` + `/v1/snapshot/latest` return top-level `data_provenance: { last_backfill_at, backfilled_points_count }` (computed live; excluded from `state_hash`/ETag).
- **Config**: optional serde-defaulted `market_data` section (base URLs, pool, thresholds, lookbacks); `config.yaml.example` documented.
- **Contracts/release**: `specs/core/snapshot.yaml` documents `DataProvenance`, `rsi_14_1d`, and updated `rsi_14`/`regime` descriptions; `Cargo.toml` bumped to 0.3.0; CHANGELOG `[Unreleased]` entry added.

## Testing
- Unit tests (43/43 pass): GeckoTerminal/DEXScreener parsing against live-verified fixtures, `plan_backfill` decision matrix, daily-close bucketing, RSI provenance/gap invariance, and the core regression (micro-overbought + macro-normal → no cooldown; macro-overbought → cooldown fires).
- DB-gated integration tests verified against live Postgres: idempotent re-upsert (1 row), backfill cannot overwrite a contemporaneous row.
- Pre-flight green: `make test-brain` (3/3), `make test-worker` (43/43), `make test-frontend` (25/25), `SQLX_OFFLINE=true cargo build --release`.
- Live E2E (sandbox Postgres, real providers + real Alchemy config): boot on empty table backfilled **29 daily + 161 hourly candles**; second check no-op; cycle recorded the contemporaneous spot; `rsi_14_1d = 38.3` from 30 real daily closes; `/v1/snapshot` returned `data_provenance` with `backfilled_points_count: 190`.
- Note: `make validate-specs` already fails on `main` with 3 pre-existing errors/13 warnings; this branch adds zero new lint problems (identical counts before/after).
