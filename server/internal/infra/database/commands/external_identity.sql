------------------------------------COMMANDS--------------------------------------
-- name: AddExternalIdentity :exec
INSERT INTO
    external_identity (
        id,
        user_id,
        email,
        provider,
        provider_user_id,
        application_oauth_provider_id,
        created_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('user_id'),
        sqlc.arg('email'),
        sqlc.arg('provider'),
        sqlc.arg('provider_user_id'),
        sqlc.arg('application_oauth_provider_id'),
        sqlc.arg('created_at')
    );

------------------------------------QUERIES--------------------------------------
-- name: GetExternalIdentityByProviderUserId :one
SELECT
    id,
    user_id,
    email,
    provider,
    provider_user_id,
    application_oauth_provider_id,
    created_at,
    updated_at
FROM
    external_identity
WHERE
    provider = sqlc.arg('provider')
    AND provider_user_id = sqlc.arg('provider_user_id');

-- name: GetExternalIdentitiesByUserID :many
SELECT
    id,
    user_id,
    email,
    provider,
    provider_user_id,
    application_oauth_provider_id,
    created_at,
    updated_at
FROM
    external_identity
WHERE
    user_id = sqlc.arg('user_id');