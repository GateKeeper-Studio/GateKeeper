// POST /api/mfa/step-up — which proof the account gives before a sensitive change,
// and its ceremony: { method: 'password' }, { method: 'passkey', challengeId, options }
// or { method: 'email', challengeId, expiresAt } (the IdP emailed a code).
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ locals }) => {
  return json(await idpFetch(locals, '/me/step-up', { method: 'POST' }));
};
