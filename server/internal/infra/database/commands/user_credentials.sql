------------------------------------COMMANDS--------------------------------------
-- name: AddUserCredentials :exec
INSERT INTO
    user_credentials (
        id,
        user_id,
        password_hash,
        password_algorithm,
        should_change_pass,
        created_at,
        updated_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('user_id'),
        sqlc.arg('password_hash'),
        sqlc.arg('password_algorithm'),
        sqlc.arg('should_change_pass'),
        sqlc.arg('created_at'),
        sqlc.arg('updated_at')
    );

-- name: RemoveUserCredentials :exec
DELETE FROM
    user_credentials
WHERE
    user_id = sqlc.arg('user_id');

-- name: UpdateUserCredentials :exec
UPDATE
    user_credentials
SET
    password_hash = sqlc.arg('password_hash'),
    password_algorithm = sqlc.arg('password_algorithm'),
    should_change_pass = sqlc.arg('should_change_pass'),
    updated_at = sqlc.arg('updated_at')
WHERE
    user_id = sqlc.arg('user_id');

------------------------------------QUERIES--------------------------------------
-- name: GetUserCredentialsByUserID :one
SELECT
    id,
    user_id,
    password_hash,
    password_algorithm,
    should_change_pass,
    created_at,
    updated_at
FROM
    user_credentials
WHERE
    user_id = sqlc.arg('user_id');