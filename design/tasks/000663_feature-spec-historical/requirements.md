# Requirements: Historical Price Backfill & Multi-Timeframe Regime Engine

## Background

NockNock polls on-chain state every 5 minutes from a local tmux session. When the operator's machine sleeps (laptop lid closed) or the engine restarts, multi-hour gaps appear in the PostgreSQL history. Today the market regime indicator builds Wilder RSI(14) from the **last 50 snapshots** (`ingestion-worker/src/main.rs` L1228-1235) — at one snapshot per 5 minutes that is only ~4.2 hours of data. The result is an ultra-short **70-minute micro-momentum pulse masquerading as RSI(14)**: a normal 4-hour intraday climb pushes the micro RSI above 70 and falsely fires the `SWEEP TO STABLE (Overextension Cooldown)` directive, suppressing deployment while the true multi-day trend is healthy.

This feature (v0.3.0) adds an idempotent **historical price backfill engine** fed by free, unauthenticated third-party market data (GeckoTerminal, fallback DEXScreener), a `market_price_history` table with strict provenance, and splits the regime engine into **macro** (multi-day, drives the Overextension directive) and **micro** (5-minute, informational) RSI series.

## Goals

- Heal data gaps automatically on worker boot and during every ingestion cycle, without operator intervention.
- Compute the Overextension Cooldown from a true 14-day daily-close RSI instead of the 70-minute pulse.
- Never fabricate on-chain state: third-party backfilled prices are always labeled with source and `is_backfilled`.

## Non-Goals

- No frontend changes (the cockpit does not consume regime fields today).
- No changes to the Brain WASM (pure math stays pure; backfill is host-runtime I/O).
- No paid API keys, no authenticated providers.
- No backdating or synthetic interpolation of missing candles.

## User Stories

1. **As an operator**, when my laptop wakes from sleep or the worker restarts, the engine detects the price-history gap and backfills real candles from GeckoTerminal automatically — macro indicators are intact when the next snapshot lands.
2. **As an operator**, the Overextension Cooldown fires only when the multi-day RSI is genuinely overbought; a short 4-hour bounce no longer suppresses deployment while the 14-day trend is normal.
3. **As an operator (API/MCP consumer)**, I can see whether price points were live on-chain observations or third-party backfills (`data_provenance` on `/v1/snapshot`), so I never mistake retroactive market data for contemporaneous RPC truth.
4. **As a developer**, backfill is idempotent (reruns upsert, never duplicate or overwrite) and fail-open (provider outages log warnings and skip; the ingestion loop never blocks or crashes).

## Acceptance Criteria

- **AC1** — A sqlx migration creates `market_price_history` with `timestamp TIMESTAMPTZ PRIMARY KEY`, `price_usd FLOAT8 NOT NULL`, nullable OHLCV columns (`open/high/low/close/volume_usd`), and provenance columns `source VARCHAR NOT NULL`, `is_backfilled BOOLEAN NOT NULL DEFAULT false`, `backfilled_at TIMESTAMPTZ`.
- **AC2** — Every ingestion cycle with a valid live spot price inserts a contemporaneous row (`source='aerodrome_rpc'`, `is_backfilled=false`) — including cycles whose full snapshot is deduplicated by `state_hash`.
- **AC3** — On boot and at the start of each cycle, if the newest stored point is older than **15 minutes** ago, the worker fetches hourly OHLCV (`/ohlcv/hour?limit=168`) from GeckoTerminal and upserts the missing points with `is_backfilled=true`, `source='geckoterminal'`, `backfilled_at=now()`. Rerunning produces no duplicates and no overwrites.
- **AC4** — If stored history spans fewer than **14 days**, the worker fetches daily OHLCV (`/ohlcv/day?limit=30`) to extend depth. On an empty table it performs both (daily + hourly) so macro RSI is meaningful on the very first cycle after boot.
- **AC5** — Provenance is strict: only real provider data is stored; the DEXScreener fallback (which exposes only the current `priceUsd`) may seed at most a single point stamped with its **observation time** — never backdated to a past candle boundary.
- **AC6** — Fail-open: GeckoTerminal/DEXScreener timeouts, HTTP 429, and malformed responses log warnings and abort the backfill attempt only; the ingestion cycle and local RPC polling proceed normally.
- **AC7** — Macro RSI(14) is Wilder RSI over one close per UTC day spanning ≥ 14 days, drawn from `market_price_history` (daily candles + last contemporaneous point per day), with the live spot price appended as today's close. Fewer than 15 daily closes → macro RSI is `None` and overextension stays `false` (same fail-open behavior as today's insufficient-data path).
- **AC8** — `MarketRegime`/`GlobalStatus` distinguish macro and micro RSI: the Overextension Cooldown directive uses the **macro** series; the existing short-term series remains exposed as `rsi_14` and a new `rsi_macro_14` field carries the multi-day value that `regime` is keyed on.
- **AC9** — `GET /v1/snapshot` (and `/v1/snapshot/latest`) responses include a top-level `data_provenance: { last_backfill_at, backfilled_points_count }`. It is computed live from the table, is not persisted in the snapshot JSONB, and does not affect `state_hash`/ETag.
- **AC10** — `cd ingestion-worker && SQLX_OFFLINE=true cargo build --release` passes; any new sqlx queries have committed entries in `.sqlx/`.
- **AC11** — `make test-brain`, `make test-worker`, `make test-frontend` pass; worker `Cargo.toml` bumped to 0.3.0 with a `CHANGELOG.md` entry.
- **AC12** — New/changed response fields are documented in `specs/core/snapshot.yaml` (and `api.yaml` where applicable); `make validate-specs` passes.

## Invariants (from AGENTS.md / OPERATOR_GUIDANCE.md)

- **Mock Data is a Bug**: no synthetic numbers, no silent mock fallbacks. Contemporaneous on-chain RPC observations (Alchemy reserves → spot price) remain a distinct data class from retroactively backfilled third-party pricing, and every row carries explicit provenance.
- **Brain purity**: all new logic lives in the ingestion worker host runtime.

## Open Questions

1. **RSI field naming** (AC8): I propose keeping `rsi_14` as the existing short-term (micro) series for backward compatibility and adding `rsi_macro_14` for the multi-day series that drives `regime`/Overextension. The alternative is repurposing `rsi_14` as macro and adding `rsi_micro_14`. Confirm the preferred contract.
2. **Price-history retention**: snapshots are cleaned at 90 days. Should `market_price_history` be kept indefinitely (it is tiny and is the basis for macro indicators), or share the 90-day retention policy?
3. **DEXScreener fallback scope**: it can only seed one current-price point (no OHLCV). Keep it as a single-point fallback when GeckoTerminal is down (as specified), or drop it and rely solely on fail-open + retry on the next cycle?
4. **Public candle endpoint**: is a `GET /v1/market/prices` read API wanted in v0.3.0 (e.g., for future charts), or should candle history stay DB-internal this release?
