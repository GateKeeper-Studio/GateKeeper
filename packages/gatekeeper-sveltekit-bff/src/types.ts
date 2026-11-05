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

export interface CookiesLike {
  get(name: string): string | undefined;
  set(name: string, value: string, options: CookieOptions): void;
  delete(name: string, options: Pick<CookieOptions, 'path'>): void;
}

export interface LocalsLike {
  session?: GateKeeperSession | null;
}

export interface SvelteKitEventLike {
  cookies: unknown;
  locals: LocalsLike;
}

export type SvelteKitResolveLike = (event: SvelteKitEventLike) => Response | Promise<Response>;

export interface SvelteKitHandleInput {
  event: SvelteKitEventLike;
  resolve: SvelteKitResolveLike;
}

export interface SvelteKitSecurityHeaders {
  frameOptions?: string | false;
  contentTypeOptions?: string | false;
  referrerPolicy?: string | false;
}

export interface SvelteKitBffOptions {
  refreshSkewSeconds?: number;
  securityHeaders?: SvelteKitSecurityHeaders | false;
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
  sessionWithUpdatedProfile(
    session: GateKeeperSession,
    profile: UserProfileView
  ): GateKeeperSession;
  idpFetch<T = unknown>(
    sessionOrLocals: GateKeeperSession | LocalsLike | null | undefined,
    path: string,
    init?: RequestInit
  ): Promise<T>;
}

export interface SvelteKitGateKeeperClient extends GateKeeperClient {
  loadSession(event: SvelteKitEventLike): Promise<GateKeeperSession | null>;
  refreshSession(event: SvelteKitEventLike): Promise<GateKeeperSession | null>;
  handle(input: SvelteKitHandleInput): Promise<Response>;
  getSession(source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined): GateKeeperSession | null;
  requireSession(source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined): GateKeeperSession;
  getUser(source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined): GateKeeperProfile | null;
  getAccessToken(source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined): string | null;
  getIdToken(source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined): string | null;
  getRefreshToken(source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined): string | null;
  getBrowserSession(
    source: GateKeeperSession | LocalsLike | SvelteKitEventLike | null | undefined
  ): BrowserSafeSession;
  updateMyProfileForEvent(
    event: SvelteKitEventLike,
    input: UpdateUserProfileInput
  ): Promise<UserProfileView>;
}
