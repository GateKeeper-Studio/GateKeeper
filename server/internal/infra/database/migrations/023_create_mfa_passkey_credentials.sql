-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS mfa_passkey_credentials (
    id UUID PRIMARY KEY,
    mfa_method_id UUID NOT NULL,
    credential_id TEXT NOT NULL,
    public_key TEXT NOT NULL,
    sign_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL,
    backup_eligible BOOLEAN NOT NULL DEFAULT FALSE,
    backup_state BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT fk_mfa_method_passkey_credentials FOREIGN KEY (mfa_method_id) REFERENCES "mfa_method" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS mfa_passkey_credentials;