-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS "tenant_user" (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    email VARCHAR(128) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_email_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    preferred_2fa_method VARCHAR(24) NULL,
    /* tenant_user >- tenant = fk_tenant_user_tenant */
    CONSTRAINT fk_tenant_user_tenant FOREIGN KEY (tenant_id) REFERENCES "tenant" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS tenant_user;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.