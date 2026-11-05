<script lang="ts">
	import { toast } from 'svelte-sonner';
	import DOMPurify from 'isomorphic-dompurify';
	import MailsIcon from '@lucide/svelte/icons/mails';
	import MailPlusIcon from '@lucide/svelte/icons/mail-plus';
	import SaveIcon from '@lucide/svelte/icons/save';

	import { Button } from '$lib/components/ui/button/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import Separator from '$lib/components/ui/separator/separator.svelte';
	import MainContent from '$lib/components/main-content.svelte';
	import DashboardHeader from '$lib/components/dashboard-header/index.svelte';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import CodeEditor from '$lib/components/code-editor.svelte';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { validate } from '$lib/utils/form';
	import { formSchema as providerFormSchema } from '../email-providers/schema.js';
	import { formSchema as templateFormSchema } from '../email-templates/schema.js';
	import {
		listEmailProviders,
		createEmailProvider,
		updateEmailProvider,
		deleteEmailProvider,
		sendTestEmail,
		type EmailProvider
	} from '$lib/services/admin/email-providers';
	import {
		getEmailTemplate,
		updateEmailTemplate,
		resetEmailTemplate,
		previewEmailTemplate,
		type EmailTemplateSummary
	} from '$lib/services/admin/email-templates';
	import {
		listOAuthClients,
		assignEmailProvider,
		type OAuthClientView
	} from '$lib/services/admin/oauth-clients';
	import { m } from '$lib/paraglide/messages.js';

	let { data } = $props();

	type EmailTab = 'providers' | 'templates';
	const tenantId = $derived(data.tenantId);

	let activeSection = $state<EmailTab>('providers');
	let providers = $state<EmailProvider[]>([]);
	let clients = $state<OAuthClientView[]>([]);

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
	let providerErrors = $state<Record<string, string>>({});
	let isSavingProvider = $state(false);
	let testingId = $state<string | null>(null);

	const isEditing = $derived(editingId !== null);

	const ENCRYPTION_OPTIONS = $derived([
		{ value: 'starttls', label: m.email_encryption_starttls() },
		{ value: 'tls', label: m.email_encryption_tls() },
		{ value: 'none', label: m.email_encryption_none() }
	]);
	const encryptionLabel = $derived(
		ENCRYPTION_OPTIONS.find((o) => o.value === encryption)?.label ?? m.email_select_placeholder()
	);

	const PRESETS: Record<string, { label: string; host: string; port: string; encryption: string }> =
		{
			gmail: { label: 'Gmail', host: 'smtp.gmail.com', port: '587', encryption: 'starttls' },
			outlook: {
				label: 'Outlook / Microsoft 365',
				host: 'smtp.office365.com',
				port: '587',
				encryption: 'starttls'
			},
			hostinger: { label: 'Hostinger', host: 'smtp.hostinger.com', port: '465', encryption: 'tls' }
		};

	type EditorState = {
		loaded: boolean;
		loadError: string | null;
		subject: string;
		bodyMjml: string;
		variables: string[];
		customized: boolean;
		updatedAt: string | null;
		saving: boolean;
		resetting: boolean;
		errors: Record<string, string>;
		previewHtml: string;
		previewError: string | null;
	};

	let summaries = $state<EmailTemplateSummary[]>([]);
	let editors = $state<Record<string, EditorState>>({});
	let activeTemplate = $state<string>('');
	let editorRef = $state<{ insertAtCursor: (text: string) => void } | null>(null);

	const activeEditor = $derived(editors[activeTemplate]);

	$effect(() => {
		activeSection = data.activeTab as EmailTab;
		providers = data.providers;
		clients = data.clients;
		summaries = data.templates;
		activeTemplate = data.templates[0]?.templateType ?? '';
		editors = {};
	});

	function setSection(value: string) {
		if (value !== 'providers' && value !== 'templates') return;
		activeSection = value;
		if (typeof window === 'undefined') return;

		const url = new URL(window.location.href);
		url.searchParams.set('tab', value);
		window.history.replaceState(null, '', `${url.pathname}?${url.searchParams.toString()}`);
	}

	function applyPreset(key: string) {
		const preset = PRESETS[key];
		if (!preset) return;
		host = preset.host;
		port = preset.port;
		encryption = preset.encryption;
		if (name.trim() === '') name = `SMTP ${preset.label}`;
	}

	function resetProviderForm() {
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
		providerErrors = {};
	}

	function startEdit(provider: EmailProvider) {
		editingId = provider.id;
		name = provider.name;
		host = provider.host;
		port = String(provider.port);
		username = provider.username;
		password = '';
		fromEmail = provider.fromEmail;
		fromName = provider.fromName ?? '';
		encryption = provider.encryption;
		isDefault = provider.isDefault;
		isActive = provider.isActive;
		providerErrors = {};
	}

	async function refreshProviders() {
		const [list] = await listEmailProviders(tenantId);
		if (list) providers = list.items;
	}

	async function refreshClients() {
		const [list] = await listOAuthClients(tenantId);
		if (list) clients = list.items;
	}

	async function onProviderSubmit(event: SubmitEvent) {
		event.preventDefault();

		const result = validate(providerFormSchema, {
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
			providerErrors = result.errors;
			return;
		}
		if (!isEditing && password.trim() === '') {
			providerErrors = { password: m.email_password_required() };
			return;
		}
		providerErrors = {};

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

		isSavingProvider = true;
		const [err] = isEditing
			? await updateEmailProvider(tenantId, editingId!, input)
			: await createEmailProvider(tenantId, input);
		isSavingProvider = false;

		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}

		toast.success(isEditing ? m.email_provider_updated() : m.email_provider_created());
		resetProviderForm();
		await refreshProviders();
	}

	async function onTest(provider: EmailProvider) {
		const to = window.prompt(m.email_test_email_prompt(), provider.fromEmail);
		if (to === null || to.trim() === '') return;

		testingId = provider.id;
		const [err] = await sendTestEmail(tenantId, provider.id, to.trim());
		testingId = null;

		if (err) {
			toast.error(err.message);
			return;
		}
		toast.success(m.email_test_email_sent({ email: to.trim() }));
	}

	async function onDelete(provider: EmailProvider) {
		if (!confirm(m.email_delete_provider_confirm({ name: provider.name }))) return;
		const [err] = await deleteEmailProvider(tenantId, provider.id);
		if (err) {
			toast.error(err.data?.message || err.message);
			return;
		}
		toast.success(m.email_provider_deleted());
		if (editingId === provider.id) resetProviderForm();
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
		toast.success(m.email_provider_assigned({ client: client.name }));
		await refreshClients();
	}

	function blankEditor(): EditorState {
		return {
			loaded: false,
			loadError: null,
			subject: '',
			bodyMjml: '',
			variables: [],
			customized: false,
			updatedAt: null,
			saving: false,
			resetting: false,
			errors: {},
			previewHtml: '',
			previewError: null
		};
	}

	async function loadTemplate(type: string) {
		editors[type] = blankEditor();

		const [view, err] = await getEmailTemplate(tenantId, type);
		if (err || !view) {
			editors[type].loadError = err?.message ?? m.email_template_load_failed();
			return;
		}

		editors[type] = {
			...blankEditor(),
			loaded: true,
			subject: view.subject,
			bodyMjml: view.bodyMjml,
			variables: view.variables,
			customized: view.isCustomized,
			updatedAt: view.updatedAt
		};
	}

	$effect(() => {
		const type = activeTemplate;
		if (type && !editors[type]) {
			void loadTemplate(type);
		}
	});

	$effect(() => {
		const type = activeTemplate;
		const editor = editors[type];
		if (!editor || !editor.loaded) return;

		const subject = editor.subject;
		const body = editor.bodyMjml;
		const handle = setTimeout(() => void refreshPreview(type, subject, body), 500);
		return () => clearTimeout(handle);
	});

	async function refreshPreview(type: string, subject: string, body: string) {
		if (!subject || !body) return;
		const [preview, err] = await previewEmailTemplate(tenantId, type, subject, body);
		const editor = editors[type];
		if (!editor) return;

		if (err || !preview) {
			editor.previewError = err?.message ?? m.email_template_preview_failed();
			return;
		}
		editor.previewError = null;
		editor.previewHtml = DOMPurify.sanitize(preview.html, { WHOLE_DOCUMENT: true });
	}

	function insertVariable(variable: string) {
		if (!editors[activeTemplate]) return;
		editorRef?.insertAtCursor(`{{${variable}}}`);
	}

	function syncSummary(type: string, customized: boolean, updatedAt: string | null) {
		const summary = summaries.find((s) => s.templateType === type);
		if (summary) {
			summary.isCustomized = customized;
			summary.updatedAt = updatedAt;
		}
	}

	async function saveTemplate(type: string) {
		const editor = editors[type];
		if (!editor) return;

		const result = validate(templateFormSchema, {
			subject: editor.subject,
			bodyMjml: editor.bodyMjml
		});
		if (!result.success) {
			editor.errors = result.errors;
			return;
		}
		editor.errors = {};
		editor.saving = true;

		const [view, err] = await updateEmailTemplate(tenantId, type, editor.subject, editor.bodyMjml);
		editor.saving = false;

		if (err || !view) {
			toast.error(err?.message ?? m.email_template_save_failed());
			return;
		}

		editor.customized = view.isCustomized;
		editor.updatedAt = view.updatedAt;
		syncSummary(type, view.isCustomized, view.updatedAt);
		toast.success(m.email_template_saved({ name: labelOf(type) }));
	}

	async function resetTemplateToDefault(type: string) {
		const editor = editors[type];
		if (!editor) return;
		if (!confirm(m.email_template_reset_confirm({ name: labelOf(type) }))) {
			return;
		}

		editor.resetting = true;
		const [view, err] = await resetEmailTemplate(tenantId, type);
		editor.resetting = false;

		if (err || !view) {
			toast.error(err?.message ?? m.email_template_reset_failed());
			return;
		}

		editor.subject = view.subject;
		editor.bodyMjml = view.bodyMjml;
		editor.customized = view.isCustomized;
		editor.updatedAt = view.updatedAt;
		editor.errors = {};
		syncSummary(type, view.isCustomized, view.updatedAt);
		toast.success(m.email_template_reset_success({ name: labelOf(type) }));
	}

	function labelOf(type: string): string {
		return summaries.find((s) => s.templateType === type)?.label ?? type;
	}
</script>

<svelte:head><title>{m.email_page_title()} - GateKeeper</title></svelte:head>

<DashboardHeader
	breadcrumbs={{
		items: [
			{ name: selectedTenant.current?.name ?? m.sidebar_tenant(), path: `/tenants/${tenantId}` },
			{ name: m.nav_email(), path: `/tenants/${tenantId}/email` }
		]
	}}
/>

<MainContent
	goBack={{ message: m.email_back_to_tenant(), href: `/tenants/${tenantId}` }}
	title={m.nav_email()}
	description={m.email_page_description()}
>
	<Separator class="my-4" />

	<Tabs.Root class="w-full" value={activeSection} onValueChange={setSection}>
		<Tabs.List class="mb-4">
			<Tabs.Trigger value="providers">
				<MailsIcon class="size-4" />
				{m.email_smtp_providers_tab()}
			</Tabs.Trigger>
			<Tabs.Trigger value="templates">
				<MailPlusIcon class="size-4" />
				{m.email_templates_tab()}
			</Tabs.Trigger>
		</Tabs.List>

		<Tabs.Content value="providers">
			{#if data.providersLoadError}
				<Alert.Root variant="destructive" class="mb-4">
					<Alert.Title>{m.email_providers_load_error_title()}</Alert.Title>
					<Alert.Description>{data.providersLoadError.message}</Alert.Description>
				</Alert.Root>
			{/if}

			<div class="flex flex-col gap-6">
				<section class="flex w-full flex-col gap-4">
					<div class="flex flex-col gap-1">
						<h2 class="text-base font-medium">
							{isEditing ? m.email_edit_provider_title() : m.email_new_provider_title()}
						</h2>
						<p class="text-sm text-muted-foreground">
							{isEditing ? m.email_edit_provider_hint() : m.email_new_provider_hint()}
						</p>
					</div>

					<form onsubmit={onProviderSubmit} class="grid gap-4 sm:grid-cols-2">
						<div class="space-y-2 sm:col-span-2">
							<Label for="preset">{m.email_quick_preset_label()}</Label>
							<Select.Root
								type="single"
								value=""
								onValueChange={(value) => {
									if (value) applyPreset(value);
								}}
							>
								<Select.Trigger id="preset" class="w-full text-muted-foreground">
									{m.email_choose_preset_placeholder()}
								</Select.Trigger>

								<Select.Content class="p-1">
									{#each Object.entries(PRESETS) as [key, preset] (key)}
										<Select.Item value={key} label={preset.label}>{preset.label}</Select.Item>
									{/each}
								</Select.Content>
							</Select.Root>
						</div>

						<div class="space-y-2">
							<Label for="name">{m.email_name_label()}</Label>
							<Input id="name" placeholder={m.email_name_placeholder()} bind:value={name} />
							{#if providerErrors.name}<p class="text-sm text-destructive">
									{providerErrors.name}
								</p>{/if}
						</div>

						<div class="space-y-2">
							<Label for="host">{m.email_host_label()}</Label>
							<Input id="host" placeholder={m.email_host_placeholder()} bind:value={host} />
							{#if providerErrors.host}<p class="text-sm text-destructive">
									{providerErrors.host}
								</p>{/if}
						</div>

						<div class="space-y-2">
							<Label for="port">{m.email_port_label()}</Label>
							<Input id="port" type="number" placeholder="587" bind:value={port} />
							{#if providerErrors.port}<p class="text-sm text-destructive">
									{providerErrors.port}
								</p>{/if}
						</div>

						<div class="space-y-2">
							<Label for="encryption">{m.email_encryption_label()}</Label>
							<Select.Root type="single" bind:value={encryption}>
								<Select.Trigger id="encryption" class="w-full">
									{encryptionLabel}
								</Select.Trigger>
								<Select.Content>
									{#each ENCRYPTION_OPTIONS as option (option.value)}
										<Select.Item value={option.value} label={option.label}
											>{option.label}</Select.Item
										>
									{/each}
								</Select.Content>
							</Select.Root>
						</div>

						<div class="space-y-2">
							<Label for="username">{m.email_username_label()}</Label>
							<Input id="username" autocomplete="off" bind:value={username} />
							{#if providerErrors.username}<p class="text-sm text-destructive">
									{providerErrors.username}
								</p>{/if}
						</div>

						<div class="space-y-2">
							<Label for="password">{m.email_password_label()}</Label>
							<Input
								id="password"
								type="password"
								autocomplete="new-password"
								placeholder={isEditing ? m.email_password_unchanged_placeholder() : ''}
								bind:value={password}
							/>
							{#if providerErrors.password}<p class="text-sm text-destructive">
									{providerErrors.password}
								</p>{/if}
						</div>

						<div class="space-y-2">
							<Label for="fromEmail">{m.email_from_email_label()}</Label>
							<Input
								id="fromEmail"
								type="email"
								placeholder={m.email_from_email_placeholder()}
								bind:value={fromEmail}
							/>
							{#if providerErrors.fromEmail}<p class="text-sm text-destructive">
									{providerErrors.fromEmail}
								</p>{/if}
						</div>

						<div class="space-y-2">
							<Label for="fromName">{m.email_from_name_label()}</Label>
							<Input id="fromName" placeholder={m.email_from_name_placeholder()} bind:value={fromName} />
						</div>

						<label class="flex items-center gap-2 text-sm">
							<Checkbox bind:checked={isDefault} />
							{m.email_tenant_default_label()}
						</label>

						<label class="flex items-center gap-2 text-sm">
							<Checkbox bind:checked={isActive} />
							{m.email_active_label()}
						</label>

						<div class="ml-auto flex gap-2 sm:col-span-2">
							<Button type="submit" disabled={isSavingProvider}>
								{#if isSavingProvider}
									<LoadingSpinner class="size-4" />
								{:else if isEditing}
									<SaveIcon />
								{:else}
									<MailPlusIcon />
								{/if}
								{isEditing ? m.email_save_changes() : m.email_create_provider()}
							</Button>
							{#if isEditing}
								<Button type="button" variant="outline" onclick={resetProviderForm}
									>{m.email_cancel()}</Button
								>
							{/if}
						</div>
					</form>
				</section>

				<Separator />

				<section class="flex w-full flex-col gap-4">
					<h2 class="text-base font-medium">{m.email_providers_title()}</h2>

					{#if providers.length === 0}
						<p class="text-sm text-muted-foreground">{m.email_no_providers()}</p>
					{:else}
						<div class="overflow-hidden rounded-lg border">
							<Table.Root>
								<Table.Header>
									<Table.Row>
										<Table.Head class="text-muted-foreground">{m.email_name_label()}</Table.Head>
										<Table.Head class="text-muted-foreground">{m.email_table_server()}</Table.Head>
										<Table.Head class="text-muted-foreground">{m.email_table_from()}</Table.Head>
										<Table.Head class="text-muted-foreground">{m.email_table_status()}</Table.Head>
										<Table.Head class="text-muted-foreground" />
									</Table.Row>
								</Table.Header>

								<Table.Body>
									{#each providers as provider (provider.id)}
										<Table.Row>
											<Table.Cell>
												{provider.name}
												{#if provider.isDefault}<Badge class="ml-2"
														>{m.email_default_badge()}</Badge
													>{/if}
											</Table.Cell>
											<Table.Cell class="text-muted-foreground">
												{provider.host}:{provider.port} / {provider.encryption}
											</Table.Cell>
											<Table.Cell class="text-muted-foreground">{provider.fromEmail}</Table.Cell>
											<Table.Cell>
												{#if provider.isActive}{m.email_active_label()}{:else}<span
														class="text-muted-foreground">{m.email_status_inactive()}</span
													>{/if}
											</Table.Cell>
											<Table.Cell>
												<div class="flex flex-wrap justify-end gap-2">
													<Button
														variant="outline"
														size="sm"
														disabled={testingId === provider.id}
														onclick={() => onTest(provider)}
													>
														{testingId === provider.id ? m.email_sending() : m.email_send_test()}
													</Button>
													<Button variant="outline" size="sm" onclick={() => startEdit(provider)}
														>{m.email_edit_button()}</Button
													>
													<Button variant="destructive" size="sm" onclick={() => onDelete(provider)}
														>{m.email_delete_button()}</Button
													>
												</div>
											</Table.Cell>
										</Table.Row>
									{/each}
								</Table.Body>
							</Table.Root>
						</div>
					{/if}
				</section>

				<Separator />

				<section class="flex w-full flex-col gap-4">
					<div class="flex flex-col gap-1">
						<h2 class="text-base font-medium">{m.email_application_provider_title()}</h2>
						<p class="text-sm text-muted-foreground">
							{m.email_application_provider_description()}
						</p>
					</div>

					{#if clients.length === 0}
						<p class="text-sm text-muted-foreground">{m.email_no_applications()}</p>
					{:else}
						<div class="space-y-3">
							{#each clients as client (client.id)}
								<div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
									<span class="text-sm">{client.name}</span>
									<Select.Root
										type="single"
										value={client.email_provider_id ?? ''}
										onValueChange={(value) => onAssign(client, value)}
									>
										<Select.Trigger class="w-full sm:w-64">
											{providers.find((p) => p.id === client.email_provider_id)?.name ??
												m.email_tenant_default_label()}
										</Select.Trigger>
										<Select.Content>
											<Select.Item value="" label={m.email_tenant_default_label()}
												>{m.email_tenant_default_label()}</Select.Item
											>
											{#each providers as provider (provider.id)}
												<Select.Item value={provider.id} label={provider.name}
													>{provider.name}</Select.Item
												>
											{/each}
										</Select.Content>
									</Select.Root>
								</div>
							{/each}
						</div>
					{/if}
				</section>
			</div>
		</Tabs.Content>

		<Tabs.Content value="templates">
			{#if data.templatesLoadError}
				<Alert.Root variant="destructive" class="mb-4">
					<Alert.Title>{m.email_templates_load_error_title()}</Alert.Title>
					<Alert.Description>{data.templatesLoadError.message}</Alert.Description>
				</Alert.Root>
			{:else if summaries.length === 0}
				<p class="text-sm text-muted-foreground">{m.email_no_templates()}</p>
			{:else}
				<div class="grid gap-6 lg:grid-cols-[240px_1fr]">
					<nav class="flex flex-col gap-1">
						{#each summaries as summary (summary.templateType)}
							<button
								type="button"
								class="flex flex-col items-start gap-1 rounded-lg border px-3 py-2 text-left text-sm transition-colors hover:bg-muted {activeTemplate ===
								summary.templateType
									? 'border-gray-300 bg-muted dark:border-gray-600'
									: 'border-transparent'}"
								onclick={() => (activeTemplate = summary.templateType)}
							>
								<span class="font-medium">{summary.label}</span>
								<span class="text-xs text-muted-foreground">{summary.description}</span>
								{#if summary.isCustomized}
									<Badge variant="secondary" class="mt-1">{m.email_customized_badge()}</Badge>
								{/if}
							</button>
						{/each}
					</nav>

					<div>
						{#if !activeEditor || !activeEditor.loaded}
							{#if activeEditor?.loadError}
								<Alert.Root variant="destructive">
									<Alert.Title>{m.email_template_load_error_title()}</Alert.Title>
									<Alert.Description>{activeEditor.loadError}</Alert.Description>
								</Alert.Root>
							{:else}
								<div class="flex h-64 items-center justify-center text-sm text-muted-foreground">
									<LoadingSpinner class="mr-2" /> {m.email_loading_template()}
								</div>
							{/if}
						{:else}
							<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
								<span class="text-sm text-muted-foreground">
									{#if activeEditor.customized}
										{m.email_customized_badge()}{#if activeEditor.updatedAt}
											{m.email_last_updated({
												date: new Date(activeEditor.updatedAt).toLocaleString()
											})}{/if}
									{:else}
										{m.email_showing_default()}
									{/if}
								</span>
								<Button
									variant="outline"
									size="sm"
									disabled={!activeEditor.customized || activeEditor.resetting}
									onclick={() => resetTemplateToDefault(activeTemplate)}
								>
									{activeEditor.resetting ? m.email_resetting() : m.email_reset_to_default()}
								</Button>
							</div>

							<div class="space-y-2">
								<Label for="subject">{m.email_subject_label()}</Label>
								<Input id="subject" bind:value={activeEditor.subject} />
								{#if activeEditor.errors.subject}
									<p class="text-sm text-destructive">{activeEditor.errors.subject}</p>
								{/if}
							</div>

							<div class="mt-3 flex flex-wrap items-center gap-2">
								<span class="text-xs text-muted-foreground">{m.email_insert_variable_label()}</span>
								{#each activeEditor.variables as variable (variable)}
									<Button
										type="button"
										variant="outline"
										size="sm"
										class="font-mono text-xs"
										onclick={() => insertVariable(variable)}
									>
										{`{{${variable}}}`}
									</Button>
								{/each}
							</div>

							<div class="mt-3 grid gap-4 xl:grid-cols-2">
								<div class="space-y-2">
									<Label>{m.email_mjml_body_label()}</Label>
									<CodeEditor
										bind:this={editorRef}
										bind:value={activeEditor.bodyMjml}
										class="h-[28rem] overflow-hidden rounded-lg border bg-input/50"
									/>
									{#if activeEditor.errors.bodyMjml}
										<p class="text-sm text-destructive">{activeEditor.errors.bodyMjml}</p>
									{/if}
								</div>

								<div class="space-y-2">
									<Label>{m.email_preview_label()}</Label>
									{#if activeEditor.previewError}
										<Alert.Root variant="destructive">
											<Alert.Title>{m.email_preview_error_title()}</Alert.Title>
											<Alert.Description>{activeEditor.previewError}</Alert.Description>
										</Alert.Root>
									{:else}
										<iframe
											title={m.email_preview_iframe_title()}
											sandbox=""
											srcdoc={activeEditor.previewHtml}
											class="min-h-[28rem] w-full rounded-lg border bg-white"
										></iframe>
									{/if}
								</div>
							</div>

							<div class="mt-4 flex justify-end">
								<Button
									disabled={activeEditor.saving}
									onclick={() => saveTemplate(activeTemplate)}
								>
									{#if activeEditor.saving}<LoadingSpinner class="size-4" />{:else}<SaveIcon />{/if}
									{activeEditor.saving ? m.email_saving() : m.email_save_changes()}
								</Button>
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</Tabs.Content>
	</Tabs.Root>
</MainContent>
