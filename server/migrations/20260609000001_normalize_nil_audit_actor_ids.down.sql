-- Irreversible: once the nil-UUID sentinels are collapsed to NULL they are
-- indistinguishable from rows that were always legitimately actor-less, so there is
-- nothing safe to restore. The audit log is append-only; this down step is a no-op.
SELECT 1;
