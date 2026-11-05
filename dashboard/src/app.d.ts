// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces

import type { AdminIdentity } from '$lib/server/session';

declare global {
	namespace App {
		interface Locals {
			/**
			 * The signed-in platform admin, resolved in `hooks.server.ts` from the
			 * session cookie. `null` when signed out.
			 *
			 * The IdP access/refresh tokens deliberately do NOT live here — they never
			 * leave `$lib/server/session`, so nothing can serialise them into a page
			 * payload by accident.
			 */
			admin: AdminIdentity | null;
			/** Opaque session id from the cookie, for the API proxy to resolve tokens. */
			sessionId: string | null;
		}
		// interface Error {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
