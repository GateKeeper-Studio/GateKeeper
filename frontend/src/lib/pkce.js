// PKCE (RFC 7636) and random-value helpers, built on the Web Crypto API.
//
// Everything here uses crypto.getRandomValues / crypto.subtle — never Math.random
// — so verifiers, state, and nonce are cryptographically unpredictable.

// URL-safe base64 without padding, matching what the provider expects for PKCE.
export function base64UrlEncode(input) {
  const bytes = input instanceof ArrayBuffer ? new Uint8Array(input) : input;
  let binary = '';
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

// `byteLength` bytes of CSPRNG entropy, returned URL-safe base64.
export function randomToken(byteLength = 32) {
  const bytes = new Uint8Array(byteLength);
  crypto.getRandomValues(bytes);
  return base64UrlEncode(bytes);
}

async function sha256(text) {
  const data = new TextEncoder().encode(text);
  return crypto.subtle.digest('SHA-256', data);
}

// Derives the S256 challenge from a verifier: BASE64URL(SHA256(verifier)).
export async function s256Challenge(verifier) {
  return base64UrlEncode(await sha256(verifier));
}

// Generates the full PKCE pair plus a state and nonce in one shot.
export async function generatePkce() {
  // RFC 7636 allows 43–128 chars; 32 random bytes → 43 chars after base64url.
  const codeVerifier = randomToken(32);
  const codeChallenge = await s256Challenge(codeVerifier);
  return {
    codeVerifier,
    codeChallenge,
    codeChallengeMethod: 'S256',
    state: randomToken(16), // CSRF protection
    nonce: randomToken(16) // OIDC replay protection
  };
}
