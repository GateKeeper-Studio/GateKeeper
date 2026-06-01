-- Add up migration script here
CREATE TABLE IF NOT EXISTS user_profiles (
    user_id UUID PRIMARY KEY NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    phone_number VARCHAR(255) NULL,
    address VARCHAR(255) NULL,
    photo_url VARCHAR(255) NULL,
    /* user_profile -- user = fk_user_profile_user */
    CONSTRAINT fk_user_profile_user FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);