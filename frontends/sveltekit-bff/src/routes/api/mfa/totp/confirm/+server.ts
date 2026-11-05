// POST /api/mfa/totp/confirm — prove possession with a 6-digit code.
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ locals, request }) => {
  const body = await request.json();
  const confirmed = await idpFetch(locals, '/me/mfa/totp/confirm', {
    method: 'POST',
    body: JSON.stringify({ code: String(body?.code ?? '') })
  });
  return json(confirmed);
};
