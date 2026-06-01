-- Add up migration script here
CREATE TABLE IF NOT EXISTS "users" (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  email VARCHAR(128) NOT NULL,
  created_at TIMESTAMP NOT NULL,
  updated_at TIMESTAMP NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  is_email_verified BOOLEAN NOT NULL DEFAULT FALSE,
  preferred_2fa_method VARCHAR(24) NULL,
  
  /* tenant_user >- tenant = fk_user_tenant */
  CONSTRAINT fk_user_tenant FOREIGN KEY (tenant_id) REFERENCES "tenants" (id) ON DELETE CASCADE
);

CREATE INDEX idx_users_tenant_id_created_at ON users (tenant_id, created_at DESC);