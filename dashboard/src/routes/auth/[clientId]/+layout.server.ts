import type { LayoutServerLoad } from './$types';
import { getClientAuthData } from '$lib/services/auth/get-client-auth-data';

/**
 * Loads the public auth data for the client once for every page in the auth
 * flow. Runs only on the server: the browser never calls the API for this data —
 * it receives the result serialized in the page payload. Errors are returned
 * (not thrown) as a plain, serializable object so each page can render an
 * `ErrorAlert` while still showing the layout shell.
 */
export const load: LayoutServerLoad = async ({ params, fetch }) => {
	const [client, error] = await getClientAuthData(params.clientId, fetch);

	return {
		clientId: params.clientId,
		client,
		loadError: error ? { message: error.message } : null
	};
};
