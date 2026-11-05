<script lang="ts">
	import { afterNavigate } from '$app/navigation';
	import { createQuery } from '@tanstack/svelte-query';
	import SearchIcon from '@lucide/svelte/icons/search';

	import { Button } from '$lib/components/ui/button/index.js';
	import * as Command from '$lib/components/ui/command/index.js';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { navigationCommands } from '$lib/commands/navigation';
	import { searchProviders } from '$lib/commands/search-providers';
	import { m } from '$lib/paraglide/messages.js';

	let open = $state(false);
	let search = $state('');

	const tenantId = $derived(selectedTenant.tenantId);
	const hasQuery = $derived(search.trim().length > 0);

	// Navigation targets resolved for the current context. Tenant-scoped entries whose
	// href resolves to null (no tenant selected) are hidden in the template.
	const navItems = $derived(
		navigationCommands.map((cmd) => ({
			id: cmd.id,
			label: cmd.label,
			icon: cmd.icon,
			keywords: cmd.keywords,
			href: cmd.href({ tenantId })
		}))
	);

	// Entity search: a single query loads every provider in parallel, keyed (and cached)
	// per tenant, and only fires once the user starts typing. The command's built-in
	// filter then narrows the cached results as they keep typing — no refetch per key.
	const results = createQuery(() => ({
		queryKey: ['command-search', tenantId],
		enabled: open && hasQuery,
		staleTime: 60_000,
		queryFn: async () =>
			Promise.all(
				searchProviders.map(async (provider) => {
					const base = { key: provider.key, heading: provider.heading, icon: provider.icon };
					if (provider.requiresTenant && !tenantId) return { ...base, items: [] };
					try {
						return { ...base, items: await provider.load(tenantId) };
					} catch {
						// One entity failing must not blank the whole palette.
						return { ...base, items: [] };
					}
				})
			)
	}));

	const groups = $derived(results.data ?? []);

	// Selecting an item navigates via its anchor; close the palette once navigation
	// settles. `onSelect` on each item covers same-page selections that don't navigate.
	afterNavigate(() => {
		open = false;
	});

	// Start each session fresh.
	$effect(() => {
		if (!open) search = '';
	});
</script>

<div class="ml-auto flex flex-col gap-4">
	<Button onclick={() => (open = true)} variant="outline" class="w-fit">
		<SearchIcon class="mr-2 size-4" />
		<span class="font-medium text-muted-foreground">{m.search_command_button()}</span>
	</Button>

	<Command.Dialog bind:open>
		<Command.Input placeholder={m.search_command_placeholder()} bind:value={search} />
		<Command.List>
			{#if results.isFetching}
				<Command.Loading>
					<div class="px-3 py-2 text-sm text-muted-foreground">{m.search_command_loading()}</div>
				</Command.Loading>
			{/if}

			{#if !results.isFetching}
				<Command.Empty>{m.search_command_empty()}</Command.Empty>
			{/if}

			<Command.Group heading={m.search_command_navigation_group()}>
				{#each navItems as cmd (cmd.id)}
					{#if cmd.href}
						<Command.LinkItem
							href={cmd.href}
							value={cmd.label}
							keywords={cmd.keywords}
							onSelect={() => (open = false)}
						>
							<cmd.icon />
							<span>{cmd.label}</span>
						</Command.LinkItem>
					{/if}
				{/each}
			</Command.Group>

			{#if hasQuery}
				{#each groups as group (group.key)}
					{#if group.items.length > 0}
						<Command.Group heading={group.heading}>
							{#each group.items as item (item.id)}
								<Command.LinkItem
									href={item.href}
									value={`${item.label} ${item.sublabel ?? ''} ${item.id}`}
									onSelect={() => (open = false)}
								>
									<group.icon />
									<div class="flex min-w-0 flex-col">
										<span class="truncate">{item.label}</span>
										{#if item.sublabel}
											<span class="truncate text-xs text-muted-foreground">{item.sublabel}</span>
										{/if}
									</div>
								</Command.LinkItem>
							{/each}
						</Command.Group>
					{/if}
				{/each}
			{/if}
		</Command.List>
	</Command.Dialog>
</div>
