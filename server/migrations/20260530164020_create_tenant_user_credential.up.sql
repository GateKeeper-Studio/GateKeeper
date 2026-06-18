-- Add up migration script here
CREATE TABLE IF NOT EXISTS user_credentials (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  password_algorithm VARCHAR(50) NOT NULL,
  should_change_pass BOOLEAN NOT NULL DEFAULT FALSE,
  updated_at TIMESTAMPTZ NULL,
  created_at TIMESTAMPTZ NOT NULL,
  /* user_credentials >- tenant_user = fk_user_credentials_tenant_user */
  CONSTRAINT fk_user_credentials_tenant_user FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX uq_user_credentials_user_id ON user_credentials (user_id);
