-- Which factor opened the login session.
--
-- 'password': the password was verified before the session existed; an MFA gate may be
--             cleared with any second factor the client allows.
-- 'passkey':  passwordless login. The account has no password, so the session was opened
--             from the email alone and may be cleared ONLY by a WebAuthn assertion.
--
-- The DEFAULT stays on purpose: during a rolling deploy an older binary inserts rows
-- without this column, and every session it creates is a password session.
ALTER TABLE auth_sessions
    ADD COLUMN primary_factor VARCHAR(16) NOT NULL DEFAULT 'password',
    ADD CONSTRAINT chk_auth_sessions_primary_factor
        CHECK (primary_factor IN ('password', 'passkey'));
