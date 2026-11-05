<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import UserIcon from '@lucide/svelte/icons/user';
	import MailIcon from '@lucide/svelte/icons/mail';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import EyeOffIcon from '@lucide/svelte/icons/eye-off';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import LockIcon from '@lucide/svelte/icons/lock';
	import SaveIcon from '@lucide/svelte/icons/save';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { validate } from '$lib/utils/form';
	import {
		createTenantUser,
		updateTenantUser,
		type TenantUserView
	} from '$lib/services/admin/users';
	import { createUserFormSchema, editUserFormSchema } from './schema.js';
	import { m } from '$lib/paraglide/messages.js';

	type Props = {
		mode: 'create' | 'edit';
		tenantId: string;
		user?: TenantUserView;
	};

	let { mode, tenantId, user }: Props = $props();

	const queryClient = useQueryClient();
	const listHref = `/tenants/${tenantId}/users`;

	// ---- form state (seeded from `user` in edit mode) ------------------------
	let displayName = $state(user?.display_name ?? '');
	let firstName = $state(user?.first_name ?? '');
	let lastName = $state(user?.last_name ?? '');
	let email = $state(user?.email ?? '');
	let isEmailVerified = $state(false);
	let temporaryPassword = $state('');
	let showPassword = $state(false);

	let errors = $state<Record<string, string>>({});
	let isLoading = $state(false);

	// Live avatar preview — initials and tint update as the admin types.
	const previewName = $derived(
		displayName.trim() ||
			`${firstName} ${lastName}`.trim() ||
			email.split('@')[0] ||
			m.users_form_preview_default_name()
	);
	const previewInitials = $derived(
		(previewName
			.split(/\s+/)
			.filter(Boolean)
			.slice(0, 2)
			.map((p) => p[0])
			.join('') || '?'
		).toUpperCase()
	);

	// Fills a strong random temporary password and reveals it so the admin can
	// hand it off. Uses the crypto RNG, not Math.random.
	function generatePassword() {
		const alphabet = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789!@#$%';
		const bytes = crypto.getRandomValues(new Uint8Array(16));
		temporaryPassword = Array.from(bytes, (b) => alphabet[b % alphabet.length]).join('');
		showPassword = true;
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		if (mode === 'create') {
			const result = validate(createUserFormSchema, {
				displayName: displayName.trim(),
				firstName: firstName.trim(),
				lastName: lastName.trim(),
				email: email.trim(),
				isEmailVerified,
				temporaryPassword
			});

			if (!result.success) {
				errors = result.errors;
				return;
			}
			errors = {};
			isLoading = true;

			const [data, err] = await createTenantUser(tenantId, {
				email: result.data.email,
				displayName: result.data.displayName || null,
				firstName: result.data.firstName,
				lastName: result.data.lastName,
				isEmailVerified: result.data.isEmailVerified,
				temporaryPassword: result.data.temporaryPassword
			});
			isLoading = false;

			if (err) {
				toast.error(err.data?.message || m.users_form_create_error());
				return;
			}

			queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId] });
			toast.success(m.users_form_create_success({ email: data.email }));
			await goto(listHref);
			return;
		}

		// edit mode
		const result = validate(editUserFormSchema, {
			displayName: displayName.trim(),
			firstName: firstName.trim(),
			lastName: lastName.trim()
		});

		if (!result.success) {
			errors = result.errors;
			return;
		}
		errors = {};
		isLoading = true;

		const [err] = await updateTenantUser(tenantId, user!.user_id, {
			displayName: result.data.displayName || null,
			firstName: result.data.firstName,
			lastName: result.data.lastName
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || m.users_form_update_error());
			return;
		}

		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['tenant-users', tenantId, user!.user_id] });
		toast.success(m.users_form_update_success());
		await goto(listHref);
	}
</script>

<form onsubmit={onSubmit} class="mt-4 grid max-w-[700px] gap-6">
	<!-- Live identity preview -->
	<div class="bg-muted/40 flex items-center gap-3 rounded-xl border p-4">
		<span
			class="flex size-12 shrink-0 items-center justify-center rounded-full bg-sky-500/10 text-base font-semibold text-sky-600 dark:text-sky-400"
		>
			{previewInitials}
		</span>
		<div class="min-w-0">
			<p class="truncate font-semibold">{previewName}</p>
			<p class="text-muted-foreground truncate text-sm">
				{email.trim() || m.users_form_no_email_yet()}
			</p>
		</div>
	</div>

	<!-- Identity -->
	<div class="grid gap-4">
		<div class="flex items-center gap-2">
			<UserIcon class="text-muted-foreground size-4" />
			<span class="text-sm font-medium">{m.users_form_section_identity()}</span>
		</div>

		<div class="space-y-2">
			<Label for="displayName">{m.users_form_label_display_name()}</Label>
			<Input
				id="displayName"
				type="text"
				placeholder={m.users_form_placeholder_display_name()}
				autocomplete="name"
				bind:value={displayName}
			/>
			{#if errors.displayName}
				<p class="text-destructive text-sm font-medium">{errors.displayName}</p>
			{/if}
		</div>

		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-2">
				<Label for="firstName" class="flex gap-1">
					{m.users_form_label_first_name()}
					<span class="text-red-500">*</span>
				</Label>
				<Input
					id="firstName"
					type="text"
					placeholder={m.users_form_placeholder_first_name()}
					autocomplete="given-name"
					bind:value={firstName}
				/>
				{#if errors.firstName}
					<p class="text-destructive text-sm font-medium">{errors.firstName}</p>
				{/if}
			</div>

			<div class="space-y-2">
				<Label for="lastName" class="flex gap-1">
					{m.users_form_label_last_name()}
					<span class="text-red-500">*</span>
				</Label>
				<Input
					id="lastName"
					type="text"
					placeholder={m.users_form_placeholder_last_name()}
					autocomplete="family-name"
					bind:value={lastName}
				/>
				{#if errors.lastName}
					<p class="text-destructive text-sm font-medium">{errors.lastName}</p>
				{/if}
			</div>
		</div>
	</div>

	<Separator />

	<!-- Contact -->
	<div class="grid gap-4">
		<div class="flex items-center gap-2">
			<MailIcon class="text-muted-foreground size-4" />
			<span class="text-sm font-medium">{m.users_form_section_contact()}</span>
		</div>

		<div class="space-y-2">
			<Label for="email" class="flex gap-1">
				{m.users_form_label_email()}
				{#if mode === 'create'}<span class="text-red-500">*</span>{/if}
			</Label>
			<Input
				id="email"
				type="email"
				placeholder={m.users_form_placeholder_email()}
				autocomplete="email"
				bind:value={email}
				disabled={mode === 'edit'}
			/>
			{#if mode === 'edit'}
				<p class="text-muted-foreground text-sm">
					{m.users_form_email_readonly_note()}
				</p>
			{:else if errors.email}
				<p class="text-destructive text-sm font-medium">{errors.email}</p>
			{/if}
		</div>

		{#if mode === 'create'}
			<label class="bg-muted/40 flex items-start gap-3 rounded-lg border p-3">
				<Checkbox class="mt-0.5" bind:checked={isEmailVerified} />
				<span class="space-y-0.5">
					<span class="block text-sm font-medium">{m.users_form_email_verified_label()}</span>
					<span class="text-muted-foreground block text-sm">
						{m.users_form_email_verified_description()}
					</span>
				</span>
			</label>
		{/if}
	</div>

	{#if mode === 'create'}
		<Separator />

		<!-- Security -->
		<div class="grid gap-4">
			<div class="flex items-center gap-2">
				<KeyRoundIcon class="text-muted-foreground size-4" />
				<span class="text-sm font-medium">{m.users_form_section_security()}</span>
			</div>

			<div class="space-y-2">
				<Label for="temporaryPassword" class="flex gap-1">
					{m.users_form_label_temp_password()}
					<span class="text-red-500">*</span>
				</Label>
				<div class="flex gap-2">
					<div class="relative flex-1">
						<Input
							id="temporaryPassword"
							type={showPassword ? 'text' : 'password'}
							placeholder={m.users_form_placeholder_temp_password()}
							autocomplete="new-password"
							class="pr-10 font-mono"
							bind:value={temporaryPassword}
						/>
						<button
							type="button"
							onclick={() => (showPassword = !showPassword)}
							class="text-muted-foreground hover:text-foreground absolute top-1/2 right-2 -translate-y-1/2"
							aria-label={showPassword ? m.users_form_hide_password() : m.users_form_show_password()}
						>
							{#if showPassword}<EyeOffIcon class="size-4" />{:else}<EyeIcon class="size-4" />{/if}
						</button>
					</div>
					<Button type="button" variant="outline" onclick={generatePassword}>
						<SparklesIcon />
						{m.users_form_generate_password_button()}
					</Button>
				</div>
				{#if errors.temporaryPassword}
					<p class="text-destructive text-sm font-medium">{errors.temporaryPassword}</p>
				{/if}
				<div
					class="text-muted-foreground flex items-start gap-2 rounded-lg border border-amber-500/20 bg-amber-500/5 p-3 text-sm"
				>
					<LockIcon class="mt-0.5 size-4 shrink-0 text-amber-600 dark:text-amber-400" />
					<span>
						{m.users_form_temp_password_warning()}
					</span>
				</div>
			</div>
		</div>
	{/if}

	<div class="flex justify-end gap-2">
		<Button type="button" variant="outline" href={listHref}>{m.users_action_cancel()}</Button>
		<Button type="submit" disabled={isLoading}>
			{#if isLoading}
				<LoadingSpinner class="size-4" />
			{:else if mode === 'edit'}
				<SaveIcon />
			{/if}
			{#if mode === 'create'}
				{isLoading ? m.users_form_creating_user() : m.users_form_create_user_button()}
			{:else}
				{isLoading ? m.users_form_saving_changes() : m.users_action_save_changes()}
			{/if}
		</Button>
	</div>
</form>
