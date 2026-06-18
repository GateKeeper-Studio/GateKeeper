-- MFA state on the login session: a session opened by a password check against
-- a client that requires MFA stays "pending" (mfa_required AND mfa_verified_at
-- IS NULL) and cannot be spent for an authorization code until verified.
ALTER TABLE auth_sessions
  ADD COLUMN mfa_required BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN mfa_verified_at TIMESTAMPTZ NULL,
  ADD COLUMN mfa_method VARCHAR(16) NULL;
