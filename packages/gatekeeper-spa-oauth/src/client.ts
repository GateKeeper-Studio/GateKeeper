// The framework-agnostic public SPA client: a small subscribable state machine
// wrapping the PKCE + token logic. Every framework adapter (React/Vue/Angular/
// AngularJS) binds its reactivity to `subscribe` / `getSnapshot`.

import { codeChallengeS256, generatePkce } from './pkce.js';
import {
  buildAuthorizeUrl,
  exchangeCode,
  fetchUserInfo,
  profileFromClaims,
  refreshTokens,
  tokensFromResponse,
  trimTrailingSlash
} from './oauth.js';
import type {
  GateKeeperProfile,
  GateKeeperSpaClient,
  GateKeeperSpaListener,
  GateKeeperSpaSnapshot,
  GateKeeperSpaStatus,
  GateKeeperSpaConfig,
  JwtClaims,
  PkceTransaction,
  ResolvedSpaConfig,
  SignInOptions,
  StorageLike,
  StoredTokens
} from './types.js';

const DEFAULT_STORAGE_PREFIX = 'gatekeeper.spa-oauth';
const DEFAULT_AUTH_REQUEST_TTL_SECONDS = 1200;
/** Refresh this many seconds before the access token actually expires. */
const REFRESH_SKEW_SECONDS = 60;

function memoryStorage(): StorageLike {
  const map = new Map<string, string>();
  return {
    getItem: (key) => (map.has(key) ? (map.get(key) as string) : null),
    setItem: (key, value) => void map.set(key, value),
    removeItem: (key) => void map.delete(key)
  };
}

function resolveConfig(config: GateKeeperSpaConfig): ResolvedSpaConfig {
  const defaultRedirect =
    typeof window !== 'undefined' && window.location ? `${window.location.origin}/` : '';

  const cfg: ResolvedSpaConfig = {
    idpBaseUrl: trimTrailingSlash(config.idpBaseUrl ?? ''),
    authUiBaseUrl: trimTrailingSlash(config.authUiBaseUrl ?? ''),
    tenantId: config.tenantId ?? '',
    clientId: config.clientId ?? '',
    redirectUri: config.redirectUri ?? defaultRedirect,
    scope: config.scope || 'openid profile email',
    authRequestTtlSeconds: config.authRequestTtlSeconds ?? DEFAULT_AUTH_REQUEST_TTL_SECONDS
  };

  const missing = (['idpBaseUrl', 'authUiBaseUrl', 'tenantId', 'clientId', 'redirectUri'] as const).filter(
    (key) => !cfg[key]
  );
  if (missing.length) {
    throw new Error(`Missing GateKeeper SPA config: ${missing.join(', ')}`);
  }

  return cfg;
}

function resolveStorage(config: GateKeeperSpaConfig): StorageLike {
  if (config.storage) return config.storage;
  if (typeof window !== 'undefined' && window.sessionStorage) return window.sessionStorage;
  return memoryStorage();
}

export function createGateKeeperSpaClient(config: GateKeeperSpaConfig): GateKeeperSpaClient {
  const cfg = resolveConfig(config);
  const storage = resolveStorage(config);
  const prefix = config.storageKeyPrefix || DEFAULT_STORAGE_PREFIX;
  const tokenKey = `${prefix}.tokens`;
  const transactionKey = `${prefix}.transaction`;

  const listeners = new Set<GateKeeperSpaListener>();
  let tokens: StoredTokens | null = loadTokens();
  let status: GateKeeperSpaStatus = tokens && !isExpired(tokens) ? 'authenticated' : 'unauthenticated';
  let error: Error | null = null;
  let snapshot = computeSnapshot();
  let inflightRefresh: Promise<void> | null = null;

  function loadTokens(): StoredTokens | null {
    try {
      return JSON.parse(storage.getItem(tokenKey) || 'null') as StoredTokens | null;
    } catch {
      return null;
    }
  }

  function isExpired(current: StoredTokens): boolean {
    if (!current.expiresAt) return false;
    return Math.floor(Date.now() / 1000) >= current.expiresAt;
  }

  function needsRefresh(current: StoredTokens): boolean {
    if (!current.expiresAt) return false;
    return Math.floor(Date.now() / 1000) >= current.expiresAt - REFRESH_SKEW_SECONDS;
  }

  function computeSnapshot(): GateKeeperSpaSnapshot {
    const profile = profileFromClaims(tokens?.claims);
    return Object.freeze({
      status,
      authenticated: status === 'authenticated',
      user: profile,
      expiresAt: tokens?.expiresAt ?? null,
      error
    });
  }

  function emit(): void {
    snapshot = computeSnapshot();
    for (const listener of listeners) listener(snapshot);
  }

  function persist(next: StoredTokens | null): void {
    tokens = next;
    if (next) storage.setItem(tokenKey, JSON.stringify(next));
    else storage.removeItem(tokenKey);
    status = next && !isExpired(next) ? 'authenticated' : 'unauthenticated';
    error = null;
    emit();
  }

  function readTransaction(): PkceTransaction | null {
    try {
      return JSON.parse(storage.getItem(transactionKey) || 'null') as PkceTransaction | null;
    } catch {
      return null;
    }
  }

  async function signIn(options: SignInOptions = {}): Promise<void> {
    const transaction = await generatePkce(cfg.redirectUri);
    const challenge = await codeChallengeS256(transaction.codeVerifier);
    storage.setItem(transactionKey, JSON.stringify(transaction));

    const url = buildAuthorizeUrl(cfg, transaction, challenge, {
      scope: options.scope,
      loginHint: options.loginHint,
      extraParams: options.extraParams
    });
    window.location.href = url;
  }

  async function handleRedirectCallback(url?: string): Promise<GateKeeperProfile | null> {
    const target = new URL(url ?? window.location.href);
    const code = target.searchParams.get('code');
    const state = target.searchParams.get('state');
    if (!code) return null;

    const transaction = readTransaction();
    if (!transaction?.codeVerifier || transaction.state !== state) {
      storage.removeItem(transactionKey);
      const mismatch = new Error('OAuth state mismatch. Start sign-in again.');
      error = mismatch;
      emit();
      throw mismatch;
    }

    try {
      const response = await exchangeCode(cfg, {
        code,
        codeVerifier: transaction.codeVerifier,
        redirectUri: transaction.redirectUri
      });
      storage.removeItem(transactionKey);
      persist(tokensFromResponse(response));
      cleanUrl(target);
      return snapshot.user;
    } catch (err) {
      storage.removeItem(transactionKey);
      error = err instanceof Error ? err : new Error('Code exchange failed');
      status = 'unauthenticated';
      emit();
      throw error;
    }
  }

  function cleanUrl(target: URL): void {
    if (typeof window === 'undefined' || !window.history) return;
    for (const param of ['code', 'state', 'redirect_uri', 'client_id']) {
      target.searchParams.delete(param);
    }
    window.history.replaceState({}, document.title, target.pathname + target.search + target.hash);
  }

  function refresh(): Promise<void> {
    if (inflightRefresh) return inflightRefresh;
    const current = tokens;
    if (!current?.refreshToken) {
      return Promise.reject(new Error('No refresh token available'));
    }

    inflightRefresh = (async () => {
      try {
        const response = await refreshTokens(cfg, current.refreshToken as string);
        const next = tokensFromResponse(response);
        // The server may omit id_token on refresh — keep prior claims if so.
        if (!next.claims) next.claims = current.claims;
        persist(next);
      } catch (err) {
        const failure = err as { oauthError?: string; status?: number };
        const definitive =
          failure?.oauthError === 'invalid_grant' ||
          (typeof failure?.status === 'number' && failure.status >= 400 && failure.status < 500);
        if (definitive) persist(null);
        throw err;
      } finally {
        inflightRefresh = null;
      }
    })();

    return inflightRefresh;
  }

  async function getAccessToken(): Promise<string | null> {
    if (!tokens) return null;
    if (needsRefresh(tokens) && tokens.refreshToken) {
      try {
        await refresh();
      } catch {
        return tokens?.accessToken ?? null;
      }
    }
    return tokens?.accessToken ?? null;
  }

  async function fetchUserInfoNow(): Promise<JwtClaims> {
    const accessToken = await getAccessToken();
    if (!accessToken) throw new Error('Not signed in');
    return fetchUserInfo(cfg, accessToken);
  }

  function signOut(): void {
    storage.removeItem(transactionKey);
    persist(null);
  }

  return {
    config: cfg,
    getSnapshot: () => snapshot,
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    signIn,
    handleRedirectCallback,
    refresh,
    getAccessToken,
    fetchUserInfo: fetchUserInfoNow,
    getUser: () => snapshot.user,
    isAuthenticated: () => status === 'authenticated',
    signOut
  };
}
