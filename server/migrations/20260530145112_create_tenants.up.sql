-- Add up migration script here
CREATE TABLE IF NOT EXISTS "tenants" (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description VARCHAR(255) NULL,
    can_self_register BOOLEAN NOT NULL DEFAULT FALSE,
    can_self_forgot_password BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    password_hash_secret VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NULL
);

-- unique one so is_tenant_with_name_exists is race-safe.
CREATE UNIQUE INDEX uq_tenants_name ON tenants (name);
