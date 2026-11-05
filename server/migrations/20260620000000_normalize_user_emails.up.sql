-- Treat email case-insensitively: merge any case-variant duplicate accounts into
-- one user, lowercase all stored emails, and enforce a case-insensitive unique
-- index. This fixes accounts that were duplicated when a social login (e.g.
-- GitHub) returned the same address in a different case than the native account.
--
-- Safe because every table that references users(id) does so ON DELETE CASCADE,
-- so deleting a duplicate cleans up its dependents. The only thing worth keeping
-- from a duplicate is its social-identity link, which we re-point first.

-- The case-sensitive unique index must go before we can lowercase colliding rows.
DROP INDEX IF EXISTS uq_users_tenant_email;

-- Rank users sharing a (tenant, lower(email)); the primary keeps the account.
-- Prefer the one that has a password credential, then the earliest created.
WITH ranked AS (
  SELECT u.id,
         u.tenant_id,
         lower(u.email) AS lemail,
         row_number() OVER (
           PARTITION BY u.tenant_id, lower(u.email)
           ORDER BY (EXISTS (SELECT 1 FROM user_credentials c WHERE c.user_id = u.id)) DESC,
                    u.created_at ASC
         ) AS rn
  FROM users u
),
primaries AS (
  SELECT tenant_id, lemail, id AS primary_id FROM ranked WHERE rn = 1
),
dups AS (
  SELECT r.id AS dup_id, p.primary_id
  FROM ranked r
  JOIN primaries p ON r.tenant_id = p.tenant_id AND r.lemail = p.lemail
  WHERE r.rn > 1
)
-- Re-point the social identities of duplicates onto the surviving primary user.
UPDATE social_identities s
SET user_id = d.primary_id
FROM dups d
WHERE s.user_id = d.dup_id;

-- Delete the now-redundant duplicate users; CASCADE removes their other rows.
WITH ranked AS (
  SELECT u.id,
         u.tenant_id,
         lower(u.email) AS lemail,
         row_number() OVER (
           PARTITION BY u.tenant_id, lower(u.email)
           ORDER BY (EXISTS (SELECT 1 FROM user_credentials c WHERE c.user_id = u.id)) DESC,
                    u.created_at ASC
         ) AS rn
  FROM users u
)
DELETE FROM users WHERE id IN (SELECT id FROM ranked WHERE rn > 1);

-- Canonicalize the surviving emails to lowercase.
UPDATE users SET email = lower(email) WHERE email <> lower(email);

-- Enforce case-insensitive uniqueness going forward.
CREATE UNIQUE INDEX uq_users_tenant_lower_email ON users (tenant_id, lower(email));
