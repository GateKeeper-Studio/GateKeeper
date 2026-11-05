-- OIDC `initiate_login_uri` (Dynamic Client Registration): the RP endpoint that
-- starts a fresh authorization request. The login SPA sends the browser here when
-- the OAuth state/PKCE it was handed has expired — only the RP holds the PKCE
-- code_verifier, so only the RP can mint a new one.
--
-- It lives in the client registration, never in a query parameter: that is what
-- keeps "restart my sign-in" from being an open redirect.
ALTER TABLE oauth_clients
  ADD COLUMN initiate_login_uri TEXT NULL;
