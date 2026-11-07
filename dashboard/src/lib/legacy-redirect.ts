import { websiteHref } from '$lib/external-urls';

// Old bookmarks: the console used to live under /dashboard/*, then at the site root.
// It now lives under /console/* (the root paths belong to the identity provider's
// API, which this app shares an origin with).

function appendSearch(path: string, source: URL): string {
	return `${path}${source.search}`;
}

export function legacyDashboardTarget(rest: string | undefined, source: URL): string {
	if (!rest) return appendSearch('/console', source);

	if (rest === 'create-tenant') return appendSearch('/console/tenants/new', source);

	if (rest === 'account' || rest === 'organization') {
		const params = new URLSearchParams(source.searchParams);
		params.set('settings', rest);
		return `/console?${params}`;
	}

	return appendSearch(`/console/tenants/${rest}`, source);
}

export function websiteDocsTarget(rest: string | undefined, source: URL): string {
	const suffix = rest ? `/docs/${rest}` : '/docs';
	const target = new URL(websiteHref(suffix));
	target.search = source.search;
	return target.toString();
}
