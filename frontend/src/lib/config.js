// Client configuration for the OAuth 2.1 / OIDC test app.
//
// SECURITY: a browser is a PUBLIC client — it cannot keep a secret. The default
// flow here is PKCE-only with NO client_secret. The optional secret field exists
// solely to exercise the confidential-client code path against a local provider;
// never ship a real client secret in a browser app.

import { browser } from '$app/environment';

const STORAGE_KEY = 'oauth_client_config';

// Values that are safe-ish to remember between reloads for developer convenience.
// The client_secret is intentionally NOT persisted.
const PERSISTED_FIELDS = ['issuer', 'tenantId', 'clientId', 'redirectUri', 'scope', 'userId'];

// The page this client is served from — used as the default redirect_uri so a real
// redirect round-trip lands somewhere that is actually served.
function selfUrl() {
  if (!browser) return '';
  return window.location.origin + window.location.pathname;
}

export function defaultConfig() {
  return {
    // Base URL of the Rust provider (the OAuth Authorization Server / OIDC OP).
    issuer: 'http://localhost:8080',
    // Multi-tenant discriminator — this provider scopes everything by tenant.
    tenantId: '',
    // The registered OAuth client (relying party) id.
    clientId: '',
    // Public clients leave this empty (PKCE is the only proof).
    clientSecret: '',
    // MUST exactly match a redirect_uri registered for the client. Defaults to
    // this very page.
    redirectUri: selfUrl(),
    // Space-separated OAuth scopes. "openid" is required to get an id_token.
    scope: 'openid profile email',
    // The subject to issue the grant for. NOTE: the provider's /oauth/login use
    // case is not implemented yet, so we pass the user id directly here.
    userId: ''
  };
}

export function loadConfig() {
  const cfg = defaultConfig();
  if (!browser) return cfg;
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
    for (const field of PERSISTED_FIELDS) {
      if (typeof saved[field] === 'string') cfg[field] = saved[field];
    }
  } catch {
    // Corrupt storage — fall back to defaults.
  }
  return cfg;
}

export function saveConfig(cfg) {
  if (!browser) return;
  const toPersist = {};
  for (const field of PERSISTED_FIELDS) toPersist[field] = cfg[field] ?? '';
  localStorage.setItem(STORAGE_KEY, JSON.stringify(toPersist));
}

// Normalizes the issuer to a base URL without a trailing slash.
export function issuerBase(cfg) {
  return cfg.issuer.replace(/\/+$/, '');
}
