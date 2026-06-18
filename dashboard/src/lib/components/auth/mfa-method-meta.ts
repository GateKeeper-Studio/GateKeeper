import MailIcon from '@lucide/svelte/icons/mail';
import SmartphoneIcon from '@lucide/svelte/icons/smartphone';
import FingerprintIcon from '@lucide/svelte/icons/fingerprint';

import { EMfaType } from '$lib/services/auth/login';

// Single source of truth for how each second factor is presented across the
// auth pages and the "try another method" modal. Centralized so the page header
// and the picker can never drift out of sync (icon, wording, copy).
export type MfaMethodMeta = {
	type: EMfaType;
	/** Short name used in the picker card title and the page badge. */
	label: string;
	/** Prominent page heading — the main signal of which method is in use. */
	heading: string;
	/** Method-specific instruction shown under the heading. */
	instruction: string;
	icon: typeof MailIcon;
	/** Longer explanation shown on the picker card. */
	description: string;
};

export const mfaMethodMeta: Record<EMfaType, MfaMethodMeta> = {
	[EMfaType.MfaEmail]: {
		type: EMfaType.MfaEmail,
		label: 'E-mail',
		heading: 'E-mail Verification',
		instruction: 'Enter the code we sent to your e-mail to sign in',
		icon: MailIcon,
		description:
			'E-mail is a common two-factor authentication method that sends a verification code to your registered e-mail address.'
	},
	[EMfaType.MfaApp]: {
		type: EMfaType.MfaApp,
		label: 'Authenticator App',
		heading: 'Authenticator App',
		instruction: 'Enter the one-time code from your authenticator app to sign in',
		icon: SmartphoneIcon,
		description:
			'Authenticator apps generate time-based one-time codes on your device, providing a secure and convenient way to verify your identity.'
	},
	[EMfaType.MfaWebauthn]: {
		type: EMfaType.MfaWebauthn,
		label: 'Passkey',
		heading: 'Passkey Authentication',
		instruction: 'Use your passkey or security key to verify your identity',
		icon: FingerprintIcon,
		description:
			"Passkeys are a secure and convenient way to authenticate without passwords. They use your device's built-in security features to verify your identity."
	}
};

// Stable display order for the method picker.
export const mfaMethods: MfaMethodMeta[] = [
	mfaMethodMeta[EMfaType.MfaEmail],
	mfaMethodMeta[EMfaType.MfaApp],
	mfaMethodMeta[EMfaType.MfaWebauthn]
];
