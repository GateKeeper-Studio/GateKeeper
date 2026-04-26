------------------------------------COMMANDS--------------------------------------
-- name: AddTenant :exec
INSERT INTO
    tenant (
        id,
        name,
        description,
        password_hash_secret,
        created_at
    )
VALUES
    (
        sqlc.arg('id'),
        -- id
        sqlc.arg('name'),
        -- name
        sqlc.arg('description'),
        -- description
        sqlc.arg('password_hash_secret'),
        -- password_hash_secret
        sqlc.arg('created_at') -- created_at
    );

-- name: RemoveTenant :exec
DELETE FROM
    tenant
WHERE
    id = sqlc.arg('tenant_id');

-- name: UpdateTenant :exec
UPDATE
    tenant
SET
    name = sqlc.arg('name'),
    description = sqlc.arg('description'),
    password_hash_secret = sqlc.arg('password_hash_secret'),
    updated_at = sqlc.arg('updated_at')
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetTenantByID :one
SELECT
    id,
    name,
    description,
    password_hash_secret,
    created_at,
    updated_at
FROM
    "tenant"
WHERE
    id = sqlc.arg('tenant_id');

-- name: ListTenants :many
SELECT
    id,
    name,
    description,
    password_hash_secret,
    created_at,
    updated_at
FROM
    tenant
ORDER BY
    created_at DESC;