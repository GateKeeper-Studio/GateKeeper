-- Add up migration script here
CREATE TABLE IF NOT EXISTS consent_grants (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  client_id UUID NOT NULL,
  scopes TEXT[] NOT NULL,
  revoked_at TIMESTAMPTZ NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_consent_user   FOREIGN KEY (user_id)   REFERENCES users (id)         ON DELETE CASCADE,
  CONSTRAINT fk_consent_client FOREIGN KEY (client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE
);
-- Exactly one grant row per (user, client); the upsert relies on this.
CREATE UNIQUE INDEX uq_consent_grants_user_client ON consent_grants (user_id, client_id);