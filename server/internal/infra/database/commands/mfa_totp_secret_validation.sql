------------------------------------COMMANDS--------------------------------------
-- name: AddMfaTotpSecretValidation :exec
INSERT INTO
    mfa_totp_secret_validation (
        id,
        user_id,
        secret,
        is_validated,
        created_at,
        expires_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('user_id'),
        sqlc.arg('secret'),
        sqlc.arg('is_validated'),
        sqlc.arg('created_at'),
        sqlc.arg('expires_at')
    );

-- name: RevokeMfaTotpSecretValidationFromUser :exec
DELETE FROM
    mfa_totp_secret_validation
WHERE
    user_id = sqlc.arg('user_id');

-- name: DeleteExpiredMfaTotpSecretValidationByUserID :exec
DELETE FROM
    mfa_totp_secret_validation
WHERE
    user_id = sqlc.arg('user_id')
    AND expires_at < NOW()
    AND is_validated = false;

-- name: UpdateMfaTotpSecretValidation :exec
UPDATE
    mfa_totp_secret_validation
SET
    secret = sqlc.arg('secret'),
    is_validated = sqlc.arg('is_validated'),
    created_at = sqlc.arg('created_at'),
    expires_at = sqlc.arg('expires_at')
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetMfaTotpSecretValidationByUserId :one
SELECT
    id,
    user_id,
    secret,
    is_validated,
    created_at,
    expires_at
FROM
    mfa_totp_secret_validation
WHERE
    user_id = sqlc.arg('user_id');

-- name: GetLastValidMfaTotpSecretByUserID :one
SELECT
    id,
    user_id,
    secret,
    is_validated,
    created_at,
    expires_at
FROM
    mfa_totp_secret_validation
WHERE
    user_id = sqlc.arg('user_id')
    AND is_validated = false
    AND expires_at > NOW()
ORDER BY
    created_at DESC
LIMIT
    1;