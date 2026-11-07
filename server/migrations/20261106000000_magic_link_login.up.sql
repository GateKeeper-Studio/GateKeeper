-- Magic link login: "email me a sign-in link" as an alternative to the password.
--
-- 1. Per-client opt-in. FALSE keeps today's behaviour for every existing client.
ALTER TABLE oauth_clients
  ADD COLUMN magic_link_enabled BOOLEAN NOT NULL DEFAULT FALSE;

-- 2. A magic-link auth session: opened from the email alone with no user bound, then
--    authenticated when the emailed link is redeemed from the same browser.
ALTER TABLE auth_sessions
  DROP CONSTRAINT chk_auth_sessions_primary_factor,
  ADD CONSTRAINT chk_auth_sessions_primary_factor
    CHECK (primary_factor IN ('password', 'passkey', 'magic_link'));

-- 3. The one-time links. Only the SHA-256 hash is stored; the raw 256-bit token lives
--    in the emailed link and is shown exactly once. Looked up by `id` (carried as
--    `&id=` in the link), then hash-compared. Each link is tied to the auth session
--    whose code the requesting browser holds, so it cannot be redeemed anywhere
--    else. The session cascade also handles retention: purge_expired_data deletes
--    expired auth_sessions, and their links go with them.
CREATE TABLE IF NOT EXISTS magic_link_tokens (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  auth_session_id UUID NOT NULL,
  token_hash VARCHAR(64) NOT NULL,          -- hex SHA-256 of the raw token
  consumed_at TIMESTAMPTZ NULL,             -- set once redeemed (single-use)
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_magic_link_tokens_tenant  FOREIGN KEY (tenant_id)       REFERENCES tenants (id)       ON DELETE CASCADE,
  CONSTRAINT fk_magic_link_tokens_user    FOREIGN KEY (user_id)         REFERENCES users (id)         ON DELETE CASCADE,
  CONSTRAINT fk_magic_link_tokens_session FOREIGN KEY (auth_session_id) REFERENCES auth_sessions (id) ON DELETE CASCADE
);

-- The resend cooldown reads the newest link of a user.
CREATE INDEX idx_magic_link_tokens_user_created ON magic_link_tokens (user_id, created_at DESC);
-- Cascade deletes from auth_sessions look links up by session.
CREATE INDEX idx_magic_link_tokens_auth_session ON magic_link_tokens (auth_session_id);
