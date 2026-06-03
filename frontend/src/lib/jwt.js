// Minimal JWT decoding plus RS256 signature verification via the Web Crypto API.
//
// Verifying the id_token signature against the provider's JWKS (rather than just
// trusting the decoded body) is a modern best practice. Public clients often skip
// this, but doing it here demonstrates the full OIDC verification path.

function base64UrlToString(segment) {
  let s = segment.replace(/-/g, '+').replace(/_/g, '/');
  const padding = s.length % 4;
  if (padding) s += '='.repeat(4 - padding);
  return atob(s);
}

function base64UrlToBytes(segment) {
  const binary = base64UrlToString(segment);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

export function decodeJwt(token) {
  const parts = token.split('.');
  if (parts.length !== 3) throw new Error('not a well-formed JWT');
  return {
    header: JSON.parse(base64UrlToString(parts[0])),
    payload: JSON.parse(base64UrlToString(parts[1]))
  };
}

// Verifies an RS256 JWT against a JWKS document. Returns true/false; throws only
// when no usable key is found.
export async function verifyRs256(token, jwks) {
  const [headerB64, payloadB64, signatureB64] = token.split('.');
  const header = JSON.parse(base64UrlToString(headerB64));

  if (header.alg !== 'RS256') {
    // Reject algorithm confusion / "alg=none" outright.
    throw new Error(`unsupported alg: ${header.alg}`);
  }

  const jwk =
    jwks.keys.find((k) => k.kid === header.kid) ||
    (jwks.keys.length === 1 ? jwks.keys[0] : null);
  if (!jwk) throw new Error("no JWK matches the token's kid");

  const key = await crypto.subtle.importKey(
    'jwk',
    jwk,
    { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' },
    false,
    ['verify']
  );

  const data = new TextEncoder().encode(`${headerB64}.${payloadB64}`);
  const signature = base64UrlToBytes(signatureB64);
  return crypto.subtle.verify('RSASSA-PKCS1-v1_5', key, signature, data);
}

// Standard OIDC id_token claim checks. Returns an array of {label, ok} results.
export function validateIdTokenClaims(payload, { issuer, clientId, nonce }) {
  const now = Math.floor(Date.now() / 1000);
  return [
    { label: `iss == ${issuer}`, ok: payload.iss === issuer },
    { label: `aud == ${clientId}`, ok: payload.aud === clientId },
    { label: 'exp in the future', ok: typeof payload.exp === 'number' && payload.exp > now },
    { label: 'nonce matches request', ok: payload.nonce === nonce }
  ];
}
