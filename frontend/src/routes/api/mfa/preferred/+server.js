// PUT /api/mfa/preferred — set the default method for future logins.
// Body: { method: "email"|"totp"|"webauthn"|null }
import { idpFetch } from '$lib/server/idp.js';

export async function PUT({ locals, request }) {
  const body = await request.json();
  await idpFetch(locals, '/me/mfa/preferred-method', {
    method: 'PUT',
    body: JSON.stringify({ method: body?.method ?? null })
  });
  return new Response(null, { status: 204 });
}
