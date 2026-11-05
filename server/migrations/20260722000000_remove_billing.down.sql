-- Recreate the previous schema with neutral legacy records. Historical payment
-- processor identifiers and usage rollups cannot be reconstructed.
ALTER TABLE "organizations" ADD COLUMN stripe_customer_id TEXT NULL;
CREATE UNIQUE INDEX uq_organizations_stripe_customer
    ON organizations (stripe_customer_id) WHERE stripe_customer_id IS NOT NULL;

CREATE TABLE "subscription_plans" (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description VARCHAR(255) NULL,
    price_cents BIGINT NOT NULL DEFAULT 0,
    billing_interval VARCHAR(32) NOT NULL,
    max_tenants INTEGER NOT NULL,
    max_users INTEGER NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL,
    stripe_price_id TEXT NULL,
    stripe_overage_price_id TEXT NULL,
    stripe_meter_event_name TEXT NULL,
    included_mau INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX uq_subscription_plans_name ON subscription_plans (name);

CREATE TABLE "subscriptions" (
    id UUID PRIMARY KEY,
    plan_id UUID NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL,
    organization_id UUID NOT NULL,
    stripe_subscription_id TEXT NULL,
    stripe_subscription_item_id TEXT NULL,
    CONSTRAINT fk_subscription_plan FOREIGN KEY (plan_id)
        REFERENCES "subscription_plans" (id) ON DELETE RESTRICT,
    CONSTRAINT fk_subscription_organization FOREIGN KEY (organization_id)
        REFERENCES "organizations" (id) ON DELETE CASCADE
);
CREATE INDEX idx_subscriptions_plan_id ON subscriptions (plan_id);
CREATE UNIQUE INDEX uq_subscriptions_organization_id ON subscriptions (organization_id);
CREATE UNIQUE INDEX uq_subscriptions_stripe_subscription
    ON subscriptions (stripe_subscription_id) WHERE stripe_subscription_id IS NOT NULL;

DO $$
DECLARE
    v_plan_id UUID := gen_random_uuid();
BEGIN
    INSERT INTO subscription_plans
        (id, name, description, price_cents, billing_interval, max_tenants,
         max_users, is_active, created_at, included_mau)
    VALUES
        (v_plan_id, 'Legacy', 'Recreated while rolling back billing removal',
         0, 'monthly', 2147483647, 2147483647, TRUE, now(), 2147483647);

    INSERT INTO subscriptions (id, plan_id, is_active, created_at, organization_id)
    SELECT gen_random_uuid(), v_plan_id, TRUE, now(), id
    FROM organizations;
END $$;

ALTER TABLE "tenants" ADD COLUMN subscription_id UUID NULL;
ALTER TABLE "tenants" ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE tenants AS tenant
SET subscription_id = subscription.id
FROM subscriptions AS subscription
WHERE tenant.organization_id = subscription.organization_id;

WITH defaults AS (
    SELECT DISTINCT ON (subscription_id) id
    FROM tenants
    ORDER BY subscription_id, created_at, id
)
UPDATE tenants
SET is_default = TRUE
WHERE id IN (SELECT id FROM defaults);

ALTER TABLE "tenants" ALTER COLUMN subscription_id SET NOT NULL;
ALTER TABLE "tenants" ADD CONSTRAINT fk_tenant_subscription
    FOREIGN KEY (subscription_id) REFERENCES "subscriptions" (id) ON DELETE CASCADE;
CREATE INDEX idx_tenants_subscription_id ON tenants (subscription_id);
CREATE UNIQUE INDEX uq_tenant_default_per_subscription
    ON tenants (subscription_id) WHERE is_default;

CREATE TABLE "usage_periods" (
    id UUID PRIMARY KEY,
    subscription_id UUID NOT NULL,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    metric TEXT NOT NULL,
    value BIGINT NOT NULL,
    last_reported_to_stripe_at TIMESTAMPTZ NULL,
    stripe_meter_event_id TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL,
    CONSTRAINT fk_usage_period_subscription FOREIGN KEY (subscription_id)
        REFERENCES "subscriptions" (id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX uq_usage_periods_sub_period_metric
    ON usage_periods (subscription_id, period_start, metric);
CREATE INDEX idx_usage_periods_unreported
    ON usage_periods (subscription_id) WHERE last_reported_to_stripe_at IS NULL;

DROP INDEX IF EXISTS idx_tenants_organization_id;
ALTER TABLE "tenants" DROP CONSTRAINT IF EXISTS fk_tenant_organization;
ALTER TABLE "tenants" DROP COLUMN organization_id;
