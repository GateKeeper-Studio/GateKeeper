// POST /api/mfa/webauthn/start — begin a passkey registration ceremony.
// Returns { mfaId, options }; the ceremony state stays inside the IdP.
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/idp.js';

export async function POST({ locals }) {
  return json(await idpFetch(locals, '/me/mfa/webauthn/start', { method: 'POST' }));
}
