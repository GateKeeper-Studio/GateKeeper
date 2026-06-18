-- Optional per-client legal documents shown on the hosted login screens. When
-- set, the auth UI links to the client application's own Terms of Service /
-- Privacy Policy instead of the GateKeeper platform defaults. These mirror the
-- OIDC dynamic-client-registration metadata tos_uri / policy_uri (RFC 7591).
-- NULL on either column → fall back to the platform's /terms or /privacy page.
ALTER TABLE oauth_clients
  ADD COLUMN tos_uri TEXT NULL,
  ADD COLUMN policy_uri TEXT NULL;
