// Expose ONLY a browser-safe view of the session to all pages. Tokens stay on the
// server; the client just learns who (if anyone) is signed in.
import type { LayoutServerLoad } from './$types';
import { gatekeeper } from '$lib/server/gatekeeper.js';

export const load: LayoutServerLoad = ({ locals }) => {
  const session = gatekeeper().getBrowserSession(locals);
  return {
    user: session.user,
    gatekeeper: session
  };
};
