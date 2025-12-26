-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS "application_authorization_code" (
    id UUID PRIMARY KEY,
    application_id UUID NOT NULL,
    user_id UUID NOT NULL,
    expired_at TIMESTAMP NOT NULL,
    code VARCHAR(255) NOT NULL,
    redirect_uri VARCHAR(255) NOT NULL,
    code_challenge VARCHAR(255) NOT NULL,
    code_challenge_method VARCHAR(255) NOT NULL,
    /* authorization_code >- application_user = fk_application_authorization_code_user*/
    CONSTRAINT fk_application_authorization_code_user FOREIGN KEY (user_id) REFERENCES "application_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS application_authorization_code;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.