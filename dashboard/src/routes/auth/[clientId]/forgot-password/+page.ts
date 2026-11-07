import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ parent, params }) => {
	const { client } = await parent();

	if (!client?.canSelfForgotPass) {
		redirect(307, `/auth/${params.clientId}/sign-in`);
	}
};
