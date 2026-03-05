-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS "application" (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    has_mfa_auth_app BOOLEAN NOT NULL DEFAULT FALSE,
    has_mfa_email BOOLEAN NOT NULL DEFAULT FALSE,
    has_mfa_webauthn BOOLEAN NOT NULL DEFAULT FALSE,
    password_hash_secret VARCHAR(255) NOT NULL,
    badges TEXT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    can_self_sign_up BOOLEAN NOT NULL DEFAULT FALSE,
    can_self_forgot_pass BOOLEAN NOT NULL DEFAULT FALSE,
    refresh_token_ttl_days INT NOT NULL DEFAULT 7,
    requires_high_security BOOLEAN NOT NULL DEFAULT FALSE,
    /* application >- tenant = fk_tenant_application */
    CONSTRAINT fk_tenant_application FOREIGN KEY (tenant_id) REFERENCES "tenant" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS "application";

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.