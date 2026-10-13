# Add historical price backfill & daily RSI engine

## Summary
Implements the v0.3.0 Historical Price Backfill & Multi-Timeframe Regime Engine: an idempotent, fail-open gap-filling engine for `market_price_history` fed by free unauthenticated OHLCV feeds (GeckoTerminal, DEXScreener fallback), and a regime refactor so the Overextension Cooldown keys on the industry-standard **daily RSI(14)** instead of the ~70-minute pulse produced by the old `LIMIT 50` snapshot window. Laptop-sleep gaps no longer distort macro indicators, and no on-chain state is ever fabricated — every price row carries explicit provenance. Includes frontend observability for all the new telemetry.

## Changes
- **Schema**: new migration `20261012000004_create_market_price_history.up.sql` (`timestamp` PK, `price_usd`, nullable OHLCV, `source`, `is_backfilled`, `backfilled_at`).
- **Contemporaneous recording**: every ingestion cycle stores the live Aerodrome spot (`aerodrome_rpc`, `is_backfilled=false`) before the `state_hash` dedup short-circuit.
- **Backfill engine (`ensure_price_history`)**: runs at boot and at the start of every cycle; gap > 15 min → hourly candles (limit 168), depth < 14 days → daily candles (limit 30); idempotent `ON CONFLICT DO NOTHING`; in-progress provider candles skipped; DEXScreener fallback seeds at most one point at observation time; all failures log and never block the ingestion loop.
- **Regime refactor**: `MarketRegime` now carries `rsi_5m` and `rsi_1d`; `overextended` = daily RSI(14) ≥ 70; directive text cites "daily RSI(14) N >= 70". API adds `rsi_14_1d` while `rsi_14` keeps its 5-minute semantics (industry naming per review: RSI(14) = 14 bars of the underlying timeframe).
- **Provenance API**: `GET /v1/snapshot` + `/v1/snapshot/latest` return top-level `data_provenance: { last_backfill_at, backfilled_points_count, sources }` (computed live; excluded from `state_hash`/ETag; per-provider breakdown so consumers name the feed from data).
- **Frontend observability**: Tactical Shot Advice banner now renders a `REGIME: NORMAL` (green) / `OVEREXTENDED` (amber) badge, `RSI(14D)` + `RSI(5M)` chips (honest `—` when a series lacks enough samples), and a subtle `✓ N backfilled candles (GeckoTerminal)` provenance indicator fed by `data_provenance` (typed through `types/snapshot.ts`, `useFreshness.tsx`, and `api/client.ts`).
- **Config**: optional serde-defaulted `market_data` section (base URLs, pool, thresholds, lookbacks); `config.yaml.example` documented.
- **Version lockstep**: `ingestion-worker/Cargo.toml`, `brain/Cargo.toml`, `frontend/package.json`, `frontend/package-lock.json` all at 0.3.0.
- **Warning hygiene**: `build_v4_collect_unlock_data_decrease_only` annotated `#[allow(dead_code)]` (kept for its golden-vector test) — release builds are warning-free.
- **Contracts**: `specs/core/snapshot.yaml` documents `DataProvenance` (+`sources`), `rsi_14_1d`, and updated `rsi_14`/`regime` descriptions.
- **v0.4.0 note (Issue #56)**: Koinly work is an on-chain DeFi transaction classifier reconciling with Koinly's direct Base wallet sync — no manual CSV download buttons/tooling in this or future PRs (duplicate transactions + budget hazard). Documented in CHANGELOG deferred.

## Testing
- Unit tests (43/43 pass): GeckoTerminal/DEXScreener parsing against live-verified fixtures, `plan_backfill` decision matrix, daily-close bucketing, RSI provenance/gap invariance, and the core regression (micro-overbought + macro-normal → no cooldown; macro-overbought → cooldown fires).
- DB-gated integration tests verified against live Postgres: idempotent re-upsert (1 row), backfill cannot overwrite a contemporaneous row.
- Pre-flight green: `make test-brain` (3/3), `make test-worker` (43/43), `make test-frontend` (25/25), `npm run build` (tsc + vite), `SQLX_OFFLINE=true cargo build --release` with zero crate warnings.
- Live E2E (sandbox Postgres, real providers + real Alchemy config): boot on empty table backfilled **29 daily + 161 hourly candles** per boot; cycle recorded the contemporaneous spot; `rsi_14_1d` computed from 30 real daily closes (38.3–41.1 across runs); `/v1/snapshot` returned `data_provenance` with `backfilled_points_count: 190` and `sources: {geckoterminal: 190}`.
- Visual verification of the dashboard banner against the live worker + vite stack:

## Screenshots
![Dashboard: regime badge, RSI readouts, provenance indicator](https://github.com/urmanac/nocknock/raw/helix-specs/design/tasks/000663_feature-spec-historical/screenshots/01-dashboard-regime-rsi-provenance.png)

- Note: `make validate-specs` already fails on `main` with 3 pre-existing errors/13 warnings; this branch adds zero new lint problems (identical counts before/after).
