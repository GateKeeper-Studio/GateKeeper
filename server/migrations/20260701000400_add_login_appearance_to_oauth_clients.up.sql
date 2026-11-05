-- Per-client login-page appearance (layout template + validated branding tokens).
-- Stored as a single JSONB blob because it is always read wholesale with the
-- client and will keep gaining fields; every value inside is validated in the
-- application layer before it is ever written. NULL = use the default look.
ALTER TABLE oauth_clients
  ADD COLUMN login_appearance JSONB NULL;
