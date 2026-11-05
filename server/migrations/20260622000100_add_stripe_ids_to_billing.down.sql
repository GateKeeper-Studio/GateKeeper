ALTER TABLE "subscription_plans" DROP COLUMN IF EXISTS included_mau;
ALTER TABLE "subscription_plans" DROP COLUMN IF EXISTS stripe_meter_event_name;
ALTER TABLE "subscription_plans" DROP COLUMN IF EXISTS stripe_overage_price_id;
ALTER TABLE "subscription_plans" DROP COLUMN IF EXISTS stripe_price_id;

DROP INDEX IF EXISTS uq_subscriptions_stripe_subscription;
ALTER TABLE "subscriptions" DROP COLUMN IF EXISTS stripe_subscription_item_id;
ALTER TABLE "subscriptions" DROP COLUMN IF EXISTS stripe_subscription_id;

DROP INDEX IF EXISTS uq_organizations_stripe_customer;
ALTER TABLE "organizations" DROP COLUMN IF EXISTS stripe_customer_id;
