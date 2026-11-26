-- Trusted devices: a phone app the user signed in to registers a public key from its
-- secure hardware, and then approves QR code sign-ins by signing them, with no
-- password to type. See docs/FLOWS-AUTHENTICATION.md §1 ("Trusted devices").
CREATE TABLE trusted_devices (
    id                      UUID NOT NULL,
    tenant_id               UUID NOT NULL,
    user_id                 UUID NOT NULL,
    name                    VARCHAR(64) NOT NULL,
    -- ES256 (ECDSA P-256): the uncompressed SEC1 point, base64url without padding.
    algorithm               VARCHAR(16) NOT NULL DEFAULT 'ES256',
    public_key              TEXT NOT NULL,
    -- Whether its approvals satisfy a client's MFA policy: registered with the user's
    -- authenticator-app code, or by a user who had no second factor yet.
    counts_as_second_factor BOOLEAN NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL,
    last_used_at            TIMESTAMPTZ,
    CONSTRAINT trusted_devices_pkey PRIMARY KEY (id),
    CONSTRAINT chk_trusted_devices_algorithm CHECK (algorithm = 'ES256'),
    CONSTRAINT chk_trusted_devices_name CHECK (length(btrim(name)) > 0),
    CONSTRAINT fk_trusted_devices_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_trusted_devices_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- A key is registered once per user; the list is per user, within the tenant.
CREATE UNIQUE INDEX uq_trusted_devices_user_key ON trusted_devices (user_id, public_key);
CREATE INDEX ix_trusted_devices_tenant_user ON trusted_devices (tenant_id, user_id, created_at);

-- A QR code approved by a trusted device's signature.
ALTER TABLE qr_login_requests
    DROP CONSTRAINT chk_qr_login_requests_approved_via,
    ADD CONSTRAINT chk_qr_login_requests_approved_via
        CHECK (approved_via IS NULL OR approved_via IN ('hosted_page', 'signed_in_app', 'trusted_device'));
