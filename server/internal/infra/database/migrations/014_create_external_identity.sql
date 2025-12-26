-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS external_identity (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    email VARCHAR(126) NOT NULL,
    provider VARCHAR(56) NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    application_oauth_provider_id UUID NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NULL,
    /* external_identity >- application_user = fk_user_external_identity*/
    CONSTRAINT fk_user_external_identity FOREIGN KEY (user_id) REFERENCES "application_user" (id) ON DELETE CASCADE,
    CONSTRAINT fk_application_oauth_provider_external_identity FOREIGN KEY (application_oauth_provider_id) REFERENCES "application_oauth_provider" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS external_identity;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.