import { websiteHref } from '$lib/external-urls';

function appendSearch(path: string, source: URL): string {
	return `${path}${source.search}`;
}

export function legacyDashboardTarget(rest: string | undefined, source: URL): string {
	if (!rest) return appendSearch('/', source);

	if (rest === 'create-tenant') return appendSearch('/tenants/new', source);

	if (rest === 'account' || rest === 'organization') {
		const params = new URLSearchParams(source.searchParams);
		params.set('settings', rest);
		return `/?${params}`;
	}

	return appendSearch(`/tenants/${rest}`, source);
}

export function websiteDocsTarget(rest: string | undefined, source: URL): string {
	const suffix = rest ? `/docs/${rest}` : '/docs';
	const target = new URL(websiteHref(suffix));
	target.search = source.search;
	return target.toString();
}
