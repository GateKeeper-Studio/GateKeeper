/**
 * Server-side admin session store.
 *
 * WHY THIS EXISTS: the dashboard used to keep the IdP access token *and* refresh
 * token in `localStorage`, and guard `(dashboard)` routes with a client-side
 * check. Any XSS anywhere in the console therefore meant a silent, self-renewing
 * takeover of a platform-admin account — the refresh token mints new access
 * tokens indefinitely, so revoking one stolen access token achieved nothing.
 *
 * Now the tokens live only here, in the Node process. The browser gets an opaque
 * session id in an httpOnly cookie, which JavaScript cannot read at all.
 *
 * STORAGE: an in-process Map. That is the right shape for the deployment this
 * targets (a single long-lived `node build` process on a VPS, see
 * svelte.config.js) and keeps the tokens off the wire entirely. Two consequences
 * to know about:
 *   - restarting the dashboard signs every admin out. Acceptable for deploys;
 *     they sign back in.
 *   - running more than one dashboard instance needs sticky sessions, or this
 *     Map has to move to a shared store.
 */

import { randomBytes, createHmac, timingSafeEqual } from 'node:crypto';
import type { Cookies } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';

export const SESSION_COOKIE = 'gk_session';

/** How long a dashboard session may live without being used. */
const IDLE_TIMEOUT_MS = 12 * 60 * 60 * 1000; // 12h

/** The admin identity shown in the UI. Carries no credential material. */
export type AdminIdentity = {
	id: string;
	email: string;
	organizationId: string;
	/** Friendly label: the token's `name` claim, else derived from the email. */
	name: string;
	/** 1–2 letter avatar fallback derived from {@link name}. */
	initials: string;
};

type SessionRecord = {
	accessToken: string;
	refreshToken: string;
	admin: AdminIdentity;
	lastSeen: number;
};

const sessions = new Map<string, SessionRecord>();

/**
 * The cookie value is `<id>.<hmac>`. The HMAC is not confidentiality — the id is
 * already opaque and unguessable — it lets us reject forged/truncated cookies
 * without a Map lookup, and makes tampering unambiguous rather than "unknown id".
 */
function secret(): string {
	const value = env.SESSION_SECRET;
	if (!value || value.length < 32) {
		throw new Error(
			'SESSION_SECRET must be set to at least 32 characters (openssl rand -base64 48)'
		);
	}
	return value;
}

/**
 * Boot-time check, called from `hooks.server.ts`.
 *
 * Without it a missing SESSION_SECRET stays invisible until the first sign-in,
 * where it surfaces as an unexplained 500 on POST /session — the failure looks
 * like a broken login, not a missing variable. Failing at startup names the
 * actual cause once, before anyone tries to sign in.
 */
export function assertSessionSecret(): void {
	secret();
}

function sign(id: string): string {
	return createHmac('sha256', secret()).update(id).digest('base64url');
}

function verify(value: string | undefined): string | null {
	if (!value) return null;
	const [id, mac] = value.split('.');
	if (!id || !mac) return null;

	const expected = Buffer.from(sign(id));
	const actual = Buffer.from(mac);
	// timingSafeEqual throws on a length mismatch, so check that first.
	if (expected.length !== actual.length) return null;
	return timingSafeEqual(expected, actual) ? id : null;
}

/** Drops sessions idle past the timeout. Called opportunistically on each read. */
function evictExpired(now: number): void {
	for (const [id, record] of sessions) {
		if (now - record.lastSeen > IDLE_TIMEOUT_MS) sessions.delete(id);
	}
}

export function createSession(
	cookies: Cookies,
	tokens: { accessToken: string; refreshToken: string },
	admin: AdminIdentity
): void {
	const id = randomBytes(32).toString('base64url');
	sessions.set(id, { ...tokens, admin, lastSeen: Date.now() });

	cookies.set(SESSION_COOKIE, `${id}.${sign(id)}`, {
		path: '/',
		// The whole point: script cannot read this.
		httpOnly: true,
		// Lax rather than Strict so following a link into the dashboard keeps the
		// session; the API proxy is same-origin, so this is not a CSRF gap for it.
		sameSite: 'lax',
		// Requires HTTPS. The dashboard runs behind a TLS-terminating proxy in
		// production; over plain http on localhost the browser still accepts it.
		secure: true,
		maxAge: IDLE_TIMEOUT_MS / 1000
	});
}

export function readSession(cookies: Cookies): { id: string; record: SessionRecord } | null {
	const id = verify(cookies.get(SESSION_COOKIE));
	if (!id) return null;

	const now = Date.now();
	evictExpired(now);

	const record = sessions.get(id);
	if (!record) return null;

	record.lastSeen = now;
	return { id, record };
}

/** The tokens for an already-verified session id. Server-only, never serialised. */
export function tokensFor(id: string): { accessToken: string; refreshToken: string } | null {
	const record = sessions.get(id);
	return record ? { accessToken: record.accessToken, refreshToken: record.refreshToken } : null;
}

/** Stores the pair returned by a refresh. Both rotate, so both are replaced. */
export function updateTokens(
	id: string,
	tokens: { accessToken: string; refreshToken: string }
): void {
	const record = sessions.get(id);
	if (!record) return;
	record.accessToken = tokens.accessToken;
	record.refreshToken = tokens.refreshToken;
	record.lastSeen = Date.now();
}

export function destroySession(cookies: Cookies, id: string | null): void {
	if (id) sessions.delete(id);
	cookies.delete(SESSION_COOKIE, { path: '/' });
}

/**
 * Builds the display identity from the access token's claims.
 *
 * The signature is NOT verified here and does not need to be: this token was just
 * received over a server-to-server call to the IdP, and the only thing derived
 * from it is what to render in the sidebar. Every actual authorization decision
 * is made by the IdP, which does verify it.
 */
export function identityFromAccessToken(token: string): AdminIdentity | null {
	const claims = decodeJwtClaims(token);
	if (!claims) return null;

	const email = typeof claims.email === 'string' ? claims.email : '';
	if (!email) return null;

	const claimName = typeof claims.name === 'string' ? claims.name.trim() : '';
	const name = claimName || nameFromEmail(email);

	return {
		id: typeof claims.sub === 'string' ? claims.sub : '',
		email,
		organizationId: typeof claims.org === 'string' ? claims.org : '',
		name,
		initials: initialsFromName(name)
	};
}

function decodeJwtClaims(token: string): Record<string, unknown> | null {
	const payload = token.split('.')[1];
	if (!payload) return null;
	try {
		return JSON.parse(Buffer.from(payload, 'base64url').toString('utf8')) as Record<
			string,
			unknown
		>;
	} catch {
		return null;
	}
}

/** "jane.doe@x.com" → "Jane Doe". */
function nameFromEmail(email: string): string {
	const local = email.split('@')[0] ?? email;
	return (
		local
			.split(/[._-]+/)
			.filter(Boolean)
			.map((part) => part.charAt(0).toUpperCase() + part.slice(1))
			.join(' ') || email
	);
}

function initialsFromName(name: string): string {
	const parts = name.split(/\s+/).filter(Boolean);
	if (parts.length === 0) return '?';
	if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
	return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}
