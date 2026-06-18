-- Add down migration script here
DROP TABLE IF EXISTS users;

DROP INDEX IF EXISTS uq_users_tenant_email;
DROP INDEX IF EXISTS idx_users_tenant_id_created_at;