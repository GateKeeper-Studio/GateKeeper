<script lang="ts">
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { resendConfirmEmailApi } from '$lib/services/auth/resend-confirm-email';

	const clientId = page.params.clientId as string;
	const email = page.url.searchParams.get('email') || '';

	let isLoading = $state(false);

	async function resendConfirmationToken() {
		isLoading = true;

		const [err] = await resendConfirmEmailApi({ clientId, email });

		if (err) {
			console.error(err);
			toast.error(err.data?.message || err.message);
			isLoading = false;
			return;
		}

		isLoading = false;
		toast.success('Confirmation code sent successfully');
	}
</script>

<div
	class="flex flex-col items-center justify-center rounded-sm bg-gray-50 p-3 shadow-sm dark:bg-gray-900"
>
	<p class="text-sm text-muted-foreground">
		It seems you are trying to sign in. Please confirm your e-mail before signing in.
	</p>

	<button
		type="button"
		onclick={resendConfirmationToken}
		disabled={isLoading}
		class="mt-2 text-sm font-semibold text-primary hover:underline disabled:pointer-events-none"
	>
		{isLoading ? 'Sending...' : 'Click here to resend the confirmation code'}
	</button>
</div>
