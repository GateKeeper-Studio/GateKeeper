import { browser } from '$app/environment';

import type { LoginResponse } from '$lib/services/auth/login';

/**
 * Magic link login: what this browser remembers between "Email me a sign-in link" and
 * the click on the link.
 *
 * `sessionCode` is the same-browser binding. The IdP only redeems a link together with
 * the code of the session that asked for it, so a forwarded link — or a mail scanner
 * that opens it — fails: it has the token but not this code. It lives in localStorage
 * (not sessionStorage) because the link opens in a NEW tab, which must be able to read
 * it. The code binds no user until the link is redeemed, and redemption rotates it.
 *
 * One pending link per client per browser: a new request replaces the previous binding,
 * which is why only the newest email works.
 */
export type MagicLinkBinding = {
	sessionCode: string;
	email: string;
	/** The hosted page's OIDC query string (`buildOidcParams`), to finish the flow. */
	oidcSearch: string;
	/** ISO-8601. When the waiting session (and the link) dies. */
	expiresAt: string;
};

const bindingKey = (clientId: string) => `gk_magic_link:${clientId}`;

export function saveMagicLinkBinding(clientId: string, binding: MagicLinkBinding) {
	if (!browser) return;
	localStorage.setItem(bindingKey(clientId), JSON.stringify(binding));
}

/** The pending binding, or null when there is none or it has expired. */
export function loadMagicLinkBinding(clientId: string): MagicLinkBinding | null {
	if (!browser) return null;
	const raw = localStorage.getItem(bindingKey(clientId));
	if (!raw) return null;
	try {
		const binding = JSON.parse(raw) as MagicLinkBinding;
		if (!binding.sessionCode || Date.parse(binding.expiresAt) <= Date.now()) {
			clearMagicLinkBinding(clientId);
			return null;
		}
		return binding;
	} catch {
		clearMagicLinkBinding(clientId);
		return null;
	}
}

export function clearMagicLinkBinding(clientId: string) {
	if (!browser) return;
	localStorage.removeItem(bindingKey(clientId));
}

/**
 * Hand-off between the tab the link opened and the tab that asked for it. The sign-in
 * must FINISH in the asking tab: the relying party keeps its PKCE verifier and `state`
 * in that tab's sessionStorage, which a new tab cannot read.
 *
 * `requestCode` is the binding's (now rotated, dead) session code. It only tells the
 * asking tabs apart, so a second waiting tab does not pick up someone else's sign-in.
 */
export type MagicLinkMessage =
	| {
			type: 'redeemed';
			requestCode: string;
			email: string;
			oidcSearch: string;
			response: LoginResponse;
	  }
	| { type: 'ack'; requestCode: string };

export function openMagicLinkChannel(clientId: string): BroadcastChannel | null {
	if (!browser || typeof BroadcastChannel === 'undefined') return null;
	return new BroadcastChannel(`gk-magic-link:${clientId}`);
}
