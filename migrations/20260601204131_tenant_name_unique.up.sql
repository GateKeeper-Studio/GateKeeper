-- Tenant names must be unique. Replace the non-unique lookup index with a
-- unique one so is_tenant_with_name_exists is race-safe.
DROP INDEX IF EXISTS idx_tenants_name;
CREATE UNIQUE INDEX uq_tenants_name ON tenants (name);
