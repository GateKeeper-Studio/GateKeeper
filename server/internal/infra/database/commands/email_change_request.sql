------------------------------------COMMANDS--------------------------------------
-- name: AddEmailChangeRequest :exec
INSERT INTO
  email_change_request (
    id,
    user_id,
    application_id,
    new_email,
    token,
    created_at,
    expires_at,
    is_confirmed
  )
VALUES
  (
    sqlc.arg('id'),
    sqlc.arg('user_id'),
    sqlc.arg('application_id'),
    sqlc.arg('new_email'),
    sqlc.arg('token'),
    sqlc.arg('created_at'),
    sqlc.arg('expires_at'),
    sqlc.arg('is_confirmed')
  );

-- name: ConfirmEmailChangeRequest :exec
UPDATE
  email_change_request
SET
  is_confirmed = TRUE
WHERE
  id = sqlc.arg('id');

-- name: RevokeEmailChangeRequestsByUserID :exec
DELETE FROM
  email_change_request
WHERE
  user_id = sqlc.arg('user_id')
  AND is_confirmed = FALSE;

------------------------------------QUERIES--------------------------------------
-- name: GetEmailChangeRequestByToken :one
SELECT
  id,
  user_id,
  application_id,
  new_email,
  token,
  created_at,
  expires_at,
  is_confirmed
FROM
  email_change_request
WHERE
  token = sqlc.arg('token')
  AND is_confirmed = FALSE;