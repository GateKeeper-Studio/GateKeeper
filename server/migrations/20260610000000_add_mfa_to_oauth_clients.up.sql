-- Per-client MFA policy: whether a second factor is mandatory, and which
-- methods this client accepts. Methods default to enabled so flipping
-- mfa_required on immediately offers every factor.
ALTER TABLE oauth_clients
  ADD COLUMN mfa_required BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN mfa_email_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN mfa_totp_enabled BOOLEAN NOT NULL DEFAULT TRUE,
  ADD COLUMN mfa_webauthn_enabled BOOLEAN NOT NULL DEFAULT TRUE;
