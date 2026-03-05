--CREATE TABLE IF NOT EXISTS mfa_totp_code (
--    id UUID PRIMARY KEY,
--    mfa_method_id UUID NOT NULL,
--    secret VARCHAR(64) NOT NULL,
--    created_at TIMESTAMP NOT NULL,
--    /* mfa_totp_code >- tenant_user = fk_user_mfa_totp_code*/
--    CONSTRAINT fk_mfa_method_mfa_totp_code FOREIGN KEY (mfa_method_id) REFERENCES "mfa_method" (id) ON DELETE CASCADE
--);
------------------------------------COMMANDS--------------------------------------
-- name: AddMfaTotpCode :exec
INSERT INTO
    mfa_totp_code (
        id,
        mfa_method_id,
        secret,
        created_at
    )
VALUES
    (
        sqlc.arg('id'),
        sqlc.arg('mfa_method_id'),
        sqlc.arg('secret'),
        sqlc.arg('created_at')
    );

-- name: UpdateMfaTotpCode :exec
UPDATE
    mfa_totp_code
SET
    mfa_method_id = sqlc.arg('mfa_method_id'),
    secret = sqlc.arg('secret'),
    created_at = sqlc.arg('created_at')
WHERE
    id = sqlc.arg('id');

-- name: DeleteMfaTotpCode :exec
DELETE FROM
    mfa_totp_code
WHERE
    id = sqlc.arg('id');

------------------------------------QUERIES--------------------------------------
-- name: GetMfaTotpCodeByID :one
SELECT
    id,
    mfa_method_id,
    secret,
    created_at
FROM
    mfa_totp_code
WHERE
    id = sqlc.arg('id');