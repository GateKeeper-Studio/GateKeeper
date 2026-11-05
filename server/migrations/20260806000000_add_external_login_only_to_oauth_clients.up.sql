-- Per-client authentication policy: when true, this client authenticates ONLY
-- through its configured external identity providers (Google/GitHub). Every
-- password-credential path — self-registration, password sign-in, forgot/reset
-- password, forced password change — is refused for it, and its users never pick
-- a password (the social callback already provisions passwordless accounts).
--
-- FALSE preserves today's behaviour for every existing client.
ALTER TABLE oauth_clients
  ADD COLUMN external_login_only BOOLEAN NOT NULL DEFAULT FALSE;
