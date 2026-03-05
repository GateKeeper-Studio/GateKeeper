------------------------------------COMMANDS--------------------------------------
-- name: AddMfaPasskeyCredential :exec
INSERT INTO
    mfa_passkey_credentials (
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

-- name: UpdateMfaPasskeyCredentialSignCount :exec
UPDATE
    mfa_passkey_credentials
SET
    sign_count = sqlc.arg('sign_count')
WHERE
    id = sqlc.arg('id');

-- name: DeleteMfaPasskeyCredential :exec
DELETE FROM
    mfa_passkey_credentials
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetMfaPasskeyCredentialsByMfaMethodID :many
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
    mfa_passkey_credentials
WHERE
    mfa_method_id = sqlc.arg('mfa_method_id');

-- name: GetMfaPasskeyCredentialByCredentialID :one
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
    mfa_passkey_credentials
WHERE
    credential_id = sqlc.arg('credential_id')
    AND mfa_method_id = sqlc.arg('mfa_method_id');