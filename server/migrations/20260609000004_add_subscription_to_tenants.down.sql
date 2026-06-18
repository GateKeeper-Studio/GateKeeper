DROP INDEX IF EXISTS uq_tenant_default_per_subscription;
DROP INDEX IF EXISTS idx_tenants_subscription_id;
ALTER TABLE "tenants" DROP CONSTRAINT IF EXISTS fk_tenant_subscription;
ALTER TABLE "tenants" DROP COLUMN IF EXISTS is_default;
ALTER TABLE "tenants" DROP COLUMN IF EXISTS subscription_id;
