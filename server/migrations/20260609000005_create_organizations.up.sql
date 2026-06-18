-- Organization is the new top-level owner: it has many admin_users and may own
-- one subscription. Tenants (and their children) hang off the subscription, so
-- they remain bound by that subscription's plan restrictions.
CREATE TABLE IF NOT EXISTS "organizations" (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description VARCHAR(255) NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX uq_organizations_name ON organizations (name);
