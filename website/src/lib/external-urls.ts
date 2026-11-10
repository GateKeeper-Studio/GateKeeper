import { env } from '$env/dynamic/public';

// The identity provider serves the dashboard (admin sign-in at /login, console at
// /console) on its own origin.
const dashboardBaseUrl = (env.PUBLIC_DASHBOARD_BASE_URL ?? 'http://localhost:8080').replace(
	/\/+$/,
	''
);

export function dashboardHref(path = '/'): string {
	const normalizedPath = path.startsWith('/') ? path : `/${path}`;
	return new URL(normalizedPath, `${dashboardBaseUrl}/`).toString();
}
