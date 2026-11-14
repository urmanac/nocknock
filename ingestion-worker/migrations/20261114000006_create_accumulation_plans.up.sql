-- Plan registry: when the worker first saw each accumulation plan id (issue #79).
-- A buy can only count toward a plan if it was confirmed at or after the plan's
-- anchor (config `starts_at` when set, otherwise this first-seen time), so an old
-- transaction hash can never advance or complete a new plan.
CREATE TABLE IF NOT EXISTS accumulation_plans (
    plan_id TEXT PRIMARY KEY,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
