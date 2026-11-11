import { api } from '../base/api';
import { ApiError, type Result } from '../types';
import type { EMfaType, LoginResponse } from './login';

// QR code sign-in. The browser that wants to sign in shows a code (`startQrLoginApi`)
// and polls (`pollQrLoginApi`); the device that scans it claims it
// (`scanQrLoginApi`) and decides (`approveQrLoginApi` / `denyQrLoginApi`).

type StartQrLoginRequest = {
	clientId: string;
	redirectUri: string;
	responseType: string;
	scope: string;
	state: string;
	codeChallenge: string;
	codeChallengeMethod: string;
	nonce?: string;
};

export type StartQrLoginResponse = {
	requestId: string;
	/** What the QR code encodes: the hosted approval page, with the secret in the fragment. */
	url: string;
	/** The code of the session waiting for the approval. Kept in this page only. */
	sessionCode: string;
	/** ISO-8601. When the code can no longer be scanned. */
	expiresAt: string;
	pollIntervalSeconds: number;
	sessionExpiresAt: string;
	sessionAbsoluteExpiresAt: string;
};

export async function startQrLoginApi(
	request: StartQrLoginRequest
): Promise<Result<StartQrLoginResponse>> {
	try {
		return [await api.post<StartQrLoginResponse>('/v1/auth/qr-login', request), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export type QrLoginStatus = 'pending' | 'scanned' | 'approved' | 'denied' | 'expired';

export type PollQrLoginResponse = {
	status: QrLoginStatus;
	expiresAt?: string;
	/** When approved: the account this browser is now signed in to. */
	email?: string;
	/** When approved: answers like a password login (MFA routing, then authorize). */
	login?: LoginResponse;
};

export async function pollQrLoginApi(request: {
	clientId: string;
	requestId: string;
	sessionCode: string;
	mfaMethods?: EMfaType[];
	mfaDefaultMethod?: EMfaType;
}): Promise<Result<PollQrLoginResponse>> {
	try {
		return [await api.post<PollQrLoginResponse>('/v1/auth/qr-login/status', request), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export type ScanQrLoginResponse = {
	/** Approving or denying needs it; only the first device to scan gets it. */
	claimCode: string;
	clientId: string;
	clientName: string;
	requestedAt: string;
	/** ISO-8601. The decision deadline. */
	expiresAt: string;
	requester: { userAgent: string | null; location: string | null };
	/** What this device signs in with before it approves (the client's own values). */
	signIn?: { redirectUri: string; scope: string };
};

export async function scanQrLoginApi(request: {
	clientId: string;
	requestId: string;
	token: string;
}): Promise<Result<ScanQrLoginResponse>> {
	try {
		return [await api.post<ScanQrLoginResponse>('/v1/auth/qr-login/scan', request), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

type DecisionRequest = {
	clientId: string;
	requestId: string;
	claimCode: string;
	/** Approving only: the sign-in just completed on this device. */
	sessionCode?: string;
};

export async function approveQrLoginApi(request: DecisionRequest): Promise<Result<unknown>> {
	try {
		return [await api.post<unknown>('/v1/auth/qr-login/approve', request), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}

export async function denyQrLoginApi(request: DecisionRequest): Promise<Result<unknown>> {
	try {
		return [await api.post<unknown>('/v1/auth/qr-login/deny', request), null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
