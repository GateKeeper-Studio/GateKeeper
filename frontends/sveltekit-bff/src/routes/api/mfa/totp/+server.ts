// POST /api/mfa/totp — start an authenticator-app enrollment.
// The IdP returns the otpauth:// URL once; we render the QR server-side so the
// page only ever embeds a data URL (no QR library shipped to the browser).
import { json } from '@sveltejs/kit';
import QRCode from 'qrcode';
import { idpFetch } from '$lib/server/gatekeeper.js';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ locals }) => {
  const started = await idpFetch<{ otpauthUrl: string }>(locals, '/me/mfa/totp', { method: 'POST' });
  const qrDataUrl = await QRCode.toDataURL(started.otpauthUrl, { margin: 1, width: 220 });
  return json({ ...started, qrDataUrl });
};
