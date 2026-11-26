-- Approvals by a trusted device read as approvals by a signed-in app.
UPDATE qr_login_requests SET approved_via = 'signed_in_app' WHERE approved_via = 'trusted_device';

ALTER TABLE qr_login_requests
    DROP CONSTRAINT chk_qr_login_requests_approved_via,
    ADD CONSTRAINT chk_qr_login_requests_approved_via
        CHECK (approved_via IS NULL OR approved_via IN ('hosted_page', 'signed_in_app'));

DROP TABLE trusted_devices;
