------------------------------------COMMANDS--------------------------------------
-- name: AddApplication :exec
INSERT INTO
    "application" (
        id,
        organization_id,
        name,
        is_active,
        has_mfa_auth_app,
        has_mfa_email,
        has_mfa_webauthn,
        password_hash_secret,
        badges,
        description,
        created_at,
        updated_at,
        can_self_sign_up,
        can_self_forgot_pass
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('organization_id'),
        sqlc.arg('name'),
        sqlc.arg('is_active'),
        sqlc.arg('has_mfa_auth_app'),
        sqlc.arg('has_mfa_email'),
        sqlc.arg('has_mfa_webauthn'),
        sqlc.arg('password_hash_secret'),
        sqlc.narg('badges'),
        sqlc.narg('description'),
        sqlc.arg('created_at'),
        sqlc.arg('updated_at'),
        sqlc.arg('can_self_sign_up'),
        sqlc.arg('can_self_forgot_pass')
    );

-- name: UpdateApplication :exec
UPDATE
    "application"
SET
    name = sqlc.arg('name'),
    description = sqlc.narg('description'),
    has_mfa_auth_app = sqlc.arg('has_mfa_auth_app'),
    has_mfa_webauthn = sqlc.arg('has_mfa_webauthn'),
    badges = sqlc.narg('badges'),
    is_active = sqlc.arg('is_active'),
    has_mfa_email = sqlc.arg('has_mfa_email'),
    updated_at = sqlc.arg('updated_at'),
    can_self_sign_up = sqlc.arg('can_self_sign_up'),
    can_self_forgot_pass = sqlc.arg('can_self_forgot_pass'),
    refresh_token_ttl_days = sqlc.arg('refresh_token_ttl_days')
WHERE
    id = sqlc.arg('id');

-- name: DeleteApplication :exec
DELETE FROM
    "application"
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: CheckIfApplicationExists :one
SELECT
    EXISTS (
        SELECT
            1
        FROM
            "application"
        WHERE
            id = sqlc.arg('id')
    );

-- name: GetApplicationByID :one
SELECT
    id,
    organization_id,
    name,
    description,
    badges,
    is_active,
    has_mfa_auth_app,
    has_mfa_email,
    has_mfa_webauthn,
    password_hash_secret,
    created_at,
    updated_at,
    can_self_sign_up,
    can_self_forgot_pass,
    refresh_token_ttl_days
FROM
    "application"
WHERE
    id = sqlc.arg('id');

-- name: ListApplicationsFromOrganization :many
SELECT
    id,
    organization_id,
    name,
    description,
    badges,
    has_mfa_webauthn,
    created_at,
    updated_at
FROM
    "application"
WHERE
    organization_id = sqlc.arg('organization_id');