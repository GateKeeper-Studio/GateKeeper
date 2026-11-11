-- QR code sign-in: a browser shows a QR code, another device (a phone where the
-- user signs in, or an app they are already signed in to) scans it and approves,
-- and the browser's own login session then completes. See
-- docs/FLOWS-AUTHENTICATION.md §1 ("QR code sign-in").

-- Off by default: an admin opts each application in, like sign-in links.
ALTER TABLE oauth_clients
    ADD COLUMN qr_login_enabled BOOLEAN NOT NULL DEFAULT false;

-- A login session opened by a QR code binds no user until a scan is approved.
ALTER TABLE auth_sessions
    DROP CONSTRAINT chk_auth_sessions_primary_factor,
    ADD CONSTRAINT chk_auth_sessions_primary_factor
        CHECK (primary_factor IN ('password', 'passkey', 'magic_link', 'qr_code'));

-- One row per QR code. Only SHA-256 hashes of the two secrets are stored: the QR
-- token (in the code itself) and the claim code (given to the first device that
-- scans it; approving or denying needs it). The status only moves forward, each
-- step one conditional UPDATE:
--   pending -> scanned -> approved -> completed   (the browser finished signing in)
--                      -> denied
CREATE TABLE qr_login_requests (
    id                    UUID NOT NULL,
    tenant_id             UUID NOT NULL,
    client_id             UUID NOT NULL,
    auth_session_id       UUID NOT NULL,
    token_hash            VARCHAR(64) NOT NULL,
    status                VARCHAR(16) NOT NULL DEFAULT 'pending',
    claim_hash            VARCHAR(64),
    -- Set at scan time when the scanner is a signed-in app (its user is known).
    scanned_by_user_id    UUID,
    approved_user_id      UUID,
    -- Whether the approval itself passed a second factor (a phone that signed in
    -- with MFA). When it did, the browser is not asked for another one.
    approver_mfa_verified BOOLEAN NOT NULL DEFAULT false,
    approved_via          VARCHAR(16),
    -- What the approving device is shown about the browser asking to sign in.
    requester_ip          TEXT,
    requester_user_agent  TEXT,
    requester_location    TEXT,
    scanned_at            TIMESTAMPTZ,
    decided_at            TIMESTAMPTZ,
    completed_at          TIMESTAMPTZ,
    expires_at            TIMESTAMPTZ NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL,
    CONSTRAINT qr_login_requests_pkey PRIMARY KEY (id),
    CONSTRAINT chk_qr_login_requests_status
        CHECK (status IN ('pending', 'scanned', 'approved', 'denied', 'completed')),
    CONSTRAINT chk_qr_login_requests_approved_via
        CHECK (approved_via IS NULL OR approved_via IN ('hosted_page', 'signed_in_app')),
    -- Past `pending`, a device has claimed the code.
    CONSTRAINT chk_qr_login_requests_claimed
        CHECK (status = 'pending' OR claim_hash IS NOT NULL),
    -- Approved and completed requests name the user who approved.
    CONSTRAINT chk_qr_login_requests_approved_user
        CHECK (status NOT IN ('approved', 'completed') OR approved_user_id IS NOT NULL),
    CONSTRAINT fk_qr_login_requests_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_qr_login_requests_client FOREIGN KEY (client_id) REFERENCES oauth_clients(id) ON DELETE CASCADE,
    -- Purged with its login session (retention purges expired sessions).
    CONSTRAINT fk_qr_login_requests_session FOREIGN KEY (auth_session_id) REFERENCES auth_sessions(id) ON DELETE CASCADE,
    CONSTRAINT fk_qr_login_requests_scanned_by FOREIGN KEY (scanned_by_user_id) REFERENCES users(id) ON DELETE CASCADE,
    CONSTRAINT fk_qr_login_requests_approved_user FOREIGN KEY (approved_user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- A login session shows one QR code; a new code opens a new session.
CREATE UNIQUE INDEX uq_qr_login_requests_auth_session ON qr_login_requests (auth_session_id);
-- Foreign keys: deleting a tenant, client or user finds its rows by index.
CREATE INDEX ix_qr_login_requests_tenant ON qr_login_requests (tenant_id, created_at DESC);
CREATE INDEX ix_qr_login_requests_client ON qr_login_requests (client_id);
CREATE INDEX ix_qr_login_requests_scanned_by ON qr_login_requests (scanned_by_user_id)
    WHERE scanned_by_user_id IS NOT NULL;
CREATE INDEX ix_qr_login_requests_approved_user ON qr_login_requests (approved_user_id)
    WHERE approved_user_id IS NOT NULL;
