CREATE TABLE IF NOT EXISTS external_oauth_state (
  id UUID PRIMARY KEY,
  provider_state VARCHAR(126) NOT NULL,
  application_oauth_provider_id UUID NOT NULL,
  client_state VARCHAR(256),
  client_code_challenge_method VARCHAR(50),
  client_code_challenge VARCHAR(256),
  client_scope VARCHAR(256),
  client_response_type VARCHAR(50),
  code_verifier VARCHAR(512) NOT NULL,
  client_redirect_uri VARCHAR(512),
  created_at TIMESTAMP NOT NULL,
  /* external_oauth_state >- application_oauth_provider_id = fk_external_oauth_state_application_oauth_provider*/
  CONSTRAINT fk_external_oauth_state_application_oauth_provider FOREIGN KEY (application_oauth_provider_id) REFERENCES "application_oauth_provider" (id) ON DELETE CASCADE
);

---- create above / drop below ----
DROP TABLE IF EXISTS external_oauth_state;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.