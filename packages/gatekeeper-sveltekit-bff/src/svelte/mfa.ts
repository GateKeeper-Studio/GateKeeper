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

export function createMfaClient(options: MfaClientOptions = {}): MfaClient {
  const basePath = options.basePath ?? '/api/mfa';
  const fetchFn = options.fetch ?? fetch;
  const passkeyRegistrar = options.registerPasskey ?? registerPasskey;

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
    async removeMethod(id) {
      await request(`/methods/${id}`, { method: 'DELETE' });
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
    startPasskeyEnrollment() {
      return request<WebAuthnStartResponse>('/webauthn/start', { method: 'POST' });
    },
    finishPasskeyEnrollment(input) {
      return request<MfaMethodEntry>('/webauthn/finish', {
        method: 'POST',
        body: JSON.stringify(input)
      });
    },
    async addPasskey(name) {
      const { mfaId, options } = await this.startPasskeyEnrollment();
      const attestation = await passkeyRegistrar(options);
      return this.finishPasskeyEnrollment({ mfaId, name: name?.trim() || null, attestation });
    }
  };
}
