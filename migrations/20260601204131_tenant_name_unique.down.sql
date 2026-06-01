DROP INDEX IF EXISTS uq_tenants_name;
CREATE INDEX idx_tenants_name ON tenants (name);
