-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS "tenant_user" (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL,
    email VARCHAR(128) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_email_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    preferred_2fa_method VARCHAR(24) NULL,
    /* tenant_user >- organization = fk_tenant_user_organization */
    CONSTRAINT fk_tenant_user_organization FOREIGN KEY (organization_id) REFERENCES "organization" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS tenant_user;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.