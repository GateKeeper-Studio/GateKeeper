<script lang="ts">
	import { toast } from 'svelte-sonner';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	
	import { validate } from '$lib/utils/form';
	import { formSchema } from './schema.js';
	import {
		listEmailProviders,
		createEmailProvider,
		updateEmailProvider,
		deleteEmailProvider,
		sendTestEmail,
		type EmailProvider
	} from '$lib/services/admin/email-providers';
	import {
		listOAuthClients,
		assignEmailProvider,
		type OAuthClientView
	} from '$lib/services/admin/oauth-clients';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import Separator from '$lib/components/ui/separator/separator.svelte';

	let { data } = $props();

	const tenantId: string = data.tenantId;

	let providers = $state<EmailProvider[]>(data.providers);
	let clients = $state<OAuthClientView[]>(data.clients);

	// Form state (shared by create + edit). `editingId === null` means create.
	let editingId = $state<string | null>(null);
	let name = $state('');
	let host = $state('');
	let port = $state('587');
	let username = $state('');
	let password = $state('');
	let fromEmail = $state('');
	let fromName = $state('');
	let encryption = $state('starttls');
	let isDefault = $state(false);
	let isActive = $state(true);
	let errors = $state<Record<string, string>>({});
	let isSaving = $state(false);
	let testingId = $state<string | null>(null);

	const isEditing = $derived(editingId !== null);

	// Encryption options for the select; the trigger shows the matching label.
	const ENCRYPTION_OPTIONS = [
		{ value: 'starttls', label: 'STARTTLS (587)' },
		{ value: 'tls', label: 'TLS (465)' },
		{ value: 'none', label: 'None (dev only)' }
	];
	const encryptionLabel = $derived(
		ENCRYPTION_OPTIONS.find((o) => o.value === encryption)?.label ?? 'Select…'
	);

	// Common SMTP providers — picking one fills host/port/encryption. Hostinger
	// uses smtp.hostinger.com on 465 (implicit SSL/TLS); 587 + STARTTLS also works.
	const PRESETS: Record<string, { label: string; host: string; port: string; encryption: string }> = {
		gmail: { label: 'Gmail', host: 'smtp.gmail.com', port: '587', encryption: 'starttls' },
		outlook: { label: 'Outlook / Microsoft 365', host: 'smtp.office365.com', port: '587', encryption: 'starttls' },
		hostinger: { label: 'Hostinger', host: 'smtp.hostinger.com', port: '465', encryption: 'tls' }
	};

	function applyPreset(key: string) {
		const preset = PRESETS[key];
		if (!preset) return;
		host = preset.host;
		port = preset.port;
		encryption = preset.encryption;
		if (name.trim() === '') name = `SMTP ${preset.label}`;
	}

	function resetForm() {
		editingId = null;
		name = '';
		host = '';
		port = '587';
		username = '';
		password = '';
		fromEmail = '';
		fromName = '';
		encryption = 'starttls';
		isDefault = false;
		isActive = true;
		errors = {};
	}

	function startEdit(provider: EmailProvider) {
		editingId = provider.id;
		name = provider.name;
		host = provider.host;
		port = String(provider.port);
		username = provider.username;
		password = ''; // blank = keep the stored password
		fromEmail = provider.fromEmail;
		fromName = provider.fromName ?? '';
		encryption = provider.encryption;
		isDefault = provider.isDefault;
		isActive = provider.isActive;
		errors = {};
	}

	async function refreshProviders() {
		const [list] = await listEmailProviders(tenantId);
		if (list) providers = list.items;
	}

	async function refreshClients() {
		const [list] = await listOAuthClients(tenantId);
		if (list) clients = list.items;
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(formSchema, {
			name,
			host,
			port: Number(port),
			username,
			password,
			fromEmail,
			fromName,
			encryption,
			isDefault,
			isActive
		});
		if (!result.success) {
			errors = result.errors;
			return;
		}
		// Password is mandatory only when creating a new provider.
		if (!isEditing && password.trim() === '') {
			errors = { password: 'Password is required' };
			return;
		}
		errors = {};

		const input = {
			name,
			host,
			port: Number(port),
			username,
			fromEmail,
			fromName: fromName.trim() === '' ? null : fromName,
			encryption,
			isDefault,
			isActive,
			...(password.trim() === '' ? {} : { password })
		};

		isSaving = true;
		const [err] = isEditing
			? await updateEmailProvider(tenantId, editingId!, input)
			: await createEmailProvider(tenantId, input);
		isSaving = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		toast.success(isEditing ? 'Provider updated' : 'Provider created');
		resetForm();
		await refreshProviders();
	}

	// Sends a test email through the provider so its SMTP credentials can be
	// verified immediately. Defaults the recipient to the provider's from-address.
	async function onTest(provider: EmailProvider) {
		const to = window.prompt('Send a test email to:', provider.fromEmail);
		if (to === null || to.trim() === '') return;

		testingId = provider.id;
		const [err] = await sendTestEmail(tenantId, provider.id, to.trim());
		testingId = null;

		if (err) {
			// err.message carries the SMTP transport error (e.g. a 535 auth failure).
			toast.error(err.message);
			return;
		}
		toast.success(`Test email sent to ${to.trim()}`);
	}

	async function onDelete(provider: EmailProvider) {
		if (!confirm(`Delete provider "${provider.name}"?`)) return;
		const [err] = await deleteEmailProvider(tenantId, provider.id);
		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}
		toast.success('Provider deleted');
		if (editingId === provider.id) resetForm();
		await refreshProviders();
		await refreshClients();
	}

	async function onAssign(client: OAuthClientView, value: string) {
		const providerId = value === '' ? null : value;
		const [err] = await assignEmailProvider(tenantId, client, providerId);
		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}
		toast.success(`Updated email provider for "${client.name}"`);
		await refreshClients();
	}
</script>

<svelte:head><title>Email Providers - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? 'Tenant', path: `/dashboard/${tenantId}` },
			{ name: "E-mail Providers", path: `/dashboard/${tenantId}/email-providers` },
		]
	}}
/>

<MainContent
	goBack={{ message: "Back to tenant detail", href: `/dashboard/${tenantId}` }}
	title="SMTP Email Providers"
	description="Register the SMTP servers this tenant sends mail through, then assign one to each application. Passwords are encrypted at rest."
>
	<Separator class="my-4" />
	
	{#if data.loadError}
		<Alert.Root variant="destructive">
			<Alert.Title>Couldn't load data</Alert.Title>
			<Alert.Description>{data.loadError.message}</Alert.Description>
		</Alert.Root>
	{/if}

	<div class="flex flex-col gap-4">
		<div class="flex flex-col gap-4 w-full">
			<div class="flex flex-col gap-2">
				<span class="text-base font-medium">{isEditing ? 'Edit provider' : 'New provider'}</span>

				<span class="text-sm text-muted-foreground">
					{isEditing
						? 'Leave the password blank to keep the stored one.'
						: 'e.g. SMTP Gmail — host smtp.gmail.com, port 587, STARTTLS, an app password.'}
				</span>
			</div>

			<form onsubmit={onSubmit} class="grid gap-4 sm:grid-cols-2">
				<div class="space-y-2 sm:col-span-2">
					<Label for="preset">Quick preset</Label>
					<Select.Root
						type="single"
						value=""
						onValueChange={(value) => {
							if (value) applyPreset(value);
						}}
					>
						<Select.Trigger id="preset" class="text-muted-foreground w-full">
							Choose a provider preset…
						</Select.Trigger>

						<Select.Content class="p-1">
							{#each Object.entries(PRESETS) as [key, preset] (key)}
								<Select.Item value={key} label={preset.label}>{preset.label}</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>

				<div class="space-y-2">
					<Label for="name">Name</Label>
					<Input id="name" placeholder="SMTP Gmail" bind:value={name} />
					{#if errors.name}<p class="text-sm text-destructive">{errors.name}</p>{/if}
				</div>

				<div class="space-y-2">
					<Label for="host">Host</Label>
					<Input id="host" placeholder="smtp.gmail.com" bind:value={host} />
					{#if errors.host}<p class="text-sm text-destructive">{errors.host}</p>{/if}
				</div>

				<div class="space-y-2">
					<Label for="port">Port</Label>
					<Input id="port" type="number" placeholder="587" bind:value={port} />
					{#if errors.port}<p class="text-sm text-destructive">{errors.port}</p>{/if}
				</div>

				<div class="space-y-2">
					<Label for="encryption">Encryption</Label>
					<Select.Root type="single" bind:value={encryption}>
						<Select.Trigger id="encryption" class="w-full">
							{encryptionLabel}
						</Select.Trigger>
						<Select.Content>
							{#each ENCRYPTION_OPTIONS as option (option.value)}
								<Select.Item value={option.value} label={option.label}>{option.label}</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>

				<div class="space-y-2">
					<Label for="username">Username</Label>
					<Input id="username" autocomplete="off" bind:value={username} />
					{#if errors.username}<p class="text-sm text-destructive">{errors.username}</p>{/if}
				</div>

				<div class="space-y-2">
					<Label for="password">Password</Label>
					<Input
						id="password"
						type="password"
						autocomplete="new-password"
						placeholder={isEditing ? '•••••••• (unchanged)' : ''}
						bind:value={password}
					/>
					{#if errors.password}<p class="text-sm text-destructive">{errors.password}</p>{/if}
				</div>

				<div class="space-y-2">
					<Label for="fromEmail">From e-mail</Label>
					<Input id="fromEmail" type="email" placeholder="noreply@example.com" bind:value={fromEmail} />
					{#if errors.fromEmail}<p class="text-sm text-destructive">{errors.fromEmail}</p>{/if}
				</div>

				<div class="space-y-2">
					<Label for="fromName">From name (optional)</Label>
					<Input id="fromName" placeholder="Example" bind:value={fromName} />
				</div>

				<label class="flex items-center gap-2 text-sm">
					<Checkbox bind:checked={isDefault} />
					Tenant default (used when an app hasn't picked one)
				</label>

				<label class="flex items-center gap-2 text-sm">
					<Checkbox bind:checked={isActive} />
					Active
				</label>

				<div class="flex gap-2 sm:col-span-2 ml-auto">
					<Button type="submit" disabled={isSaving}>
						{isEditing ? 'Save changes' : 'Create provider'}
					</Button>
					{#if isEditing}
						<Button type="button" variant="outline" onclick={resetForm}>Cancel</Button>
					{/if}
				</div>
			</form>
		</div>

		<Separator class="my-4" />

		<div class="flex flex-col gap-4 w-full">
			<span class="text-base font-medium">Providers</span>

			{#if providers.length === 0}
				<p class="text-sm text-muted-foreground">No providers yet.</p>
			{:else}
				<div class="rounded-2xl border">
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head class="text-muted-foreground">Name</Table.Head>
								<Table.Head class="text-muted-foreground">Server</Table.Head>
								<Table.Head class="text-muted-foreground">From</Table.Head>
								<Table.Head class="text-muted-foreground">Status</Table.Head>
								<Table.Head class="text-muted-foreground" />
							</Table.Row>
						</Table.Header>

						<Table.Body>
							{#each providers as provider (provider.id)}
								<Table.Row>
									<Table.Cell>
										{provider.name}
										{#if provider.isDefault}<Badge class="ml-2">Default</Badge>{/if}
									</Table.Cell>
									<Table.Cell class="text-muted-foreground">
										{provider.host}:{provider.port} · {provider.encryption}
									</Table.Cell>
									<Table.Cell class="text-muted-foreground">{provider.fromEmail}</Table.Cell>
									<Table.Cell>
										{#if provider.isActive}Active{:else}<span class="text-muted-foreground">Inactive</span>{/if}
									</Table.Cell>
									<Table.Cell>
										<div class="flex gap-2">
											<Button
												variant="outline"
												size="sm"
												disabled={testingId === provider.id}
												onclick={() => onTest(provider)}
											>
												{testingId === provider.id ? 'Sending…' : 'Send test'}
											</Button>
											<Button variant="outline" size="sm" onclick={() => startEdit(provider)}>Edit</Button>
											<Button variant="destructive" size="sm" onclick={() => onDelete(provider)}>Delete</Button>
										</div>
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				</div>
			{/if}
		</div >

		<Separator class="my-4" />

		<div class="flex flex-col gap-4 w-full">
			<div class="flex flex-col gap-2">
				<span class="text-base font-medium">Application → provider</span>
				<span class="text-sm text-muted-foreground">Choose which provider sends each application's emails.</span>
			</div>

			{#if clients.length === 0}
				<p class="text-sm text-muted-foreground">No applications registered for this tenant.</p>
			{:else}
				<div class="space-y-3">
					{#each clients as client (client.id)}
						<div class="flex items-center justify-between gap-4">
							<span class="text-sm">{client.name}</span>
							<Select.Root
								type="single"
								value={client.email_provider_id ?? ''}
								onValueChange={(value) => onAssign(client, value)}
							>
								<Select.Trigger class="w-64">
									{providers.find((p) => p.id === client.email_provider_id)?.name ?? 'Tenant default'}
								</Select.Trigger>
								<Select.Content>
									<Select.Item value="" label="Tenant default">Tenant default</Select.Item>
									{#each providers as provider (provider.id)}
										<Select.Item value={provider.id} label={provider.name}>{provider.name}</Select.Item>
									{/each}
								</Select.Content>
							</Select.Root>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</MainContent>
