<script lang="ts">
	import { goto } from '$app/navigation';
	import { z } from 'zod';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import { adminLogin } from '$lib/services/auth/admin-login';
	import { setAccessToken, setRefreshToken } from '$lib/services/base/api';

	const schema = z.object({
		email: z.email('Invalid e-mail address'),
		password: z.string().min(1, 'Password is required')
	});

	let email = $state('');
	let password = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	function showError(message: string) {
		error = message;
		isLoading = false;
		setTimeout(() => (error = null), 6000);
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(schema, { email, password });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const [data, err] = await adminLogin(result.data.email.trim(), result.data.password);

		if (err || !data) {
			showError(err?.data?.error || err?.data?.message || 'Invalid credentials');
			return;
		}

		setAccessToken(data.access_token);
		setRefreshToken(data.refresh_token);
		await goto('/dashboard');
	}
</script>

<svelte:head>
	<title>Admin Sign In - GateKeeper</title>
	<meta name="description" content="Sign in to the admin dashboard" />
</svelte:head>

<div class="flex min-h-screen items-center justify-center bg-muted/40 p-4">
	<Card.Root class="w-full max-w-sm">
		<Card.Header class="space-y-1 text-center">
			<Card.Title class="text-2xl font-semibold tracking-tight">Admin sign in</Card.Title>
			<Card.Description>Enter your credentials to access the dashboard</Card.Description>
		</Card.Header>

		<Card.Content class="grid gap-4">
			{#if error}
				<ErrorAlert message={error} title="An error occurred..." />
			{/if}

			<form onsubmit={onSubmit} class="space-y-3">
				<div class="space-y-2">
					<Label for="email">E-mail</Label>
					<Input
						id="email"
						type="email"
						autocomplete="email"
						placeholder="admin@example.com"
						bind:value={email}
					/>
					{#if errors.email}
						<p class="text-sm font-medium text-destructive">{errors.email}</p>
					{/if}
				</div>

				<div class="space-y-2">
					<Label for="password">Password</Label>
					<Input
						id="password"
						type="password"
						autocomplete="current-password"
						placeholder="********"
						bind:value={password}
					/>
					{#if errors.password}
						<p class="text-sm font-medium text-destructive">{errors.password}</p>
					{/if}
				</div>

				<Button type="submit" disabled={isLoading} class="relative w-full">
					{#if isLoading}
						<LoadingSpinner class="absolute left-4" />
					{/if}
					Sign In
				</Button>
			</form>
		</Card.Content>
	</Card.Root>
</div>
