-- Every admin then sees the tour again once per browser (the console also keeps a
-- local copy of the version seen).
ALTER TABLE admin_users DROP COLUMN console_tour_version;
