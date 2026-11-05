import { type Handle } from '@sveltejs/kit';
import { sequence } from '@sveltejs/kit/hooks';
import { getTextDirection } from '$lib/paraglide/runtime';
import { paraglideMiddleware } from '$lib/paraglide/server';
import { assertSessionSecret, readSession } from '$lib/server/session';

// Fail at boot, not at the first sign-in, when the session secret is missing.
assertSessionSecret();

const handleParaglide: Handle = ({ event, resolve }) =>
	paraglideMiddleware(event.request, ({ request, locale }) => {
		event.request = request;

		return resolve(event, {
			transformPageChunk: ({ html }) =>
				html
					.replace('%paraglide.lang%', locale)
					.replace('%paraglide.dir%', getTextDirection(locale))
		});
	});

/**
 * Resolves the admin session from the httpOnly cookie onto `event.locals`.
 *
 * Runs before every load function and endpoint, so `+layout.server.ts` can refuse
 * unauthenticated requests *before* anything renders. The tokens themselves stay
 * inside `$lib/server/session`; only the opaque id and the display identity are
 * put on locals, so there is nothing credential-shaped for a load function to
 * accidentally return to the browser.
 */
const handleAuth: Handle = async ({ event, resolve }) => {
	const session = readSession(event.cookies);
	event.locals.sessionId = session?.id ?? null;
	event.locals.admin = session?.record.admin ?? null;

	return resolve(event);
};

/**
 * Response hardening.
 *
 * The IdP sets these on its own JSON responses, but that says nothing about the
 * HTML this app serves — and this app serves both the admin console and the
 * hosted login form, which are exactly the pages worth clickjacking. The CSP is
 * also the defence-in-depth half of moving tokens out of `localStorage`: it
 * shrinks what an injected script could do in the first place.
 *
 * `frame-ancestors 'none'` (plus X-Frame-Options for older agents) is safe even
 * though the OAuth-client appearance editor previews the sign-in page in an
 * iframe: that frame is same-origin, and `frame-ancestors` governs who may frame
 * *us*, which stays "nobody but ourselves" under 'none'... so the preview is
 * served with 'self' instead. Keep the two in sync if that page moves.
 */
const handleSecurityHeaders: Handle = async ({ event, resolve }) => {
	const response = await resolve(event);

	// Only guard documents; adding these to JSON/asset responses is noise.
	const contentType = response.headers.get('content-type') ?? '';
	if (!contentType.includes('text/html')) return response;

	const isAppearancePreview = event.url.searchParams.get('preview') === '1';
	const frameAncestors = isAppearancePreview ? "'self'" : "'none'";

	response.headers.set(
		'content-security-policy',
		[
			"default-src 'self'",
			// SvelteKit inlines a hydration bootstrap script, and the Svelte runtime
			// applies styles at runtime. Tightening these to nonces/hashes is
			// worthwhile follow-up work, not something to guess at here.
			"script-src 'self' 'unsafe-inline'",
			"style-src 'self' 'unsafe-inline'",
			"img-src 'self' data: blob:",
			"font-src 'self' data:",
			// The end-user auth flow calls the IdP cross-origin from the browser
			// (see lib/services/base/api.ts), so it must be connectable.
			`connect-src 'self' ${publicApiOrigin()}`.trim(),
			"object-src 'none'",
			"base-uri 'self'",
			`frame-ancestors ${frameAncestors}`,
			"form-action 'self'"
		].join('; ')
	);

	response.headers.set('x-content-type-options', 'nosniff');
	response.headers.set('x-frame-options', isAppearancePreview ? 'SAMEORIGIN' : 'DENY');
	response.headers.set('referrer-policy', 'no-referrer');
	// Only meaningful over HTTPS; harmless (ignored) over plain http in dev.
	response.headers.set('strict-transport-security', 'max-age=63072000; includeSubDomains');

	return response;
};

// The IdP origin the browser is allowed to reach directly, derived from the same
// public variable the client uses so the two cannot drift apart.
function publicApiOrigin(): string {
	try {
		// Imported lazily to keep this file usable when the variable is unset.
		const raw = process.env.PUBLIC_API_BASE_URL ?? 'http://localhost:8080';
		return new URL(raw).origin;
	} catch {
		return '';
	}
}

export const handle: Handle = sequence(handleSecurityHeaders, handleAuth, handleParaglide);
