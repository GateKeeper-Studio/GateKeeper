-- Admin password reset: the server emails the user a temporary password that signs
-- in once, before this moment, and only to choose a new password. Signing in with it
-- moves the moment to then, which burns it. NULL: not such a password (one the user
-- chose, or one set when an admin created the account).
-- See docs/FLOWS-AUTHENTICATION.md §4 ("Admin password reset").
ALTER TABLE user_credentials ADD COLUMN temporary_password_expires_at TIMESTAMPTZ;
