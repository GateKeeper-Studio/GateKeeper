-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS mfa_method (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    "type" VARCHAR(16) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NULL,
    /* mfa_method >- tenant_user = fk_user_mfa_method */
    CONSTRAINT fk_user_mfa_method FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.