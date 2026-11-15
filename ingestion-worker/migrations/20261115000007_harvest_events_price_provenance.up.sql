-- v0.5.3 (#111): harvests are valued at the claim's block, not at recording time.
-- Each row keeps the unit price used and where it came from. Both are null when no
-- price could be found, and so is usd_value: an unpriced row is never given a substitute.
ALTER TABLE harvest_events ADD COLUMN IF NOT EXISTS price_usd TEXT;
ALTER TABLE harvest_events ADD COLUMN IF NOT EXISTS price_source TEXT;
