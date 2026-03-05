-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS refresh_token (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL,
    /* refresh_token >- tenant_user = fk_user_refresh_token*/
    CONSTRAINT fk_user_refresh_token FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS refresh_token;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.