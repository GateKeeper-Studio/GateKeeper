-- audit_events is append-only. Until now that was a comment; this makes it a rule
-- the database enforces for every role, including the application's own.
--
-- UPDATE is refused outright: an audit row is never edited. The one sanctioned
-- rewrite is GDPR/LGPD erasure, which pseudonymises the actor of a deleted user;
-- it runs with `SET LOCAL gatekeeper.audit_erasure = 'on'` inside its transaction.
--
-- DELETE stays possible for the retention purge (rows past their retention window).
-- To also forbid it for the runtime role, run as the owner:
--   REVOKE DELETE ON audit_events FROM <runtime_role>;
-- and let a separate retention job run with a role that keeps it.
CREATE OR REPLACE FUNCTION audit_events_reject_update() RETURNS trigger AS $$
BEGIN
    IF current_setting('gatekeeper.audit_erasure', true) = 'on' THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'audit_events is append-only (UPDATE refused)'
        USING ERRCODE = 'insufficient_privilege';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_events_append_only
    BEFORE UPDATE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION audit_events_reject_update();
