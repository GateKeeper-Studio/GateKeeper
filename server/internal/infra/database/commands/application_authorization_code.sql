------------------------------------COMMANDS--------------------------------------
-- name: AddAuthorizationCode :exec 
-- Add Authorization Code to Application
INSERT INTO
    application_authorization_code (
        id,
        application_id,
        user_id,
        expired_at,
        code,
        redirect_uri,
        code_challenge,
        code_challenge_method,
        nonce,
        scope
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('application_id'),
        sqlc.arg('user_id'),
        sqlc.arg('expired_at'),
        sqlc.arg('code'),
        sqlc.arg('redirect_uri'),
        sqlc.arg('code_challenge'),
        sqlc.arg('code_challenge_method'),
        sqlc.arg('nonce'),
        sqlc.arg('scope')
    );

-- name: RemoveAuthorizationCode :exec
-- Remove Authorization Code from Application
DELETE FROM
    application_authorization_code
WHERE
    application_id = sqlc.arg('application_id')
    AND user_id = sqlc.arg('user_id');

------------------------------------QUERIES---------------------------------------
-- name: GetAuthorizationCodeById :one
-- List Authorization Codes by Application Id
SELECT
    id,
    application_id,
    user_id,
    expired_at,
    code,
    redirect_uri,
    code_challenge,
    code_challenge_method,
    nonce,
    scope
FROM
    application_authorization_code
WHERE
    id = sqlc.arg('id');