-- Write your migrate up statements here
ALTER TABLE
  "application"
ADD
  COLUMN IF NOT EXISTS requires_high_security BOOLEAN NOT NULL DEFAULT FALSE;

---- create above / drop below ----
ALTER TABLE
  "application" DROP COLUMN IF EXISTS requires_high_security;

-- Write your migrate down statements here. If this migration is irreversible
-- Then delete the separator line above.