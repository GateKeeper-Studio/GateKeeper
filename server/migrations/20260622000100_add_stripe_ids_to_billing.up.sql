-- Stripe linkage for the billing entities. All nullable (populated at checkout /
-- when an admin links a plan to Stripe), so existing rows need no backfill.

-- The organization is the Stripe *customer* (the payer).
ALTER TABLE "organizations" ADD COLUMN stripe_customer_id TEXT NULL;
CREATE UNIQUE INDEX uq_organizations_stripe_customer
    ON organizations (stripe_customer_id) WHERE stripe_customer_id IS NOT NULL;

-- The subscription mirrors a Stripe Subscription; the item id is the metered line
-- (overage) we report usage against.
ALTER TABLE "subscriptions" ADD COLUMN stripe_subscription_id TEXT NULL;
ALTER TABLE "subscriptions" ADD COLUMN stripe_subscription_item_id TEXT NULL;
CREATE UNIQUE INDEX uq_subscriptions_stripe_subscription
    ON subscriptions (stripe_subscription_id) WHERE stripe_subscription_id IS NOT NULL;

-- The plan carries its Stripe catalog config: the flat base price, the metered
-- overage price, the meter's event_name (what meter events are sent under), and the
-- MAU allowance included before overage applies.
ALTER TABLE "subscription_plans" ADD COLUMN stripe_price_id TEXT NULL;
ALTER TABLE "subscription_plans" ADD COLUMN stripe_overage_price_id TEXT NULL;
ALTER TABLE "subscription_plans" ADD COLUMN stripe_meter_event_name TEXT NULL;
ALTER TABLE "subscription_plans" ADD COLUMN included_mau INTEGER NOT NULL DEFAULT 0;

-- Seed a sensible MAU allowance for existing plans: reuse their per-tenant user
-- cap as the included-MAU starting point (Starter 10, Pro 100, Enterprise 1000).
UPDATE subscription_plans SET included_mau = max_users WHERE included_mau = 0;
