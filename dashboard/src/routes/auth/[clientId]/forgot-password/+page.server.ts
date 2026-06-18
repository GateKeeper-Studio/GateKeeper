import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ parent, params }) => {
	const { client } = await parent();

	if (!client?.canSelfForgotPass) {
		redirect(307, `/auth/${params.clientId}/sign-in`);
	}
};
