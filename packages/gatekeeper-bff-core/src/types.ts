export type MfaMethod = 'webauthn' | 'totp' | 'email';

export interface MfaConfig {
  methods: string[];
  defaultMethod?: string;
}

export interface NormalizedMfaConfig {
  methods: MfaMethod[];
  defaultMethod?: MfaMethod;
}

export interface GateKeeperConfig {
  idpBaseUrl?: string;
  authUiBaseUrl?: string;
  tenantId?: string;
  clientId?: string;
  clientSecret?: string;
  redirectUri?: string;
  scope?: string;
  sessionSecret?: string;
  secureCookies?: boolean;
  sessionCookieName?: string;
  authRequestTtlSeconds?: number;
}

export interface ResolvedGateKeeperConfig {
  idpBaseUrl: string;
  authUiBaseUrl: string;
  tenantId: string;
  clientId: string;
  clientSecret?: string;
  redirectUri: string;
  scope: string;
  sessionSecret: string;
  secureCookies: boolean;
  sessionCookieName: string;
  authRequestTtlSeconds: number;
}

export interface RedactedGateKeeperConfig
  extends Omit<ResolvedGateKeeperConfig, 'clientSecret' | 'sessionSecret'> {
  clientSecret: undefined;
  sessionSecret: undefined;
}

export interface AuthorizeUrlInput {
  state: string;
  nonce: string;
  codeChallenge: string;
  mfa?: NormalizedMfaConfig;
  loginHint?: string;
}

export interface PkceRequest {
  codeVerifier: string;
  codeChallenge: string;
  state: string;
  nonce: string;
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

export interface GateKeeperSession {
  profile: GateKeeperProfile | null;
  accessToken: string;
  idToken?: string;
  refreshToken?: string;
  scope?: string;
  expiresAt: number;
}

export interface UpdateUserProfileInput {
  displayName?: string | null;
  firstName: string;
  lastName: string;
}

export interface UserProfileView {
  userId: string;
  displayName: string | null;
  firstName: string;
  lastName: string;
  email: string;
}

export interface BrowserSafeSession {
  authenticated: boolean;
  user: GateKeeperProfile | null;
  expiresAt: number | null;
}

export interface CookieOptions {
  httpOnly?: boolean;
  sameSite?: 'lax' | 'strict' | 'none';
  secure?: boolean;
  path: string;
  maxAge?: number;
}

// Minimal cookie jar every host (Next.js, Remix, …) can adapt to.
export interface CookiesLike {
  get(name: string): string | undefined;
  set(name: string, value: string, options: CookieOptions): void;
  delete(name: string, options: Pick<CookieOptions, 'path'>): void;
}

export interface LocalsLike {
  session?: GateKeeperSession | null;
}

export interface SessionStore {
  sessionCookieName: string;
  encryptSession(session: GateKeeperSession): string;
  decryptSession(value: string): GateKeeperSession | null;
  setSession(cookies: CookiesLike, session: GateKeeperSession): void;
  readSession(cookies: CookiesLike): GateKeeperSession | null;
  clearSession(cookies: CookiesLike): void;
}

export interface GateKeeperClient extends SessionStore {
  config: RedactedGateKeeperConfig;
  authRequestTtlSeconds: number;
  generatePkce(): PkceRequest;
  buildAuthorizeUrl(input: AuthorizeUrlInput): string;
  decodeJwtPayload(token: string): JwtClaims | null;
  exchangeCode(input: { code: string; codeVerifier: string }): Promise<TokenResponse>;
  refreshTokens(refreshToken: string): Promise<TokenResponse>;
  fetchUserInfo(accessToken: string): Promise<JwtClaims>;
  profileFromClaims(claims: JwtClaims | null | undefined): GateKeeperProfile | null;
  sessionFromTokenResponse(tokens: TokenResponse): GateKeeperSession;
  updateMyProfile(
    sessionOrLocals: GateKeeperSession | LocalsLike | null | undefined,
    input: UpdateUserProfileInput
  ): Promise<UserProfileView>;
  updateUserProfile(
    sessionOrLocals: GateKeeperSession | LocalsLike | null | undefined,
    input: UpdateUserProfileInput
  ): Promise<UserProfileView>;
  sessionWithUpdatedProfile(session: GateKeeperSession, profile: UserProfileView): GateKeeperSession;
  idpFetch<T = unknown>(
    sessionOrLocals: GateKeeperSession | LocalsLike | null | undefined,
    path: string,
    init?: RequestInit
  ): Promise<T>;
}

// --- BFF runtime (framework-agnostic) ---

export interface BffOptions {
  /** Refresh the access token this many seconds before it expires. Default 120. */
  refreshSkewSeconds?: number;
  /** Cookie names for the short-lived PKCE transaction. */
  transactionCookiePrefix?: string;
}

export interface StartSignInInput {
  /** Where to send the user after a successful callback. Default `/`. */
  returnTo?: string;
  mfa?: NormalizedMfaConfig;
  loginHint?: string;
}

export interface StartSignInResult {
  authorizeUrl: string;
}

export interface CompleteSignInInput {
  code: string;
  state: string;
}

export interface CompleteSignInResult {
  session: GateKeeperSession;
  returnTo: string;
}

export interface GateKeeperBff extends GateKeeperClient {
  /** Mint PKCE, stash it in short-lived httpOnly cookies, and return the authorize URL. */
  startSignIn(cookies: CookiesLike, input?: StartSignInInput): StartSignInResult;
  /** Validate `state`, exchange the code, verify the id_token nonce, seal the session. */
  completeSignIn(cookies: CookiesLike, input: CompleteSignInInput): Promise<CompleteSignInResult>;
  /** Read the session; silently refresh (coalesced) when it is expiring. */
  loadSession(cookies: CookiesLike): Promise<GateKeeperSession | null>;
  /** Clear the session cookie (no server revocation endpoint exists). */
  signOut(cookies: CookiesLike): void;
  /** Reduce a session to browser-safe fields (no tokens). */
  getBrowserSession(session: GateKeeperSession | null | undefined): BrowserSafeSession;
}
