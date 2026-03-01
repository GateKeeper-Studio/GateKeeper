------------------------------------COMMANDS--------------------------------------
-- name: AddRole :exec 
-- Add Role to Application
INSERT INTO
    application_role (
        id,
        application_id,
        name,
        description,
        created_at,
        updated_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('application_id'),
        sqlc.arg('name'),
        sqlc.arg('description'),
        sqlc.arg('created_at'),
        sqlc.arg('updated_at')
    );

-- name: RemoveRole :exec
-- Remove Role from Application
DELETE FROM
    application_role
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES---------------------------------------
-- List Roles from Application
-- name: ListRolesFromApplication :many
SELECT
    id,
    application_id,
    name,
    description,
    created_at,
    updated_at
FROM
    application_role
WHERE
    application_id = sqlc.arg('application_id');

-- List Roles from Application paged
-- name: ListRolesFromApplicationPaged :many
SELECT
    id,
    application_id,
    name,
    description,
    created_at,
    updated_at,
    COUNT(*) OVER () AS total_count
FROM
    application_role
WHERE
    application_id = sqlc.arg('application_id')
ORDER BY
    created_at
LIMIT
    sqlc.arg('limit') OFFSET sqlc.arg('offset');