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
	mfaRequired: z.boolean(),
	mfaEmailEnabled: z.boolean(),
	mfaTotpEnabled: z.boolean(),
	mfaWebauthnEnabled: z.boolean(),
	isActive: z.boolean(),
	emailProviderId: z.string().nullable(),
	tosUri: optionalHttpUrl,
	policyUri: optionalHttpUrl
});

export type OAuthClientFormValues = z.infer<typeof oauthClientFormSchema>;
