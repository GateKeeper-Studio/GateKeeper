import { browser } from '$app/environment';
import { page } from '$app/state';
import type { TenantView } from '$lib/services/admin/tenants';

const STORAGE_KEY = 'gatekeeper.activeTenantId';

/**
 * App-wide selected tenant.
 *
 * The active tenant is a *persistent* concept, not merely the URL. The `[tenantId]`
 * route param selects a tenant (and is remembered), but on global pages such as
 * Organization and after a reload we fall back to the last-used tenant — or the
 * first available one — so tenant-scoped navigation keeps working everywhere. Only an
 * account with zero tenants yields an `undefined` `tenantId`, which drives the
 * "create your first tenant" empty states.
 *
 * The sidebar switcher feeds the fetched list in via `tenants`/`loading`; every other
 * component just reads `selectedTenant.current` / `selectedTenant.tenantId` reactively.
 * Import the singleton, never construct your own.
 */
class SelectedTenantStore {
	/** The full tenant list, supplied by whoever fetches it (the sidebar). */
	tenants = $state<TenantView[]>([]);

	/** Whether the tenant list is still loading — lets consumers avoid empty-state flashes. */
	loading = $state(true);

	/** The tenant-list fetch error, if any — surfaced by the sidebar switcher. */
	error = $state<Error | null>(null);

	/** Last-used tenant id, persisted across reloads and global pages. */
	#activeTenantId = $state<string | undefined>(
		browser ? (localStorage.getItem(STORAGE_KEY) ?? undefined) : undefined
	);

	/**
	 * The effective tenant id: the route's `[tenantId]` wins, otherwise the remembered
	 * tenant (honored before the list loads and while it still exists), otherwise the
	 * first available tenant. Only `undefined` when the account has no tenants at all.
	 */
	get tenantId(): string | undefined {
		const routeId = page.params.tenantId;
		if (routeId) return routeId;
		if (
			this.#activeTenantId &&
			(this.tenants.length === 0 ||
				this.tenants.some((tenant) => tenant.tenantId === this.#activeTenantId))
		) {
			return this.#activeTenantId;
		}
		return this.tenants[0]?.tenantId;
	}

	/** The route's own `[tenantId]` param — `undefined` on global pages. */
	get routeTenantId(): string | undefined {
		return page.params.tenantId;
	}

	/** The tenant matching the effective `tenantId`, once the list has loaded. */
	readonly current = $derived(
		this.tenants.find((tenant) => tenant.tenantId === this.tenantId) ?? null
	);

	/** Remember a tenant as the active one (persisted across reloads). */
	setActive(tenantId: string) {
		this.#activeTenantId = tenantId;
		if (browser) localStorage.setItem(STORAGE_KEY, tenantId);
	}

	/** Forget the active tenant (e.g. after the last remaining tenant is deleted). */
	clearActive() {
		this.#activeTenantId = undefined;
		if (browser) localStorage.removeItem(STORAGE_KEY);
	}
}

export const selectedTenant = new SelectedTenantStore();
