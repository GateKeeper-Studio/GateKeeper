-- Add up migration script here
-- Backs self-registration email confirmation: a single-use, short-lived token
-- mailed to the user. Only the SHA-256 hash is stored; the raw value lives in
-- the email link. The pending OAuth request is NOT stored here — it rides in the
-- link and is re-validated at confirm time.
CREATE TABLE IF NOT EXISTS email_verification_tokens (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  token_hash VARCHAR(64) NOT NULL,          -- hex SHA-256 of the raw token
  consumed_at TIMESTAMPTZ NULL,             -- set once redeemed (single-use)
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_email_verification_tokens_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE,
  CONSTRAINT fk_email_verification_tokens_user   FOREIGN KEY (user_id)   REFERENCES users (id)   ON DELETE CASCADE
);
-- The token hash is the lookup key when the user clicks the link.
CREATE UNIQUE INDEX uq_email_verification_tokens_token_hash ON email_verification_tokens (token_hash);
-- a background job deletes rows past expires_at
CREATE INDEX idx_email_verification_tokens_expires_at ON email_verification_tokens (expires_at);
