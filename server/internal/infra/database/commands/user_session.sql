------------------------------------COMMANDS--------------------------------------
-- name: AddUserSession :exec
INSERT INTO
  user_session (
    id,
    user_id,
    application_id,
    ip_address,
    user_agent,
    location,
    created_at,
    last_active_at,
    expires_at,
    is_revoked
  )
VALUES
  (
    sqlc.arg('id'),
    sqlc.arg('user_id'),
    sqlc.arg('application_id'),
    sqlc.arg('ip_address'),
    sqlc.arg('user_agent'),
    sqlc.arg('location'),
    sqlc.arg('created_at'),
    sqlc.arg('last_active_at'),
    sqlc.arg('expires_at'),
    sqlc.arg('is_revoked')
  );

-- name: RevokeUserSessionByID :exec
UPDATE
  user_session
SET
  is_revoked = TRUE
WHERE
  id = sqlc.arg('id')
  AND user_id = sqlc.arg('user_id');

-- name: RevokeAllUserSessions :exec
UPDATE
  user_session
SET
  is_revoked = TRUE
WHERE
  user_id = sqlc.arg('user_id')
  AND is_revoked = FALSE;

-- name: UpdateUserSessionLastActive :exec
UPDATE
  user_session
SET
  last_active_at = NOW()
WHERE
  id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetActiveUserSessions :many
SELECT
  id,
  user_id,
  application_id,
  ip_address,
  user_agent,
  location,
  created_at,
  last_active_at,
  expires_at,
  is_revoked
FROM
  user_session
WHERE
  user_id = sqlc.arg('user_id')
  AND is_revoked = FALSE
  AND expires_at > NOW()
ORDER BY
  last_active_at DESC;

-- name: GetUserSessionByID :one
SELECT
  id,
  user_id,
  application_id,
  ip_address,
  user_agent,
  location,
  created_at,
  last_active_at,
  expires_at,
  is_revoked
FROM
  user_session
WHERE
  id = sqlc.arg('id')
  AND user_id = sqlc.arg('user_id');