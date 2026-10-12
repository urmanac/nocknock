-- Harvest event ledger: receipt-verified fee claims (v0.2.0 Golden Path).
-- One row per (transaction, token): a claim typically emits two ERC-20 Transfer
-- logs (USDC + NOCK) from the pool/PoolManager to the sovereign wallet. Rows are
-- only ever written after the worker verifies the on-chain receipt; the split
-- into the virtual 60/40 sub-ledger and the Koinly export both derive from here.
CREATE TABLE IF NOT EXISTS harvest_events (
    id BIGSERIAL PRIMARY KEY,
    platform TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    token_address TEXT NOT NULL,
    token_symbol TEXT NOT NULL,
    from_address TEXT NOT NULL,
    recipient TEXT NOT NULL,
    amount_raw TEXT NOT NULL,
    amount_human TEXT NOT NULL,
    decimals INTEGER NOT NULL,
    usd_value TEXT,
    block_number BIGINT NOT NULL,
    confirmed_at TIMESTAMPTZ NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tx_hash, token_address)
);
CREATE INDEX IF NOT EXISTS idx_harvest_events_recorded_at ON harvest_events (recorded_at);
