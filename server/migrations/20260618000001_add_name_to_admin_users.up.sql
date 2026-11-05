-- Platform admins gain a human display name (shown in the dashboard sidebar and the
-- account page). Existing rows default to '' so the column can be NOT NULL without a
-- backfill; admins set their name from the account page.
ALTER TABLE "admin_users" ADD COLUMN name VARCHAR(128) NOT NULL DEFAULT '';
