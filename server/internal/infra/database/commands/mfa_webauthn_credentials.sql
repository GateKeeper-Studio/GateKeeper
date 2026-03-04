------------------------------------COMMANDS--------------------------------------
-- name: AddMfaWebauthnCredential :exec
INSERT INTO
    mfa_webauthn_credentials (
        id,
        mfa_method_id,
        credential_id,
        public_key,
        sign_count,
        backup_eligible,
        backup_state,
        created_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('mfa_method_id'),
        sqlc.arg('credential_id'),
        sqlc.arg('public_key'),
        sqlc.arg('sign_count'),
        sqlc.arg('backup_eligible'),
        sqlc.arg('backup_state'),
        sqlc.arg('created_at')
    );

-- name: UpdateMfaWebauthnCredentialSignCount :exec
UPDATE
    mfa_webauthn_credentials
SET
    sign_count = sqlc.arg('sign_count')
WHERE
    id = sqlc.arg('id');

-- name: DeleteMfaWebauthnCredential :exec
DELETE FROM
    mfa_webauthn_credentials
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetMfaWebauthnCredentialsByMfaMethodID :many
SELECT
    id,
    mfa_method_id,
    credential_id,
    public_key,
    sign_count,
    backup_eligible,
    backup_state,
    created_at
FROM
    mfa_webauthn_credentials
WHERE
    mfa_method_id = sqlc.arg('mfa_method_id');

-- name: GetMfaWebauthnCredentialByCredentialID :one
SELECT
    id,
    mfa_method_id,
    credential_id,
    public_key,
    sign_count,
    backup_eligible,
    backup_state,
    created_at
FROM
    mfa_webauthn_credentials
WHERE
    credential_id = sqlc.arg('credential_id')
    AND mfa_method_id = sqlc.arg('mfa_method_id');