import { requestJson } from './http.js';
import type {
  MfaClient,
  MfaClientOptions,
  MfaConfigurationState,
  MfaMethod,
  MfaMethodEntry,
  PasskeyCreationOptions,
  PasskeyPublicKeyOptionsJson,
  TotpEnrollment,
  WebAuthnStartResponse
} from './types.js';

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

export function createMfaClient(options: MfaClientOptions = {}): MfaClient {
  const basePath = options.basePath ?? '/api/mfa';
  const fetchFn = options.fetch ?? fetch;
  const passkeyRegistrar = options.registerPasskey ?? registerPasskey;

  function request<T>(path: string, init: RequestInit = {}): Promise<T> {
    return requestJson<T>(`${basePath}${path}`, { ...init, fetch: fetchFn });
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
      const { mfaId, options: passkeyOptions } = await this.startPasskeyEnrollment();
      const attestation = await passkeyRegistrar(passkeyOptions);
      return this.finishPasskeyEnrollment({ mfaId, name: name?.trim() || null, attestation });
    }
  };
}
