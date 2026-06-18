ALTER TABLE oauth_clients
  DROP COLUMN mfa_required,
  DROP COLUMN mfa_email_enabled,
  DROP COLUMN mfa_totp_enabled,
  DROP COLUMN mfa_webauthn_enabled;
