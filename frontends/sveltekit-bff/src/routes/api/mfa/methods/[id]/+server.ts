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

// The IdP asks for a step-up to remove a factor: `current_password`, or the
// `step_up` proof of an account without a password. Forward whichever came.
export const DELETE: RequestHandler = async ({ locals, params, request }) => {
  const body = await request.json().catch(() => ({}));
  await idpFetch(locals, `/me/mfa/methods/${params.id}`, {
    method: 'DELETE',
    body: JSON.stringify({
      current_password: body?.current_password ?? null,
      step_up: body?.step_up ?? null
    })
  });
  return new Response(null, { status: 204 });
};
