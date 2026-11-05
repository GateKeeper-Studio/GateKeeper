// PUT /api/me/profile — update the signed-in user's profile through GateKeeper.
// The package helper updates the IdP and refreshes the sealed local session profile.
import { json } from '@sveltejs/kit';
import { updateMyProfile } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const PUT: RequestHandler = async ({ locals, cookies, request }) => {
  const body = await request.json();
  const profile = await updateMyProfile(locals, cookies, {
    displayName: body?.displayName ?? body?.display_name ?? null,
    firstName: String(body?.firstName ?? body?.first_name ?? ''),
    lastName: String(body?.lastName ?? body?.last_name ?? '')
  });

  return json(profile);
};
