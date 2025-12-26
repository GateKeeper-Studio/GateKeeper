-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS "application_user" (
    id UUID PRIMARY KEY,
    application_id UUID NOT NULL,
    email VARCHAR(128) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_email_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    preferred_2fa_method VARCHAR(24) NULL,
    /* application_user >- application = fk_application_user_application */
    CONSTRAINT fk_application_user_application FOREIGN KEY (application_id) REFERENCES "application" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS application_user;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.