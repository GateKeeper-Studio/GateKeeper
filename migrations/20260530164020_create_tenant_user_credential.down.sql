-- Add down migration script here
DROP TABLE IF EXISTS user_credentials;

DROP INDEX IF EXISTS uq_user_credentials_user_id;