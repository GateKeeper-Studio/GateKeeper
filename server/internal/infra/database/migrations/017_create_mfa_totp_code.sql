-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS mfa_totp_code (
    id UUID PRIMARY KEY,
    mfa_method_id UUID NOT NULL,
    secret VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    /* mfa_totp_code >- tenant_user = fk_user_mfa_totp_code*/
    CONSTRAINT fk_mfa_method_mfa_totp_code FOREIGN KEY (mfa_method_id) REFERENCES "mfa_method" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS mfa_totp_code;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.