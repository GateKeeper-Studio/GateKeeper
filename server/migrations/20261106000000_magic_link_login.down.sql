DROP TABLE IF EXISTS magic_link_tokens;

-- Sessions opened by a magic link cannot be represented once the value is gone.
DELETE FROM auth_sessions WHERE primary_factor = 'magic_link';

ALTER TABLE auth_sessions
  DROP CONSTRAINT chk_auth_sessions_primary_factor,
  ADD CONSTRAINT chk_auth_sessions_primary_factor
    CHECK (primary_factor IN ('password', 'passkey'));

ALTER TABLE oauth_clients DROP COLUMN IF EXISTS magic_link_enabled;
