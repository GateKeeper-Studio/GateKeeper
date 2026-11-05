<script lang="ts">
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import MailIcon from '@lucide/svelte/icons/mail';

	import * as Select from '$lib/components/ui/select/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { listEmailProviders } from '$lib/services/admin/email-providers';
	import { assignEmailProvider, type OAuthClientView } from '$lib/services/admin/oauth-clients';
	import { m } from '$lib/paraglide/messages.js';

	type Props = { tenantId: string; client: OAuthClientView };
	let { tenantId, client }: Props = $props();

	const queryClient = useQueryClient();

	const providersQuery = createQuery(() => ({
		queryKey: ['email-providers', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listEmailProviders(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));

	// Empty string means "Tenant default" (email_provider_id = null).
	let value = $state(client.email_provider_id ?? '');
	let saving = $state(false);

	// Keep the local selection in sync if the client query refreshes underneath us.
	$effect(() => {
		value = client.email_provider_id ?? '';
	});

	const label = $derived(
		value
			? (providersQuery.data?.find((p) => p.id === value)?.name ?? m.oauth_clients_email_custom_provider())
			: m.oauth_clients_tenant_default()
	);

	async function onValueChange(next: string) {
		const normalized = next || null;
		if (normalized === (client.email_provider_id ?? null)) return;

		const previous = value;
		value = next;
		saving = true;
		const [err] = await assignEmailProvider(tenantId, client, normalized);
		saving = false;

		if (err) {
			value = previous;
			toast.error(err.data?.message || m.oauth_clients_email_provider_update_error());
			return;
		}

		toast.success(m.oauth_clients_email_provider_updated());
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId, client.id] });
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId] });
	}
</script>

<div class="flex items-center gap-3">
	<span
		class="bg-muted text-muted-foreground flex size-9 shrink-0 items-center justify-center rounded-lg"
	>
		<MailIcon class="size-4" />
	</span>
	<div class="min-w-0 flex-1">
		<p class="text-muted-foreground text-xs">{m.oauth_clients_email_delivery_label()}</p>
		<Select.Root type="single" {value} {onValueChange} disabled={saving}>
			<Select.Trigger class="mt-1 h-8 w-full max-w-xs">
				{label}
			</Select.Trigger>
			<Select.Content>
				<Select.Item value="" label={m.oauth_clients_tenant_default()}>{m.oauth_clients_tenant_default()}</Select.Item>
				{#each providersQuery.data ?? [] as provider (provider.id)}
					<Select.Item value={provider.id} label={provider.name}>{provider.name}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
	</div>
	{#if saving}
		<LoadingSpinner class="text-muted-foreground shrink-0" />
	{/if}
</div>
