<script lang="ts">
	import { page } from '$app/state';
	import { afterNavigate } from '$app/navigation';
	import * as Breadcrumb from '$lib/components/ui/breadcrumb/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import { settingsDialog } from '$lib/state/settings-dialog.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { settingsSections, settingsGroups } from './sections.js';

	// The active section's panel. Falls back to the first section if the id is unknown.
	const active = $derived(
		settingsSections.find((s) => s.id === settingsDialog.section) ?? settingsSections[0]
	);
	const activeScreen = $derived(
		active.screens?.find((screen) => screen.id === settingsDialog.screen) ?? null
	);

	// Reflect settings deep links into the dialog after each navigation.
	afterNavigate(() => {
		const params = page.url.searchParams;
		const section = params.get('settings');
		const validSection = settingsSections.some((candidate) => candidate.id === section);
		settingsDialog.syncFromUrl(validSection ? section : null, params.get('screen'));
	});

	function handleOpenChange(open: boolean) {
		if (open) {
			settingsDialog.open = true;
		} else {
			settingsDialog.close();
		}
	}
</script>

<Dialog.Root open={settingsDialog.open} onOpenChange={handleOpenChange}>
	<Dialog.Content
		class="flex h-[90vh] min-h-0 w-full flex-col gap-0 overflow-hidden p-0 sm:max-w-7xl"
	>
		<Dialog.Header class="sr-only">
			<Dialog.Title>{m.settings_dialog_title()}</Dialog.Title>
			<Dialog.Description>{m.settings_dialog_description()}</Dialog.Description>
		</Dialog.Header>

		<div class="flex min-h-0 flex-1 flex-col sm:flex-row">
			<!-- Left rail: sections grouped by `group`. Plain nav (not ui/sidebar) to stay
			     decoupled and avoid nesting a second Sidebar.Provider inside the app's. -->
			<aside
				class="flex shrink-0 gap-1 overflow-x-auto border-b bg-muted/30 p-2 sm:w-56 sm:flex-col sm:gap-0 sm:overflow-x-visible sm:overflow-y-auto sm:border-r sm:border-b-0"
			>
				{#each settingsGroups as group (group.name)}
					<div class="sm:px-1 sm:py-1">
						<p class="hidden px-2 py-1 text-xs font-medium text-muted-foreground sm:block">
							{group.name}
						</p>
						{#each group.items as item (item.id)}
							{@const Icon = item.icon}
							<button
								type="button"
								data-active={settingsDialog.section === item.id}
								class="flex w-full shrink-0 items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm whitespace-nowrap transition-colors hover:bg-accent hover:text-accent-foreground data-[active=true]:bg-accent data-[active=true]:font-medium"
								onclick={() => settingsDialog.select(item.id)}
							>
								<Icon class="size-4 shrink-0" />
								{item.label}
							</button>
						{/each}
					</div>
				{/each}
			</aside>

			<!-- Right pane: only the active section is rendered, so its query fires on first
			     view. `{#key}` forces a fresh mount when the section changes. -->
			<div class="min-h-0 min-w-0 flex-1 overflow-y-auto p-6">
				{#if activeScreen}
					<Breadcrumb.Root class="mb-5">
						<Breadcrumb.List>
							<Breadcrumb.Item>
								<button
									type="button"
									class="text-muted-foreground transition-colors hover:text-foreground"
									onclick={() => settingsDialog.select(active.id)}
								>
									{active.label}
								</button>
							</Breadcrumb.Item>
							<Breadcrumb.Separator />
							<Breadcrumb.Item>
								<Breadcrumb.Page class="font-medium">{activeScreen.label}</Breadcrumb.Page>
							</Breadcrumb.Item>
						</Breadcrumb.List>
					</Breadcrumb.Root>
				{/if}

				{#key active.id}
					{@const Pane = active.component}
					<Pane />
				{/key}
			</div>
		</div>
	</Dialog.Content>
</Dialog.Root>
