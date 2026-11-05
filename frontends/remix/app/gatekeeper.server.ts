import { createRemixGateKeeper } from '@gatekeeper/remix';

// Server-only module (the `.server` naming keeps it out of the client bundle too).
export const gatekeeper = createRemixGateKeeper({
  idpBaseUrl: process.env.IDP_BASE_URL ?? 'http://localhost:8080',
  authUiBaseUrl: process.env.AUTH_UI_BASE_URL ?? 'http://localhost:5173',
  tenantId: process.env.OAUTH_TENANT_ID ?? '',
  clientId: process.env.OAUTH_CLIENT_ID ?? '',
  clientSecret: process.env.OAUTH_CLIENT_SECRET,
  redirectUri: process.env.OAUTH_REDIRECT_URI ?? 'http://localhost:5180/auth/callback',
  scope: process.env.OAUTH_SCOPE ?? 'openid profile email',
  sessionSecret: process.env.SESSION_SECRET ?? '',
  secureCookies: process.env.NODE_ENV === 'production'
});
