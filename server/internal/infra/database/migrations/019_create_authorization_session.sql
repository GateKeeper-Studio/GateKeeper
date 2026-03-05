-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS authorization_session (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    token VARCHAR(128) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    is_used BOOLEAN NOT NULL,
    /* authorization_session >- tenant_user = fk_user_authorization_session */
    CONSTRAINT fk_user_authorization_session FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS authorization_session;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.