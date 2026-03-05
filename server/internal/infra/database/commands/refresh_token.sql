------------------------------------COMMANDS--------------------------------------
-- name: AddRefreshToken :exec
INSERT INTO
    refresh_token (
        id,
        user_id,
        expires_at,
        created_at
    )
VALUES
    (
        sqlc.arg('id'),
        -- id
        sqlc.arg('user_id'),
        -- user_id
        sqlc.arg('expires_at'),
        -- expires_at
        sqlc.arg('created_at') -- created_at
    );

-- name: RevokeRefreshTokenFromUser :exec
DELETE FROM
    refresh_token
WHERE
    user_id = sqlc.arg('user_id');

-- name: RevokeRefreshTokenByID :exec
DELETE FROM
    refresh_token
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetRefreshTokensFromUser :many
SELECT
    id,
    user_id,
    expires_at,
    created_at
FROM
    refresh_token
WHERE
    user_id = sqlc.arg('user_id');

-- name: GetRefreshTokensByApplicationUser :many
SELECT
    rt.id,
    rt.user_id,
    rt.expires_at,
    rt.created_at
FROM
    refresh_token rt
    INNER JOIN tenant_user au ON au.id = rt.user_id
WHERE
    au.id = sqlc.arg('user_id')
    AND au.organization_id = sqlc.arg('organization_id')
ORDER BY
    rt.created_at DESC;