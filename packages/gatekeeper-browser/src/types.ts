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

export interface PasskeyRequestOptionsJson
  extends Omit<PublicKeyCredentialRequestOptions, 'challenge' | 'allowCredentials'> {
  challenge: string;
  allowCredentials?: Array<Omit<PublicKeyCredentialDescriptor, 'id'> & { id: string }>;
}

/** A passkey assertion request, as the identity provider sends it (base64url fields). */
export interface PasskeyRequestOptions {
  publicKey: PasskeyRequestOptionsJson;
}

/**
 * How the account proves itself before a sensitive change (`POST /me/step-up`): its
 * current password, a passkey assertion, or a 6-digit code just emailed to it.
 */
export type StepUpStart =
  | { method: 'password' }
  | { method: 'passkey'; challengeId: string; options: PasskeyRequestOptions }
  | { method: 'email'; challengeId: string; expiresAt: string };

/**
 * The proof sent with a sensitive change. A plain string is the current password;
 * the other shapes come from a started step-up.
 */
export type StepUpProof =
  | string
  | { currentPassword: string }
  | { challengeId: string; assertion: unknown }
  | { challengeId: string; code: string };

/** Asked for what only the user can give: the current password or the emailed code. */
export type StepUpPrompt = (
  kind: 'password' | 'email'
) => string | null | undefined | Promise<string | null | undefined>;

export interface MfaClientOptions {
  basePath?: string;
  fetch?: typeof fetch;
  registerPasskey?: (options: PasskeyCreationOptions) => Promise<unknown>;
  getPasskeyAssertion?: (options: PasskeyRequestOptions) => Promise<unknown>;
}

export interface MfaClient {
  listMethods(): Promise<MfaConfigurationState>;
  setPreferredMethod(method: MfaMethod | null): Promise<void>;
  renameMethod(id: string, name: string | null): Promise<MfaMethodEntry>;
  /**
   * Removes an enrollment. The identity provider asks for a step-up first, so a
   * stolen session alone cannot strip a sign-in method: the current password (a
   * string), or for an account without one the proof of a started step-up
   * (`collectStepUp`).
   */
  removeMethod(id: string, proof?: StepUpProof): Promise<void>;
  startTotpEnrollment(): Promise<TotpEnrollment>;
  confirmTotpEnrollment(code: string): Promise<MfaMethodEntry>;
  /**
   * An account without a password must send a step-up proof: there a passkey signs
   * in on its own, so a stolen session must not be able to add one. Accounts with a
   * password need none.
   */
  startPasskeyEnrollment(proof?: StepUpProof): Promise<WebAuthnStartResponse>;
  finishPasskeyEnrollment(input: {
    mfaId: string;
    name?: string | null;
    attestation: unknown;
  }): Promise<MfaMethodEntry>;
  addPasskey(name?: string | null, proof?: StepUpProof): Promise<MfaMethodEntry>;
  /** Asks the identity provider which proof the account gives, and opens its ceremony. */
  startStepUp(): Promise<StepUpStart>;
  /**
   * Turns a started step-up into its proof: runs the passkey assertion, or asks
   * `ask` for the password or the emailed code. `undefined` when the user cancels.
   */
  proveStepUp(start: StepUpStart, ask: StepUpPrompt): Promise<StepUpProof | undefined>;
  /** `startStepUp` then `proveStepUp`. */
  collectStepUp(ask: StepUpPrompt): Promise<StepUpProof | undefined>;
}
