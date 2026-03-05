-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS step_up_token (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  application_id UUID NOT NULL,
  token VARCHAR(128) NOT NULL,
  created_at TIMESTAMP NOT NULL,
  expires_at TIMESTAMP NOT NULL,
  is_used BOOLEAN NOT NULL DEFAULT FALSE,
  CONSTRAINT fk_step_up_token_user FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE,
  CONSTRAINT fk_step_up_token_application FOREIGN KEY (application_id) REFERENCES "application" (id) ON DELETE CASCADE
);

CREATE INDEX idx_step_up_token_user_id ON step_up_token (user_id);

CREATE INDEX idx_step_up_token_token ON step_up_token (token);

---- create above / drop below ----
DROP TABLE IF EXISTS step_up_token;