// Encrypted session storage for the BFF.
//
// The session (tokens + a small user profile) is sealed with AES-256-GCM and
// stored in a single httpOnly cookie. The browser can never read it; only this
// server, holding SESSION_SECRET, can decrypt it. This is the same scheme as the
// reference client: cookie layout is base64( salt | iv | authTag | ciphertext ).
import crypto from 'node:crypto';
import { env } from '$env/dynamic/private';
import { dev } from '$app/environment';

const ALGORITHM = 'aes-256-gcm';
const SALT_LENGTH = 16;
const IV_LENGTH = 12;
const AUTH_TAG_LENGTH = 16;
const KEY_LENGTH = 32;

export const SESSION_COOKIE = 'gk_session';

// Derive a per-message key from the shared secret + random salt (HKDF-SHA256).
function deriveKey(password, salt) {
  return Buffer.from(crypto.hkdfSync('sha256', Buffer.from(password, 'utf8'), salt, Buffer.alloc(0), KEY_LENGTH));
}

function sessionSecret() {
  const secret = env.SESSION_SECRET;
  if (!secret) throw new Error('SESSION_SECRET is not set');
  return secret;
}

// Seal an arbitrary JSON-serializable object into a base64 cookie value.
export function encryptSession(session) {
  const salt = crypto.randomBytes(SALT_LENGTH);
  const iv = crypto.randomBytes(IV_LENGTH);
  const key = deriveKey(sessionSecret(), salt);

  const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
  const plaintext = Buffer.from(JSON.stringify(session), 'utf8');
  const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const authTag = cipher.getAuthTag();

  return Buffer.concat([salt, iv, authTag, ciphertext]).toString('base64');
}

// Reverse of encryptSession. Returns null on any tampering/decode failure so
// callers treat a bad cookie as "no session" rather than crashing.
export function decryptSession(value) {
  try {
    const data = Buffer.from(value, 'base64');
    const salt = data.subarray(0, SALT_LENGTH);
    const iv = data.subarray(SALT_LENGTH, SALT_LENGTH + IV_LENGTH);
    const authTag = data.subarray(SALT_LENGTH + IV_LENGTH, SALT_LENGTH + IV_LENGTH + AUTH_TAG_LENGTH);
    const ciphertext = data.subarray(SALT_LENGTH + IV_LENGTH + AUTH_TAG_LENGTH);

    const key = deriveKey(sessionSecret(), salt);
    const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
    decipher.setAuthTag(authTag);

    const plaintext = Buffer.concat([decipher.update(ciphertext), decipher.final()]);
    return JSON.parse(plaintext.toString('utf8'));
  } catch {
    return null;
  }
}

const COOKIE_OPTS = {
  httpOnly: true,
  sameSite: 'lax',
  secure: !dev,
  path: '/'
};

// Persist the session into the httpOnly cookie.
export function setSession(cookies, session) {
  cookies.set(SESSION_COOKIE, encryptSession(session), COOKIE_OPTS);
}

// Read + decrypt the current session, or null if absent/invalid.
export function readSession(cookies) {
  const value = cookies.get(SESSION_COOKIE);
  if (!value) return null;
  return decryptSession(value);
}

export function clearSession(cookies) {
  cookies.delete(SESSION_COOKIE, { path: '/' });
}
