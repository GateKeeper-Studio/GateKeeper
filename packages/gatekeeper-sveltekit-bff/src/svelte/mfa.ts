import type { MfaMethod } from '../types.js';

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

export interface WebAuthnStartResponse {
  mfaId: string;
  options: PasskeyCreationOptions;
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

export type MfaConfigurationClasses = Partial<
  Record<
    | 'root'
    | 'error'
    | 'section'
    | 'title'
    | 'muted'
    | 'methodList'
    | 'methodRow'
    | 'methodInfo'
    | 'methodName'
    | 'methodMeta'
    | 'methodActions'
    | 'defaultLabel'
    | 'actions'
    | 'button'
    | 'primaryButton'
    | 'dangerButton'
    | 'input'
    | 'totpSetup'
    | 'qr'
    | 'totpSecret',
    string
  >
>;

export const defaultMfaMethodLabels = Object.freeze({
  email: 'Email code',
  totp: 'Authenticator app',
  webauthn: 'Passkey'
} satisfies Record<MfaMethod, string>);

function base64urlToBuffer(value: string): ArrayBuffer {
  const base64 = value.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes.buffer;
}

function bufferToBase64url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function parseCreationOptions(publicKey: PasskeyPublicKeyOptionsJson): PublicKeyCredentialCreationOptions {
  return {
    ...publicKey,
    challenge: base64urlToBuffer(publicKey.challenge),
    user: { ...publicKey.user, id: base64urlToBuffer(publicKey.user.id) },
    excludeCredentials: (publicKey.excludeCredentials ?? []).map((cred) => ({
      ...cred,
      id: base64urlToBuffer(cred.id)
    }))
  } as PublicKeyCredentialCreationOptions;
}

function credentialToJSON(credential: PublicKeyCredential) {
  const response = credential.response as AuthenticatorAttestationResponse;
  return {
    id: credential.id,
    rawId: bufferToBase64url(credential.rawId),
    type: credential.type,
    authenticatorAttachment: credential.authenticatorAttachment ?? null,
    clientExtensionResults: credential.getClientExtensionResults?.() ?? {},
    response: {
      attestationObject: bufferToBase64url(response.attestationObject),
      clientDataJSON: bufferToBase64url(response.clientDataJSON),
      transports: response.getTransports?.() ?? []
    }
  };
}

export async function registerPasskey(options: PasskeyCreationOptions): Promise<unknown> {
  if (!window.PublicKeyCredential) {
    throw new Error('This browser does not support passkeys');
  }

  const credentialConstructor = PublicKeyCredential as unknown as {
    parseCreationOptionsFromJSON?: (options: unknown) => PublicKeyCredentialCreationOptions;
  };

  const publicKey =
    typeof credentialConstructor.parseCreationOptionsFromJSON === 'function'
      ? credentialConstructor.parseCreationOptionsFromJSON(options.publicKey)
      : parseCreationOptions(options.publicKey);

  const credential = (await navigator.credentials.create({
    publicKey
  })) as PublicKeyCredential | null;
  if (!credential) throw new Error('Passkey registration was cancelled');

  const jsonCredential = credential as PublicKeyCredential & { toJSON?: () => unknown };
  return typeof jsonCredential.toJSON === 'function'
    ? jsonCredential.toJSON()
    : credentialToJSON(credential);
}

async function readJsonError(response: Response): Promise<Error> {
  const detail = await response.json().catch(() => null);
  return new Error(detail?.message ?? detail?.error ?? `Request failed (${response.status})`);
}

function parseRequestOptions(publicKey: PasskeyRequestOptionsJson): PublicKeyCredentialRequestOptions {
  return {
    ...publicKey,
    challenge: base64urlToBuffer(publicKey.challenge),
    allowCredentials: (publicKey.allowCredentials ?? []).map((cred) => ({
      ...cred,
      id: base64urlToBuffer(cred.id)
    }))
  } as PublicKeyCredentialRequestOptions;
}

function assertionToJSON(credential: PublicKeyCredential) {
  const response = credential.response as AuthenticatorAssertionResponse;
  return {
    id: credential.id,
    rawId: bufferToBase64url(credential.rawId),
    type: credential.type,
    authenticatorAttachment: credential.authenticatorAttachment ?? null,
    clientExtensionResults: credential.getClientExtensionResults?.() ?? {},
    response: {
      authenticatorData: bufferToBase64url(response.authenticatorData),
      clientDataJSON: bufferToBase64url(response.clientDataJSON),
      signature: bufferToBase64url(response.signature),
      userHandle: response.userHandle ? bufferToBase64url(response.userHandle) : null
    }
  };
}

/** Runs a passkey assertion (the step-up's ceremony) and returns it as JSON. */
export async function getPasskeyAssertion(options: PasskeyRequestOptions): Promise<unknown> {
  if (!window.PublicKeyCredential) {
    throw new Error('This browser does not support passkeys');
  }

  const credentialConstructor = PublicKeyCredential as unknown as {
    parseRequestOptionsFromJSON?: (options: unknown) => PublicKeyCredentialRequestOptions;
  };

  const publicKey =
    typeof credentialConstructor.parseRequestOptionsFromJSON === 'function'
      ? credentialConstructor.parseRequestOptionsFromJSON(options.publicKey)
      : parseRequestOptions(options.publicKey);

  const credential = (await navigator.credentials.get({ publicKey })) as PublicKeyCredential | null;
  if (!credential) throw new Error('Passkey confirmation was cancelled');

  const jsonCredential = credential as PublicKeyCredential & { toJSON?: () => unknown };
  return typeof jsonCredential.toJSON === 'function'
    ? jsonCredential.toJSON()
    : assertionToJSON(credential);
}

// The body a sensitive change carries its proof in.
function stepUpBody(proof?: StepUpProof): Record<string, unknown> {
  if (proof === undefined) return {};
  if (typeof proof === 'string') return { current_password: proof };
  if ('currentPassword' in proof) return { current_password: proof.currentPassword };
  if ('assertion' in proof) {
    return { step_up: { challenge_id: proof.challengeId, assertion: proof.assertion } };
  }
  return { step_up: { challenge_id: proof.challengeId, code: proof.code } };
}

export function createMfaClient(options: MfaClientOptions = {}): MfaClient {
  const basePath = options.basePath ?? '/api/mfa';
  const fetchFn = options.fetch ?? fetch;
  const passkeyRegistrar = options.registerPasskey ?? registerPasskey;
  const passkeyAsserter = options.getPasskeyAssertion ?? getPasskeyAssertion;

  async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const response = await fetchFn(`${basePath}${path}`, {
      ...init,
      headers: init.body ? { 'Content-Type': 'application/json', ...init.headers } : init.headers
    });

    if (!response.ok) throw await readJsonError(response);
    return response.status === 204 ? (null as T) : ((await response.json()) as T);
  }

  return {
    listMethods() {
      return request<MfaConfigurationState>('/methods');
    },
    async setPreferredMethod(method) {
      await request('/preferred', {
        method: 'PUT',
        body: JSON.stringify({ method })
      });
    },
    renameMethod(id, name) {
      return request<MfaMethodEntry>(`/methods/${id}`, {
        method: 'PATCH',
        body: JSON.stringify({ name })
      });
    },
    async removeMethod(id, proof) {
      await request(`/methods/${id}`, {
        method: 'DELETE',
        body: JSON.stringify(stepUpBody(proof))
      });
    },
    startTotpEnrollment() {
      return request<TotpEnrollment>('/totp', { method: 'POST' });
    },
    confirmTotpEnrollment(code) {
      return request<MfaMethodEntry>('/totp/confirm', {
        method: 'POST',
        body: JSON.stringify({ code })
      });
    },
    startPasskeyEnrollment(proof) {
      return request<WebAuthnStartResponse>('/webauthn/start', {
        method: 'POST',
        body: JSON.stringify(stepUpBody(proof))
      });
    },
    finishPasskeyEnrollment(input) {
      return request<MfaMethodEntry>('/webauthn/finish', {
        method: 'POST',
        body: JSON.stringify(input)
      });
    },
    async addPasskey(name, proof) {
      const { mfaId, options: passkeyOptions } = await this.startPasskeyEnrollment(proof);
      const attestation = await passkeyRegistrar(passkeyOptions);
      return this.finishPasskeyEnrollment({ mfaId, name: name?.trim() || null, attestation });
    },
    startStepUp() {
      return request<StepUpStart>('/step-up', { method: 'POST' });
    },
    async proveStepUp(start, ask) {
      if (start.method === 'passkey') {
        const assertion = await passkeyAsserter(start.options);
        return { challengeId: start.challengeId, assertion };
      }
      const answer = (await ask(start.method))?.trim();
      if (!answer) return undefined;
      return start.method === 'password' ? answer : { challengeId: start.challengeId, code: answer };
    },
    async collectStepUp(ask) {
      return this.proveStepUp(await this.startStepUp(), ask);
    }
  };
}
