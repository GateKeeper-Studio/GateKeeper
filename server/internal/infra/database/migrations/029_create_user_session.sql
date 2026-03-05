-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS user_session (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  application_id UUID NOT NULL,
  ip_address VARCHAR(45) NOT NULL,
  user_agent TEXT NOT NULL,
  location TEXT,
  created_at TIMESTAMP NOT NULL,
  last_active_at TIMESTAMP NOT NULL,
  expires_at TIMESTAMP NOT NULL,
  is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
  CONSTRAINT fk_user_session_user FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE,
  CONSTRAINT fk_user_session_application FOREIGN KEY (application_id) REFERENCES "application" (id) ON DELETE CASCADE
);

CREATE INDEX idx_user_session_user_id ON user_session (user_id);

CREATE INDEX idx_user_session_application_id ON user_session (application_id);

---- create above / drop below ----
DROP TABLE IF EXISTS user_session;