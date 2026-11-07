import { dashboardHref } from '$lib/external-urls';

function withSearch(path: string, source: URL): string {
	const target = new URL(dashboardHref(path));
	target.search = source.search;
	return target.toString();
}

export function legacyDashboardTarget(rest: string | undefined, source: URL): string {
	if (!rest) return withSearch('/', source);

	if (rest === 'create-tenant') return withSearch('/tenants/new', source);

	if (rest === 'account' || rest === 'organization') {
		const target = new URL(dashboardHref('/'));
		target.search = source.search;
		target.searchParams.set('settings', rest);
		return target.toString();
	}

	return withSearch(`/tenants/${rest}`, source);
}

export function dashboardRuntimeTarget(path: string, source: URL): string {
	return withSearch(path, source);
}
