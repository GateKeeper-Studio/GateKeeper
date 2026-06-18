-- Refresh tokens for platform-admin dashboard sessions. Global (no tenant/client),
-- unlike the per-tenant `refresh_tokens` table. Opaque tokens are stored hashed
-- (SHA-256, never raw). Rotation chains share a `family_id` so presenting an
-- already-rotated token (reuse / theft) can revoke the whole family at once.
CREATE TABLE admin_refresh_tokens (
  id UUID PRIMARY KEY,
  admin_id UUID NOT NULL REFERENCES admin_users (id) ON DELETE CASCADE,

  token_hash VARCHAR(64) NOT NULL,
  family_id UUID NOT NULL,

  rotated_at TIMESTAMPTZ NULL,
  revoked_at TIMESTAMPTZ NULL,

  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL
);

CREATE UNIQUE INDEX uq_admin_refresh_tokens_token_hash ON admin_refresh_tokens (token_hash);
CREATE INDEX ix_admin_refresh_tokens_family ON admin_refresh_tokens (family_id);
CREATE INDEX ix_admin_refresh_tokens_admin ON admin_refresh_tokens (admin_id);
