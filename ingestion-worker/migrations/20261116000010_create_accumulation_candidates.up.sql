-- Fill detection (issue #118): swaps NockNock noticed on-chain that look like plan buys.
-- A candidate is a hint that has been checked against its receipt once, with the same
-- rules the record endpoint applies. It is never plan progress: only rows in
-- accumulation_events count, and those are written only by
-- POST /v1/execute/record-accumulation, which verifies the receipt again.
--   offered         the receipt shows USDC leaving and NOCK entering the wallet
--   dismissed       the operator marked it "not part of the plan" (kept, restorable)
--   not_a_purchase  the receipt is not a purchase (a fee claim, a deposit, a sale)
-- A receipt that could not be read is not stored at all, so it is tried again.
-- Like the rest of this database the table is disposable: after a loss the next cycle
-- reads the wallet's transfer history again and rebuilds it.
CREATE TABLE IF NOT EXISTS accumulation_candidates (
    wallet TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('offered', 'dismissed', 'not_a_purchase')),
    -- Rejection code for not_a_purchase rows (TX_REVERTED, NOT_A_NOCK_PURCHASE).
    reason TEXT,
    block_number BIGINT,
    confirmed_at TIMESTAMPTZ,
    usdc_spent_raw TEXT,
    nock_received_raw TEXT,
    usdc_spent TEXT,
    nock_received TEXT,
    effective_price_usd TEXT,
    -- Reference spot at the swap's block time and the cost over it; all null when no
    -- stored price was near enough (never a substitute).
    reference_spot_usd TEXT,
    reference_spot_source TEXT,
    execution_cost_bps INTEGER,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet, tx_hash)
);
CREATE INDEX IF NOT EXISTS idx_accumulation_candidates_state ON accumulation_candidates (wallet, state, confirmed_at);

-- When the wallet's transfer history was last read successfully for this plan.
ALTER TABLE accumulation_plans ADD COLUMN IF NOT EXISTS fills_checked_at TIMESTAMPTZ;
