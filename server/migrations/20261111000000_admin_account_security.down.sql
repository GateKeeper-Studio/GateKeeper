DROP INDEX ix_admin_refresh_tokens_expires;

DROP TABLE admin_invitations;

ALTER TABLE organizations DROP COLUMN require_admin_mfa;

DROP TABLE admin_mfa;

ALTER TABLE admin_users
    DROP CONSTRAINT chk_admin_users_failed_sign_in_count,
    DROP COLUMN last_sign_in_at,
    DROP COLUMN locked_until,
    DROP COLUMN failed_sign_in_count;

-- Emails stay lowercased: the original case is not recoverable, and lowercase
-- addresses still work with the previous code.
ALTER TABLE admin_users
    DROP CONSTRAINT chk_admin_users_email_lowercase;
