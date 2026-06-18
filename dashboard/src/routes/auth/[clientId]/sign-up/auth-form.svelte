<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate, buildOidcParams } from '$lib/utils/form';
	import { formSchema } from './schema';
	import { signUpApi } from '$lib/services/auth/sign-up';

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;
	const urlParams = buildOidcParams(searchParams);

	let displayName = $state('');
	let firstName = $state('');
	let lastName = $state('');
	let email = $state('');
	let password = $state('');
	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, { displayName, firstName, lastName, email, password });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const [err] = await signUpApi({
			clientId,
			displayName: result.data.displayName.trim(),
			firstName: result.data.firstName.trim(),
			lastName: result.data.lastName.trim(),
			email: result.data.email.trim(),
			password: result.data.password.trim()
		});

		isLoading = false;

		if (err) {
			console.error(err);
			toast.error(err.data?.message || 'Error creating account');
			return;
		}

		toast.success('Account created successfully');

		await goto(
			resolve(
				`/auth/[clientId]/confirm-email?${urlParams.toString()}&email=${encodeURIComponent(
					result.data.email.trim()
				)}`,
				{ clientId }
			)
		);
	}
</script>

<div class="grid gap-4">
	<form onsubmit={onSubmit} class="space-y-3">
		<div class="space-y-2">
			<Label for="displayName">Display Name</Label>
			<Input
				id="displayName"
				type="text"
				autocomplete="name"
				placeholder="Type your display name"
				bind:value={displayName}
			/>
			{#if errors.displayName}
				<p class="text-sm font-medium text-destructive">{errors.displayName}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="firstName">First Name</Label>
			<Input
				id="firstName"
				type="text"
				autocomplete="given-name"
				placeholder="Type your first name"
				bind:value={firstName}
			/>
			{#if errors.firstName}
				<p class="text-sm font-medium text-destructive">{errors.firstName}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="lastName">Last Name</Label>
			<Input
				id="lastName"
				type="text"
				autocomplete="family-name"
				placeholder="Type your last name"
				bind:value={lastName}
			/>
			{#if errors.lastName}
				<p class="text-sm font-medium text-destructive">{errors.lastName}</p>
			{/if}
		</div>

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

		<div class="space-y-2">
			<Label for="password">Password</Label>
			<Input
				id="password"
				type="password"
				autocomplete="new-password"
				placeholder="********"
				bind:value={password}
			/>
			{#if errors.password}
				<p class="text-sm font-medium text-destructive">{errors.password}</p>
			{/if}
		</div>

		<a
			href={resolve(`/auth/[clientId]/sign-in?${urlParams.toString()}`, { clientId })}
			class="text-md mx-auto flex justify-center text-center font-semibold hover:underline"
		>
			Already has an account? Sign in
		</a>

		<Button type="submit" disabled={isLoading} class="relative w-full">
			{#if isLoading}
				<LoadingSpinner class="absolute left-4" />
			{/if}
			Sign Up with Email
		</Button>
	</form>
</div>
