import { env } from '$env/dynamic/public';

const websiteBaseUrl = (env.PUBLIC_WEBSITE_BASE_URL ?? 'http://localhost:5174').replace(/\/+$/, '');

export function websiteHref(path = '/'): string {
	const normalizedPath = path.startsWith('/') ? path : `/${path}`;
	return new URL(normalizedPath, `${websiteBaseUrl}/`).toString();
}
