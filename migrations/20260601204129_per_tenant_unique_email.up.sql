-- An email must be unique within a tenant (the same address may exist in
-- different tenants). This also serves as the lookup index for email queries.
CREATE UNIQUE INDEX uq_users_tenant_email ON users (tenant_id, email);
