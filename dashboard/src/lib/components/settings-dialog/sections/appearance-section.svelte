<script lang="ts">
	import * as Card from '$lib/components/ui/card/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import AppearanceForm from './appearance-form.svelte';

	// The active tenant already carries its saved appearance (the sidebar's
	// tenant list includes it), so no extra request is needed here.
	const tenant = $derived(selectedTenant.current);
</script>

<header>
	<h1 class="text-lg font-semibold tracking-tight">{m.settings_appearance_title()}</h1>
	<p class="text-sm text-muted-foreground">
		{m.settings_appearance_description()}
	</p>
</header>

{#if selectedTenant.loading && !tenant}
	<div class="mt-6 flex items-center gap-2 text-muted-foreground">
		<LoadingSpinner />
		{m.settings_appearance_loading()}
	</div>
{:else if !tenant}
	<div class="mt-6 text-sm text-muted-foreground">
		{m.settings_appearance_no_org()}
	</div>
{:else}
	<Card.Root class="mt-6">
		<Card.Header>
			<Card.Title>{m.settings_appearance_theme_card_title()}</Card.Title>
			<Card.Description>
				{m.settings_appearance_theme_card_description({ tenantName: tenant.name })}
			</Card.Description>
		</Card.Header>
		<Card.Content>
			{#key tenant.tenantId}
				<AppearanceForm tenantId={tenant.tenantId} appearance={tenant.appearance ?? null} />
			{/key}
		</Card.Content>
	</Card.Root>
{/if}
