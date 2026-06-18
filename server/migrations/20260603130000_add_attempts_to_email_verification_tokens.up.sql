-- Switch email confirmation from an opaque link token to a 6-digit code.
-- A 6-digit code (10^6 combinations) is not globally unique and is brute-forceable,
-- so: (1) drop the global unique index on token_hash, (2) look the code up per-user
-- (index on user_id), and (3) cap wrong tries with an attempts counter.
ALTER TABLE email_verification_tokens
  ADD COLUMN attempts INT NOT NULL DEFAULT 0;

DROP INDEX IF EXISTS uq_email_verification_tokens_token_hash;

CREATE INDEX idx_email_verification_tokens_user_id
  ON email_verification_tokens (user_id);
