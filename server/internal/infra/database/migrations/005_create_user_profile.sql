-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS user_profile (
    user_id UUID PRIMARY KEY NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    phone_number VARCHAR(255) NULL,
    address VARCHAR(255) NULL,
    photo_url VARCHAR(255) NULL,
    /* user_profile -- user = fk_user_profile_user */
    CONSTRAINT fk_user_profile_user FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS user_profile;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.