-- One in-flight MFA ceremony: an email OTP waiting to be typed, or a WebAuthn
-- challenge waiting for the authenticator's response. Single-use, short-lived,
-- attempt-counted (like email_verification_tokens).
CREATE TABLE IF NOT EXISTS mfa_challenges (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  auth_session_id UUID NULL,              -- NULL when enrolling via bearer token (no login session)
  purpose VARCHAR(16) NOT NULL,           -- 'login' | 'enroll'
  method VARCHAR(16) NOT NULL,            -- 'email' | 'webauthn'
  code_hash VARCHAR(64) NULL,             -- email: sha256 hex of the 6-digit code
  state_json JSONB NULL,                  -- webauthn: serialized ceremony state (never sent to clients)
  attempts INT NOT NULL DEFAULT 0,
  consumed_at TIMESTAMPTZ NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_mfa_challenges_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE,
  CONSTRAINT fk_mfa_challenges_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
  CONSTRAINT fk_mfa_challenges_session FOREIGN KEY (auth_session_id) REFERENCES auth_sessions (id) ON DELETE CASCADE
);
CREATE INDEX idx_mfa_challenges_session ON mfa_challenges (auth_session_id);
CREATE INDEX idx_mfa_challenges_user ON mfa_challenges (user_id);
