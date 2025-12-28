------------------------------------COMMANDS--------------------------------------
-- name: AddExternalOAuthState :exec
INSERT INTO
  external_oauth_state (
    id,
    provider_state,
    application_oauth_provider_id,
    client_state,
    client_code_challenge_method,
    client_code_challenge,
    client_scope,
    code_verifier,
    client_response_type,
    client_redirect_uri,
    created_at
  )
VALUES
  (
    sqlc.arg('id'),
    -- id
    sqlc.arg('provider_state'),
    -- state
    sqlc.arg('application_oauth_provider_id'),
    -- application_oauth_provider_id
    sqlc.arg('client_state'),
    -- client_state
    sqlc.arg('client_code_challenge_method'),
    -- client_code_challenge_method
    sqlc.arg('client_code_challenge'),
    -- client_code_challenge
    sqlc.arg('client_scope'),
    -- client_scope
    sqlc.arg('code_verifier'),
    -- code_verifier
    sqlc.arg('client_response_type'),
    -- client_response_type
    sqlc.arg('client_redirect_uri'),
    -- client_redirect_uri
    sqlc.arg('created_at') -- created_at
  );

------------------------------------QUERIES--------------------------------------
-- name: GetExternalOAuthStateByState :one
SELECT
  id,
  provider_state,
  application_oauth_provider_id,
  client_state,
  client_code_challenge_method,
  client_code_challenge,
  client_scope,
  code_verifier,
  client_response_type,
  client_redirect_uri,
  created_at
FROM
  external_oauth_state
WHERE
  provider_state = sqlc.arg('provider_state');