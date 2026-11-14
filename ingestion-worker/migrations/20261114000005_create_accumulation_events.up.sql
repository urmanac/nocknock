-- Accumulation ledger: receipt-verified NOCK purchases belonging to a dollar-cost-
-- averaging plan (issue #79). One row per swap transaction. Rows are only written
-- after the worker verifies on-chain that the transaction moved USDC out of, and
-- NOCK into, the sovereign wallet; amounts come from the Transfer logs, never from
-- the request. Plan progress (tranche index, spend, average cost) derives from here.
CREATE TABLE IF NOT EXISTS accumulation_events (
    id BIGSERIAL PRIMARY KEY,
    plan_id TEXT NOT NULL,
    tx_hash TEXT NOT NULL UNIQUE,
    venue TEXT NOT NULL,
    usdc_spent_raw TEXT NOT NULL,
    nock_received_raw TEXT NOT NULL,
    usdc_spent TEXT NOT NULL,
    nock_received TEXT NOT NULL,
    effective_price_usd TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    confirmed_at TIMESTAMPTZ NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_accumulation_events_plan ON accumulation_events (plan_id, confirmed_at);
