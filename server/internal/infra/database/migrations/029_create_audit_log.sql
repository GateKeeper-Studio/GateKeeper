-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS audit_log (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  application_id UUID NOT NULL,
  event_type VARCHAR(64) NOT NULL,
  ip_address VARCHAR(45) NOT NULL,
  user_agent TEXT NOT NULL,
  result VARCHAR(16) NOT NULL,
  details TEXT,
  created_at TIMESTAMP NOT NULL,
  CONSTRAINT fk_audit_log_user FOREIGN KEY (user_id) REFERENCES "application_user" (id) ON DELETE CASCADE,
  CONSTRAINT fk_audit_log_application FOREIGN KEY (application_id) REFERENCES "application" (id) ON DELETE CASCADE
);

CREATE INDEX idx_audit_log_user_id ON audit_log (user_id);

CREATE INDEX idx_audit_log_application_id ON audit_log (application_id);

CREATE INDEX idx_audit_log_event_type ON audit_log (event_type);

CREATE INDEX idx_audit_log_created_at ON audit_log (created_at);

---- create above / drop below ----
DROP TABLE IF EXISTS audit_log;