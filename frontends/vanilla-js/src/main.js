import './styles.css';

const transactionKey = 'gatekeeper.public-oauth.transaction';
const tokenKey = 'gatekeeper.public-oauth.tokens';

const config = {
  idpBaseUrl: trimTrailingSlash(import.meta.env.VITE_GATEKEEPER_IDP_BASE_URL || 'http://localhost:8080'),
  authUiBaseUrl: trimTrailingSlash(
    import.meta.env.VITE_GATEKEEPER_AUTH_UI_BASE_URL || 'http://localhost:5173'
  ),
  tenantId: import.meta.env.VITE_GATEKEEPER_TENANT_ID || '',
  clientId: import.meta.env.VITE_GATEKEEPER_CLIENT_ID || '',
  redirectUri: import.meta.env.VITE_GATEKEEPER_REDIRECT_URI || window.location.origin + '/',
  scope: import.meta.env.VITE_GATEKEEPER_SCOPE || 'openid profile email'
};

const els = {
  signIn: document.querySelector('[data-action="sign-in"]'),
  signOut: document.querySelector('[data-action="sign-out"]'),
  refresh: document.querySelector('[data-action="refresh-token"]'),
  userinfo: document.querySelector('[data-action="load-userinfo"]'),
  clearLog: document.querySelector('[data-action="clear-log"]'),
  status: document.querySelector('#session-status'),
  email: document.querySelector('#session-email'),
  expiry: document.querySelector('#session-expiry'),
  config: document.querySelector('#runtime-config'),
  tokenClaims: document.querySelector('#token-claims'),
  userinfoBody: document.querySelector('#userinfo-body'),
  eventLog: document.querySelector('#event-log')
};

function trimTrailingSlash(value) {
  return String(value).replace(/\/+$/, '');
}

function randomBase64Url(byteLength = 32) {
  const bytes = new Uint8Array(byteLength);
  crypto.getRandomValues(bytes);
  return base64Url(bytes);
}

function base64Url(bytes) {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function decodeBase64Url(value) {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(value.length / 4) * 4, '=');
  return atob(padded);
}

async function codeChallengeS256(codeVerifier) {
  const bytes = new TextEncoder().encode(codeVerifier);
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return base64Url(new Uint8Array(digest));
}

function decodeJwtPayload(token) {
  try {
    const payload = token.split('.')[1];
    if (!payload) return null;
    return JSON.parse(decodeBase64Url(payload));
  } catch {
    return null;
  }
}

function loadTokens() {
  try {
    return JSON.parse(sessionStorage.getItem(tokenKey) || 'null');
  } catch {
    return null;
  }
}

function saveTokens(response) {
  const accessClaims = decodeJwtPayload(response.access_token);
  const idClaims = decodeJwtPayload(response.id_token || '') || accessClaims;
  const expiresAt = response.expires_in
    ? Math.floor(Date.now() / 1000) + Number(response.expires_in)
    : accessClaims?.exp || null;

  const tokens = {
    accessToken: response.access_token,
    idToken: response.id_token || null,
    refreshToken: response.refresh_token,
    tokenType: response.token_type,
    scope: response.scope,
    expiresAt,
    claims: idClaims || accessClaims || null
  };

  sessionStorage.setItem(tokenKey, JSON.stringify(tokens));
  return tokens;
}

function clearAuthState() {
  sessionStorage.removeItem(transactionKey);
  sessionStorage.removeItem(tokenKey);
}

function formatExpiry(expiresAt) {
  if (!expiresAt) return '-';
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short'
  }).format(new Date(expiresAt * 1000));
}

function appendLog(message) {
  const item = document.createElement('li');
  item.textContent = `${new Date().toLocaleTimeString()} - ${message}`;
  els.eventLog.prepend(item);

  while (els.eventLog.children.length > 10) {
    els.eventLog.lastElementChild?.remove();
  }
}

function renderConfig() {
  els.config.textContent = JSON.stringify(
    {
      idpBaseUrl: config.idpBaseUrl,
      authUiBaseUrl: config.authUiBaseUrl,
      tenantId: config.tenantId || '(missing)',
      clientId: config.clientId || '(missing)',
      redirectUri: config.redirectUri,
      scope: config.scope
    },
    null,
    2
  );
}

function render(tokens = loadTokens()) {
  const authenticated = Boolean(tokens?.accessToken);
  const claims = tokens?.claims || null;

  els.status.textContent = authenticated ? 'Signed in' : 'Signed out';
  els.email.textContent = claims?.email || claims?.sub || '-';
  els.expiry.textContent = formatExpiry(tokens?.expiresAt);
  els.tokenClaims.textContent = claims ? JSON.stringify(claims, null, 2) : 'No token claims loaded.';

  els.signIn.disabled = !config.tenantId || !config.clientId;
  els.signOut.disabled = !authenticated;
  els.refresh.disabled = !tokens?.refreshToken;
  els.userinfo.disabled = !tokens?.accessToken;
}

async function startSignIn() {
  if (!config.tenantId || !config.clientId) {
    appendLog('Missing tenant id or client id');
    return;
  }

  const codeVerifier = randomBase64Url(48);
  const codeChallenge = await codeChallengeS256(codeVerifier);
  const state = randomBase64Url(24);
  const nonce = randomBase64Url(24);
  const requestExpiresAt = Math.floor(Date.now() / 1000) + 1200;

  sessionStorage.setItem(
    transactionKey,
    JSON.stringify({
      state,
      nonce,
      codeVerifier,
      redirectUri: config.redirectUri,
      createdAt: Date.now()
    })
  );

  const params = new URLSearchParams({
    redirect_uri: config.redirectUri,
    response_type: 'code',
    scope: config.scope,
    state,
    nonce,
    code_challenge: codeChallenge,
    code_challenge_method: 'S256',
    request_expires_at: String(requestExpiresAt)
  });

  window.location.href = `${config.authUiBaseUrl}/auth/${config.clientId}/sign-in?${params.toString()}`;
}

async function exchangeCode(code, state) {
  const transaction = JSON.parse(sessionStorage.getItem(transactionKey) || 'null');
  if (!transaction?.codeVerifier || transaction.state !== state) {
    throw new Error('OAuth state mismatch. Start sign-in again.');
  }

  const response = await tokenRequest({
    grant_type: 'authorization_code',
    code,
    redirect_uri: transaction.redirectUri,
    code_verifier: transaction.codeVerifier
  });

  sessionStorage.removeItem(transactionKey);
  const tokens = saveTokens(response);
  appendLog('Authorization code exchanged for tokens');
  render(tokens);

  const cleanUrl = new URL(window.location.href);
  cleanUrl.searchParams.delete('code');
  cleanUrl.searchParams.delete('state');
  cleanUrl.searchParams.delete('redirect_uri');
  cleanUrl.searchParams.delete('client_id');
  window.history.replaceState({}, document.title, cleanUrl.pathname + cleanUrl.search + cleanUrl.hash);
}

async function tokenRequest(fields) {
  const body = new URLSearchParams({
    tenant_id: config.tenantId,
    client_id: config.clientId,
    ...fields
  });

  const response = await fetch(`${config.idpBaseUrl}/oauth/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body
  });

  if (!response.ok) {
    const detail = await response.text().catch(() => '');
    throw new Error(`Token endpoint ${response.status}: ${detail}`);
  }

  return response.json();
}

async function refreshTokens() {
  const tokens = loadTokens();
  if (!tokens?.refreshToken) return;

  const response = await tokenRequest({
    grant_type: 'refresh_token',
    refresh_token: tokens.refreshToken
  });
  const next = saveTokens(response);
  appendLog('Refresh token rotated');
  render(next);
}

async function loadUserInfo() {
  const tokens = loadTokens();
  if (!tokens?.accessToken) return;

  const url = `${config.idpBaseUrl}/oauth/userinfo?tenant_id=${encodeURIComponent(config.tenantId)}`;
  const response = await fetch(url, {
    headers: { Authorization: `Bearer ${tokens.accessToken}` }
  });

  if (!response.ok) {
    const detail = await response.text().catch(() => '');
    throw new Error(`UserInfo endpoint ${response.status}: ${detail}`);
  }

  const userinfo = await response.json();
  els.userinfoBody.textContent = JSON.stringify(userinfo, null, 2);
  appendLog('Loaded userinfo with bearer access token');
}

async function runAction(label, action) {
  try {
    appendLog(label);
    await action();
  } catch (error) {
    appendLog(error instanceof Error ? error.message : 'Request failed');
  } finally {
    render();
  }
}

document.addEventListener('click', (event) => {
  const button = event.target.closest('button[data-action]');
  if (!button) return;

  if (button.dataset.action === 'sign-in') {
    runAction('Starting public Authorization Code + PKCE flow', startSignIn);
  }
  if (button.dataset.action === 'sign-out') {
    clearAuthState();
    els.userinfoBody.textContent = 'No userinfo loaded.';
    appendLog('Cleared browser session storage');
    render(null);
  }
  if (button.dataset.action === 'refresh-token') {
    runAction('Refreshing tokens as public client', refreshTokens);
  }
  if (button.dataset.action === 'load-userinfo') {
    runAction('Calling /oauth/userinfo', loadUserInfo);
  }
  if (button.dataset.action === 'clear-log') {
    els.eventLog.replaceChildren();
  }
});

renderConfig();

const params = new URLSearchParams(window.location.search);
if (params.has('code')) {
  runAction('Handling OAuth callback', () => exchangeCode(params.get('code') || '', params.get('state') || ''));
} else {
  render();
}

