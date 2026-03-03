-- Add OIDC nonce field to external_oauth_state
ALTER TABLE
  external_oauth_state
ADD
  COLUMN IF NOT EXISTS client_nonce VARCHAR(255);

---- create above / drop below ----
ALTER TABLE
  external_oauth_state DROP COLUMN IF EXISTS client_nonce;