-- Revert the unique index to the case-sensitive form. The duplicate-account merge
-- and email lowercasing are data changes and are NOT reversible.
DROP INDEX IF EXISTS uq_users_tenant_lower_email;
CREATE UNIQUE INDEX uq_users_tenant_email ON users (tenant_id, email);
