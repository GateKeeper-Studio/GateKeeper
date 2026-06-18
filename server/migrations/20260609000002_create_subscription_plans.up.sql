CREATE TABLE IF NOT EXISTS "subscription_plans" (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description VARCHAR(255) NULL,
    price_cents BIGINT NOT NULL DEFAULT 0,
    billing_interval VARCHAR(32) NOT NULL,      -- 'monthly' | 'yearly'
    max_tenants INTEGER NOT NULL,
    max_users INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL
);

-- Unique so is_plan_with_name_exists is race-safe.
CREATE UNIQUE INDEX uq_subscription_plans_name ON subscription_plans (name);
