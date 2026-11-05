import { api } from '../base/api';
import { ApiError, type Result } from '../types';
import { getLocale } from '$lib/paraglide/runtime';
import type { ResolvedLoginAppearance } from '$lib/types/login-appearance';
import type { CustomFieldDefinitionLike } from '$lib/components/custom-fields/types';

export type ClientAuthData = {
	id: string;
	name: string;
	canSelfSignUp: boolean;
	canSelfForgotPass: boolean;
	tosUri: string | null;
	policyUri: string | null;
	/**
	 * The relying party's own endpoint for starting a fresh authorization request
	 * (OIDC `initiate_login_uri`). We send the browser here when the OAuth state we
	 * were handed has expired: only the RP holds the PKCE code_verifier, so only the
	 * RP can mint a replacement. Null means the client hasn't registered one, and an
	 * expired state is a dead end the user can't self-serve out of.
	 */
	initiateLoginUri: string | null;
	loginAppearance: ResolvedLoginAppearance | null;
	/**
	 * This client authenticates only through `oauthProviders` below: the IdP refuses
	 * every password endpoint for it, so the hosted pages render no email/password
	 * form. `canSelfSignUp` / `canSelfForgotPass` are already forced false server-side
	 * when this is true, so link rendering needs no extra branching.
	 */
	externalLoginOnly: boolean;
	// Which languages this client's hosted pages offer, and which one text is
	// resolved against when a locale isn't specified/available. `loginAppearance`
	// and `customFieldDefinitions` below are already resolved server-side for
	// whichever locale was requested (see `getClientAuthData`).
	allowedLanguages: string[];
	defaultLanguage: string;
	oauthProviders: {
		id: string;
		name: string;
	}[];
	// The tenant's active custom sign-up fields, in display order — rendered as
	// additional inputs on the sign-up form via DynamicFieldsForm.
	customFieldDefinitions: CustomFieldDefinitionLike[];
};

/**
 * Fetches the public auth configuration for a client (name, self-service flags
 * and configured OAuth providers). Pass `event.fetch` when called from a
 * SvelteKit `load` function. Sends the current Paraglide locale so the server
 * resolves per-language text (appearance heading/subtext, custom-field
 * label/description/options) for the locale this page is actually rendering.
 */
export async function getClientAuthData(
	clientId: string,
	fetchFn: typeof fetch = fetch
): Promise<Result<ClientAuthData>> {
	try {
		const data = await api.get<ClientAuthData>(
			`/v1/auth/client/${clientId}/auth-data?locale=${getLocale()}`,
			fetchFn
		);
		return [data, null];
	} catch (error) {
		return [null, error as ApiError];
	}
}
