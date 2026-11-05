// Browser PKCE + random-token helpers built on the Web Crypto API. No secret is
// ever involved — the code_verifier stays in the browser and is proven at the
// token endpoint via its S256 challenge.

import type { PkceTransaction } from './types.js';

export function base64UrlEncode(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function base64UrlDecode(value: string): string {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(value.length / 4) * 4, '=');
  return atob(padded);
}

export function randomBase64Url(byteLength = 32): string {
  const bytes = new Uint8Array(byteLength);
  crypto.getRandomValues(bytes);
  return base64UrlEncode(bytes);
}

export async function codeChallengeS256(codeVerifier: string): Promise<string> {
  const bytes = new TextEncoder().encode(codeVerifier);
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return base64UrlEncode(new Uint8Array(digest));
}

/** Generate a fresh PKCE transaction (verifier + S256 challenge + state + nonce). */
export async function generatePkce(redirectUri: string): Promise<PkceTransaction> {
  const codeVerifier = randomBase64Url(48);
  const state = randomBase64Url(24);
  const nonce = randomBase64Url(24);
  return {
    state,
    nonce,
    codeVerifier,
    redirectUri,
    createdAt: Date.now()
  };
}
