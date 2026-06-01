-- Credentials are 1:1 with a user; enforce it so the upsert/update path is safe.
ALTER TABLE user_credentials
  ADD CONSTRAINT uq_user_credentials_user_id UNIQUE (user_id);
