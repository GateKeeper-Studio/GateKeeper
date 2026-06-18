-- Every tenant now belongs to exactly one subscription, and each subscription
-- has exactly one default ("master") tenant.
ALTER TABLE "tenants" ADD COLUMN subscription_id UUID NULL;
ALTER TABLE "tenants" ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT FALSE;

-- Backfill: tenants created before subscriptions existed have no owner. Attach
-- them to a bootstrap "Legacy" subscription so the column can become NOT NULL.
-- No-op on a fresh database (no orphan tenants).
DO $$
DECLARE
    v_plan_id UUID;
    v_sub_id UUID;
    v_default_tenant UUID;
BEGIN
    IF EXISTS (SELECT 1 FROM tenants WHERE subscription_id IS NULL) THEN
        v_plan_id := gen_random_uuid();
        INSERT INTO subscription_plans
            (id, name, description, price_cents, billing_interval,
             max_tenants, max_users, is_active, created_at)
        VALUES
            (v_plan_id, 'Legacy', 'Auto-created for tenants that predate subscriptions',
             0, 'monthly', 2147483647, 2147483647, TRUE, now());

        v_sub_id := gen_random_uuid();
        INSERT INTO subscriptions (id, plan_id, is_active, created_at)
        VALUES (v_sub_id, v_plan_id, TRUE, now());

        UPDATE tenants SET subscription_id = v_sub_id WHERE subscription_id IS NULL;

        -- Exactly one default tenant for the bootstrap subscription (earliest created).
        SELECT id INTO v_default_tenant
        FROM tenants WHERE subscription_id = v_sub_id
        ORDER BY created_at ASC LIMIT 1;
        UPDATE tenants SET is_default = TRUE WHERE id = v_default_tenant;
    END IF;
END $$;

ALTER TABLE "tenants" ALTER COLUMN subscription_id SET NOT NULL;
ALTER TABLE "tenants" ADD CONSTRAINT fk_tenant_subscription
    FOREIGN KEY (subscription_id) REFERENCES "subscriptions" (id) ON DELETE CASCADE;

CREATE INDEX idx_tenants_subscription_id ON tenants (subscription_id);

-- At most one default tenant per subscription (partial unique index).
CREATE UNIQUE INDEX uq_tenant_default_per_subscription
    ON tenants (subscription_id) WHERE is_default;
