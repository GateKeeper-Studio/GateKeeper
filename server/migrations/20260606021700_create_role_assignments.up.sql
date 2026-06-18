CREATE TABLE IF NOT EXISTS "role_assignments" (
  role_id UUID NOT NULL,
  user_id UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  PRIMARY KEY (role_id, user_id),
  CONSTRAINT fk_ra_role FOREIGN KEY (role_id) REFERENCES "roles" (id) ON DELETE CASCADE,
  CONSTRAINT fk_ra_user FOREIGN KEY (user_id) REFERENCES "users" (id) ON DELETE CASCADE
);
CREATE INDEX idx_role_assignments_user_id ON role_assignments (user_id);
