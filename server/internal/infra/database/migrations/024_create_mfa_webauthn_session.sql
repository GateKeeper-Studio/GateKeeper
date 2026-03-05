-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS mfa_webauthn_session (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    session_data TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    CONSTRAINT fk_user_mfa_webauthn_session FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS mfa_webauthn_session;