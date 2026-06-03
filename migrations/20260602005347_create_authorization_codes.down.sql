-- Add down migration script here
DROP TABLE IF EXISTS "authorization_codes";
DROP INDEX IF EXISTS uq_authorization_codes_code_hash;
DROP INDEX IF EXISTS idx_authorization_codes_expires_at;