-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS "application_mail_config" (
    id UUID PRIMARY KEY,
    application_id UUID NOT NULL,
    host VARCHAR(255) NOT NULL,
    username VARCHAR(255) NOT NULL,
    PASSWORD VARCHAR(255) NOT NULL,
    port INT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    /* application_mail_config -- application (one-to-one) = fk_application_mail_config_application */
    CONSTRAINT fk_application_mail_config_application FOREIGN KEY (application_id) REFERENCES "application" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS application_mail_config;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.