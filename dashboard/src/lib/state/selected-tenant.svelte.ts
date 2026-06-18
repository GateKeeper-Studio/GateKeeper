import { page } from '$app/state';
import type { TenantView } from '$lib/services/admin/tenants';

/**
 * App-wide selected tenant, anchored to the URL.
 *
 * The `[tenantId]` route param is the source of truth: `current` is whichever
 * loaded tenant matches `/dashboard/{tenantId}`. The sidebar switcher feeds the
 * fetched list in via `tenants` and navigates to change the selection; every
 * other component just reads `selectedTenant.current` and re-renders reactively.
 * Import the singleton, never construct your own.
 */
class SelectedTenantStore {
	/** The full tenant list, supplied by whoever fetches it (the sidebar). */
	tenants = $state<TenantView[]>([]);

	/** The tenant matching the current `[tenantId]` route param, if any. */
	readonly current = $derived(
		this.tenants.find((tenant) => tenant.tenantId === page.params.tenantId) ?? null
	);

	/** The selected tenant id straight from the URL — available before the list loads. */
	get tenantId(): string | undefined {
		return page.params.tenantId;
	}
}

export const selectedTenant = new SelectedTenantStore();
