<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import DynamicFieldsForm from '$lib/components/custom-fields/dynamic-fields-form.svelte';
	import { buildCustomFieldsZodSchema } from '$lib/components/custom-fields/schema';
	import type { CustomFieldDefinitionLike } from '$lib/components/custom-fields/types';

	import { validate, buildOidcParams } from '$lib/utils/form';
	import { formSchema } from './schema';
	import { signUpApi } from '$lib/services/auth/sign-up';
	import { m } from '$lib/paraglide/messages.js';

	let { customFieldDefinitions = [] }: { customFieldDefinitions?: CustomFieldDefinitionLike[] } =
		$props();

	const clientId = page.params.clientId as string;
	const searchParams = page.url.searchParams;
	const urlParams = buildOidcParams(searchParams);

	let displayName = $state('');
	let firstName = $state('');
	let lastName = $state('');
	let email = $state('');
	let password = $state('');
	let customFieldValues = $state<Record<string, unknown>>({});
	let errors = $state<Record<string, string>>({});
	let customFieldErrors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, { displayName, firstName, lastName, email, password });
		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};

		let validatedCustomFieldValues: Record<string, unknown> | undefined;
		if (customFieldDefinitions.length > 0) {
			const customResult = validate(
				buildCustomFieldsZodSchema(customFieldDefinitions),
				customFieldValues
			);
			if (!customResult.success) {
				customFieldErrors = customResult.errors;
				return;
			}
			validatedCustomFieldValues = customResult.data;
		}
		customFieldErrors = {};
		isLoading = true;

		const [err] = await signUpApi({
			clientId,
			displayName: result.data.displayName.trim(),
			firstName: result.data.firstName.trim(),
			lastName: result.data.lastName.trim(),
			email: result.data.email.trim(),
			password: result.data.password.trim(),
			customFieldValues: validatedCustomFieldValues
		});

		isLoading = false;

		if (err) {
			console.error(err);
			toast.error(err.message || m.signup_error());
			return;
		}

		toast.success(m.signup_success());

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
			<Label for="displayName">{m.signup_display_name_label()}</Label>
			<Input
				id="displayName"
				type="text"
				autocomplete="name"
				placeholder={m.signup_display_name_placeholder()}
				bind:value={displayName}
			/>
			{#if errors.displayName}
				<p class="text-sm font-medium text-destructive">{errors.displayName}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="firstName">{m.signup_first_name_label()}</Label>
			<Input
				id="firstName"
				type="text"
				autocomplete="given-name"
				placeholder={m.signup_first_name_placeholder()}
				bind:value={firstName}
			/>
			{#if errors.firstName}
				<p class="text-sm font-medium text-destructive">{errors.firstName}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="lastName">{m.signup_last_name_label()}</Label>
			<Input
				id="lastName"
				type="text"
				autocomplete="family-name"
				placeholder={m.signup_last_name_placeholder()}
				bind:value={lastName}
			/>
			{#if errors.lastName}
				<p class="text-sm font-medium text-destructive">{errors.lastName}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="email">{m.auth_email_label()}</Label>
			<Input
				id="email"
				type="email"
				autocomplete="email"
				placeholder={m.auth_email_placeholder()}
				bind:value={email}
			/>
			{#if errors.email}
				<p class="text-sm font-medium text-destructive">{errors.email}</p>
			{/if}
		</div>

		<div class="space-y-2">
			<Label for="password">{m.auth_password_label()}</Label>
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

		{#if customFieldDefinitions.length > 0}
			<DynamicFieldsForm
				definitions={customFieldDefinitions}
				bind:values={customFieldValues}
				errors={customFieldErrors}
			/>
		{/if}

		<a
			href={resolve(`/auth/[clientId]/sign-in?${urlParams.toString()}`, { clientId })}
			class="text-md mx-auto flex justify-center text-center font-semibold hover:underline"
		>
			{m.signup_already_have_account()}
		</a>

		<Button type="submit" disabled={isLoading} class="relative w-full">
			{#if isLoading}
				<LoadingSpinner class="absolute left-4" />
			{/if}
			{m.signup_submit()}
		</Button>
	</form>
</div>
