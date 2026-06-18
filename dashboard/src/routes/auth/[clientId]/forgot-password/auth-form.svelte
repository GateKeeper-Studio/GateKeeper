<script lang="ts">
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';
	import CheckCircleIcon from '@lucide/svelte/icons/circle-check';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { formSchema } from './schema';
	import { forgotPasswordApi } from '$lib/services/auth/forgot-password';

	const clientId = page.params.clientId as string;

	let email = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);
	let isSent = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, { email });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};

		if (!clientId) {
			toast.error('Application ID is required');
			return;
		}

		isLoading = true;

		const [err] = await forgotPasswordApi({
			clientId,
			email: result.data.email
		});

		if (err) {
			console.error(err);
			toast.error(err.data?.message || err.message);
			isLoading = false;
			return;
		}

		isLoading = false;
		isSent = true;
	}
</script>

{#if isSent}
	<div class="grid gap-6">
		<Alert.Root variant="default" class="bg-green-100 dark:bg-green-700">
			<CheckCircleIcon class="h-5 w-5 text-green-500 dark:text-green-400" />
			<Alert.Title>E-mail sent</Alert.Title>

			<Alert.Description>
				Check your email for a link to reset your password. If it doesn't appear within a few
				minutes, check your spam folder.
			</Alert.Description>
		</Alert.Root>
	</div>

	<Button onclick={() => (isSent = false)} class="mt-4">Go Back</Button>
{:else}
	<div class="grid gap-4">
		<form onsubmit={onSubmit} class="space-y-3">
			<div class="space-y-2">
				<Label for="email">E-mail</Label>
				<Input
					id="email"
					type="email"
					autocomplete="email"
					placeholder="example@email.com"
					bind:value={email}
				/>
				{#if errors.email}
					<p class="text-sm font-medium text-destructive">{errors.email}</p>
				{/if}
			</div>

			<Button type="submit" disabled={isLoading} class="relative w-full">
				{#if isLoading}
					<LoadingSpinner class="absolute left-4" />
				{/if}
				Send Mail
			</Button>
		</form>
	</div>
{/if}
