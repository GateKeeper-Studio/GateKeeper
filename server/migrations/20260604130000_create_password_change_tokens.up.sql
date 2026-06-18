-- Backs the forced "change password" flow for credentials flagged
-- should_change_password (e.g. an admin-issued temporary password). At login the
-- provider mints a single-use, short-lived token returned to the just-
-- authenticated SPA; redeeming it sets a new password and clears the flag. Only
-- the SHA-256 hash is stored; the raw 256-bit token is returned exactly once and
-- resolved per user (the change request carries the user id), then hash-compared.
CREATE TABLE IF NOT EXISTS password_change_tokens (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  token_hash VARCHAR(64) NOT NULL,          -- hex SHA-256 of the raw token
  consumed_at TIMESTAMPTZ NULL,             -- set once redeemed (single-use)
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_password_change_tokens_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE,
  CONSTRAINT fk_password_change_tokens_user   FOREIGN KEY (user_id)   REFERENCES users (id)   ON DELETE CASCADE
);

-- The token is looked up per user (and older ones invalidated by user).
CREATE INDEX idx_password_change_tokens_user_id ON password_change_tokens (user_id);
-- A background job deletes rows past expires_at.
CREATE INDEX idx_password_change_tokens_expires_at ON password_change_tokens (expires_at);
