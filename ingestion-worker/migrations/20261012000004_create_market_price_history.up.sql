-- Market price history (v0.3.0): one row per observed/backfilled NOCK-USD price point
-- on the tracked Aerodrome pool. Contemporaneous rows come from live Aerodrome pool
-- reserves observed via RPC (source='aerodrome_rpc', is_backfilled=false); gap fills
-- come from free third-party OHLCV feeds (source='geckoterminal'/'dexscreener',
-- is_backfilled=true). Provenance columns are mandatory so third-party retro data
-- can never masquerade as contemporaneous on-chain observation ("mock data is a bug").
CREATE TABLE IF NOT EXISTS market_price_history (
    timestamp      TIMESTAMPTZ NOT NULL PRIMARY KEY,
    price_usd      FLOAT8 NOT NULL,
    open_usd       FLOAT8,
    high_usd       FLOAT8,
    low_usd        FLOAT8,
    close_usd      FLOAT8,
    volume_usd     FLOAT8,
    source         VARCHAR NOT NULL,
    is_backfilled  BOOLEAN NOT NULL DEFAULT false,
    backfilled_at  TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_market_price_history_ts_desc
    ON market_price_history (timestamp DESC);
