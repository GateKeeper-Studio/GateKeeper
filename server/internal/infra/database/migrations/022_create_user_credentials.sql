CREATE TABLE IF NOT EXISTS user_credentials (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  password_algorithm VARCHAR(50) NOT NULL,
  should_change_pass BOOLEAN NOT NULL DEFAULT FALSE,
  updated_at TIMESTAMP NULL,
  created_at TIMESTAMP NOT NULL,
  /* user_credentials >- application_user = fk_user_credentials_application_user */
  CONSTRAINT fk_user_credentials_application_user FOREIGN KEY (user_id) REFERENCES "application_user" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS user_credentials;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.