CREATE TABLE IF NOT EXISTS "roles" (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  name VARCHAR(128) NOT NULL,
  description TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_role_tenant FOREIGN KEY (tenant_id) REFERENCES "tenants" (id) ON DELETE CASCADE
);
CREATE INDEX idx_roles_tenant_id_created_at ON roles (tenant_id, created_at DESC);
CREATE UNIQUE INDEX uq_roles_tenant_name ON roles (tenant_id, name);
