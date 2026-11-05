-- Organizations now own tenants directly. Preserve the relationship before
-- removing subscriptions and the rest of the billing schema.
ALTER TABLE "tenants" ADD COLUMN organization_id UUID NULL;

UPDATE tenants AS tenant
SET organization_id = subscription.organization_id
FROM subscriptions AS subscription
WHERE tenant.subscription_id = subscription.id;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM tenants WHERE organization_id IS NULL) THEN
        RAISE EXCEPTION 'cannot remove billing: one or more tenants have no organization';
    END IF;
END $$;

ALTER TABLE "tenants" ALTER COLUMN organization_id SET NOT NULL;
ALTER TABLE "tenants" ADD CONSTRAINT fk_tenant_organization
    FOREIGN KEY (organization_id) REFERENCES "organizations" (id) ON DELETE CASCADE;
CREATE INDEX idx_tenants_organization_id ON tenants (organization_id);

DROP TABLE IF EXISTS "usage_periods";

DROP INDEX IF EXISTS uq_tenant_default_per_subscription;
DROP INDEX IF EXISTS idx_tenants_subscription_id;
ALTER TABLE "tenants" DROP CONSTRAINT IF EXISTS fk_tenant_subscription;
ALTER TABLE "tenants" DROP COLUMN subscription_id;
ALTER TABLE "tenants" DROP COLUMN is_default;

DROP TABLE IF EXISTS "subscriptions";
DROP TABLE IF EXISTS "subscription_plans";

DROP INDEX IF EXISTS uq_organizations_stripe_customer;
ALTER TABLE "organizations" DROP COLUMN IF EXISTS stripe_customer_id;
