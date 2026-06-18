-- Groups: tenant-scoped collections that users can be joined to.
CREATE TABLE IF NOT EXISTS "groups" (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  name VARCHAR(128) NOT NULL,
  description TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,

  /* group >- tenant = fk_group_tenant */
  CONSTRAINT fk_group_tenant FOREIGN KEY (tenant_id) REFERENCES "tenants" (id) ON DELETE CASCADE
);

CREATE INDEX idx_groups_tenant_id_created_at ON groups (tenant_id, created_at DESC);
-- Group names are unique within a tenant (but may repeat across tenants).
CREATE UNIQUE INDEX uq_groups_tenant_name ON groups (tenant_id, name);
