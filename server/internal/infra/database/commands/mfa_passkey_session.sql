------------------------------------COMMANDS--------------------------------------
-- name: AddMfaPasskeySession :exec
INSERT INTO
    mfa_passkey_session (
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

-- name: DeleteMfaPasskeySession :exec
DELETE FROM
    mfa_passkey_session
WHERE
    id = sqlc.arg('id');

-- name: DeleteMfaPasskeySessionsByUserID :exec
DELETE FROM
    mfa_passkey_session
WHERE
    user_id = sqlc.arg('user_id');

------------------------------------QUERIES--------------------------------------
-- name: GetMfaPasskeySessionByID :one
SELECT
    id,
    user_id,
    session_data,
    created_at,
    expires_at
FROM
    mfa_passkey_session
WHERE
    id = sqlc.arg('id');