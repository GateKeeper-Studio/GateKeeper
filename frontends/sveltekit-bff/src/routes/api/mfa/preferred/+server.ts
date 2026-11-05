// PUT /api/mfa/preferred — set the default method for future logins.
// Body: { method: "email"|"totp"|"webauthn"|null }
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const PUT: RequestHandler = async ({ locals, request }) => {
  const body = await request.json();
  await idpFetch(locals, '/me/mfa/preferred-method', {
    method: 'PUT',
    body: JSON.stringify({ method: body?.method ?? null })
  });
  return new Response(null, { status: 204 });
};
