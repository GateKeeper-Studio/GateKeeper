// POST /api/mfa/webauthn/finish — complete a passkey registration.
// Body: { mfaId, name?, attestation } where attestation is the JSON form of
// the credential returned by navigator.credentials.create().
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/idp.js';

export async function POST({ locals, request }) {
  const body = await request.json();
  const enrolled = await idpFetch(locals, '/me/mfa/webauthn/finish', {
    method: 'POST',
    body: JSON.stringify({
      mfaId: body?.mfaId,
      name: body?.name ?? null,
      attestation: body?.attestation
    })
  });
  return json(enrolled, { status: 201 });
}
