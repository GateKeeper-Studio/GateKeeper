-- Add up migration script here
CREATE TABLE IF NOT EXISTS refresh_tokens (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  token_hash VARCHAR(64) NOT NULL,
  client_id UUID NOT NULL,
  user_id UUID NOT NULL,
  scopes TEXT[] NOT NULL,
  family_id UUID NOT NULL,                        -- rotation lineage
  rotated_at TIMESTAMPTZ NULL,
  revoked_at TIMESTAMPTZ NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_refresh_client FOREIGN KEY (client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_refresh_user   FOREIGN KEY (user_id)   REFERENCES users (id)        ON DELETE CASCADE
);
CREATE UNIQUE INDEX uq_refresh_tokens_token_hash ON refresh_tokens (token_hash);
CREATE INDEX idx_refresh_tokens_family ON refresh_tokens (family_id);