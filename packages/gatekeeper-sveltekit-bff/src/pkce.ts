import crypto from 'node:crypto';

import type { PkceRequest } from './types.js';

function base64Url(buffer: Buffer): string {
  return buffer.toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
}

function randomToken(bytes = 32): string {
  return crypto.randomBytes(bytes).toString('hex');
}

function codeChallengeS256(codeVerifier: string): string {
  return base64Url(crypto.createHash('sha256').update(codeVerifier).digest());
}

export function generatePkce(): PkceRequest {
  const codeVerifier = randomToken(32);
  return {
    codeVerifier,
    codeChallenge: codeChallengeS256(codeVerifier),
    state: randomToken(16),
    nonce: randomToken(16)
  };
}
