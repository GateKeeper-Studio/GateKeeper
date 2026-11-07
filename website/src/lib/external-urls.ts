import { env } from '$env/dynamic/public';

const dashboardBaseUrl = (env.PUBLIC_DASHBOARD_BASE_URL ?? 'http://localhost:5173').replace(
	/\/+$/,
	''
);

export function dashboardHref(path = '/'): string {
	const normalizedPath = path.startsWith('/') ? path : `/${path}`;
	return new URL(normalizedPath, `${dashboardBaseUrl}/`).toString();
}
