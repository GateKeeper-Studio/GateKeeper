-- Add down migration script here
DROP TABLE IF EXISTS "oauth_clients";
DROP INDEX IF EXISTS uq_oauth_clients_client_id;