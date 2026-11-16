-- Route quotes for a due tranche (issue #115). Each row is what the venues' own quoter
-- contracts answered, at one block, for the exact tranche amount the plan said was due:
-- read-only calls, never an estimate. A row is written only when at least one venue was
-- quoted. `quotes` keeps the whole comparison as it was shown; the best_* columns are
-- the venue that returned the most NOCK. Like the rest of this database the table is
-- disposable: after a loss, fills recorded later show "no quote on record".
CREATE TABLE IF NOT EXISTS accumulation_quotes (
    id BIGSERIAL PRIMARY KEY,
    plan_id TEXT NOT NULL,
    tranche_index INTEGER NOT NULL,
    -- The plan's tranche amount that was quoted, in USDC.
    amount_usdc TEXT NOT NULL,
    -- Block every quote in the row was read at, and that block's time.
    block_number BIGINT NOT NULL,
    quoted_at TIMESTAMPTZ NOT NULL,
    best_venue TEXT NOT NULL,
    best_route TEXT NOT NULL,
    best_nock_out TEXT NOT NULL,
    best_effective_price_usd TEXT NOT NULL,
    quotes JSONB NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (plan_id, block_number)
);

-- A fill's cost over the best quote that was current for its tranche: the latest stored
-- quote read before the fill's block and not before the plan's previous fill. All eight
-- are null together when there is no such quote ("no quote on record"); a later quote
-- is never used. The gap between quote_block_number and the fill's block includes price
-- movement, so quote_cost_bps is not execution cost alone. Existing rows stay null.
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quote_venue TEXT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quote_route TEXT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quote_usdc TEXT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quote_nock_out TEXT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quote_effective_price_usd TEXT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quote_block_number BIGINT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quoted_at TIMESTAMPTZ;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS quote_cost_bps INTEGER;
