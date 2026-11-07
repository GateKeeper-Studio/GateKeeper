<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';
	import Maximize2Icon from '@lucide/svelte/icons/maximize-2';
	import SaveIcon from '@lucide/svelte/icons/save';

	import * as Card from '$lib/components/ui/card/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import * as Tabs from '$lib/components/ui/tabs/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Textarea } from '$lib/components/ui/textarea/index.js';
	import { Checkbox } from '$lib/components/ui/checkbox/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';

	import { updateOAuthClient, type OAuthClientView } from '$lib/services/admin/oauth-clients';
	import { validate } from '$lib/utils/form';
	import { LOCALES } from '$lib/i18n/locales';
	import { m } from '$lib/paraglide/messages.js';
	import { appearanceSchema, validateLocalizedText } from './appearance-schema';
	import {
		LOGIN_LAYOUTS,
		LAYOUT_LABELS,
		LAYOUT_DESCRIPTIONS,
		LOGIN_FONTS,
		FONT_LABELS,
		RADIUS_PRESETS,
		type LoginAppearance,
		type LoginLayout,
		type LoginFont,
		type HeroSide
	} from '$lib/types/login-appearance';

	type Props = { client: OAuthClientView; tenantId: string; clientId: string };
	let { client, tenantId, clientId }: Props = $props();

	const queryClient = useQueryClient();

	// ── editable form state (initialised once from the client) ─────────────────
	const initial = client.login_appearance;
	let enabled = $state(initial !== null);
	let layout = $state<LoginLayout>(initial?.layout ?? 'split_image');
	let heroPosition = $state<HeroSide>(initial?.heroPosition ?? 'left');
	let logoUri = $state(initial?.logoUri ?? '');
	let backgroundImageUri = $state(initial?.backgroundImageUri ?? '');
	let heading = $state<Record<string, string>>({ ...(initial?.heading ?? {}) });
	let subtext = $state<Record<string, string>>({ ...(initial?.subtext ?? {}) });
	let primaryColor = $state(initial?.primaryColor ?? '');
	let accentColor = $state(initial?.accentColor ?? '');
	let borderRadius = $state(initial?.borderRadius ?? '');
	let fontFamily = $state<LoginFont | ''>(initial?.fontFamily ?? '');

	// Which languages this client's hosted pages offer, and which is the
	// default absent a user override. Independent of `enabled` — languages
	// apply to the whole client, not just its custom appearance.
	let allowedLanguages = $state<string[]>(
		client.allowed_languages.length ? [...client.allowed_languages] : ['en']
	);
	let defaultLanguage = $state<string>(client.default_language || 'en');
	let activeLanguageTab = $state<string>(defaultLanguage);

	function toggleLanguage(code: string, checked: boolean) {
		if (checked) {
			if (!allowedLanguages.includes(code)) allowedLanguages = [...allowedLanguages, code];
		} else {
			if (allowedLanguages.length <= 1) return; // at least one language must remain
			allowedLanguages = allowedLanguages.filter((c) => c !== code);
			if (defaultLanguage === code) defaultLanguage = allowedLanguages[0];
			if (activeLanguageTab === code) activeLanguageTab = allowedLanguages[0];
		}
	}

	let errors = $state<Record<string, string>>({});
	let saving = $state(false);

	function dropEmpty(map: Record<string, string>): Record<string, string> | null {
		const cleaned = Object.fromEntries(
			Object.entries(map)
				.map(([k, v]) => [k, v.trim()])
				.filter(([, v]) => v)
		);
		return Object.keys(cleaned).length ? cleaned : null;
	}

	// The appearance object (or null when customization is off) built from the form.
	function buildAppearance(): LoginAppearance | null {
		if (!enabled) return null;
		return {
			layout,
			heroPosition,
			logoUri: logoUri.trim() || null,
			backgroundImageUri: backgroundImageUri.trim() || null,
			heading: dropEmpty(heading),
			subtext: dropEmpty(subtext),
			primaryColor: primaryColor.trim() || null,
			accentColor: accentColor.trim() || null,
			borderRadius: borderRadius || null,
			fontFamily: fontFamily || null
		};
	}

	const draft = $derived(buildAppearance());

	const fontLabel = $derived(
		fontFamily ? FONT_LABELS[fontFamily] : m.oauth_clients_appearance_tab_default_option()
	);
	const radiusLabel = $derived(
		RADIUS_PRESETS.find((r) => r.value === borderRadius)?.label ??
			m.oauth_clients_appearance_tab_default_option()
	);
	const heroPositionLabel = $derived(
		heroPosition === 'right'
			? m.oauth_clients_appearance_tab_position_right()
			: m.oauth_clients_appearance_tab_position_left()
	);

	// ── live preview (sandboxed iframe rendering the real sign-in page) ─────────
	let iframe = $state<HTMLIFrameElement | null>(null);
	let fullscreenIframe = $state<HTMLIFrameElement | null>(null);
	let previewReady = $state(false);
	let previewDialogOpen = $state(false);
	// The hosted login route is keyed by the PUBLIC client_id (not the internal id
	// used by the admin route param), since /auth/[clientId] resolves via the
	// public auth-data endpoint.
	const previewSrc = $derived(`/auth/${client.client_id}/sign-in?preview=1`);

	function postPreview() {
		for (const target of [iframe, fullscreenIframe]) {
			target?.contentWindow?.postMessage(
				{ type: 'gk-preview-appearance', appearance: draft },
				window.location.origin
			);
		}
	}

	// The embedded page announces readiness; then we keep it in sync with the draft.
	$effect(() => {
		function onMessage(event: MessageEvent) {
			if (event.origin !== window.location.origin) return;
			if (event.data?.type === 'gk-preview-ready') {
				previewReady = true;
				postPreview();
			}
		}
		window.addEventListener('message', onMessage);
		return () => window.removeEventListener('message', onMessage);
	});

	// Debounced re-post whenever the draft changes.
	$effect(() => {
		void draft; // track all form fields
		if (!previewReady) return;
		const t = setTimeout(postPreview, 250);
		return () => clearTimeout(t);
	});

	$effect(() => {
		if (!previewDialogOpen || !fullscreenIframe) return;
		const t = setTimeout(postPreview, 0);
		return () => clearTimeout(t);
	});

	async function save() {
		errors = {};
		if (enabled) {
			const res = validate(appearanceSchema, {
				logoUri,
				backgroundImageUri,
				primaryColor,
				accentColor
			});
			const headingError = validateLocalizedText(heading, 120);
			const subtextError = validateLocalizedText(subtext, 300);
			if (!res.success || headingError || subtextError) {
				errors = {
					...(res.success ? {} : res.errors),
					...(headingError ? { heading: headingError } : {}),
					...(subtextError ? { subtext: subtextError } : {})
				};
				toast.error(m.oauth_clients_appearance_tab_fix_fields_error());
				return;
			}
		}

		saving = true;
		// Full-replacement PUT: resend every editable field, changing only appearance.
		const [err] = await updateOAuthClient(tenantId, clientId, {
			name: client.name,
			redirectUris: client.redirect_uris,
			postLogoutRedirectUris: client.post_logout_redirect_uris,
			allowedScopes: client.allowed_scopes,
			allowedGrantTypes: client.allowed_grant_types,
			requirePkce: client.require_pkce,
			requireConsent: client.require_consent,
			mfaRequired: client.mfa_required,
			mfaEmailEnabled: client.mfa_email_enabled,
			mfaTotpEnabled: client.mfa_totp_enabled,
			mfaWebauthnEnabled: client.mfa_webauthn_enabled,
			isActive: client.is_active,
			emailProviderId: client.email_provider_id,
			tosUri: client.tos_uri,
			policyUri: client.policy_uri,
			initiateLoginUri: client.initiate_login_uri,
			externalLoginOnly: client.external_login_only,
			// The update replaces the whole client: omitting this would switch it off.
			magicLinkEnabled: client.magic_link_enabled,
			backchannelLogoutUri: client.backchannel_logout_uri,
			backchannelLogoutSessionRequired: client.backchannel_logout_session_required,
			loginAppearance: buildAppearance(),
			allowedLanguages,
			defaultLanguage
		});
		saving = false;

		if (err) {
			toast.error(
				err.data?.message || err.data?.error || m.oauth_clients_appearance_tab_save_error()
			);
			return;
		}
		toast.success(m.oauth_clients_appearance_tab_save_success());
		queryClient.invalidateQueries({ queryKey: ['oauth-clients', tenantId, clientId] });
	}
</script>

{#snippet colorField(
	id: string,
	label: string,
	value: string,
	set: (v: string) => void,
	error?: string
)}
	<div class="space-y-2">
		<Label for={id}>{label}</Label>
		<div class="flex items-center gap-2">
			<input
				type="color"
				aria-label={m.oauth_clients_appearance_tab_color_picker_aria_label({ label })}
				value={value || '#6366f1'}
				oninput={(e) => set(e.currentTarget.value)}
				class="h-9 w-10 shrink-0 cursor-pointer rounded-md border bg-transparent p-1"
			/>
			<Input
				{id}
				placeholder={m.oauth_clients_appearance_tab_color_placeholder()}
				{value}
				oninput={(e) => set(e.currentTarget.value)}
			/>
			{#if value}
				<Button type="button" variant="ghost" size="sm" onclick={() => set('')}
					>{m.oauth_clients_action_clear()}</Button
				>
			{/if}
		</div>
		{#if error}<p class="text-sm font-medium text-destructive">{error}</p>{/if}
	</div>
{/snippet}

<div class="grid gap-6 xl:grid-cols-2">
	<!-- ── Controls ─────────────────────────────────────────────────────────── -->
	<Card.Root>
		<Card.Header class="flex flex-row items-start justify-between gap-3">
			<div>
				<Card.Title>{m.oauth_clients_appearance_tab_card_title()}</Card.Title>
				<Card.Description>
					{m.oauth_clients_appearance_tab_card_description()}
				</Card.Description>
			</div>
			<Button size="sm" onclick={save} disabled={saving}>
				{#if saving}<LoadingSpinner class="mr-1" />{:else}<SaveIcon />{/if}
				{m.oauth_clients_action_save_changes()}
			</Button>
		</Card.Header>

		<Card.Content class="space-y-6">
			<label class="flex items-center gap-2 text-sm font-medium">
				<Checkbox bind:checked={enabled} />
				{m.oauth_clients_appearance_tab_customize_checkbox_label()}
			</label>

			<div class="space-y-2">
				<Label>{m.oauth_clients_appearance_tab_languages_label()}</Label>
				<div class="flex flex-wrap gap-4">
					{#each LOCALES as locale (locale.code)}
						<label class="flex items-center gap-2 text-sm">
							<Checkbox
								checked={allowedLanguages.includes(locale.code)}
								onCheckedChange={(checked) => toggleLanguage(locale.code, !!checked)}
							/>
							{locale.nativeLabel}
						</label>
					{/each}
				</div>
			</div>

			<div class="space-y-2">
				<Label>{m.oauth_clients_appearance_tab_default_language_label()}</Label>
				<Select.Root
					type="single"
					value={defaultLanguage}
					onValueChange={(v) => v && (defaultLanguage = v)}
				>
					<Select.Trigger class="w-full">
						{LOCALES.find((l) => l.code === defaultLanguage)?.nativeLabel ?? defaultLanguage}
					</Select.Trigger>
					<Select.Content>
						{#each LOCALES.filter((l) => allowedLanguages.includes(l.code)) as locale (locale.code)}
							<Select.Item value={locale.code} label={locale.nativeLabel}>
								{locale.nativeLabel}
							</Select.Item>
						{/each}
					</Select.Content>
				</Select.Root>
			</div>

			{#if enabled}
				<!-- Layout picker -->
				<div class="space-y-2">
					<Label>{m.oauth_clients_appearance_tab_layout_label()}</Label>
					<div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
						{#each LOGIN_LAYOUTS as l (l)}
							<button
								type="button"
								onclick={() => (layout = l)}
								aria-pressed={layout === l}
								class="flex flex-col gap-2 rounded-lg border p-2 text-left transition hover:bg-muted/50 {layout ===
								l
									? 'border-primary ring-2 ring-primary/30'
									: 'border-border'}"
							>
								<span
									class="flex h-12 w-full items-center justify-center gap-1 rounded bg-muted p-1"
								>
									{#if l === 'centered'}
										<span class="h-6 w-8 rounded-sm bg-primary/60"></span>
									{:else if l === 'split_image'}
										<span class="h-full w-1/2 rounded-sm bg-primary/60"></span>
										<span class="h-6 w-1/2 rounded-sm bg-foreground/20"></span>
									{:else if l === 'card'}
										<span
											class="h-8 w-9 rounded-sm border border-foreground/30 bg-background shadow-sm"
										></span>
									{:else}
										<span class="h-5 w-8 rounded-sm bg-foreground/20"></span>
									{/if}
								</span>
								<span class="text-xs font-medium">{LAYOUT_LABELS[l]}</span>
							</button>
						{/each}
					</div>
					<p class="text-xs text-muted-foreground">{LAYOUT_DESCRIPTIONS[layout]}</p>
				</div>

				{#if layout === 'split_image'}
					<div class="space-y-2">
						<Label>{m.oauth_clients_appearance_tab_image_position_label()}</Label>
						<Select.Root
							type="single"
							value={heroPosition}
							onValueChange={(v) => (heroPosition = (v as HeroSide) ?? 'left')}
						>
							<Select.Trigger class="w-full capitalize">{heroPositionLabel}</Select.Trigger>
							<Select.Content>
								<Select.Item value="left" label={m.oauth_clients_appearance_tab_position_left()}
									>{m.oauth_clients_appearance_tab_position_left()}</Select.Item
								>
								<Select.Item value="right" label={m.oauth_clients_appearance_tab_position_right()}
									>{m.oauth_clients_appearance_tab_position_right()}</Select.Item
								>
							</Select.Content>
						</Select.Root>
					</div>
				{/if}

				<!-- Branding assets -->
				<div class="space-y-2">
					<Label for="logo-uri">{m.oauth_clients_appearance_tab_logo_url_label()}</Label>
					<Input
						id="logo-uri"
						bind:value={logoUri}
						placeholder="https://cdn.example.com/logo.svg"
					/>
					{#if errors.logoUri}<p class="text-sm font-medium text-destructive">
							{errors.logoUri}
						</p>{/if}
				</div>

				<div class="space-y-2">
					<Label for="bg-uri">{m.oauth_clients_appearance_tab_bg_image_url_label()}</Label>
					<Input
						id="bg-uri"
						bind:value={backgroundImageUri}
						placeholder="https://cdn.example.com/hero.jpg"
					/>
					{#if errors.backgroundImageUri}
						<p class="text-sm font-medium text-destructive">{errors.backgroundImageUri}</p>
					{/if}
				</div>

				<!-- Heading/subtext are per-language — one tab per language this client
				     currently allows (set above). -->
				<div class="space-y-2">
					<Tabs.Root value={activeLanguageTab} onValueChange={(v) => v && (activeLanguageTab = v)}>
						<Tabs.List>
							{#each LOCALES.filter( (l) => allowedLanguages.includes(l.code) ) as locale (locale.code)}
								<Tabs.Trigger value={locale.code}>{locale.nativeLabel}</Tabs.Trigger>
							{/each}
						</Tabs.List>
						{#each LOCALES.filter((l) => allowedLanguages.includes(l.code)) as locale (locale.code)}
							<Tabs.Content value={locale.code} class="space-y-4">
								<div class="space-y-2">
									<Label for="heading-{locale.code}"
										>{m.oauth_clients_appearance_tab_heading_label()}</Label
									>
									<Input
										id="heading-{locale.code}"
										value={heading[locale.code] ?? ''}
										oninput={(e) =>
											(heading = { ...heading, [locale.code]: e.currentTarget.value })}
										placeholder={m.oauth_clients_appearance_tab_heading_placeholder()}
										maxlength={120}
									/>
								</div>
								<div class="space-y-2">
									<Label for="subtext-{locale.code}"
										>{m.oauth_clients_appearance_tab_subtext_label()}</Label
									>
									<Textarea
										id="subtext-{locale.code}"
										value={subtext[locale.code] ?? ''}
										oninput={(e) =>
											(subtext = { ...subtext, [locale.code]: e.currentTarget.value })}
										placeholder={m.oauth_clients_appearance_tab_subtext_placeholder()}
										rows={2}
									/>
								</div>
							</Tabs.Content>
						{/each}
					</Tabs.Root>
					{#if errors.heading}<p class="text-sm font-medium text-destructive">
							{errors.heading}
						</p>{/if}
					{#if errors.subtext}<p class="text-sm font-medium text-destructive">
							{errors.subtext}
						</p>{/if}
				</div>

				<!-- Theme tokens -->
				<div class="grid gap-4 sm:grid-cols-2">
					{@render colorField(
						'primary-color',
						m.oauth_clients_appearance_tab_primary_color_label(),
						primaryColor,
						(v) => (primaryColor = v),
						errors.primaryColor
					)}
					{@render colorField(
						'accent-color',
						m.oauth_clients_appearance_tab_accent_color_label(),
						accentColor,
						(v) => (accentColor = v),
						errors.accentColor
					)}
				</div>

				<div class="grid gap-4 sm:grid-cols-2">
					<div class="space-y-2">
						<Label>{m.oauth_clients_appearance_tab_corner_radius_label()}</Label>
						<Select.Root
							type="single"
							value={borderRadius}
							onValueChange={(v) => (borderRadius = v ?? '')}
						>
							<Select.Trigger class="w-full">{radiusLabel}</Select.Trigger>
							<Select.Content>
								<Select.Item value="" label={m.oauth_clients_appearance_tab_default_option()}
									>{m.oauth_clients_appearance_tab_default_option()}</Select.Item
								>
								{#each RADIUS_PRESETS as r (r.value)}
									<Select.Item value={r.value} label={r.label}>{r.label}</Select.Item>
								{/each}
							</Select.Content>
						</Select.Root>
					</div>

					<div class="space-y-2">
						<Label>{m.oauth_clients_appearance_tab_font_label()}</Label>
						<Select.Root
							type="single"
							value={fontFamily}
							onValueChange={(v) => (fontFamily = (v as LoginFont | '') ?? '')}
						>
							<Select.Trigger class="w-full">{fontLabel}</Select.Trigger>
							<Select.Content>
								<Select.Item value="" label={m.oauth_clients_appearance_tab_default_option()}
									>{m.oauth_clients_appearance_tab_default_option()}</Select.Item
								>
								{#each LOGIN_FONTS as f (f)}
									<Select.Item value={f} label={FONT_LABELS[f]}>{FONT_LABELS[f]}</Select.Item>
								{/each}
							</Select.Content>
						</Select.Root>
					</div>
				</div>
			{:else}
				<p class="text-sm text-muted-foreground">
					{m.oauth_clients_appearance_tab_disabled_hint()}
				</p>
			{/if}
		</Card.Content>
	</Card.Root>

	<!-- ── Live preview ─────────────────────────────────────────────────────── -->
	<Card.Root class="overflow-hidden">
		<Card.Header>
			<div>
				<Card.Title>{m.oauth_clients_appearance_tab_preview_card_title()}</Card.Title>
				<Card.Description
					>{m.oauth_clients_appearance_tab_preview_card_description()}</Card.Description
				>
			</div>
			<Card.Action>
				<Button variant="outline" size="sm" onclick={() => (previewDialogOpen = true)}>
					<Maximize2Icon data-icon="inline-start" />
					{m.oauth_clients_appearance_tab_open_preview_button()}
				</Button>
			</Card.Action>
		</Card.Header>
		<Card.Content>
			<div class="overflow-hidden rounded-xl border bg-muted">
				<iframe
					bind:this={iframe}
					title={m.oauth_clients_appearance_tab_preview_title()}
					src={previewSrc}
					sandbox="allow-scripts allow-same-origin allow-forms"
					class="h-[36rem] w-full bg-background"
				></iframe>
			</div>
			<p class="mt-2 text-xs text-muted-foreground">
				{m.oauth_clients_appearance_tab_preview_hint()}
			</p>
		</Card.Content>
	</Card.Root>
</div>

<Dialog.Root bind:open={previewDialogOpen}>
	<Dialog.Content
		class="grid h-[calc(100dvh-2rem)] w-[calc(100vw-2rem)] max-w-[calc(100vw-2rem)] grid-rows-[auto_1fr] gap-0 overflow-hidden rounded-xl p-0 sm:max-w-[calc(100vw-2rem)]"
	>
		<Dialog.Header class="border-b px-5 py-4 pr-16">
			<Dialog.Title>{m.oauth_clients_appearance_tab_preview_title()}</Dialog.Title>
			<Dialog.Description
				>{m.oauth_clients_appearance_tab_preview_fullscreen_description()}</Dialog.Description
			>
		</Dialog.Header>
		<div class="min-h-0 bg-muted">
			<iframe
				bind:this={fullscreenIframe}
				title={m.oauth_clients_appearance_tab_preview_fullscreen_iframe_title()}
				src={previewSrc}
				sandbox="allow-scripts allow-same-origin allow-forms"
				class="h-full w-full bg-background"
			></iframe>
		</div>
	</Dialog.Content>
</Dialog.Root>
