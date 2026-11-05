-- Add down migration script here
ALTER TABLE oauth_clients
  DROP COLUMN allowed_languages,
  DROP COLUMN default_language;
