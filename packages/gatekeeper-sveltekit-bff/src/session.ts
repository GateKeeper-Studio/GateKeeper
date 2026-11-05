import crypto from 'node:crypto';

import type {
  CookiesLike,
  GateKeeperSession,
  ResolvedGateKeeperConfig,
  SessionStore
} from './types.js';

const ALGORITHM = 'aes-256-gcm';
const SALT_LENGTH = 16;
const IV_LENGTH = 12;
const AUTH_TAG_LENGTH = 16;
const KEY_LENGTH = 32;

function deriveKey(password: string, salt: Buffer): Buffer {
  return Buffer.from(
    crypto.hkdfSync('sha256', Buffer.from(password, 'utf8'), salt, Buffer.alloc(0), KEY_LENGTH)
  );
}

export function createSessionStore({
  sessionSecret,
  secureCookies,
  sessionCookieName
}: ResolvedGateKeeperConfig): SessionStore {
  if (!sessionSecret) {
    throw new Error('sessionSecret is required');
  }

  const cookieName = sessionCookieName || 'gk_session';
  const cookieOptions = {
    httpOnly: true,
    sameSite: 'lax' as const,
    secure: Boolean(secureCookies),
    path: '/'
  };

  function encryptSession(session: GateKeeperSession): string {
    const salt = crypto.randomBytes(SALT_LENGTH);
    const iv = crypto.randomBytes(IV_LENGTH);
    const key = deriveKey(sessionSecret, salt);

    const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
    const plaintext = Buffer.from(JSON.stringify(session), 'utf8');
    const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
    const authTag = cipher.getAuthTag();

    return Buffer.concat([salt, iv, authTag, ciphertext]).toString('base64');
  }

  function decryptSession(value: string): GateKeeperSession | null {
    try {
      const data = Buffer.from(value, 'base64');
      const salt = data.subarray(0, SALT_LENGTH);
      const iv = data.subarray(SALT_LENGTH, SALT_LENGTH + IV_LENGTH);
      const authTag = data.subarray(
        SALT_LENGTH + IV_LENGTH,
        SALT_LENGTH + IV_LENGTH + AUTH_TAG_LENGTH
      );
      const ciphertext = data.subarray(SALT_LENGTH + IV_LENGTH + AUTH_TAG_LENGTH);

      const key = deriveKey(sessionSecret, salt);
      const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
      decipher.setAuthTag(authTag);

      const plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]);
      return JSON.parse(plaintext.toString('utf8')) as GateKeeperSession;
    } catch {
      return null;
    }
  }

  function setSession(cookies: CookiesLike, session: GateKeeperSession): void {
    cookies.set(cookieName, encryptSession(session), cookieOptions);
  }

  function readSession(cookies: CookiesLike): GateKeeperSession | null {
    const value = cookies.get(cookieName);
    if (!value) return null;
    return decryptSession(value);
  }

  function clearSession(cookies: CookiesLike): void {
    cookies.delete(cookieName, { path: '/' });
  }

  return {
    sessionCookieName: cookieName,
    encryptSession,
    decryptSession,
    setSession,
    readSession,
    clearSession
  };
}
