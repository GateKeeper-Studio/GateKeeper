------------------------------------COMMANDS--------------------------------------
-- name: AddStepUpToken :exec
INSERT INTO
  step_up_token (
    id,
    user_id,
    application_id,
    token,
    created_at,
    expires_at,
    is_used
  )
VALUES
  (
    sqlc.arg('id'),
    sqlc.arg('user_id'),
    sqlc.arg('application_id'),
    sqlc.arg('token'),
    sqlc.arg('created_at'),
    sqlc.arg('expires_at'),
    sqlc.arg('is_used')
  );

-- name: MarkStepUpTokenUsed :exec
UPDATE
  step_up_token
SET
  is_used = TRUE
WHERE
  id = sqlc.arg('id');

-- name: RevokeStepUpTokensByUserID :exec
DELETE FROM
  step_up_token
WHERE
  user_id = sqlc.arg('user_id');

------------------------------------QUERIES--------------------------------------
-- name: GetStepUpTokenByToken :one
SELECT
  id,
  user_id,
  application_id,
  token,
  created_at,
  expires_at,
  is_used
FROM
  step_up_token
WHERE
  token = sqlc.arg('token')
  AND user_id = sqlc.arg('user_id');