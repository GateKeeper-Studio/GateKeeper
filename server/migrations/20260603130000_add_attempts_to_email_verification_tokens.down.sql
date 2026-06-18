-- Reverse the 6-digit-code migration: restore the unique token_hash index and
-- drop the per-user index and the attempts counter.
DROP INDEX IF EXISTS idx_email_verification_tokens_user_id;

CREATE UNIQUE INDEX uq_email_verification_tokens_token_hash
  ON email_verification_tokens (token_hash);

ALTER TABLE email_verification_tokens
  DROP COLUMN attempts;
