-- Add OIDC nonce and scope fields to application_authorization_code
ALTER TABLE
  application_authorization_code
ADD
  COLUMN IF NOT EXISTS nonce VARCHAR(255),
ADD
  COLUMN IF NOT EXISTS scope VARCHAR(512);

---- create above / drop below ----
ALTER TABLE
  application_authorization_code DROP COLUMN IF EXISTS nonce,
  DROP COLUMN IF EXISTS scope;