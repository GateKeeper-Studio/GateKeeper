// GET /api/mfa/methods — list my second factors (proxied to the IdP).
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/idp.js';

export async function GET({ locals }) {
  return json(await idpFetch(locals, '/me/mfa/methods'));
}
