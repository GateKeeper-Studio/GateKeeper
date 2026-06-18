// Browser-side WebAuthn registration glue. Prefers the native JSON helpers
// (PublicKeyCredential.parseCreationOptionsFromJSON / credential.toJSON,
// baseline in evergreen browsers) and falls back to manual base64url <->
// ArrayBuffer conversion elsewhere. No npm dependency.

function base64urlToBuffer(value) {
  const base64 = value.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes.buffer;
}

function bufferToBase64url(buffer) {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function parseCreationOptions(publicKey) {
  return {
    ...publicKey,
    challenge: base64urlToBuffer(publicKey.challenge),
    user: { ...publicKey.user, id: base64urlToBuffer(publicKey.user.id) },
    excludeCredentials: (publicKey.excludeCredentials ?? []).map((cred) => ({
      ...cred,
      id: base64urlToBuffer(cred.id)
    }))
  };
}

function credentialToJSON(credential) {
  return {
    id: credential.id,
    rawId: bufferToBase64url(credential.rawId),
    type: credential.type,
    authenticatorAttachment: credential.authenticatorAttachment ?? null,
    clientExtensionResults: credential.getClientExtensionResults?.() ?? {},
    response: {
      attestationObject: bufferToBase64url(credential.response.attestationObject),
      clientDataJSON: bufferToBase64url(credential.response.clientDataJSON),
      transports: credential.response.getTransports?.() ?? []
    }
  };
}

// `options` is the IdP's creation options ({ publicKey: { ... } }, base64url
// encoded). Returns the attestation in the JSON form the IdP verifies.
export async function registerPasskey(options) {
  if (!window.PublicKeyCredential) {
    throw new Error('This browser does not support passkeys');
  }

  const publicKey =
    typeof PublicKeyCredential.parseCreationOptionsFromJSON === 'function'
      ? PublicKeyCredential.parseCreationOptionsFromJSON(options.publicKey)
      : parseCreationOptions(options.publicKey);

  const credential = await navigator.credentials.create({ publicKey });
  if (!credential) throw new Error('Passkey registration was cancelled');

  return typeof credential.toJSON === 'function'
    ? credential.toJSON()
    : credentialToJSON(credential);
}
