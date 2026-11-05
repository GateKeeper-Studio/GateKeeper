-- An idle timeout alone is open-ended: a client that keeps calling the refresh
-- endpoint could hold a pre-authorization session open forever. The absolute
-- ceiling bounds total lifetime no matter how many times the idle window slides.
--
-- The DEFAULT exists only to backfill rows that are already in flight; new rows
-- always carry an explicit value from the domain, so it is dropped immediately.
ALTER TABLE auth_sessions
  ADD COLUMN absolute_expires_at TIMESTAMPTZ NOT NULL DEFAULT (now() + interval '30 minutes');

ALTER TABLE auth_sessions
  ALTER COLUMN absolute_expires_at DROP DEFAULT;
