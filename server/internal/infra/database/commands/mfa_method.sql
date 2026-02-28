-- CREATE TABLE IF NOT EXISTS mfa_method (
--    id UUID PRIMARY KEY,
--    user_id UUID NOT NULL,
--    "type" VARCHAR(16) NOT NULL,
--    enabled BOOLEAN NOT NULL DEFAULT TRUE,
--    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
--    last_used_at TIMESTAMP NULL,
--    /* mfa_method >- application_user = fk_user_mfa_method */
--    CONSTRAINT fk_user_mfa_method FOREIGN KEY (user_id) REFERENCES "application_user" (id) ON DELETE CASCADE
-- );
------------------------------------COMMANDS--------------------------------------
-- name: AddMfaMethod :exec
INSERT INTO
    mfa_method (
        id,
        user_id,
        "type",
        enabled,
        created_at,
        last_used_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('user_id'),
        sqlc.arg('type'),
        sqlc.arg('enabled'),
        sqlc.arg('created_at'),
        sqlc.arg('last_used_at')
    );

-- name: EnableMfaMethod :exec
UPDATE
    mfa_method
SET
    enabled = true
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetMfaMethodByUserIDAndMethod :one
SELECT
    id,
    user_id,
    "type",
    enabled,
    created_at,
    last_used_at
FROM
    mfa_method
WHERE
    user_id = sqlc.arg('user_id')
    AND "type" = sqlc.arg('type');

-- name: GetUserMfaMethods :many
SELECT
    id,
    user_id,
    "type",
    enabled,
    created_at,
    last_used_at
FROM
    mfa_method
WHERE
    user_id = sqlc.arg('user_id');