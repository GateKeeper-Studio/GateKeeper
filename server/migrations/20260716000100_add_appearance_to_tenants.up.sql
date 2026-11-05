-- Per-organization dashboard appearance (main color + component style tokens).
-- Stored as a single JSONB blob because it is always read wholesale with the
-- tenant and will keep gaining fields; every value inside is validated in the
-- application layer before it is ever written. NULL = use the default theme.
ALTER TABLE tenants
  ADD COLUMN appearance JSONB NULL;
