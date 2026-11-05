// Runs on every request. Decrypts the session cookie into event.locals.session,
// transparently refreshes the access token shortly before it expires (no client
// timers needed), and stamps a few hardening headers on every response.
import { gatekeeper } from '$lib/server/gatekeeper.js';
import type { Handle } from '@sveltejs/kit';

export const handle: Handle = async ({ event, resolve }) => {
  return gatekeeper().handle({
    event,
    resolve: (resolvedEvent) => resolve(resolvedEvent as typeof event)
  });
};
