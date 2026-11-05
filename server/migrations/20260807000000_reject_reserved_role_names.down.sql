-- Drops the reserved-name constraint. The data check in the up migration has no
-- inverse (it only aborted; it never modified rows).
ALTER TABLE roles DROP CONSTRAINT IF EXISTS ck_roles_name_not_reserved;
