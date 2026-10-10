-- NockNock Database Initialization
-- Run as postgres superuser

-- Enable TimescaleDB extension
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Create snapshots table (regular table, not hypertable — snapshot_id is PK)
CREATE TABLE IF NOT EXISTS snapshots (
    snapshot_id   UUID PRIMARY KEY,
    generated_at  TIMESTAMPTZ NOT NULL,
    state_hash    CHAR(64) NOT NULL,
    version       VARCHAR(20) NOT NULL,
    data          JSONB NOT NULL,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);

-- Index for time-range queries
CREATE INDEX IF NOT EXISTS idx_snapshots_generated_at ON snapshots(generated_at DESC);

-- Index for hash lookups (conditional GET)
CREATE INDEX IF NOT EXISTS idx_snapshots_state_hash ON snapshots(state_hash);

-- Hypertable for historical metrics (optional, for high-cardinality time-series)
-- Uncomment if you want to store derived metrics as separate time-series rows
/*
CREATE TABLE IF NOT EXISTS metrics_history (
    time        TIMESTAMPTZ NOT NULL,
    metric_name TEXT NOT NULL,
    value       NUMERIC NOT NULL,
    labels      JSONB DEFAULT '{}'
);
SELECT create_hypertable('metrics_history', 'time', if_not_exists => TRUE);
CREATE INDEX IF NOT EXISTS idx_metrics_history_name_time ON metrics_history(metric_name, time DESC);
*/

-- Function to clean old snapshots (retention policy)
-- Keep last 90 days of snapshots (adjust as needed)
CREATE OR REPLACE FUNCTION cleanup_old_snapshots()
RETURNS void AS $$
BEGIN
    DELETE FROM snapshots
    WHERE generated_at < NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql;

-- Schedule cleanup daily (requires pg_cron extension, optional)
-- SELECT cron.schedule('cleanup-snapshots', '0 3 * * *', 'SELECT cleanup_old_snapshots();');