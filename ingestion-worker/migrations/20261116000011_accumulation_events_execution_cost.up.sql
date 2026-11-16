-- Execution cost per fill (issue #118): what the buy paid over the reference NOCK/USD
-- spot at its block time. The reference is the stored price nearest the fill (the same
-- lookup that values harvests, v0.5.3), kept with its provenance. All three are null
-- when no stored price was near enough: an unknown reference is never given a
-- substitute, and the fill then shows no cost figure.
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS reference_spot_usd TEXT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS reference_spot_source TEXT;
ALTER TABLE accumulation_events ADD COLUMN IF NOT EXISTS execution_cost_bps INTEGER;
