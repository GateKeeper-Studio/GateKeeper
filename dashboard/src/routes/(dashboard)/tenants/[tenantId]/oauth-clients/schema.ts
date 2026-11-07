import { z } from 'zod';

const optionalHttpUrl = z
	.string()
	.trim()
	.refine(
		(value) => {
			if (value === '') return true;
			try {
				const url = new URL(value);
				return url.protocol === 'http:' || url.protocol === 'https:';
			} catch {
				return false;
			}
		},
		{
			message: 'Use an absolute http(s) URL.'
		}
	)
	.transform((value) => value || null);

// initiate_login_uri is a navigation target, not a rendered link, so the IdP holds it to
// the redirect-URI bar: https, or plain http only on loopback for local development.
// Mirrored here so the form fails the same way the backend would, instead of letting the
// admin save and then bounce off a 400.
const optionalSecureUrl = z
	.string()
	.trim()
	.refine(
		(value) => {
			if (value === '') return true;
			try {
				const url = new URL(value);
				if (url.protocol === 'https:') return true;
				return (
					url.protocol === 'http:' &&
					(url.hostname === 'localhost' || url.hostname === '127.0.0.1' || url.hostname === '[::1]')
				);
			} catch {
				return false;
			}
		},
		{
			message: 'Use an absolute https URL (http is allowed for localhost only).'
		}
	)
	.transform((value) => value || null);

// backchannel_logout_uri is a server-to-server target the IdP POSTs to, so the IdP
// applies its SSRF rules (https only; http and private addresses only when the
// operator allows them for local development). The form checks the shape; the
// IdP's 400 explains anything else.
const optionalOutboundUrl = z
	.string()
	.trim()
	.refine(
		(value) => {
			if (value === '') return true;
			try {
				const url = new URL(value);
				return (url.protocol === 'https:' || url.protocol === 'http:') && !value.includes('#');
			} catch {
				return false;
			}
		},
		{
			message: 'Use an absolute https URL without a fragment (#).'
		}
	)
	.transform((value) => value || null);

// Shared schema for the create + edit OAuth client forms. Arrays are pre-filtered
// (empty strings dropped) before validation, so the `.min(1)` checks reflect real
// content. Redirect-URI shape is validated only loosely here — the IdP performs the
// authoritative check (absolute, no fragment, https/loopback/custom scheme), and we
// don't want the form to reject URIs the backend would actually accept.
export const oauthClientFormSchema = z.object({
	name: z
		.string()
		.trim()
		.min(1, 'Name is required.')
		.max(75, 'Name must have at most 75 characters.'),

	clientType: z.enum(['public', 'confidential']),

	redirectUris: z
		.array(z.string())
		.min(1, 'At least one redirect URI is required.')
		.refine((uris) => uris.every((uri) => uri.includes(':')), {
			message: 'Each redirect URI must be an absolute URI (include a scheme).'
		})
		.refine((uris) => uris.every((uri) => !uri.includes('#')), {
			message: 'Redirect URIs must not contain a fragment (#).'
		}),

	postLogoutRedirectUris: z.array(z.string()),

	allowedScopes: z.array(z.string()).min(1, 'Select at least one scope.'),

	allowedGrantTypes: z.array(z.string()).min(1, 'Select at least one grant type.'),

	requirePkce: z.boolean(),
	requireConsent: z.boolean(),
	// Not validated further here: whether the client has an active external provider
	// is server-side state this form doesn't know, so the IdP is the authority and its
	// 400 surfaces as a form error.
	externalLoginOnly: z.boolean(),
	magicLinkEnabled: z.boolean(),
	mfaRequired: z.boolean(),
	mfaEmailEnabled: z.boolean(),
	mfaTotpEnabled: z.boolean(),
	mfaWebauthnEnabled: z.boolean(),
	isActive: z.boolean(),
	emailProviderId: z.string().nullable(),
	tosUri: optionalHttpUrl,
	policyUri: optionalHttpUrl,
	initiateLoginUri: optionalSecureUrl,
	backchannelLogoutUri: optionalOutboundUrl
});

export type OAuthClientFormValues = z.infer<typeof oauthClientFormSchema>;
