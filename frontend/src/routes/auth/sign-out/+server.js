// POST /auth/sign-out — drop the local session and return home.
//
// This clears the BFF session cookie so the browser is immediately logged out.
// (The refresh token will also age out / rotate server-side at the IdP.)
import { redirect } from '@sveltejs/kit';
import { clearSession } from '$lib/server/session.js';

export function POST({ cookies }) {
  clearSession(cookies);
  redirect(303, '/');
}
