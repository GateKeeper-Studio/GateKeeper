-- Write your migrate up statements here
CREATE TABLE IF NOT EXISTS backup_code (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  code_hash VARCHAR(64) NOT NULL,
  is_used BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMP NOT NULL,
  used_at TIMESTAMP,
  CONSTRAINT fk_backup_code_user FOREIGN KEY (user_id) REFERENCES "tenant_user" (id) ON DELETE CASCADE
);

CREATE INDEX idx_backup_code_user_id ON backup_code (user_id);

---- create above / drop below ----
DROP TABLE IF EXISTS backup_code;