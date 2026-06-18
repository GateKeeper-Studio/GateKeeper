-- One-off data normalization: earlier rows recorded an actor-less event (the
-- synthetic `system()` principal, written while REQUIRE_ADMIN_AUTH was off) with the
-- nil-UUID sentinel instead of a real NULL. `actor_id` is semantically nullable —
-- "no authenticated actor" must be NULL, never 00000000-…-000000000000 — so collapse
-- the sentinel to NULL. New writes already do the right thing (AuthPrincipal::actor_id).
UPDATE audit_events
SET actor_id = NULL
WHERE actor_id = '00000000-0000-0000-0000-000000000000';
