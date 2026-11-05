import { createNextGateKeeper } from '@gatekeeper/nextjs';

// All values are server-only (never NEXT_PUBLIC_*). The client secret stays here.
export const gatekeeper = createNextGateKeeper({
  idpBaseUrl: process.env.IDP_BASE_URL ?? 'http://localhost:8080',
  authUiBaseUrl: process.env.AUTH_UI_BASE_URL ?? 'http://localhost:5173',
  tenantId: process.env.OAUTH_TENANT_ID ?? '',
  clientId: process.env.OAUTH_CLIENT_ID ?? '',
  clientSecret: process.env.OAUTH_CLIENT_SECRET,
  redirectUri: process.env.OAUTH_REDIRECT_URI ?? 'http://localhost:5179/auth/callback',
  scope: process.env.OAUTH_SCOPE ?? 'openid profile email account',
  sessionSecret: process.env.SESSION_SECRET ?? '',
  secureCookies: process.env.NODE_ENV === 'production'
});
