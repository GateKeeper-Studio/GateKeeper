<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { MarkdownEditor } from 'carta-md';
	import 'carta-md/default.css';

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

	const LABELS: Record<LegalDocType, string> = {
		terms: 'Terms of Service',
		privacy: 'Privacy Policy'
	};

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
		toast.success(`${LABELS[type]} saved`);
	}

	async function resetToDefault(type: LegalDocType) {
		if (
			!confirm(`Reset ${LABELS[type]} to the default text? Your custom content will be removed.`)
		) {
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
		toast.success(`${LABELS[type]} reset to default`);
	}
</script>

<svelte:head><title>Legal Pages - GateKeeper</title></svelte:head>

{#snippet docEditor(type: LegalDocType)}
	<div class="mt-4 flex flex-col gap-3">
		<div class="flex flex-wrap items-center justify-between gap-2">
			<span class="text-sm text-muted-foreground">
				{#if docs[type].customized}
					Customized{#if docs[type].updatedAt}
						· last updated {new Date(docs[type].updatedAt).toLocaleString()}{/if}
				{:else}
					Showing the built-in default — edit and save to customize it.
				{/if}
			</span>

			<Button
				variant="outline"
				size="sm"
				disabled={!docs[type].customized || docs[type].resetting}
				onclick={() => resetToDefault(type)}
			>
				{docs[type].resetting ? 'Resetting…' : 'Reset to default'}
			</Button>
		</div>

		<div class="legal-editor rounded-2xl border">
			{#if mounted}
				<MarkdownEditor {carta} mode="tabs" bind:value={docs[type].content} />
			{:else}
				<div class="flex h-96 items-center justify-center text-sm text-muted-foreground">
					Loading editor…
				</div>
			{/if}
		</div>

		<div class="flex justify-end">
			<Button class="relative" disabled={docs[type].saving} onclick={() => save(type)}>
				{#if docs[type].saving}<LoadingSpinner class="absolute left-3" />{/if}
				{docs[type].saving ? 'Saving…' : 'Save changes'}
			</Button>
		</div>
	</div>
{/snippet}

<DashboardHeader
	breadcrumbs={{
		items: [
			{
				name: selectedTenant.current?.name ?? 'Tenant',
				path: `/dashboard/${tenantId}`
			},
			{ name: 'Legal Pages', path: `/dashboard/${tenantId}/legal` }
		]
	}}
/>

<MainContent
	goBack={{ message: 'Back to tenant detail', href: `/dashboard/${tenantId}` }}
	title="Legal Pages"
	description="Edit the Terms of Service and Privacy Policy shown on this tenant's hosted sign-in pages. Each starts from a sensible default — customize it only if you want."
>
	<Separator class="my-4" />

	{#if data.loadError}
		<Alert.Root variant="destructive" class="mb-4">
			<Alert.Title>Couldn't load the legal documents</Alert.Title>
			<Alert.Description>{data.loadError.message}</Alert.Description>
		</Alert.Root>
	{/if}

	<Tabs.Root value="terms" class="w-full">
		<Tabs.List>
			<Tabs.Trigger value="terms">Terms of Service</Tabs.Trigger>
			<Tabs.Trigger value="privacy">Privacy Policy</Tabs.Trigger>
		</Tabs.List>

		<Tabs.Content value="terms">{@render docEditor('terms')}</Tabs.Content>
		<Tabs.Content value="privacy">{@render docEditor('privacy')}</Tabs.Content>
	</Tabs.Root>
</MainContent>

<style>
	.legal-editor :global(.carta-input),
	.legal-editor :global(.carta-renderer) {
		min-height: 26rem;
		padding: 1rem;
	}

	/* Carta's default theme is light; map it onto the app tokens in dark mode so
	   the editor and preview stay legible. */
	:global(.dark) .legal-editor :global(.carta-toolbar),
	:global(.dark) .legal-editor :global(.carta-input),
	:global(.dark) .legal-editor :global(.carta-renderer),
	:global(.dark) .legal-editor :global(.carta-input textarea) {
		background: var(--card);
		color: var(--foreground);
	}

	:global(.dark) .legal-editor :global(.carta-toolbar) {
		border-color: var(--border);
	}
</style>
