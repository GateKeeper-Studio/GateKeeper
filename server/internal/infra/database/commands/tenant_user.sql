------------------------------------COMMANDS--------------------------------------
-- name: AddUser :exec
-- Add user
INSERT INTO
    "tenant_user" (
        id,
        email,
        tenant_id,
        created_at,
        updated_at,
        is_active,
        is_email_confirmed,
        preferred_2fa_method
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('email'),
        sqlc.arg('tenant_id'),
        sqlc.arg('created_at'),
        sqlc.narg('updated_at'),
        sqlc.arg('is_active'),
        sqlc.arg('is_email_confirmed'),
        sqlc.arg('preferred_2fa_method')
    );

-- name: UpdateUser :exec
-- Update user
UPDATE
    "tenant_user"
SET
    email = sqlc.arg('email'),
    updated_at = sqlc.arg('updated_at'),
    is_active = sqlc.arg('is_active'),
    is_email_confirmed = sqlc.arg('is_email_confirmed'),
    preferred_2fa_method = sqlc.arg('preferred_2fa_method')
WHERE
    id = sqlc.arg('id');

-- name: DeleteTenantUser :exec
DELETE FROM
    "tenant_user"
WHERE
    id = sqlc.arg('id')
    AND tenant_id = sqlc.arg('tenant_id');

------------------------------------QUERIES--------------------------------------
-- name: GetUserById :one
-- Get user by id
SELECT
    id,
    email,
    tenant_id,
    created_at,
    updated_at,
    is_active,
    is_email_confirmed,
    preferred_2fa_method
FROM
    "tenant_user"
WHERE
    id = sqlc.arg('id');

-- name: GetUserByEmail :one
-- Get user by email
SELECT
    id,
    email,
    tenant_id,
    created_at,
    updated_at,
    is_active,
    is_email_confirmed,
    preferred_2fa_method
FROM
    "tenant_user"
WHERE
    email = sqlc.arg('email')
    AND tenant_id = sqlc.arg('tenant_id');

-- name: IsUserExistsByEmail :one
-- Check if user exists by email
SELECT
    EXISTS (
        SELECT
            1
        FROM
            "tenant_user"
        WHERE
            email = sqlc.arg('email')
            AND tenant_id = sqlc.arg('tenant_id')
    );

-- name: IsUserExistsById :one
-- Check if user exists by id
SELECT
    EXISTS (
        SELECT
            1
        FROM
            "tenant_user"
        WHERE
            id = sqlc.arg('id')
    );

-- name: GetUsersByTenantID :many
-- Get users by tenant id paged, ordered by created_at, that includes the application roles
SELECT
    au.id,
    au.email,
    au.tenant_id,
    up.display_name,
    au.created_at,
    au.updated_at,
    au.is_active,
    au.is_email_confirmed,
    COALESCE(r.roles, '[]' :: jsonb) AS roles,
    COUNT(*) OVER () AS total_users
FROM
    "tenant_user" au
    LEFT JOIN "user_profile" up ON up.user_id = au.id
    LEFT JOIN LATERAL (
        SELECT
            jsonb_agg(
                jsonb_build_object(
                    'id',
                    ar.id,
                    'name',
                    ar.name,
                    'description',
                    ar.description
                )
            ) AS roles
        FROM
            "user_role" ur
            JOIN "application_role" ar ON ar.id = ur.role_id
        WHERE
            ur.user_id = au.id
    ) r ON TRUE
WHERE
    au.tenant_id = sqlc.arg('tenant_id')
ORDER BY
    au.created_at
LIMIT
    sqlc.arg('limit') OFFSET sqlc.arg('offset');