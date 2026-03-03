-- Write your migrate up statements here
ALTER TABLE
  "application"
ADD
  COLUMN IF NOT EXISTS refresh_token_ttl_days INT NOT NULL DEFAULT 7;

---- create above / drop below ----
ALTER TABLE
  "application" DROP COLUMN IF EXISTS refresh_token_ttl_days;