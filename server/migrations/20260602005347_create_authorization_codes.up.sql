-- Add up migration script here
CREATE TABLE IF NOT EXISTS authorization_codes (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  code_hash VARCHAR(64) NOT NULL,                -- hex SHA-256
  client_id UUID NOT NULL,
  user_id UUID NOT NULL,
  redirect_uri TEXT NOT NULL,
  scopes TEXT[] NOT NULL,
  code_challenge VARCHAR(128) NOT NULL,
  code_challenge_method VARCHAR(8) NOT NULL,     -- only 'S256'
  nonce VARCHAR(255) NULL,
  consumed_at TIMESTAMPTZ NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_authcode_client FOREIGN KEY (client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_authcode_user   FOREIGN KEY (user_id)   REFERENCES users (id)        ON DELETE CASCADE
);
CREATE UNIQUE INDEX uq_authorization_codes_code_hash ON authorization_codes (code_hash);
-- a background job deletes rows past expires_at
CREATE INDEX idx_authorization_codes_expires_at ON authorization_codes (expires_at);