-- Backs the self-service "forgot password" flow: a single-use, short-lived token
-- mailed to the user as a reset link. Only the SHA-256 hash is stored; the raw
-- 256-bit token lives in the link and is shown exactly once. The token is looked
-- up by its `id` (carried as `&id=` in the link), then the secret is hash-
-- compared — so the high-entropy value is never itself a lookup key. The pending
-- OAuth request is NOT stored here.
CREATE TABLE IF NOT EXISTS password_reset_tokens (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  token_hash VARCHAR(64) NOT NULL,          -- hex SHA-256 of the raw token
  consumed_at TIMESTAMPTZ NULL,             -- set once redeemed (single-use)
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_password_reset_tokens_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE,
  CONSTRAINT fk_password_reset_tokens_user   FOREIGN KEY (user_id)   REFERENCES users (id)   ON DELETE CASCADE
);

-- Invalidating a user's older reset links (single-live-token rule) filters by user.
CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens (user_id);
-- A background job deletes rows past expires_at.
CREATE INDEX idx_password_reset_tokens_expires_at ON password_reset_tokens (expires_at);
