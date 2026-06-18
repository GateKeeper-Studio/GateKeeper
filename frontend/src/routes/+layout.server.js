// Expose ONLY a browser-safe view of the session to all pages. Tokens stay on the
// server; the client just learns who (if anyone) is signed in.
export function load({ locals }) {
  return { user: locals.session?.profile ?? null };
}
