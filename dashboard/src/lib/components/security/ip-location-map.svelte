<script lang="ts">
	import { createQuery, keepPreviousData, useQueryClient } from '@tanstack/svelte-query';
	import { LngLatBounds, type Map as MapLibreMap } from 'maplibre-gl';
	import { untrack } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { toast } from 'svelte-sonner';

	import ErrorAlert from '$lib/components/error-alert.svelte';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';
	import {
		Map,
		MapControls,
		MapMarker,
		MarkerContent,
		MarkerPopup,
		MarkerTooltip
	} from '$lib/components/ui/map/index.js';
	import { m } from '$lib/paraglide/messages.js';
	import {
		blockIp,
		listIpLocations,
		unblockIp,
		type IpLocationView
	} from '$lib/services/admin/security';
	import {
		BADGE_CLASS,
		CATEGORIES,
		DOT_CLASS,
		RANGES,
		categoryOf,
		dotSize,
		networkOf,
		placeOf,
		type Category,
		type Range
	} from './ip-location.js';

	type Props = {
		tenantId: string;
		/** One user's sign-ins (the user page); omitted, the whole tenant's. */
		userId?: string;
		/** The window shown first. */
		defaultRange?: Range;
	};

	let { tenantId, userId, defaultRange = '7d' }: Props = $props();

	const queryClient = useQueryClient();

	// The initial window only; the select owns it afterwards.
	let range = $state<Range>(untrack(() => defaultRange));
	const rangeOptions = Object.keys(RANGES) as Range[];
	let map = $state.raw<MapLibreMap | null>(null);
	let busyIp = $state<string | null>(null);
	const hidden = new SvelteSet<Category>();

	const locationsQuery = createQuery(() => ({
		queryKey: ['security-ip-locations', tenantId, userId ?? 'tenant', range],
		enabled: !!tenantId,
		placeholderData: keepPreviousData,
		queryFn: async () => {
			const from = new Date(Date.now() - RANGES[range] * 24 * 60 * 60 * 1000).toISOString();
			const [data, err] = await listIpLocations(tenantId, { userId, from, limit: 300 });
			if (err) throw err;
			return data;
		}
	}));

	const points = $derived(locationsQuery.data?.items ?? []);
	const counts = $derived.by(() => {
		const byCategory: Record<Category, number> = { blocked: 0, attack: 0, warning: 0, normal: 0 };
		for (const point of points) byCategory[categoryOf(point)] += 1;
		return byCategory;
	});
	// The server sends the riskiest first; draw them last so they sit on top.
	const visible = $derived(points.filter((p) => !hidden.has(categoryOf(p))).reverse());

	const categoryLabel: Record<Category, () => string> = {
		blocked: m.security_map_legend_blocked,
		attack: m.security_map_legend_attack,
		warning: m.security_map_legend_warning,
		normal: m.security_map_legend_normal
	};

	const rangeLabel: Record<Range, () => string> = {
		'1d': m.security_map_range_1d,
		'7d': m.security_map_range_7d,
		'30d': m.security_map_range_30d,
		'90d': m.security_map_range_90d
	};

	function toggle(category: Category) {
		if (hidden.has(category)) hidden.delete(category);
		else hidden.add(category);
	}

	// Frame the points whenever a different set of IPs arrives, but not when the same
	// set comes back (a refetch after blocking one), so the admin's view stays put.
	let framed = '';
	$effect(() => {
		if (!map || points.length === 0) return;
		const key = points.map((p) => p.ip_address).join(',');
		if (key === framed) return;
		framed = key;

		if (points.length === 1) {
			map.jumpTo({ center: [points[0].longitude, points[0].latitude], zoom: 5 });
			return;
		}
		const bounds = new LngLatBounds();
		for (const point of points) bounds.extend([point.longitude, point.latitude]);
		map.fitBounds(bounds, { padding: 64, maxZoom: 6, duration: 600 });
	});

	function refreshAfterBlockChange() {
		queryClient.invalidateQueries({ queryKey: ['security-ip-locations', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['suspicious-ips', tenantId] });
		queryClient.invalidateQueries({ queryKey: ['security-overview', tenantId] });
	}

	async function block(ip: string) {
		busyIp = ip;
		const [, err] = await blockIp(tenantId, { ip_address: ip, reason: 'Blocked from the map' });
		busyIp = null;
		if (err) {
			toast.error(m.security_suspicious_ips_block_error({ ip, message: err.message }));
			return;
		}
		toast.success(m.security_suspicious_ips_block_success({ ip }));
		refreshAfterBlockChange();
	}

	async function unblock(ip: string, blockId: string) {
		busyIp = ip;
		const [err] = await unblockIp(tenantId, blockId);
		busyIp = null;
		if (err) {
			toast.error(m.security_suspicious_ips_unblock_error({ ip, message: err.message }));
			return;
		}
		toast.success(m.security_suspicious_ips_unblock_success({ ip }));
		refreshAfterBlockChange();
	}

	const formatDate = (iso: string) => new Date(iso).toLocaleString();
</script>

<div class="space-y-3">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="flex flex-wrap gap-1.5">
			{#each CATEGORIES as category (category)}
				<button
					type="button"
					class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs transition-opacity {hidden.has(
						category
					)
						? 'opacity-40'
						: ''}"
					aria-pressed={!hidden.has(category)}
					aria-label={m.security_map_legend_toggle({ category: categoryLabel[category]() })}
					onclick={() => toggle(category)}
				>
					<span class="size-2.5 rounded-full {DOT_CLASS[category]}"></span>
					{categoryLabel[category]()}
					<span class="text-muted-foreground">{counts[category]}</span>
				</button>
			{/each}
		</div>
		<label class="flex items-center gap-2 text-sm">
			<span class="text-muted-foreground">{m.security_map_range_label()}</span>
			<select
				class="rounded-md border border-input bg-background px-3 py-1.5 text-sm"
				bind:value={range}
			>
				{#each rangeOptions as option (option)}
					<option value={option}>{rangeLabel[option]()}</option>
				{/each}
			</select>
		</label>
	</div>

	{#if locationsQuery.isPending}
		<Skeleton class="h-[26rem] w-full rounded-xl" />
	{:else if locationsQuery.isError}
		<ErrorAlert title={m.security_map_load_error_title()} message={locationsQuery.error.message} />
	{:else}
		<div
			class="relative h-[26rem] overflow-hidden rounded-xl border"
			role="region"
			aria-label={m.security_map_aria_label()}
		>
			<Map bind:map center={[0, 20]} zoom={1.2}>
				{#each visible as point (point.ip_address)}
					{@const category = categoryOf(point)}
					{@const size = dotSize(point.attempts)}
					<MapMarker longitude={point.longitude} latitude={point.latitude}>
						<MarkerContent>
							<span class="relative flex items-center justify-center">
								{#if category === 'attack' || category === 'blocked'}
									<span
										class="absolute rounded-full opacity-50 motion-safe:animate-ping {DOT_CLASS[
											category
										]}"
										style="width: {size}px; height: {size}px"
									></span>
								{/if}
								<span
									class="relative rounded-full border-2 border-white shadow-md dark:border-neutral-900 {DOT_CLASS[
										category
									]}"
									style="width: {size}px; height: {size}px"
								></span>
							</span>
						</MarkerContent>
						<MarkerTooltip anchor="top">
							<span class="font-mono">{point.ip_address}</span>
							· {placeOf(point) ?? m.security_map_unknown_location()}
						</MarkerTooltip>
						<MarkerPopup class="w-72" closeButton>
							{@render details(point, category)}
						</MarkerPopup>
					</MapMarker>
				{/each}
				<MapControls position="bottom-right" showFullscreen />
			</Map>

			{#if points.length === 0}
				<div class="pointer-events-none absolute inset-0 flex items-center justify-center p-6">
					<p
						class="max-w-sm rounded-lg bg-background/90 px-4 py-3 text-center text-sm text-muted-foreground shadow-sm"
					>
						{m.security_map_empty()}
					</p>
				</div>
			{/if}
		</div>

		{#if locationsQuery.data.unlocated_attempts > 0 || locationsQuery.data.attribution}
			<div
				class="flex flex-wrap items-start justify-between gap-x-4 gap-y-1 text-xs text-muted-foreground"
			>
				{#if locationsQuery.data.unlocated_attempts > 0}
					<p>{m.security_map_unlocated({ count: locationsQuery.data.unlocated_attempts })}</p>
				{/if}
				{#if locationsQuery.data.attribution}
					{@const attribution = locationsQuery.data.attribution}
					<!-- Required by the geolocation database's licence. An external page, not an app
					     route, so resolve() does not apply. -->
					<!-- eslint-disable svelte/no-navigation-without-resolve -->
					<a
						class="ml-auto underline-offset-2 hover:underline"
						href={attribution.url}
						target="_blank"
						rel="noopener noreferrer">{attribution.name}</a
					>
					<!-- eslint-enable svelte/no-navigation-without-resolve -->
				{/if}
			</div>
		{/if}
	{/if}
</div>

{#snippet details(point: IpLocationView, category: Category)}
	<div class="space-y-3 text-xs">
		<div class="flex items-start justify-between gap-2 pr-5">
			<div class="min-w-0">
				<p class="truncate font-mono text-sm font-semibold">{point.ip_address}</p>
				<p class="text-muted-foreground">{placeOf(point) ?? m.security_map_unknown_location()}</p>
				{#if networkOf(point)}
					<p class="truncate text-muted-foreground">{networkOf(point)}</p>
				{/if}
			</div>
			<span class="shrink-0 rounded px-2 py-0.5 font-medium {BADGE_CLASS[category]}">
				{categoryLabel[category]()}
			</span>
		</div>

		<dl class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1">
			<dt class="text-muted-foreground">{m.security_map_attempts()}</dt>
			<dd class="text-right font-medium">{point.attempts}</dd>
			<dt class="text-muted-foreground">{m.security_map_successful()}</dt>
			<dd class="text-right font-medium text-green-600">{point.successful_logins}</dd>
			<dt class="text-muted-foreground">{m.security_map_failed()}</dt>
			<dd class="text-right font-medium text-red-600">{point.failed_attempts}</dd>
			{#if !userId}
				<dt class="text-muted-foreground">{m.security_map_accounts()}</dt>
				<dd class="text-right font-medium">{point.distinct_users}</dd>
			{/if}
			{#if point.blocked_attempts > 0}
				<dt class="text-muted-foreground">{m.security_map_refused()}</dt>
				<dd class="text-right font-medium">{point.blocked_attempts}</dd>
			{/if}
			<dt class="text-muted-foreground">{m.security_map_highest_risk()}</dt>
			<dd class="text-right font-medium">{point.max_risk_level} ({point.max_risk_score})</dd>
			<dt class="text-muted-foreground">{m.security_map_first_seen()}</dt>
			<dd class="text-right">{formatDate(point.first_seen)}</dd>
			<dt class="text-muted-foreground">{m.security_map_last_seen()}</dt>
			<dd class="text-right">{formatDate(point.last_seen)}</dd>
		</dl>

		{#if point.blocked && point.block_expires_at}
			<p class="font-medium text-red-600">
				{m.security_map_blocked_until({ date: formatDate(point.block_expires_at) })}
			</p>
		{/if}

		<div class="flex flex-wrap gap-2">
			<Button
				size="xs"
				variant="outline"
				href={`/console/tenants/${tenantId}/security/login-events?ip=${encodeURIComponent(point.ip_address)}`}
			>
				{m.security_map_view_attempts()}
			</Button>
			{#if point.blocked && point.block_id}
				{@const blockId = point.block_id}
				<Button
					size="xs"
					variant="outline"
					disabled={busyIp === point.ip_address}
					onclick={() => unblock(point.ip_address, blockId)}
				>
					{m.security_map_unblock()}
				</Button>
			{:else}
				<Button
					size="xs"
					variant="destructive"
					disabled={busyIp === point.ip_address}
					onclick={() => block(point.ip_address)}
				>
					{m.security_map_block()}
				</Button>
			{/if}
		</div>
	</div>
{/snippet}
