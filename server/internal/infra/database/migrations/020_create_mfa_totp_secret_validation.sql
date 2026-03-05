-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS mfa_totp_secret_validation (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    secret VARCHAR(255) NOT NULL,
    is_validated BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    /* mfa_totp_secret_validation >- tenant_user = fk_user_mfa_totp_secret_validation*/
    CONSTRAINT fk_user_mfa_totp_secret_validation FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS mfa_totp_secret_validation;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.