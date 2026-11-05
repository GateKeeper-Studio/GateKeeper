<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from 'svelte-sonner';
	import { useQueryClient } from '@tanstack/svelte-query';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import SaveIcon from '@lucide/svelte/icons/save';

	import { updateTenant, type TenantDetail } from '$lib/services/admin/tenants';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { LOCALES } from '$lib/i18n/locales';
	import { m } from '$lib/paraglide/messages.js';

	let { tenant }: { tenant: TenantDetail } = $props();

	const queryClient = useQueryClient();

	let name = $state(tenant.name);
	let description = $state(tenant.description ?? '');
	let isActive = $state(tenant.is_active);
	let canSelfRegister = $state(tenant.can_self_register);
	let canSelfForgotPassword = $state(tenant.can_self_forgot_password);
	let defaultLanguage = $state(tenant.default_language);

	const defaultLanguageLabel = $derived(
		LOCALES.find((l) => l.code === defaultLanguage)?.nativeLabel ?? defaultLanguage
	);

	let error = $state('');
	let isLoading = $state(false);

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		if (name.trim().length === 0) {
			error = m.tenant_name_required();
			return;
		}
		error = '';
		isLoading = true;

		const [err] = await updateTenant(tenant.tenant_id, {
			name: name.trim(),
			description: description.trim() || null,
			isActive,
			canSelfRegister,
			canSelfForgotPassword,
			defaultLanguage
		});
		isLoading = false;

		if (err) {
			toast.error(err.data?.message || err.message || m.tenant_update_failed());
			return;
		}

		// Reflect the new name/status in the sidebar list right away.
		selectedTenant.tenants = selectedTenant.tenants.map((t) =>
			t.tenantId === tenant.tenant_id
				? {
						...t,
						name: name.trim(),
						description: description.trim() || null,
						isActive,
						canSelfRegister,
						canSelfForgotPassword,
						defaultLanguage
					}
				: t
		);
		queryClient.invalidateQueries({ queryKey: ['tenant', tenant.tenant_id] });
		queryClient.invalidateQueries({ queryKey: ['tenants'] });

		toast.success(m.tenant_updated());
		await goto(`/tenants/${tenant.tenant_id}`);
	}
</script>

<form onsubmit={onSubmit} class="grid max-w-[640px] gap-4">
	<div class="space-y-2">
		<Label for="name" class="flex gap-1">
			{m.tenant_name_label()}
			<span class="text-red-500">*</span>
		</Label>
		<Input id="name" type="text" placeholder={m.tenant_name_placeholder()} bind:value={name} />
		{#if error}<p class="text-destructive text-sm font-medium">{error}</p>{/if}
	</div>

	<div class="space-y-2">
		<Label for="description">{m.tenant_description_label()}</Label>
		<Textarea
			id="description"
			placeholder={m.tenant_description_placeholder()}
			bind:value={description}
		/>
	</div>

	<Separator class="my-2" />

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={isActive} />
		{m.tenant_active_label()}
		<span class="text-muted-foreground">{m.tenant_inactive_hint()}</span>
	</label>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfRegister} />
		{m.tenant_allow_self_register()}
	</label>

	<label class="flex items-center gap-2 text-sm">
		<Checkbox bind:checked={canSelfForgotPassword} />
		{m.tenant_allow_self_forgot_password()}
	</label>

	<Separator class="my-2" />

	<div class="space-y-2">
		<Label>{m.tenant_default_language_label()}</Label>
		<Select.Root type="single" value={defaultLanguage} onValueChange={(v) => v && (defaultLanguage = v)}>
			<Select.Trigger class="w-full max-w-xs">{defaultLanguageLabel}</Select.Trigger>
			<Select.Content>
				{#each LOCALES as locale (locale.code)}
					<Select.Item value={locale.code} label={locale.nativeLabel}>{locale.nativeLabel}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
		<p class="text-sm text-muted-foreground">{m.tenant_default_language_hint()}</p>
	</div>

	<div class="flex justify-end">
		<Button type="submit" disabled={isLoading} class="w-fit">
			{#if isLoading}<LoadingSpinner class="size-4" />{:else}<SaveIcon />{/if}
			{isLoading ? m.tenant_saving() : m.tenant_save_changes()}
		</Button>
	</div>
</form>
