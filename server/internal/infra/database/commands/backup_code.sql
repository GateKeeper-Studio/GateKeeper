------------------------------------COMMANDS--------------------------------------
-- name: AddBackupCode :exec
INSERT INTO
  backup_code (
    id,
    user_id,
    code_hash,
    is_used,
    created_at,
    used_at
  )
VALUES
  (
    sqlc.arg('id'),
    sqlc.arg('user_id'),
    sqlc.arg('code_hash'),
    sqlc.arg('is_used'),
    sqlc.arg('created_at'),
    sqlc.arg('used_at')
  );

-- name: MarkBackupCodeUsed :exec
UPDATE
  backup_code
SET
  is_used = TRUE,
  used_at = NOW()
WHERE
  id = sqlc.arg('id');

-- name: DeleteBackupCodesByUserID :exec
DELETE FROM
  backup_code
WHERE
  user_id = sqlc.arg('user_id');

------------------------------------QUERIES--------------------------------------
-- name: GetUnusedBackupCodesByUserID :many
SELECT
  id,
  user_id,
  code_hash,
  is_used,
  created_at,
  used_at
FROM
  backup_code
WHERE
  user_id = sqlc.arg('user_id')
  AND is_used = FALSE;

-- name: CountUnusedBackupCodesByUserID :one
SELECT
  COUNT(*)
FROM
  backup_code
WHERE
  user_id = sqlc.arg('user_id')
  AND is_used = FALSE;