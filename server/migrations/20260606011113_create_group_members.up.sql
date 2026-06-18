-- Join table for the many-to-many membership between groups and users.
CREATE TABLE IF NOT EXISTS "group_members" (
  group_id UUID NOT NULL,
  user_id  UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,

  PRIMARY KEY (group_id, user_id),
  CONSTRAINT fk_gm_group FOREIGN KEY (group_id) REFERENCES "groups" (id) ON DELETE CASCADE,
  CONSTRAINT fk_gm_user  FOREIGN KEY (user_id)  REFERENCES "users" (id)  ON DELETE CASCADE
);

-- Reverse lookup: which groups a given user belongs to.
CREATE INDEX idx_group_members_user_id ON group_members (user_id);
