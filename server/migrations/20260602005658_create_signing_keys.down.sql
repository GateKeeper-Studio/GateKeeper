-- Add down migration script here
DROP TABLE IF EXISTS "signing_keys";
DROP INDEX IF EXISTS uq_signing_keys_kid;
DROP INDEX IF EXISTS idx_signing_keys_tenant_status;