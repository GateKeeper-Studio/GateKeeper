-- Write your migrate up statements here
ALTER TABLE
  mfa_webauthn_credentials
ADD
  COLUMN backup_eligible BOOLEAN NOT NULL DEFAULT false,
ADD
  COLUMN backup_state BOOLEAN NOT NULL DEFAULT false;

---- create above / drop below ----
ALTER TABLE
  mfa_webauthn_credentials DROP COLUMN IF EXISTS backup_eligible,
  DROP COLUMN IF EXISTS backup_state;