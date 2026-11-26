-- A pending temporary password then reads as one set at account creation: it keeps
-- forcing a password change, but no longer expires.
ALTER TABLE user_credentials DROP COLUMN temporary_password_expires_at;
