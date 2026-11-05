-- Transient CSRF/state record bridging the two legs of an external login. The
-- start endpoint mints a random `state` (only its SHA-256 hash is stored) and
-- freezes the originating OAuth client request; the provider callback looks the
-- row up by the echoed state and replays the frozen params to mint our own
-- authorization code. Single-use and short-lived (deleted on consume).
CREATE TABLE IF NOT EXISTS external_login_flows (
  id UUID PRIMARY KEY,
  state_hash VARCHAR(64) NOT NULL,                 -- SHA-256 hex of the opaque provider-leg state
  external_provider_id UUID NOT NULL,
  oauth_client_id UUID NOT NULL,
  tenant_id UUID NOT NULL,
  provider VARCHAR(32) NOT NULL,
  -- The frozen, server-trusted client authorization request (echoed back at the end).
  client_redirect_uri TEXT NOT NULL,
  client_response_type VARCHAR(32) NOT NULL,
  client_scope TEXT NOT NULL,
  client_state TEXT NOT NULL,
  client_nonce TEXT NULL,
  client_code_challenge TEXT NOT NULL,
  client_code_challenge_method VARCHAR(16) NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_elf_provider FOREIGN KEY (external_provider_id) REFERENCES external_identity_providers (id) ON DELETE CASCADE,
  CONSTRAINT fk_elf_oauth_client FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_elf_state_hash ON external_login_flows (state_hash);
