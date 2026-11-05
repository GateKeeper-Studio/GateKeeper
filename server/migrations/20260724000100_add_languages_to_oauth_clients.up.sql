-- Add up migration script here
ALTER TABLE oauth_clients
  ADD COLUMN allowed_languages TEXT[] NOT NULL DEFAULT ARRAY['en'],
  ADD COLUMN default_language VARCHAR(10) NOT NULL DEFAULT 'en';
