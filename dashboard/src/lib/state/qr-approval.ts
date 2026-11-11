import { browser } from '$app/environment';

import type { ScanQrLoginResponse } from '$lib/services/auth/qr-login';

/**
 * QR code sign-in, on the device that scanned the code: what it remembers between the
 * scan and the approval, while the user signs in on this device (several pages).
 *
 * sessionStorage, like the sign-in session itself: it belongs to this tab's sign-in.
 * The claim code it holds is what approving needs; the QR token is not kept.
 */
export type QrApproval = ScanQrLoginResponse & { requestId: string };

/** On the hosted sign-in pages: this sign-in approves the QR code with this request id. */
export const QR_APPROVAL_PARAM = 'qr_approval';

const key = (clientId: string) => `gk_qr_approval:${clientId}`;

export function saveQrApproval(clientId: string, approval: QrApproval) {
	if (!browser) return;
	sessionStorage.setItem(key(clientId), JSON.stringify(approval));
}

/** The pending approval, or null when there is none or its deadline passed. */
export function loadQrApproval(clientId: string): QrApproval | null {
	if (!browser) return null;
	const raw = sessionStorage.getItem(key(clientId));
	if (!raw) return null;
	try {
		const approval = JSON.parse(raw) as QrApproval;
		if (!approval.claimCode || Date.parse(approval.expiresAt) <= Date.now()) {
			clearQrApproval(clientId);
			return null;
		}
		return approval;
	} catch {
		clearQrApproval(clientId);
		return null;
	}
}

export function clearQrApproval(clientId: string) {
	if (!browser) return;
	sessionStorage.removeItem(key(clientId));
}
