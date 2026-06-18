-- Add up migration script here
CREATE TABLE IF NOT EXISTS oauth_clients (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  client_id UUID NOT NULL,                       -- public identifier (UUID v7)
  client_secret_hash VARCHAR(255) NULL,          -- NULL for public clients
  client_type VARCHAR(16) NOT NULL,              -- 'public' | 'confidential'
  name VARCHAR(255) NOT NULL,
  redirect_uris TEXT[] NOT NULL,                 -- exact-match list
  post_logout_redirect_uris TEXT[] NOT NULL DEFAULT '{}',
  allowed_scopes TEXT[] NOT NULL,
  allowed_grant_types TEXT[] NOT NULL,
  require_pkce BOOLEAN NOT NULL DEFAULT TRUE,
  require_consent BOOLEAN NOT NULL DEFAULT TRUE,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_oauth_clients_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);
-- client_id is the public lookup key and must be globally unique.
CREATE UNIQUE INDEX uq_oauth_clients_client_id ON oauth_clients (client_id);