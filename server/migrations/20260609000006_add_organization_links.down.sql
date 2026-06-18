DROP INDEX IF EXISTS uq_subscriptions_organization_id;
DROP INDEX IF EXISTS idx_admin_users_organization_id;
ALTER TABLE "subscriptions" DROP CONSTRAINT IF EXISTS fk_subscription_organization;
ALTER TABLE "admin_users" DROP CONSTRAINT IF EXISTS fk_admin_user_organization;
ALTER TABLE "subscriptions" DROP COLUMN IF EXISTS organization_id;
ALTER TABLE "admin_users" DROP COLUMN IF EXISTS organization_id;
