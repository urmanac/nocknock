CREATE TABLE IF NOT EXISTS snapshots (
    snapshot_id   VARCHAR(26) PRIMARY KEY,
    generated_at  TIMESTAMPTZ NOT NULL,
    state_hash    CHAR(64) NOT NULL,
    version       VARCHAR(20) NOT NULL,
    data          JSONB NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_snapshots_generated_at ON snapshots(generated_at DESC);