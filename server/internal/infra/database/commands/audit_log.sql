------------------------------------COMMANDS--------------------------------------
-- name: AddAuditLog :exec
INSERT INTO
  audit_log (
    id,
    user_id,
    application_id,
    event_type,
    ip_address,
    user_agent,
    result,
    details,
    created_at
  )
VALUES
  (
    sqlc.arg('id'),
    sqlc.arg('user_id'),
    sqlc.arg('application_id'),
    sqlc.arg('event_type'),
    sqlc.arg('ip_address'),
    sqlc.arg('user_agent'),
    sqlc.arg('result'),
    sqlc.arg('details'),
    sqlc.arg('created_at')
  );

------------------------------------QUERIES--------------------------------------
-- name: GetAuditLogsByUserID :many
SELECT
  id,
  user_id,
  application_id,
  event_type,
  ip_address,
  user_agent,
  result,
  details,
  created_at
FROM
  audit_log
WHERE
  user_id = sqlc.arg('user_id')
ORDER BY
  created_at DESC
LIMIT
  sqlc.arg('limit_count') OFFSET sqlc.arg('offset_count');