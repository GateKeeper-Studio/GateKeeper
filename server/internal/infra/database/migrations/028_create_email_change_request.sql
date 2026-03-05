-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS email_change_request (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  application_id UUID NOT NULL,
  new_email VARCHAR(255) NOT NULL,
  token VARCHAR(256) NOT NULL,
  created_at TIMESTAMP NOT NULL,
  expires_at TIMESTAMP NOT NULL,
  is_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
  CONSTRAINT fk_email_change_request_user FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE,
  CONSTRAINT fk_email_change_request_application FOREIGN KEY (application_id) REFERENCES "application" (id) ON DELETE CASCADE
);

CREATE INDEX idx_email_change_request_user_id ON email_change_request (user_id);

CREATE INDEX idx_email_change_request_token ON email_change_request (token);

---- create above / drop below ----
DROP TABLE IF EXISTS email_change_request;