import type { ZodType } from 'zod';

export type ValidationResult<T> =
	| { success: true; data: T }
	| { success: false; errors: Record<string, string> };

/**
 * Validates `data` against a Zod schema and flattens the first error per field
 * into a `{ field: message }` record for easy rendering next to inputs.
 */
export function validate<T>(schema: ZodType<T>, data: unknown): ValidationResult<T> {
	const result = schema.safeParse(data);

	if (result.success) {
		return { success: true, data: result.data };
	}

	const errors: Record<string, string> = {};
	for (const issue of result.error.issues) {
		const key = issue.path[0]?.toString();
		if (key && !errors[key]) {
			errors[key] = issue.message;
		}
	}

	return { success: false, errors };
}

/** Builds the shared OIDC query string forwarded between auth pages. */
export function buildOidcParams(
	source: URLSearchParams,
	extra: Record<string, string> = {}
): URLSearchParams {
	const params = new URLSearchParams({
		redirect_uri: source.get('redirect_uri') || '/',
		response_type: source.get('response_type') || '',
		scope: source.get('scope') || '',
		code_challenge_method: source.get('code_challenge_method') || '',
		code_challenge: source.get('code_challenge') || '',
		state: source.get('state') || ''
	});

	const nonce = source.get('nonce');
	if (nonce) {
		params.set('nonce', nonce);
	}

	const mfaMethods = source.get('mfa_methods');
	if (mfaMethods) {
		params.set('mfa_methods', mfaMethods);
	}

	const mfaDefaultMethod = source.get('mfa_default_method');
	if (mfaDefaultMethod) {
		params.set('mfa_default_method', mfaDefaultMethod);
	}

	// The user's usable second factors (from the login response), so the MFA
	// pages can offer switching to another method mid sign-in.
	const availableMfaMethods = source.get('available_mfa_methods');
	if (availableMfaMethods) {
		params.set('available_mfa_methods', availableMfaMethods);
	}

	for (const [key, value] of Object.entries(extra)) {
		if (value) {
			params.set(key, value);
		}
	}

	return params;
}
