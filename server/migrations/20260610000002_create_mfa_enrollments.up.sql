-- A second factor a user has registered. Email OTP is implicit (every user has
-- a verified email) and never gets a row here; only 'totp' and 'webauthn' do.
CREATE TABLE IF NOT EXISTS mfa_enrollments (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  method VARCHAR(16) NOT NULL,            -- 'totp' | 'webauthn'
  secret_enc TEXT NULL,                   -- TOTP: AES-256-GCM-encrypted base32 secret
  totp_last_step BIGINT NULL,             -- TOTP: last accepted timestep (replay guard)
  passkey_json JSONB NULL,                -- WebAuthn: serialized passkey (public key + counter)
  credential_id TEXT NULL,                -- WebAuthn: base64url credential id
  name VARCHAR(100) NULL,                 -- user-facing label ("YubiKey 5C")
  confirmed_at TIMESTAMPTZ NULL,          -- NULL = enrollment started but never proven
  last_used_at TIMESTAMPTZ NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_mfa_enrollments_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE,
  CONSTRAINT fk_mfa_enrollments_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);
CREATE INDEX idx_mfa_enrollments_user ON mfa_enrollments (user_id);
-- One TOTP enrollment per user; starting a new one replaces the pending row.
CREATE UNIQUE INDEX uq_mfa_enrollments_totp ON mfa_enrollments (user_id) WHERE method = 'totp';
-- A WebAuthn credential id identifies one authenticator globally; a user may
-- register many credentials, but no credential may belong to two users.
CREATE UNIQUE INDEX uq_mfa_enrollments_cred ON mfa_enrollments (credential_id) WHERE credential_id IS NOT NULL;
