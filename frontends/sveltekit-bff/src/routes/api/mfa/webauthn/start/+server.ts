// POST /api/mfa/webauthn/start — begin a passkey registration ceremony.
// Returns { mfaId, options }; the ceremony state stays inside the IdP. An account
// without a password must send a `step_up` proof (a passkey signs it in on its own).
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ locals, request }) => {
  const body = await request.json().catch(() => ({}));
  return json(
    await idpFetch(locals, '/me/mfa/webauthn/start', {
      method: 'POST',
      body: JSON.stringify({ step_up: body?.step_up ?? null })
    })
  );
};
