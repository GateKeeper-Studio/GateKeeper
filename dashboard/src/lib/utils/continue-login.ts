import { goto } from '$app/navigation';
import { resolve } from '$app/paths';

import { authorizeApi } from '$lib/services/auth/authorize';
import { EMfaType, type LoginResponse } from '$lib/services/auth/login';
import { storePendingPasswordChange, storeSession } from '$lib/state/auth-session-expiry.svelte';
import { approveQrSignIn, isQrApproval } from '$lib/utils/qr-approval';

export type ContinueLoginContext = {
	clientId: string;
	/** The email the user signed in with; `authorize` cross-checks it. */
	email: string;
	/** The hosted page's OIDC params (`buildOidcParams`). Copied, never mutated. */
	oidcParams: URLSearchParams;
};

export type ContinueLoginOutcome =
	| { kind: 'navigated' }
	| { kind: 'enrollment-required' }
	| { kind: 'session-expired' }
	| { kind: 'error'; message: string | null };

/**
 * Everything after the primary factor succeeded — a password, or a redeemed magic link:
 * keep the session and its clock, route to the MFA or forced-password-change screen
 * when the IdP asks for one, otherwise spend the session for a code and hand it to the
 * relying party.
 */
export async function continueAfterLogin(
	loginData: LoginResponse,
	ctx: ContinueLoginContext
): Promise<ContinueLoginOutcome> {
	const { clientId } = ctx;

	// From here on the session's own clock takes over from the RP's, so the deadlines
	// travel with the code — otherwise the countdown would restart from scratch on each
	// hop to an MFA page and cheerfully over-promise.
	storeSession(
		clientId,
		loginData.sessionCode,
		loginData.sessionExpiresAt,
		loginData.sessionAbsoluteExpiresAt
	);
	// A temporary password signed in: the change code waits beside the session (never
	// in a URL) for whichever page comes after the second factor.
	const mustChangePassword = !!(loginData.changePasswordCode && loginData.userId);
	if (mustChangePassword) {
		storePendingPasswordChange(clientId, {
			code: loginData.changePasswordCode!,
			userId: loginData.userId!
		});
	}

	const urlParams = new URLSearchParams(ctx.oidcParams);
	urlParams.set('mfa_auth_app_required', 'true');
	urlParams.set('email', ctx.email);
	if (loginData.availableMfaMethods?.length) {
		urlParams.set('available_mfa_methods', loginData.availableMfaMethods.join(','));
	}

	const mfaSuffix = loginData.mfaId ? `&mfa_id=${loginData.mfaId}` : '';

	if (loginData.mfaEnrollmentRequired) {
		return { kind: 'enrollment-required' };
	}

	if (loginData.mfaType === EMfaType.MfaEmail) {
		await goto(
			resolve(`/auth/[clientId]/mfa-mail?${urlParams.toString()}${mfaSuffix}`, { clientId })
		);
		return { kind: 'navigated' };
	}

	if (loginData.mfaType === EMfaType.MfaApp) {
		await goto(
			resolve(`/auth/[clientId]/mfa-app?${urlParams.toString()}${mfaSuffix}`, { clientId })
		);
		return { kind: 'navigated' };
	}

	if (loginData.mfaType === EMfaType.MfaWebauthn && loginData.mfaId) {
		// The passkey page reads the options back by mfa id.
		sessionStorage.setItem(
			`webauthn_options_${loginData.mfaId}`,
			JSON.stringify(loginData.webAuthnOptions)
		);
		await goto(
			resolve(`/auth/[clientId]/mfa-webauthn?${urlParams.toString()}${mfaSuffix}`, { clientId })
		);
		return { kind: 'navigated' };
	}

	if (mustChangePassword) {
		await goto(
			resolve(`/auth/[clientId]/update-password?${urlParams.toString()}${mfaSuffix}`, {
				clientId
			})
		);
		return { kind: 'navigated' };
	}

	// Signed in to approve a QR code: the session approves the other device instead of
	// becoming a code here.
	if (isQrApproval(ctx.oidcParams)) {
		return approveQrSignIn(clientId, loginData.sessionCode);
	}

	const redirectUri = ctx.oidcParams.get('redirect_uri') || '/';
	const oidcState = ctx.oidcParams.get('state') || '';
	const [authorizeData, authorizeErr] = await authorizeApi({
		email: ctx.email,
		sessionCode: loginData.sessionCode,
		clientId,
		redirectUri,
		responseType: ctx.oidcParams.get('response_type') || '',
		scope: ctx.oidcParams.get('scope') || '',
		codeChallengeMethod: ctx.oidcParams.get('code_challenge_method') || '',
		codeChallenge: ctx.oidcParams.get('code_challenge') || '',
		state: oidcState,
		nonce: ctx.oidcParams.get('nonce') || undefined,
		mfaId: loginData.mfaId
	});

	if (authorizeErr) {
		console.error(authorizeErr);
		// The server is the source of truth: if it says the session is gone, believe it
		// over a local timer (clock skew, a suspended laptop).
		if (authorizeErr.status === 401) {
			return { kind: 'session-expired' };
		}
		return { kind: 'error', message: authorizeErr.message || null };
	}

	if (!authorizeData) {
		return { kind: 'error', message: null };
	}

	redirectToClient(clientId, authorizeData);
	return { kind: 'navigated' };
}

/**
 * Hands the authorization code to the relying party and leaves the hosted page.
 *
 * SECURITY: the address and the `state` are the ones the IdP answered with. It kept
 * them in the session, checked against the client's registered redirect URIs when the
 * sign-in started. The page's own `redirect_uri` parameter is never the target: it
 * comes from the address bar (or, after a social sign-in, from the URL fragment), which
 * anyone can write a link for.
 */
export function redirectToClient(
	clientId: string,
	authorized: { authorizationCode: string; redirectUri: string; state: string }
): void {
	const { redirectUri } = authorized;
	// A registration made before these schemes were refused must not run as script here.
	if (/^\s*(javascript|data|vbscript|blob|file):/i.test(redirectUri)) {
		throw new Error('refusing to navigate to this redirect URI');
	}
	const params = new URLSearchParams({
		code: authorized.authorizationCode,
		state: authorized.state,
		redirect_uri: redirectUri,
		client_id: clientId
	});
	const separator = redirectUri.includes('?') ? '&' : '?';
	window.location.href = `${redirectUri}${separator}${params.toString()}`;
}

const MFA_TYPES = [EMfaType.MfaWebauthn, EMfaType.MfaApp, EMfaType.MfaEmail];

/** The relying party's optional MFA hints, carried in the hosted page's OIDC params. */
export function mfaHintsFrom(oidcParams: URLSearchParams): {
	mfaMethods?: EMfaType[];
	mfaDefaultMethod?: EMfaType;
} {
	const mfaMethods = (oidcParams.get('mfa_methods') || '')
		.split(',')
		.map((method) => method.trim())
		.filter((method): method is EMfaType => MFA_TYPES.includes(method as EMfaType));
	const mfaDefaultMethod = oidcParams.get('mfa_default_method') || '';
	return {
		mfaMethods: mfaMethods.length ? mfaMethods : undefined,
		mfaDefaultMethod: MFA_TYPES.includes(mfaDefaultMethod as EMfaType)
			? (mfaDefaultMethod as EMfaType)
			: undefined
	};
}
