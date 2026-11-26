DROP TABLE qr_login_requests;

-- Sessions opened by a QR code cannot be read back by the previous code.
DELETE FROM auth_sessions WHERE primary_factor = 'qr_code';

ALTER TABLE auth_sessions
    DROP CONSTRAINT chk_auth_sessions_primary_factor,
    ADD CONSTRAINT chk_auth_sessions_primary_factor
        CHECK (primary_factor IN ('password', 'passkey', 'magic_link'));

ALTER TABLE oauth_clients DROP COLUMN qr_login_enabled;
