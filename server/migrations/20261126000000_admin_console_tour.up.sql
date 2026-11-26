-- The console's guided tour: the highest tour version this admin has finished or
-- closed. The console shows the tour on first access while this is below its own
-- tour version, so the tour follows the admin across browsers and devices, and a
-- new version of the tour shows once more. 0: never seen.
ALTER TABLE admin_users ADD COLUMN console_tour_version INTEGER NOT NULL DEFAULT 0;
