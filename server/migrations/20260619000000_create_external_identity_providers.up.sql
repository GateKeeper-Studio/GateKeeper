-- Per-OAuth-client external (social) identity providers (e.g. "Sign in with
-- Google/GitHub"). Each row holds the credentials an admin registered for one
-- provider at one client. The provider client secret is stored AES-256-GCM
-- encrypted (SecretCipher), never in clear, and is never projected over the API.
CREATE TABLE IF NOT EXISTS external_identity_providers (
  id UUID PRIMARY KEY,
  oauth_client_id UUID NOT NULL,
  tenant_id UUID NOT NULL,
  provider VARCHAR(32) NOT NULL,                       -- 'google' | 'github'
  provider_client_id TEXT NOT NULL,                    -- the OAuth client id at the provider
  provider_client_secret_encrypted TEXT NOT NULL,      -- AES-256-GCM ciphertext
  redirect_uri TEXT NOT NULL,                          -- callback registered at the provider
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_eip_oauth_client FOREIGN KEY (oauth_client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_eip_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);

-- At most one config per (client, provider) — a client can offer Google and
-- GitHub, but not two Google buttons.
CREATE UNIQUE INDEX uq_eip_client_provider ON external_identity_providers (oauth_client_id, provider);
CREATE INDEX idx_eip_oauth_client ON external_identity_providers (oauth_client_id);
