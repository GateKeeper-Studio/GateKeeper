// POST /api/mfa/webauthn/start — begin a passkey registration ceremony.
// Returns { mfaId, options }; the ceremony state stays inside the IdP.
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ locals }) => {
  return json(await idpFetch(locals, '/me/mfa/webauthn/start', { method: 'POST' }));
};
