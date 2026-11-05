// Public-client (SPA) OAuth types. A public client holds NO secret — PKCE S256
// is the only protection, and tokens live in the browser (sessionStorage by
// default). This is the shared contract every framework adapter binds to.

export interface GateKeeperSpaConfig {
  /** Identity Provider base URL, e.g. http://localhost:8080 (token + userinfo). */
  idpBaseUrl: string;
  /** Hosted login UI base URL, e.g. http://localhost:5173 (where the browser signs in). */
  authUiBaseUrl: string;
  /** Tenant the public client belongs to. */
  tenantId: string;
  /** This app's registered public client id. */
  clientId: string;
  /** Must exactly match a registered redirect_uri. Defaults to `${location.origin}/`. */
  redirectUri?: string;
  /** Space-separated scopes. Defaults to `openid profile email`. */
  scope?: string;
  /**
   * Storage for tokens + the in-flight PKCE transaction. Defaults to
   * `window.sessionStorage`. Pass `localStorage` to persist across tabs/reloads
   * (larger XSS blast radius — sessionStorage is recommended).
   */
  storage?: StorageLike;
  /** Key prefix for stored values. Defaults to `gatekeeper.spa-oauth`. */
  storageKeyPrefix?: string;
  /** Lifetime (seconds) of the hosted-login request. Defaults to 1200. */
  authRequestTtlSeconds?: number;
}

export interface ResolvedSpaConfig {
  idpBaseUrl: string;
  authUiBaseUrl: string;
  tenantId: string;
  clientId: string;
  redirectUri: string;
  scope: string;
  authRequestTtlSeconds: number;
}

export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface TokenResponse {
  access_token: string;
  expires_in?: number;
  id_token?: string;
  refresh_token?: string;
  scope?: string;
  token_type?: string;
  [key: string]: unknown;
}

export interface JwtClaims {
  sub?: string;
  email?: string;
  email_verified?: boolean;
  name?: string;
  given_name?: string;
  family_name?: string;
  picture?: string;
  exp?: number;
  nonce?: string;
  [claim: string]: unknown;
}

export interface GateKeeperProfile {
  sub?: string;
  email?: string;
  emailVerified?: boolean;
  name?: string;
  givenName?: string;
  familyName?: string;
  picture?: string;
}

/** Tokens + derived claims persisted in browser storage. */
export interface StoredTokens {
  accessToken: string;
  idToken: string | null;
  refreshToken: string | null;
  tokenType?: string;
  scope?: string;
  expiresAt: number | null;
  claims: JwtClaims | null;
}

/** PKCE transaction stashed between `signIn()` and the redirect callback. */
export interface PkceTransaction {
  state: string;
  nonce: string;
  codeVerifier: string;
  redirectUri: string;
  createdAt: number;
}

export interface SignInOptions {
  /** Prefill the login form's identifier. */
  loginHint?: string;
  /** Override the default scope for this request. */
  scope?: string;
  /** Extra query params to forward to the hosted login UI (e.g. mfa hints). */
  extraParams?: Record<string, string>;
}

export type GateKeeperSpaStatus = 'loading' | 'authenticated' | 'unauthenticated';

/** Immutable view of client state handed to subscribers. */
export interface GateKeeperSpaSnapshot {
  status: GateKeeperSpaStatus;
  authenticated: boolean;
  user: GateKeeperProfile | null;
  expiresAt: number | null;
  error: Error | null;
}

export type GateKeeperSpaListener = (snapshot: GateKeeperSpaSnapshot) => void;

export interface GateKeeperSpaClient {
  /** Current immutable snapshot (safe to read synchronously). */
  getSnapshot(): GateKeeperSpaSnapshot;
  /** Subscribe to state changes; returns an unsubscribe function. */
  subscribe(listener: GateKeeperSpaListener): () => void;
  /** Begin the public Auth Code + PKCE flow — redirects to the hosted login UI. */
  signIn(options?: SignInOptions): Promise<void>;
  /**
   * If the current URL carries `?code=&state=`, exchange it for tokens, persist
   * them, and strip the params from the URL. Returns the profile, or null when
   * there was no callback to handle. Call once on app boot.
   */
  handleRedirectCallback(url?: string): Promise<GateKeeperProfile | null>;
  /** Rotate the refresh token for a fresh access token. */
  refresh(): Promise<void>;
  /** Return a non-expired access token, refreshing first if needed. */
  getAccessToken(): Promise<string | null>;
  /** Fetch the current profile from `/oauth/userinfo`. */
  fetchUserInfo(): Promise<JwtClaims>;
  /** Current profile from cached token claims, or null. */
  getUser(): GateKeeperProfile | null;
  /** Whether a (non-expired) session is present. */
  isAuthenticated(): boolean;
  /** Clear all local auth state (there is no server-side logout endpoint). */
  signOut(): void;
  /** Resolved, read-only configuration. */
  readonly config: ResolvedSpaConfig;
}
