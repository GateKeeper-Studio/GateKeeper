<script lang="ts">
	import { onDestroy } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from 'svelte-sonner';

	import CheckIcon from '@lucide/svelte/icons/check';
	import SaveIcon from '@lucide/svelte/icons/save';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import LoadingSpinner from '$lib/components/loading-spinner.svelte';
	import { m } from '$lib/paraglide/messages.js';

	import { updateTenantAppearance, type TenantAppearance } from '$lib/services/admin/tenants';
	import { selectedTenant } from '$lib/state/selected-tenant.svelte';
	import { appTheme } from '$lib/state/app-theme.svelte';

	let { tenantId, appearance }: { tenantId: string; appearance: TenantAppearance | null } =
		$props();

	const queryClient = useQueryClient();

	// Curated brand presets — a fast one-click path before the custom picker.
	const PRESETS = [
		'#7C3AED', // violet
		'#2563EB', // blue
		'#0EA5E9', // sky
		'#10B981', // emerald
		'#F59E0B', // amber
		'#EF4444', // red
		'#EC4899', // pink
		'#111827' // near-black
	];

	const RADII: { key: TenantAppearance['radius']; label: string }[] = [
		{ key: 'none', label: m.settings_appearance_radius_none() },
		{ key: 'sm', label: m.settings_appearance_radius_small() },
		{ key: 'md', label: m.settings_appearance_radius_medium() },
		{ key: 'lg', label: m.settings_appearance_radius_large() }
	];

	const MODES: { key: TenantAppearance['defaultMode']; label: string }[] = [
		{ key: 'light', label: m.settings_appearance_mode_light() },
		{ key: 'dark', label: m.settings_appearance_mode_dark() },
		{ key: 'system', label: m.settings_appearance_mode_system() }
	];

	// Draft, seeded from the saved appearance (or sensible defaults for a fresh org).
	let color = $state(appearance?.primaryColor ?? '#7C3AED');
	let radius = $state<TenantAppearance['radius']>(appearance?.radius ?? 'md');
	let mode = $state<TenantAppearance['defaultMode']>(appearance?.defaultMode ?? 'system');

	let isSaving = $state(false);
	let isResetting = $state(false);

	const isHex = $derived(/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(color));

	function draft(): TenantAppearance {
		return { primaryColor: color, radius, defaultMode: mode };
	}

	// Live-preview colour/radius changes across the whole app (the mode setting is a
	// saved fallback and intentionally does NOT flip the current theme while editing).
	function livePreview() {
		if (isHex) appTheme.preview(draft());
	}

	function pickColor(next: string) {
		color = next;
		livePreview();
	}

	function pickRadius(next: TenantAppearance['radius']) {
		radius = next;
		livePreview();
	}

	// If the user leaves the section without saving, snap back to the persisted theme
	// (the shared tenant store is the source of truth after a save or a cancel).
	onDestroy(() => appTheme.apply(selectedTenant.current?.appearance ?? null));

	function patchTenant(next: TenantAppearance | null) {
		selectedTenant.tenants = selectedTenant.tenants.map((t) =>
			t.tenantId === tenantId ? { ...t, appearance: next } : t
		);
	}

	async function save() {
		if (!isHex) {
			toast.error(m.settings_appearance_invalid_hex());
			return;
		}
		isSaving = true;
		const next = draft();
		const [err] = await updateTenantAppearance(tenantId, next);
		isSaving = false;

		if (err) {
			toast.error(err.data?.message || err.message || m.settings_appearance_save_error());
			return;
		}

		patchTenant(next);
		appTheme.apply(next);
		toast.success(m.settings_appearance_save_success());
		queryClient.invalidateQueries({ queryKey: ['tenants'] });
		queryClient.invalidateQueries({ queryKey: ['tenant', tenantId] });
	}

	async function resetToDefault() {
		isResetting = true;
		const [err] = await updateTenantAppearance(tenantId, null);
		isResetting = false;

		if (err) {
			toast.error(err.data?.message || err.message || m.settings_appearance_reset_error());
			return;
		}

		patchTenant(null);
		color = '#7C3AED';
		radius = 'md';
		mode = 'system';
		appTheme.reset();
		toast.success(m.settings_appearance_reset_success());
		queryClient.invalidateQueries({ queryKey: ['tenants'] });
		queryClient.invalidateQueries({ queryKey: ['tenant', tenantId] });
	}
</script>

<div class="grid gap-8 lg:grid-cols-[1fr_320px]">
	<!-- ── Controls ─────────────────────────────────────────────────────────── -->
	<div class="grid gap-8">
		<!-- Main color -->
		<div class="space-y-3">
			<Label class="text-sm font-medium">{m.settings_appearance_main_color_label()}</Label>
			<p class="text-xs text-muted-foreground">
				{m.settings_appearance_main_color_description()}
			</p>
			<div class="flex flex-wrap gap-2.5">
				{#each PRESETS as preset (preset)}
					<button
						type="button"
						aria-label={m.settings_appearance_color_swatch_aria({ color: preset })}
						aria-pressed={color.toLowerCase() === preset.toLowerCase()}
						onclick={() => pickColor(preset)}
						class="relative size-9 rounded-full ring-offset-2 ring-offset-background transition-transform hover:scale-110 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none {color.toLowerCase() ===
						preset.toLowerCase()
							? 'ring-2 ring-ring'
							: ''}"
						style="background-color: {preset};"
					>
						{#if color.toLowerCase() === preset.toLowerCase()}
							<CheckIcon class="absolute inset-0 m-auto size-4 text-white drop-shadow" />
						{/if}
					</button>
				{/each}
			</div>

			<div class="flex items-center gap-3 pt-1">
				<label
					class="relative size-9 shrink-0 cursor-pointer overflow-hidden rounded-md border"
					style="background-color: {isHex ? color : 'transparent'};"
				>
					<input
						type="color"
						value={isHex ? color : '#7C3AED'}
						oninput={(e) => pickColor(e.currentTarget.value)}
						class="absolute inset-0 size-full cursor-pointer opacity-0"
					/>
				</label>
				<div class="space-y-1">
					<Input
						value={color}
						oninput={(e) => pickColor(e.currentTarget.value)}
						spellcheck={false}
						class="h-9 w-36 font-mono text-sm uppercase"
						aria-invalid={!isHex}
					/>
					{#if !isHex}
						<p class="text-xs text-destructive">{m.settings_appearance_hex_hint()}</p>
					{/if}
				</div>
			</div>
		</div>

		<!-- Corner radius -->
		<div class="space-y-3">
			<Label class="text-sm font-medium">{m.settings_appearance_radius_label()}</Label>
			<p class="text-xs text-muted-foreground">
				{m.settings_appearance_radius_description()}
			</p>
			<div class="inline-flex flex-wrap gap-1.5 rounded-lg bg-muted p-1">
				{#each RADII as option (option.key)}
					<Button
						type="button"
						size="sm"
						variant={radius === option.key ? 'default' : 'ghost'}
						class="h-8"
						onclick={() => pickRadius(option.key)}
					>
						{option.label}
					</Button>
				{/each}
			</div>
		</div>

		<!-- Default theme -->
		<div class="space-y-3">
			<Label class="text-sm font-medium">{m.settings_appearance_mode_label()}</Label>
			<p class="text-xs text-muted-foreground">
				{m.settings_appearance_mode_description()}
			</p>
			<div class="inline-flex flex-wrap gap-1.5 rounded-lg bg-muted p-1">
				{#each MODES as option (option.key)}
					<Button
						type="button"
						size="sm"
						variant={mode === option.key ? 'default' : 'ghost'}
						class="h-8"
						onclick={() => (mode = option.key)}
					>
						{option.label}
					</Button>
				{/each}
			</div>
		</div>

		<div class="flex flex-wrap items-center gap-3 pt-2">
			<Button type="button" disabled={isSaving || !isHex} onclick={save}>
				{#if isSaving}<LoadingSpinner class="size-4" />{:else}<SaveIcon />{/if}
				{isSaving ? m.settings_appearance_saving() : m.settings_appearance_save_button()}
			</Button>
			<Button
				type="button"
				variant="ghost"
				disabled={isResetting}
				onclick={resetToDefault}
				class="relative text-muted-foreground"
			>
				{#if isResetting}<LoadingSpinner class="absolute left-3" />{/if}
				{m.settings_appearance_reset_button()}
			</Button>
		</div>
	</div>

	<!-- ── Live preview ─────────────────────────────────────────────────────── -->
	<div class="space-y-3">
		<Label class="text-sm font-medium">{m.settings_appearance_preview_label()}</Label>
		<div class="space-y-4 rounded-xl border bg-card p-5 shadow-sm">
			<div class="space-y-1">
				<p class="text-sm font-semibold">{m.settings_appearance_preview_heading()}</p>
				<p class="text-xs text-muted-foreground">{m.settings_appearance_preview_description()}</p>
			</div>
			<Input placeholder={m.settings_appearance_preview_email_placeholder()} class="h-9" />
			<div class="flex flex-wrap gap-2">
				<Button size="sm">{m.settings_appearance_preview_primary_button()}</Button>
				<Button size="sm" variant="outline"
					>{m.settings_appearance_preview_secondary_button()}</Button
				>
			</div>
			<div class="flex items-center gap-2 text-xs">
				<span class="inline-block size-4 rounded-full bg-primary"></span>
				<span class="text-muted-foreground">{m.settings_appearance_preview_accent_label()}</span>
			</div>
		</div>
	</div>
</div>
