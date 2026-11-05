export type MfaMethod = 'webauthn' | 'totp' | 'email';

export interface GateKeeperProfile {
  sub?: string;
  email?: string;
  emailVerified?: boolean;
  name?: string;
  givenName?: string;
  familyName?: string;
  picture?: string;
  [claim: string]: unknown;
}

export interface BrowserSafeSession {
  authenticated: boolean;
  user: GateKeeperProfile | null;
  expiresAt: number | null;
  /**
   * Optional by design. Secure BFF endpoints should usually omit this and keep
   * access tokens server-side. It exists for applications that explicitly choose
   * to expose a short-lived token to browser code.
   */
  accessToken?: string;
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

export interface GateKeeperBrowserClientOptions {
  initialSession?: BrowserSafeSession | null;
  sessionEndpoint?: string;
  profileEndpoint?: string;
  fetch?: typeof fetch;
  refreshSkewSeconds?: number;
  minRefreshDelaySeconds?: number;
  autoRefresh?: boolean;
}

export interface GateKeeperBrowserState {
  session: BrowserSafeSession | null;
  refreshing: boolean;
  error: unknown;
}

export interface GateKeeperBrowserSnapshot extends GateKeeperBrowserState {
  user: GateKeeperProfile | null;
  authenticated: boolean;
  expiresAt: number | null;
}

export type GateKeeperBrowserSubscriber = (snapshot: GateKeeperBrowserSnapshot) => void;

export type GateKeeperBrowserEventName = 'change' | 'refresh' | 'error';

export type GateKeeperBrowserEventHandler = (snapshot: GateKeeperBrowserSnapshot) => void;

export interface GateKeeperBrowserClient {
  getSnapshot(): GateKeeperBrowserSnapshot;
  getSession(): BrowserSafeSession | null;
  getUser(): GateKeeperProfile | null;
  getAccessToken(): string | null;
  isAuthenticated(): boolean;
  setSession(session: BrowserSafeSession | null): void;
  refresh(): Promise<BrowserSafeSession | null>;
  startAutoRefresh(): void;
  stopAutoRefresh(): void;
  updateUserProfile(input: UpdateUserProfileInput): Promise<UserProfileView>;
  subscribe(subscriber: GateKeeperBrowserSubscriber): () => void;
  on(event: GateKeeperBrowserEventName, handler: GateKeeperBrowserEventHandler): () => void;
}

export interface MfaMethodEntry {
  id?: string;
  method: MfaMethod;
  name?: string | null;
  confirmed: boolean;
  isPreferred?: boolean;
  createdAt?: string | null;
  lastUsedAt?: string | null;
}

export interface MfaConfigurationState {
  methods: MfaMethodEntry[];
  preferredMethod: MfaMethod | null;
}

export interface TotpEnrollment {
  enrollmentId?: string;
  mfaId?: string;
  secretBase32?: string;
  otpauthUrl?: string;
  qrDataUrl?: string;
}

export interface PasskeyPublicKeyOptionsJson
  extends Omit<PublicKeyCredentialCreationOptions, 'challenge' | 'user' | 'excludeCredentials'> {
  challenge: string;
  user: Omit<PublicKeyCredentialUserEntity, 'id'> & { id: string };
  excludeCredentials?: Array<Omit<PublicKeyCredentialDescriptor, 'id'> & { id: string }>;
}

export interface PasskeyCreationOptions {
  publicKey: PasskeyPublicKeyOptionsJson;
}

export interface WebAuthnStartResponse {
  mfaId: string;
  options: PasskeyCreationOptions;
}

export interface MfaClientOptions {
  basePath?: string;
  fetch?: typeof fetch;
  registerPasskey?: (options: PasskeyCreationOptions) => Promise<unknown>;
}

export interface MfaClient {
  listMethods(): Promise<MfaConfigurationState>;
  setPreferredMethod(method: MfaMethod | null): Promise<void>;
  renameMethod(id: string, name: string | null): Promise<MfaMethodEntry>;
  removeMethod(id: string): Promise<void>;
  startTotpEnrollment(): Promise<TotpEnrollment>;
  confirmTotpEnrollment(code: string): Promise<MfaMethodEntry>;
  startPasskeyEnrollment(): Promise<WebAuthnStartResponse>;
  finishPasskeyEnrollment(input: {
    mfaId: string;
    name?: string | null;
    attestation: unknown;
  }): Promise<MfaMethodEntry>;
  addPasskey(name?: string | null): Promise<MfaMethodEntry>;
}
