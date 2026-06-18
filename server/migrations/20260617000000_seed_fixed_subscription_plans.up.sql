-- Fixed subscription catalog. These plans are application constants stored in
-- the database so subscriptions can bind to stable plan IDs.
INSERT INTO subscription_plans
    (id, name, description, price_cents, billing_interval, max_tenants, max_users, is_active, created_at, updated_at)
VALUES
    ('00000000-0000-0000-0000-000000000101', 'Starter',
     'Starter plan for small organizations', 0, 'monthly', 1, 10, TRUE, now(), NULL),
    ('00000000-0000-0000-0000-000000000102', 'Professional',
     'Professional plan for growing organizations', 2900, 'monthly', 5, 100, TRUE, now(), NULL),
    ('00000000-0000-0000-0000-000000000103', 'Enterprise',
     'Enterprise plan for large organizations', 9900, 'monthly', 50, 1000, TRUE, now(), NULL)
ON CONFLICT (id) DO UPDATE
SET name = EXCLUDED.name,
    description = EXCLUDED.description,
    price_cents = EXCLUDED.price_cents,
    billing_interval = EXCLUDED.billing_interval,
    max_tenants = EXCLUDED.max_tenants,
    max_users = EXCLUDED.max_users,
    is_active = EXCLUDED.is_active,
    updated_at = now();
