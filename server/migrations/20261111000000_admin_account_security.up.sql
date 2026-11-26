-- Platform-admin account security: case-insensitive sign-in emails, per-account
-- sign-in throttling, a second factor (TOTP plus single-use recovery codes) that an
-- organization can require, and the admins of an organization managing each other
-- (invitations by one-time link).

-- ── Emails are stored lowercased ─────────────────────────────────────────────────
-- Sign-in compared the email byte for byte, so "Ops@acme.com" and "ops@acme.com"
-- could be two admins, and an admin who typed their address in another case could
-- not sign in. From now on the application lowercases every admin email, and the
-- CHECK keeps it that way.
--
-- If two existing admins differ only in the case of their email, the UPDATE below
-- fails on uq_admin_users_email and the migration stops with nothing changed.
-- Rename or deactivate one of them, then run the migrations again.
UPDATE admin_users
SET email = lower(btrim(email))
WHERE email <> lower(btrim(email));

ALTER TABLE admin_users
    ADD CONSTRAINT chk_admin_users_email_lowercase CHECK (email = lower(email));

-- ── Sign-in throttling per account ──────────────────────────────────────────────
-- The per-IP rate limit does nothing against a password spray spread over many
-- addresses. Consecutive failed sign-ins (wrong password or wrong code) are
-- counted on the account; at the limit it is locked for a while and the counter
-- restarts. A successful sign-in clears both.
ALTER TABLE admin_users
    ADD COLUMN failed_sign_in_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN locked_until         TIMESTAMPTZ,
    ADD COLUMN last_sign_in_at      TIMESTAMPTZ,
    ADD CONSTRAINT chk_admin_users_failed_sign_in_count CHECK (failed_sign_in_count >= 0);

-- ── Second factor ───────────────────────────────────────────────────────────────
-- One row per admin who started enrolling an authenticator app. The TOTP seed is
-- sealed with SecretCipher (purpose admin_totp_seed, bound to the admin id). The
-- factor is in force once confirmed. totp_last_step is the last accepted time
-- step, so a code cannot be replayed. Recovery codes are stored as SHA-256 hashes
-- and removed from the array when used.
CREATE TABLE admin_mfa (
    admin_id             UUID NOT NULL,
    totp_secret_enc      TEXT NOT NULL,
    totp_confirmed_at    TIMESTAMPTZ,
    totp_last_step       BIGINT,
    recovery_code_hashes TEXT[] NOT NULL DEFAULT '{}',
    created_at           TIMESTAMPTZ NOT NULL,
    updated_at           TIMESTAMPTZ,
    CONSTRAINT admin_mfa_pkey PRIMARY KEY (admin_id),
    CONSTRAINT admin_mfa_admin_id_fkey FOREIGN KEY (admin_id) REFERENCES admin_users(id) ON DELETE CASCADE
);

-- An organization can require every admin to use the second factor. An admin
-- without one can then only enroll until they do (the API refuses the rest).
ALTER TABLE organizations
    ADD COLUMN require_admin_mfa BOOLEAN NOT NULL DEFAULT false;

-- ── Invitations ─────────────────────────────────────────────────────────────────
-- An admin invites a colleague by email; the invitation is a one-time link the
-- inviter hands over out of band. Only the SHA-256 of its token is stored. Accepting
-- it creates the admin account with the password the invitee chooses.
CREATE TABLE admin_invitations (
    id              UUID NOT NULL,
    organization_id UUID NOT NULL,
    email           VARCHAR(128) NOT NULL,
    token_hash      VARCHAR(64) NOT NULL,
    invited_by      UUID,
    expires_at      TIMESTAMPTZ NOT NULL,
    accepted_at     TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL,
    CONSTRAINT admin_invitations_pkey PRIMARY KEY (id),
    CONSTRAINT admin_invitations_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    CONSTRAINT admin_invitations_invited_by_fkey FOREIGN KEY (invited_by) REFERENCES admin_users(id) ON DELETE SET NULL,
    CONSTRAINT chk_admin_invitations_email_lowercase CHECK (email = lower(email))
);

CREATE UNIQUE INDEX uq_admin_invitations_token_hash ON admin_invitations (token_hash);
CREATE INDEX ix_admin_invitations_organization ON admin_invitations (organization_id, created_at DESC);
CREATE INDEX ix_admin_invitations_invited_by ON admin_invitations (invited_by);
CREATE INDEX ix_admin_invitations_expires ON admin_invitations (expires_at);
-- At most one open invitation per email and organization.
CREATE UNIQUE INDEX uq_admin_invitations_open_email
    ON admin_invitations (organization_id, email)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;

-- Expired admin refresh tokens are now purged by the retention job.
CREATE INDEX ix_admin_refresh_tokens_expires ON admin_refresh_tokens (expires_at);
