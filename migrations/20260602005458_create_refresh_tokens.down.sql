-- Add down migration script here
DROP TABLE IF EXISTS "refresh_tokens";
DROP INDEX IF EXISTS uq_refresh_tokens_token_hash;
DROP INDEX IF EXISTS idx_refresh_tokens_family;