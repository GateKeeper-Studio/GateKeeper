// PKCE (RFC 7636) + CSRF state + OIDC nonce generation, server-side.
//
// The verifier/state/nonce are minted here and parked in short-lived httpOnly
// cookies while the user is away at the login page; the callback validates them.
import crypto from 'node:crypto';

function base64Url(buffer) {
  return buffer.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}

// One opaque, high-entropy value for verifier/state/nonce.
function randomToken(bytes = 32) {
  return crypto.randomBytes(bytes).toString('hex');
}

// S256 challenge = base64url( SHA256(code_verifier) ).
function codeChallengeS256(codeVerifier) {
  return base64Url(crypto.createHash('sha256').update(codeVerifier).digest());
}

// Produce the full set of transient secrets for one authorization request.
export function generatePkce() {
  const codeVerifier = randomToken(32);
  return {
    codeVerifier,
    codeChallenge: codeChallengeS256(codeVerifier),
    state: randomToken(16),
    nonce: randomToken(16)
  };
}
