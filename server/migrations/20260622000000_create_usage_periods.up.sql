-- Per-subscription, per-period usage rollup. The source of truth for usage is the
-- append-only `audit_events` log; this table is an idempotent cache the usage
-- reporter recomputes (so dashboards don't COUNT(DISTINCT ...) the whole audit log
-- on every read) and the anchor for reporting metered usage to Stripe.
--
-- `metric` is an open string ('mau' today; room for 'm2m_tokens', etc.). MAU =
-- distinct end-users who authenticated in [period_start, period_end), aggregated
-- across all of the subscription's tenants — an org-aggregate ACTIVITY metric,
-- distinct from the structural per-tenant `max_users` quota on the plan.
CREATE TABLE IF NOT EXISTS "usage_periods" (
    id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    metric TEXT NOT NULL,
    value BIGINT NOT NULL,
    -- Set only once Stripe confirms the meter event landed; left NULL means the
    -- next reporter run retries. Never stamp before Stripe acknowledges.
    last_reported_to_stripe_at TIMESTAMPTZ NULL,
    stripe_meter_event_id TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL,
    CONSTRAINT fk_usage_period_subscription FOREIGN KEY (subscription_id)
        REFERENCES "subscriptions" (id) ON DELETE CASCADE
);

-- One rollup row per (subscription, period, metric) — the upsert conflict target.
CREATE UNIQUE INDEX uq_usage_periods_sub_period_metric
    ON usage_periods (subscription_id, period_start, metric);

-- Lets the reporter cheaply find rows still pending a Stripe push.
CREATE INDEX idx_usage_periods_unreported
    ON usage_periods (subscription_id) WHERE last_reported_to_stripe_at IS NULL;
