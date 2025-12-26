-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS email_confirmation (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    email VARCHAR(128) NOT NULL,
    token VARCHAR(12) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    cool_down TIMESTAMP NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    is_used BOOLEAN NOT NULL,
    /* email_confirmation >- application_user = fk_user_email_confirmation */
    CONSTRAINT fk_user_email_confirmation FOREIGN KEY (user_id) REFERENCES "application_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS email_confirmation;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.