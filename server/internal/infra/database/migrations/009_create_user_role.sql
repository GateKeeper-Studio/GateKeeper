-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS user_role (
    user_id UUID NOT NULL,
    role_id UUID NOT NULL,
    created_at TIMESTAMP NOT NULL,
    /* user_role >- application_role = fk_user_role_application_role */
    CONSTRAINT fk_user_role_application_role FOREIGN KEY (role_id) REFERENCES "application_role" (id) ON DELETE CASCADE,
    /* user_role >- application_user = fk_user_role_application_user */
    CONSTRAINT fk_user_role_application_user FOREIGN KEY (user_id) REFERENCES "application_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS user_role;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.