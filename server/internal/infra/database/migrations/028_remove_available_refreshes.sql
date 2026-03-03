-- Write your migrate up statements here
ALTER TABLE
  "refresh_token" DROP COLUMN IF EXISTS available_refreshes;

---- create above / drop below ----
ALTER TABLE
  "refresh_token"
ADD
  COLUMN IF NOT EXISTS available_refreshes INT NOT NULL DEFAULT 5;