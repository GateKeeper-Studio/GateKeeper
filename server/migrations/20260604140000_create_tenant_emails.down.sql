ALTER TABLE oauth_clients DROP COLUMN IF EXISTS email_provider_id;
DROP TABLE IF EXISTS tenant_emails;
