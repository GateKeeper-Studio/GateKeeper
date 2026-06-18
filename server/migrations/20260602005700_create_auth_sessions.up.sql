-- Add up migration script here
-- Backs GET /authorize → login/consent: a server-side parking spot for the
-- validated authorization request plus the browser's opaque session cookie.
CREATE TABLE IF NOT EXISTS auth_sessions (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  session_token_hash VARCHAR(64) NOT NULL,         -- hex SHA-256 of the cookie value
  user_id UUID NULL,                               -- NULL until the user authenticates
  authenticated_at TIMESTAMPTZ NULL,
  pending_client_id UUID NOT NULL,                 -- frozen /authorize request below
  pending_redirect_uri TEXT NOT NULL,
  pending_scopes TEXT[] NOT NULL,
  pending_state VARCHAR(255) NOT NULL,
  pending_code_challenge VARCHAR(128) NOT NULL,
  pending_code_challenge_method VARCHAR(8) NOT NULL,
  pending_nonce VARCHAR(255) NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_auth_sessions_tenant FOREIGN KEY (tenant_id)         REFERENCES tenants (id)       ON DELETE CASCADE,
  CONSTRAINT fk_auth_sessions_client FOREIGN KEY (pending_client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_auth_sessions_user   FOREIGN KEY (user_id)           REFERENCES users (id)         ON DELETE CASCADE
);
-- The cookie hash is the lookup key for an active session.
CREATE UNIQUE INDEX uq_auth_sessions_token_hash ON auth_sessions (session_token_hash);
-- a background job deletes rows past expires_at
CREATE INDEX idx_auth_sessions_expires_at ON auth_sessions (expires_at);
