ALTER TABLE auth_sessions
  DROP COLUMN mfa_required,
  DROP COLUMN mfa_verified_at,
  DROP COLUMN mfa_method;
