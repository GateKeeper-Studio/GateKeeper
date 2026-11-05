// PATCH /api/mfa/methods/:id — rename an enrollment.
// DELETE /api/mfa/methods/:id — remove an enrollment.
import { json } from '@sveltejs/kit';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const PATCH: RequestHandler = async ({ locals, params, request }) => {
  const body = await request.json();
  const renamed = await idpFetch(locals, `/me/mfa/methods/${params.id}`, {
    method: 'PATCH',
    body: JSON.stringify({ name: body?.name ?? null })
  });
  return json(renamed);
};

export const DELETE: RequestHandler = async ({ locals, params }) => {
  await idpFetch(locals, `/me/mfa/methods/${params.id}`, { method: 'DELETE' });
  return new Response(null, { status: 204 });
};
