-- Add down migration script here
DROP TABLE IF EXISTS "consent_grants";
DROP INDEX IF EXISTS uq_consent_grants_user_client;