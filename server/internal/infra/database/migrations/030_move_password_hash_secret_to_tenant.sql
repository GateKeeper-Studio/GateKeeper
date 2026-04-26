-- Move password_hash_secret from application to tenant
ALTER TABLE
  "tenant"
ADD
  COLUMN password_hash_secret VARCHAR(255) NOT NULL DEFAULT '';

ALTER TABLE
  "application" DROP COLUMN password_hash_secret;

---- create above / drop below ----
ALTER TABLE
  "application"
ADD
  COLUMN password_hash_secret VARCHAR(255) NOT NULL DEFAULT '';

ALTER TABLE
  "tenant" DROP COLUMN password_hash_secret;