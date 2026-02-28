------------------------------------COMMANDS--------------------------------------
-- name: AddMfaWebauthnSession :exec
INSERT INTO
    mfa_webauthn_session (
        id,
        user_id,
        session_data,
        created_at,
        expires_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('user_id'),
        sqlc.arg('session_data'),
        sqlc.arg('created_at'),
        sqlc.arg('expires_at')
    );

-- name: DeleteMfaWebauthnSession :exec
DELETE FROM
    mfa_webauthn_session
WHERE
    id = sqlc.arg('id');

-- name: DeleteMfaWebauthnSessionsByUserID :exec
DELETE FROM
    mfa_webauthn_session
WHERE
    user_id = sqlc.arg('user_id');

------------------------------------QUERIES--------------------------------------
-- name: GetMfaWebauthnSessionByID :one
SELECT
    id,
    user_id,
    session_data,
    created_at,
    expires_at
FROM
    mfa_webauthn_session
WHERE
    id = sqlc.arg('id');