-- Reserved role names (privilege escalation).
--
-- A user's roles are resolved from tenant RBAC and embedded in their access
-- token. The admin router authorizes on role NAME (`ADMIN_ROLE_NAMES` =
-- 'admin', 'tenant_admin'), so a tenant that created a role called "admin" for
-- its own application handed that role's members the platform admin surface.
--
-- `create_role` / `update_role_by_id` now reject these names. This migration
-- covers rows that already exist.
--
-- It deliberately ABORTS rather than auto-renaming: a colliding role is an
-- active privilege escalation, and silently renaming it would change how the
-- tenant's own application reads roles out of its tokens. An operator has to
-- decide. The exception message carries the affected rows and the remediation.

DO $$
DECLARE
  offending TEXT;
  offending_count INT;
BEGIN
  SELECT count(*), string_agg(format('tenant %s: role %s (%L)', tenant_id, id, name), E'\n    ')
    INTO offending_count, offending
    FROM roles
   WHERE lower(btrim(name)) IN ('admin', 'tenant_admin');

  IF offending_count > 0 THEN
    RAISE EXCEPTION
      'Found % role(s) whose name grants the platform admin surface:%',
      offending_count, E'\n    ' || offending
      USING HINT =
        'Rename each one, then re-run the migration. For example: '
        'UPDATE roles SET name = name || ''_app'' WHERE lower(btrim(name)) IN (''admin'', ''tenant_admin''); '
        'Review which users held them first — they had cross-tenant admin access.';
  END IF;
END
$$;

-- Enforce it in the database as well, so the rule cannot be bypassed by a future
-- code path (bulk import, manual SQL) that skips the use case.
ALTER TABLE roles
  ADD CONSTRAINT ck_roles_name_not_reserved
  CHECK (lower(btrim(name)) NOT IN ('admin', 'tenant_admin'));
