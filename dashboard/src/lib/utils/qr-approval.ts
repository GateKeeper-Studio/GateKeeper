import { goto } from '$app/navigation';
import { resolve } from '$app/paths';

import { approveQrLoginApi } from '$lib/services/auth/qr-login';
import { clearSession } from '$lib/state/auth-session-expiry.svelte';
import { QR_APPROVAL_PARAM, clearQrApproval, loadQrApproval } from '$lib/state/qr-approval';

/** Whether the hosted sign-in on this page is the one that approves a QR code. */
export function isQrApproval(params: URLSearchParams): boolean {
	return params.has(QR_APPROVAL_PARAM);
}

export type QrApprovalOutcome = { kind: 'navigated' } | { kind: 'error'; message: string | null };

/**
 * The sign-in on this device is complete: instead of spending its session for an
 * authorization code here, hand it to the IdP as the proof that approves the other
 * device's sign-in, then show the result.
 */
export async function approveQrSignIn(
	clientId: string,
	sessionCode: string
): Promise<QrApprovalOutcome> {
	const approval = loadQrApproval(clientId);
	if (!approval) {
		clearSession(clientId);
		await goto(resolve(`/auth/[clientId]/qr-approve?result=failed`, { clientId }));
		return { kind: 'navigated' };
	}

	const [, err] = await approveQrLoginApi({
		clientId,
		requestId: approval.requestId,
		claimCode: approval.claimCode,
		sessionCode
	});
	if (err && err.status !== 401) {
		return { kind: 'error', message: err.message || null };
	}

	// Either way the approval is over: done, or the code expired / was used.
	clearQrApproval(clientId);
	clearSession(clientId);
	const result = err ? 'failed' : 'approved';
	await goto(resolve(`/auth/[clientId]/qr-approve?result=${result}`, { clientId }));
	return { kind: 'navigated' };
}

function base64Url(bytes: Uint8Array): string {
	return btoa(String.fromCharCode(...bytes))
		.replace(/\+/g, '-')
		.replace(/\//g, '_')
		.replace(/=+$/, '');
}

/**
 * The OIDC query string for this device's own sign-in. Its state and PKCE challenge
 * are random and thrown away: this sign-in never becomes a code here, it only proves
 * who approves. The redirect URI and scopes are the client's own (from the scan).
 */
export async function approvalSignInParams(
	requestId: string,
	signIn: { redirectUri: string; scope: string }
): Promise<URLSearchParams> {
	const random = (length: number) => crypto.getRandomValues(new Uint8Array(length));
	const verifier = base64Url(random(32));
	const challenge = base64Url(
		new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier)))
	);
	return new URLSearchParams({
		redirect_uri: signIn.redirectUri,
		response_type: 'code',
		scope: signIn.scope,
		code_challenge_method: 'S256',
		code_challenge: challenge,
		state: base64Url(random(16)),
		[QR_APPROVAL_PARAM]: requestId
	});
}
