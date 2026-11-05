-- Links a local user to an external provider account. On a returning social
-- login we recognise the user by (tenant, provider, provider_subject) without
-- relying solely on the email, which can change at the provider.
CREATE TABLE IF NOT EXISTS social_identities (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  tenant_id UUID NOT NULL,
  provider VARCHAR(32) NOT NULL,            -- 'google' | 'github'
  provider_subject TEXT NOT NULL,           -- the provider's stable user id ("sub")
  email TEXT NULL,                          -- email as seen at the provider (informational)
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_social_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_social_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_social_provider_subject ON social_identities (tenant_id, provider, provider_subject);
CREATE INDEX idx_social_user ON social_identities (user_id);
