-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS change_password_code (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    email VARCHAR(128) NOT NULL,
    token VARCHAR(64) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    /* change_password_code >- tenant_user = fk_user_change_password_code */
    CONSTRAINT fk_user_change_password_code FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS change_password_code;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.