// GET /api/gatekeeper/session — browser-safe session refresh endpoint.
// Fetching this route lets the package's Svelte provider refresh the httpOnly
// token cookie without exposing raw tokens to the browser.
import { json } from '@sveltejs/kit';
import { gatekeeper } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ locals, cookies }) => {
  const event = { locals, cookies };
  await gatekeeper().loadSession(event);
  return json(gatekeeper().getBrowserSession(event));
};
