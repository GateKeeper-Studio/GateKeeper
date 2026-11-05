// GET /api/mfa/methods — list my second factors (proxied to the IdP).
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals }) => {
  return json(await idpFetch(locals, '/me/mfa/methods'));
};
