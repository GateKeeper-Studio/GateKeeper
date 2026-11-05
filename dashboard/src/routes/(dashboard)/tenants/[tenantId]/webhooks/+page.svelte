<script lang="ts">
	import { toast } from 'svelte-sonner';
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import WebhookIcon from '@lucide/svelte/icons/webhook';
	import SendIcon from '@lucide/svelte/icons/send';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import CheckIcon from '@lucide/svelte/icons/check';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import InfoIcon from '@lucide/svelte/icons/info';

	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import ErrorAlert from '$lib/components/error-alert.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';

	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import {
		WEBHOOK_EVENT_TYPES,
		createWebhookSubscription,
		deleteWebhookSubscription,
		listWebhookDeliveries,
		listWebhookSubscriptions,
		sendWebhookTestEvent,
		setWebhookSubscriptionActive,
		updateWebhookSubscription,
		type WebhookDeliveryStatus,
		type WebhookSubscriptionView
	} from '$lib/services/admin/webhooks';
	import { m } from '$lib/paraglide/messages.js';

	const tenantId = $derived(selectedTenant.tenantId!);
	const queryClient = useQueryClient();

	const subscriptionsQuery = createQuery(() => ({
		queryKey: ['webhooks', tenantId],
		enabled: !!tenantId,
		queryFn: async () => {
			const [data, err] = await listWebhookSubscriptions(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));

	// Polled while the page is open: the dispatcher drains the outbox every few
	// seconds, so a test or a fresh signup shows up here without a reload.
	const deliveriesQuery = createQuery(() => ({
		queryKey: ['webhook-deliveries', tenantId],
		enabled: !!tenantId,
		refetchInterval: 5000,
		queryFn: async () => {
			const [data, err] = await listWebhookDeliveries(tenantId);
			if (err) throw err;
			return data.items;
		}
	}));

	const subscriptionNames = $derived(
		new Map((subscriptionsQuery.data ?? []).map((s) => [s.id, s.name]))
	);

	function invalidate() {
		queryClient.invalidateQueries({ queryKey: ['webhooks', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['webhook-deliveries', tenantId] });
	}

	function formatDate(value: string | null) {
		return value ? new Date(value).toLocaleString() : '—';
	}

	const statusVariant: Record<WebhookDeliveryStatus, 'default' | 'secondary' | 'destructive'> = {
		delivered: 'default',
		pending: 'secondary',
		dead: 'destructive'
	};

	// ---- create ----------------------------------------------------------------
	let createOpen = $state(false);
	let creating = $state(false);
	let name = $state('');
	let url = $state('');
	let selectedEvents = $state<string[]>([...WEBHOOK_EVENT_TYPES]);

	function openCreate() {
		name = '';
		url = '';
		selectedEvents = [...WEBHOOK_EVENT_TYPES];
		createOpen = true;
	}

	function toggleEvent(eventType: string, checked: boolean) {
		selectedEvents = checked
			? [...selectedEvents, eventType]
			: selectedEvents.filter((e) => e !== eventType);
	}

	async function submitCreate(event: SubmitEvent) {
		event.preventDefault();
		creating = true;
		const [data, err] = await createWebhookSubscription(tenantId, {
			name: name.trim(),
			url: url.trim(),
			eventTypes: selectedEvents
		});
		creating = false;

		if (err) {
			toast.error(err.message);
			return;
		}

		createOpen = false;
		revealedSecret = data.secret;
		secretCopied = false;
		revealOpen = true;
		invalidate();
	}

	// ---- edit ------------------------------------------------------------------
	let editTarget = $state<WebhookSubscriptionView | null>(null);
	let saving = $state(false);

	function openEdit(subscription: WebhookSubscriptionView) {
		name = subscription.name;
		url = subscription.url;
		selectedEvents = [...subscription.eventTypes];
		editTarget = subscription;
	}

	async function submitEdit(event: SubmitEvent) {
		event.preventDefault();
		if (!editTarget) return;
		saving = true;
		const [err] = await updateWebhookSubscription(tenantId, editTarget, {
			name: name.trim(),
			url: url.trim(),
			eventTypes: selectedEvents
		});
		saving = false;

		if (err) {
			toast.error(err.message);
			return;
		}
		editTarget = null;
		toast.success(m.webhooks_edit_success());
		invalidate();
	}

	// ---- one-time secret reveal ----------------------------------------------
	let revealOpen = $state(false);
	let revealedSecret = $state<string | null>(null);
	let secretCopied = $state(false);

	async function copySecret() {
		if (!revealedSecret) return;
		try {
			await navigator.clipboard.writeText(revealedSecret);
			secretCopied = true;
			setTimeout(() => (secretCopied = false), 1500);
		} catch {
			toast.error(m.webhooks_copy_error());
		}
	}

	function closeReveal() {
		revealOpen = false;
		// Drop the secret from memory as soon as the dialog closes.
		revealedSecret = null;
	}

	// ---- per-endpoint actions -------------------------------------------------
	let busyId = $state<string | null>(null);

	async function sendTest(subscription: WebhookSubscriptionView) {
		busyId = subscription.id;
		const [data, err] = await sendWebhookTestEvent(tenantId, subscription.id);
		busyId = null;

		if (err) {
			toast.error(err.message);
		} else if (data.success) {
			toast.success(m.webhooks_test_success({ status: data.status ?? '' }));
		} else {
			toast.error(m.webhooks_test_failed({ reason: data.error ?? String(data.status ?? '') }));
		}
	}

	async function toggleActive(subscription: WebhookSubscriptionView) {
		busyId = subscription.id;
		const [err] = await setWebhookSubscriptionActive(
			tenantId,
			subscription,
			!subscription.isActive
		);
		busyId = null;

		if (err) {
			toast.error(err.message);
			return;
		}
		invalidate();
	}

	let deleteTarget = $state<WebhookSubscriptionView | null>(null);
	let deleting = $state(false);

	async function confirmDelete() {
		if (!deleteTarget) return;
		deleting = true;
		const [err] = await deleteWebhookSubscription(tenantId, deleteTarget.id);
		deleting = false;

		if (err) {
			toast.error(err.message);
			return;
		}
		deleteTarget = null;
		toast.success(m.webhooks_delete_success());
		invalidate();
	}
</script>

<svelte:head><title>{m.nav_webhooks()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_webhooks(), path: `/tenants/${tenantId}/webhooks` }
		]
	}}
/>

<MainContent>
	<div class="flex flex-wrap items-start justify-between gap-4">
		<div class="space-y-1">
			<h1 class="text-3xl font-bold tracking-tight">{m.nav_webhooks()}</h1>
			<p class="max-w-2xl text-sm text-muted-foreground">{m.webhooks_page_description()}</p>
		</div>

		<Button onclick={openCreate}>
			<PlusIcon />
			{m.webhooks_add_endpoint()}
		</Button>
	</div>

	<div class="mt-6">
		{#if subscriptionsQuery.isPending}
			<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
				{#each [0, 1] as i (i)}
					<Skeleton class="h-36 w-full rounded-4xl" />
				{/each}
			</div>
		{:else if subscriptionsQuery.isError}
			<ErrorAlert
				title={m.webhooks_load_error_title()}
				message={subscriptionsQuery.error.message}
			/>
		{:else if subscriptionsQuery.data.length === 0}
			<div class="flex flex-col items-center rounded-4xl border border-dashed p-12 text-center">
				<WebhookIcon class="mb-3 size-8 text-muted-foreground" />
				<p class="font-medium">{m.webhooks_empty_title()}</p>
				<p class="mt-1 text-sm text-muted-foreground">{m.webhooks_empty_description()}</p>
			</div>
		{:else}
			<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
				{#each subscriptionsQuery.data as subscription (subscription.id)}
					<Card.Root size="sm" class="h-full gap-3">
						<Card.Header>
							<div class="flex items-center justify-between gap-2">
								<Card.Title class="flex min-w-0 items-center gap-2">
									<span
										class="flex size-7 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary"
									>
										<WebhookIcon class="size-4" />
									</span>
									<span class="truncate">{subscription.name}</span>
								</Card.Title>
								{#if subscription.isActive}
									<Badge variant="default">{m.webhooks_status_active()}</Badge>
								{:else}
									<Badge variant="secondary">{m.webhooks_status_inactive()}</Badge>
								{/if}
							</div>
							<Card.Description class="truncate pl-9 font-mono text-xs" title={subscription.url}>
								{subscription.url}
							</Card.Description>
						</Card.Header>

						<Card.Content class="flex flex-col gap-3 pl-9">
							<div class="flex flex-wrap gap-1.5">
								{#if subscription.eventTypes.length === 0}
									<Badge variant="outline">{m.webhooks_all_events()}</Badge>
								{:else}
									{#each subscription.eventTypes as eventType (eventType)}
										<Badge variant="outline" class="font-mono">{eventType}</Badge>
									{/each}
								{/if}
							</div>

							<div class="flex flex-wrap gap-2">
								<Button
									size="sm"
									variant="outline"
									disabled={busyId === subscription.id || !subscription.isActive}
									onclick={() => sendTest(subscription)}
								>
									<SendIcon />
									{m.webhooks_send_test()}
								</Button>
								<Button
									size="sm"
									variant="outline"
									disabled={busyId === subscription.id}
									onclick={() => toggleActive(subscription)}
								>
									{subscription.isActive ? m.webhooks_disable() : m.webhooks_enable()}
								</Button>
								<Button
									size="sm"
									variant="outline"
									disabled={busyId === subscription.id}
									onclick={() => openEdit(subscription)}
								>
									<PencilIcon />
									{m.webhooks_edit()}
								</Button>
								<Button
									size="sm"
									variant="ghost"
									class="text-destructive"
									disabled={busyId === subscription.id}
									onclick={() => (deleteTarget = subscription)}
								>
									<Trash2Icon />
									{m.webhooks_delete()}
								</Button>
							</div>
						</Card.Content>
					</Card.Root>
				{/each}
			</div>
		{/if}
	</div>

	<div class="mt-10 space-y-3">
		<div class="space-y-1">
			<h2 class="text-xl font-semibold tracking-tight">{m.webhooks_deliveries_title()}</h2>
			<p class="text-sm text-muted-foreground">{m.webhooks_deliveries_description()}</p>
		</div>

		{#if deliveriesQuery.isPending}
			<Skeleton class="h-40 w-full rounded-xl" />
		{:else if deliveriesQuery.isError}
			<ErrorAlert title={m.webhooks_load_error_title()} message={deliveriesQuery.error.message} />
		{:else if deliveriesQuery.data.length === 0}
			<p class="rounded-xl border border-dashed p-6 text-center text-sm text-muted-foreground">
				{m.webhooks_deliveries_empty()}
			</p>
		{:else}
			<div class="rounded-xl border">
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>{m.webhooks_col_endpoint()}</Table.Head>
							<Table.Head>{m.webhooks_col_status()}</Table.Head>
							<Table.Head>{m.webhooks_col_attempts()}</Table.Head>
							<Table.Head>{m.webhooks_col_response()}</Table.Head>
							<Table.Head>{m.webhooks_col_created()}</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each deliveriesQuery.data as delivery (delivery.id)}
							<Table.Row>
								<Table.Cell class="max-w-48 truncate">
									{subscriptionNames.get(delivery.subscriptionId) ?? delivery.subscriptionId}
								</Table.Cell>
								<Table.Cell>
									<Badge variant={statusVariant[delivery.status] ?? 'secondary'}>
										{delivery.status}
									</Badge>
								</Table.Cell>
								<Table.Cell>{delivery.attempts}</Table.Cell>
								<Table.Cell class="max-w-64 truncate" title={delivery.lastError ?? ''}>
									{delivery.lastStatus ?? ''}
									{#if delivery.lastError}
										<span class="text-xs text-muted-foreground">{delivery.lastError}</span>
									{/if}
								</Table.Cell>
								<Table.Cell class="text-xs text-muted-foreground">
									{formatDate(delivery.createdAt)}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</div>
		{/if}
	</div>
</MainContent>

<!-- Create endpoint -->
<Dialog.Root bind:open={createOpen}>
	<Dialog.Content class="sm:max-w-lg">
		<form class="space-y-4" onsubmit={submitCreate}>
			<Dialog.Header>
				<Dialog.Title>{m.webhooks_add_endpoint()}</Dialog.Title>
				<Dialog.Description>{m.webhooks_create_description()}</Dialog.Description>
			</Dialog.Header>

			<div class="space-y-1.5">
				<Label for="webhook-name">{m.webhooks_field_name()}</Label>
				<Input id="webhook-name" bind:value={name} required placeholder="golang-api" />
			</div>

			<div class="space-y-1.5">
				<Label for="webhook-url">{m.webhooks_field_url()}</Label>
				<Input
					id="webhook-url"
					type="url"
					bind:value={url}
					required
					placeholder="https://api.example.com/webhooks/gatekeeper"
				/>
				<p class="text-xs text-muted-foreground">{m.webhooks_field_url_hint()}</p>
			</div>

			<fieldset class="space-y-2">
				<legend class="text-sm font-medium">{m.webhooks_field_events()}</legend>
				{#each WEBHOOK_EVENT_TYPES as eventType (eventType)}
					<label class="flex items-center gap-2 text-sm">
						<Checkbox
							checked={selectedEvents.includes(eventType)}
							onCheckedChange={(checked) => toggleEvent(eventType, checked === true)}
						/>
						<span class="font-mono">{eventType}</span>
					</label>
				{/each}
				<p class="text-xs text-muted-foreground">{m.webhooks_field_events_hint()}</p>
			</fieldset>

			<Dialog.Footer>
				<Button
					type="button"
					variant="outline"
					disabled={creating}
					onclick={() => (createOpen = false)}
				>
					{m.webhooks_cancel()}
				</Button>
				<Button type="submit" disabled={creating}>
					{#if creating}<LoadingSpinner />{/if}
					{m.webhooks_create_submit()}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<!-- Edit endpoint -->
<Dialog.Root
	open={editTarget !== null}
	onOpenChange={(open) => {
		if (!open && !saving) editTarget = null;
	}}
>
	<Dialog.Content class="sm:max-w-lg">
		<form class="space-y-4" onsubmit={submitEdit}>
			<Dialog.Header>
				<Dialog.Title>{m.webhooks_edit_title()}</Dialog.Title>
				<Dialog.Description>{m.webhooks_edit_description()}</Dialog.Description>
			</Dialog.Header>

			<div class="space-y-1.5">
				<Label for="webhook-edit-name">{m.webhooks_field_name()}</Label>
				<Input id="webhook-edit-name" bind:value={name} required />
			</div>

			<div class="space-y-1.5">
				<Label for="webhook-edit-url">{m.webhooks_field_url()}</Label>
				<Input id="webhook-edit-url" type="url" bind:value={url} required />
				<p class="text-xs text-muted-foreground">{m.webhooks_field_url_hint()}</p>
			</div>

			<fieldset class="space-y-2">
				<legend class="text-sm font-medium">{m.webhooks_field_events()}</legend>
				{#each WEBHOOK_EVENT_TYPES as eventType (eventType)}
					<label class="flex items-center gap-2 text-sm">
						<Checkbox
							checked={selectedEvents.includes(eventType)}
							onCheckedChange={(checked) => toggleEvent(eventType, checked === true)}
						/>
						<span class="font-mono">{eventType}</span>
					</label>
				{/each}
				<p class="text-xs text-muted-foreground">{m.webhooks_field_events_hint()}</p>
			</fieldset>

			<Dialog.Footer>
				<Button
					type="button"
					variant="outline"
					disabled={saving}
					onclick={() => (editTarget = null)}
				>
					{m.webhooks_cancel()}
				</Button>
				<Button type="submit" disabled={saving}>
					{#if saving}<LoadingSpinner />{/if}
					{m.webhooks_edit_submit()}
				</Button>
			</Dialog.Footer>
		</form>
	</Dialog.Content>
</Dialog.Root>

<!-- One-time signing secret -->
<Dialog.Root
	open={revealOpen}
	onOpenChange={(open) => {
		if (!open) closeReveal();
	}}
>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.webhooks_secret_title()}</Dialog.Title>
			<Dialog.Description>{m.webhooks_secret_description()}</Dialog.Description>
		</Dialog.Header>

		{#if revealedSecret}
			<div
				class="flex items-center gap-2.5 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-1.5 pl-3"
			>
				<KeyRoundIcon class="size-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
				<code class="flex-1 truncate font-mono text-sm">{revealedSecret}</code>
				<Button
					variant="ghost"
					size="icon"
					class="size-8 shrink-0 text-muted-foreground hover:text-foreground"
					aria-label={m.webhooks_copy_secret()}
					onclick={copySecret}
				>
					{#if secretCopied}<CheckIcon class="text-emerald-600 dark:text-emerald-400" />{:else}
						<CopyIcon />
					{/if}
				</Button>
			</div>

			<div class="flex items-start gap-2 rounded-md bg-muted/50 p-3 text-xs text-muted-foreground">
				<InfoIcon class="mt-0.5 size-4 shrink-0" />
				<span>{m.webhooks_secret_hint()}</span>
			</div>
		{/if}

		<Dialog.Footer>
			<Button onclick={closeReveal}>{m.webhooks_secret_done()}</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Delete confirmation -->
<Dialog.Root
	open={deleteTarget !== null}
	onOpenChange={(open) => {
		if (!open && !deleting) deleteTarget = null;
	}}
>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>{m.webhooks_delete_title()}</Dialog.Title>
			<Dialog.Description>
				{m.webhooks_delete_description({ name: deleteTarget?.name ?? '' })}
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer>
			<Button variant="outline" disabled={deleting} onclick={() => (deleteTarget = null)}>
				{m.webhooks_cancel()}
			</Button>
			<Button variant="destructive" disabled={deleting} onclick={confirmDelete}>
				{#if deleting}<LoadingSpinner />{/if}
				{m.webhooks_delete()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
