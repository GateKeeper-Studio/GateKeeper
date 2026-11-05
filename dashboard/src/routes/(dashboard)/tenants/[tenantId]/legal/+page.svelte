<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { MarkdownEditor } from 'carta-md';
	import 'carta-md/default.css';

	import SaveIcon from '@lucide/svelte/icons/save';
	import { carta } from '$lib/carta';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import Separator from '$lib/components/ui/separator/separator.svelte';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import {
		updateLegalDocument,
		resetLegalDocument,
		type LegalDocType,
		type LegalDocumentView
	} from '$lib/services/admin/legal';
	import { m } from '$lib/paraglide/messages.js';

	let { data } = $props();
	const tenantId: string = data.tenantId;

	type DocState = {
		content: string;
		customized: boolean;
		updatedAt: string | null;
		saving: boolean;
		resetting: boolean;
	};

	function initial(doc: LegalDocumentView | null): DocState {
		return {
			content: doc?.content ?? '',
			customized: doc?.isCustomized ?? false,
			updatedAt: doc?.updatedAt ?? null,
			saving: false,
			resetting: false
		};
	}

	let docs = $state<Record<LegalDocType, DocState>>({
		terms: initial(data.terms),
		privacy: initial(data.privacy)
	});

	const LABELS = $derived<Record<LegalDocType, string>>({
		terms: m.tenant_terms_of_service(),
		privacy: m.tenant_privacy_policy()
	});

	// The Carta editor is interactive (highlighter, history), so render it only on
	// the client to avoid SSR/hydration mismatches.
	let mounted = $state(false);
	onMount(() => {
		mounted = true;
	});

	async function save(type: LegalDocType) {
		docs[type].saving = true;
		const [view, err] = await updateLegalDocument(tenantId, type, docs[type].content);
		docs[type].saving = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		docs[type].customized = view.isCustomized;
		docs[type].updatedAt = view.updatedAt;
		toast.success(m.tenant_legal_saved({ name: LABELS[type] }));
	}

	async function resetToDefault(type: LegalDocType) {
		if (!confirm(m.tenant_legal_reset_confirm({ name: LABELS[type] }))) {
			return;
		}

		docs[type].resetting = true;
		const [view, err] = await resetLegalDocument(tenantId, type);
		docs[type].resetting = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		docs[type].content = view.content;
		docs[type].customized = view.isCustomized;
		docs[type].updatedAt = view.updatedAt;
		toast.success(m.tenant_legal_reset_success({ name: LABELS[type] }));
	}
</script>

<svelte:head><title>{m.tenant_legal_page_title()} - GateKeeper</title></svelte:head>

{#snippet docEditor(type: LegalDocType)}
	<div class="mt-4 flex flex-col gap-3">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<span class="text-sm text-muted-foreground">
				{#if docs[type].customized}
					{m.tenant_legal_customized()}{#if docs[type].updatedAt}
						{m.tenant_legal_last_updated({
							date: new Date(docs[type].updatedAt).toLocaleString()
						})}{/if}
				{:else}
					{m.tenant_legal_showing_default()}
				{/if}
			</span>

			<Button
				variant="outline"
				size="sm"
				disabled={!docs[type].customized || docs[type].resetting}
				onclick={() => resetToDefault(type)}
			>
				{docs[type].resetting ? m.tenant_legal_resetting() : m.tenant_legal_reset_to_default()}
			</Button>
		</div>

		<div class="legal-editor bg-background overflow-hidden rounded-lg border">
			{#if mounted}
				<MarkdownEditor {carta} mode="tabs" bind:value={docs[type].content} />
			{:else}
				<div class="flex h-96 items-center justify-center text-sm text-muted-foreground">
					{m.tenant_legal_loading_editor()}
				</div>
			{/if}
		</div>

		<div class="flex justify-end">
			<Button disabled={docs[type].saving} onclick={() => save(type)}>
				{#if docs[type].saving}<LoadingSpinner class="size-4" />{:else}<SaveIcon />{/if}
				{docs[type].saving ? m.tenant_legal_saving() : m.tenant_save_changes()}
			</Button>
		</div>
	</div>
{/snippet}

<DashboardHeader
	breadcrumbs={{
		items: [
			{
				name: selectedTenant.current?.name ?? m.sidebar_tenant(),
				path: `/tenants/${tenantId}`
			},
			{ name: m.tenant_legal_page_title(), path: `/tenants/${tenantId}/legal` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.tenant_back_to_tenant_detail(), href: `/tenants/${tenantId}` }}
	title={m.tenant_legal_page_title()}
	description={m.tenant_legal_page_description()}
>
	<Separator class="my-4" />

	{#if data.loadError}
		<Alert.Root variant="destructive" class="mb-4">
			<Alert.Title>{m.tenant_legal_load_error_title()}</Alert.Title>
			<Alert.Description>{data.loadError.message}</Alert.Description>
		</Alert.Root>
	{/if}

	<Tabs.Root value="terms" class="w-full">
		<Tabs.List>
			<Tabs.Trigger value="terms">{m.tenant_terms_of_service()}</Tabs.Trigger>
			<Tabs.Trigger value="privacy">{m.tenant_privacy_policy()}</Tabs.Trigger>
		</Tabs.List>

		<Tabs.Content value="terms">{@render docEditor('terms')}</Tabs.Content>
		<Tabs.Content value="privacy">{@render docEditor('privacy')}</Tabs.Content>
	</Tabs.Root>
</MainContent>

<style>
	/* Carta ships a light-only theme that hardcodes its colours and never sets a
	   surface background. Instead of patching per element, remap Carta's own theme
	   variables onto the shadcn tokens — because those tokens already swap between
	   light and dark, this makes the editor correct in both modes at once. */
	.legal-editor :global(.carta-theme__default) {
		--border-color: var(--border);
		--hover-color: var(--accent);
		--text-color: var(--foreground);
		--caret-color: var(--foreground);
		--focus-outline: var(--ring);
		--selection-color: color-mix(in oklch, var(--primary) 22%, transparent);
	}

	/* The wrapper already provides the shadcn border + radius; drop Carta's so we
	   don't render a double border. */
	.legal-editor :global(.carta-editor) {
		border: none;
		border-radius: 0;
	}

	/* Toolbar → muted surface, consistent with shadcn inputs. */
	.legal-editor :global(.carta-toolbar) {
		background: var(--muted);
		border-bottom: 1px solid var(--border);
		min-height: 2.75rem;
	}

	/* Write / Preview tabs. */
	.legal-editor :global(.carta-toolbar-left button) {
		color: var(--muted-foreground);
	}
	.legal-editor :global(.carta-toolbar-left button.carta-active) {
		color: var(--foreground);
		border-bottom-color: var(--primary);
	}

	/* Formatting icon buttons + their dropdown menu. */
	.legal-editor :global(.carta-icon),
	.legal-editor :global(.carta-icon-full) {
		color: var(--muted-foreground);
		border-radius: calc(var(--radius) * 0.6);
	}
	.legal-editor :global(.carta-icon:hover),
	.legal-editor :global(.carta-icon-full:hover) {
		background: var(--accent);
		color: var(--accent-foreground);
	}
	.legal-editor :global(.carta-icons-menu) {
		background: var(--popover);
		color: var(--popover-foreground);
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) * 0.8);
	}

	/* Panes: one shadcn surface, a sensible resizable height (Carta's default is a
	   fixed 600px with an always-on scrollbar), and no double horizontal padding. */
	.legal-editor :global(.carta-wrapper) {
		padding: 0;
	}
	.legal-editor :global(.carta-input),
	.legal-editor :global(.carta-renderer) {
		height: auto;
		min-height: 24rem;
		max-height: 34rem;
		overflow-y: auto;
		padding: 1rem;
		background: var(--background);
		color: var(--foreground);
	}

	/* Keep the transparent input <textarea> and the highlighted <pre> overlay on the
	   exact same metrics — Tailwind's preflight otherwise gives the textarea the sans
	   body font while the <pre> stays monospace, drifting the caret off the text. */
	.legal-editor :global(.carta-input textarea),
	.legal-editor :global(.carta-input pre),
	.legal-editor :global(.carta-input pre *) {
		font-family: ui-monospace, 'SF Mono', 'Cascadia Code', 'JetBrains Mono', Menlo, monospace;
		font-size: 0.9rem;
		line-height: 1.65;
		tab-size: 2;
	}

	/* Carta highlights the source (and fenced code) with Shiki's dual light/dark
	   themes, emitting the dark palette as `--shiki-dark` custom properties. Shiki
	   leaves the light palette active inline and expects the app to opt in for
	   class-based dark mode — otherwise the input pane shows dark light-theme tokens
	   on the dark surface and is unreadable. This is that opt-in. */
	:global(.dark) .legal-editor :global(.shiki),
	:global(.dark) .legal-editor :global(.shiki *) {
		color: var(--shiki-dark) !important;
	}

	/* Preview typography — preflight strips these, so the rendered Markdown looks flat
	   without them. Kept small and token-driven to match the rest of the dashboard. */
	.legal-editor :global(.carta-renderer) {
		font-size: 0.925rem;
		line-height: 1.7;
	}
	.legal-editor :global(.carta-renderer h1),
	.legal-editor :global(.carta-renderer h2),
	.legal-editor :global(.carta-renderer h3),
	.legal-editor :global(.carta-renderer h4) {
		font-weight: 600;
		line-height: 1.3;
		margin: 1.4em 0 0.6em;
	}
	.legal-editor :global(.carta-renderer h1) {
		font-size: 1.55rem;
	}
	.legal-editor :global(.carta-renderer h2) {
		font-size: 1.3rem;
	}
	.legal-editor :global(.carta-renderer h3) {
		font-size: 1.1rem;
	}
	.legal-editor :global(.carta-renderer h1),
	.legal-editor :global(.carta-renderer h2) {
		border-bottom: 1px solid var(--border);
		padding-bottom: 0.3em;
	}
	.legal-editor :global(.carta-renderer p) {
		margin: 0.75em 0;
	}
	.legal-editor :global(.carta-renderer ul),
	.legal-editor :global(.carta-renderer ol) {
		margin: 0.75em 0;
		padding-left: 1.5rem;
	}
	.legal-editor :global(.carta-renderer ul) {
		list-style: disc;
	}
	.legal-editor :global(.carta-renderer ol) {
		list-style: decimal;
	}
	.legal-editor :global(.carta-renderer li) {
		margin: 0.3em 0;
	}
	.legal-editor :global(.carta-renderer a) {
		color: var(--primary);
		text-decoration: underline;
		text-underline-offset: 2px;
	}
	.legal-editor :global(.carta-renderer blockquote) {
		border-left: 3px solid var(--border);
		padding-left: 1rem;
		margin: 1em 0;
		color: var(--muted-foreground);
	}
	.legal-editor :global(.carta-renderer code) {
		background: var(--muted);
		padding: 0.15em 0.35em;
		border-radius: calc(var(--radius) * 0.6);
		font-size: 0.85em;
		font-family: ui-monospace, 'SF Mono', Menlo, monospace;
	}
	.legal-editor :global(.carta-renderer pre) {
		/* Beats Shiki's inline background so fenced code matches the app surface. */
		background: var(--muted) !important;
		padding: 1rem;
		border-radius: calc(var(--radius) * 0.8);
		overflow-x: auto;
		margin: 1em 0;
	}
	.legal-editor :global(.carta-renderer pre code) {
		background: transparent;
		padding: 0;
	}
	.legal-editor :global(.carta-renderer hr) {
		border: none;
		border-top: 1px solid var(--border);
		margin: 1.5em 0;
	}
	.legal-editor :global(.carta-renderer table) {
		width: 100%;
		border-collapse: collapse;
		margin: 1em 0;
	}
	.legal-editor :global(.carta-renderer th),
	.legal-editor :global(.carta-renderer td) {
		border: 1px solid var(--border);
		padding: 0.5em 0.75em;
		text-align: left;
	}
	.legal-editor :global(.carta-renderer th) {
		background: var(--muted);
		font-weight: 600;
	}
</style>
