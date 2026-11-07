import type { Pathname } from '$app/types';

export type FrameworkLibraryStatus = 'available' | 'planned';

/**
 * `confidential` = server-side BFF that holds a client secret and keeps tokens in an
 * encrypted httpOnly cookie. `public` = browser-only SPA registered as a public client
 * (no secret, PKCE S256 only, tokens live in the browser).
 */
export type FrameworkLibraryClientType = 'confidential' | 'public';

export type FrameworkLibrary = {
	slug: string;
	name: string;
	framework: string;
	packageName: string;
	clientType: FrameworkLibraryClientType;
	status: FrameworkLibraryStatus;
	summary: string;
	docsHref: Pathname;
	sourcePath?: string;
};

/**
 * Framework integration packages documented by the public website.
 *
 * Add future adapters here first, then create the matching `/docs/framework-libraries/<slug>`
 * page and wire any examples in the sample app. The docs sidebar consumes this catalog directly.
 */
export const frameworkLibraries = [
	{
		slug: 'sveltekit',
		name: 'SvelteKit BFF',
		framework: 'SvelteKit',
		packageName: '@gatekeeper/sveltekit-bff',
		clientType: 'confidential',
		status: 'available',
		summary:
			'Server-side OAuth/OIDC helpers, encrypted httpOnly sessions, automatic token refresh, browser-safe context, profile updates, and MFA UI/headless APIs.',
		docsHref: '/docs/framework-libraries/sveltekit',
		sourcePath: 'packages/gatekeeper-sveltekit-bff'
	},
	{
		slug: 'nextjs',
		name: 'Next.js BFF',
		framework: 'Next.js',
		packageName: '@gatekeeper/nextjs',
		clientType: 'confidential',
		status: 'available',
		summary:
			'App Router route handlers for sign-in/callback/sign-out, middleware silent refresh, and RSC-safe getSession()/getUser() on the shared confidential BFF core.',
		docsHref: '/docs/framework-libraries/nextjs',
		sourcePath: 'packages/gatekeeper-nextjs'
	},
	{
		slug: 'remix',
		name: 'Remix BFF',
		framework: 'Remix',
		packageName: '@gatekeeper/remix',
		clientType: 'confidential',
		status: 'available',
		summary:
			'Web-standard loaders/actions for the Auth Code + PKCE flow and a root-loader silent refresh that merges rotated Set-Cookie headers, on the confidential BFF core.',
		docsHref: '/docs/framework-libraries/remix',
		sourcePath: 'packages/gatekeeper-remix'
	},
	{
		slug: 'browser',
		name: 'Browser client',
		framework: 'Vanilla JavaScript',
		packageName: '@gatekeeper/browser',
		clientType: 'public',
		status: 'available',
		summary:
			'Framework-agnostic browser client for safe BFF endpoints, automatic browser-safe session refresh, profile updates, MFA headless APIs, and custom elements.',
		docsHref: '/docs/framework-libraries/browser',
		sourcePath: 'packages/gatekeeper-browser'
	},
	{
		slug: 'react',
		name: 'React SPA',
		framework: 'React',
		packageName: '@gatekeeper/react',
		clientType: 'public',
		status: 'available',
		summary:
			'React bindings over the public SPA core: <GateKeeperProvider>, useGateKeeper()/useUser() hooks, and <SignInButton>/<SignOutButton>/<Authenticated> components.',
		docsHref: '/docs/framework-libraries/react',
		sourcePath: 'packages/gatekeeper-react'
	},
	{
		slug: 'vue',
		name: 'Vue 3 SPA',
		framework: 'Vue',
		packageName: '@gatekeeper/vue',
		clientType: 'public',
		status: 'available',
		summary:
			'Vue 3 plugin createGateKeeper() plus a useGateKeeper() composable exposing reactive auth state (computed refs) over the public SPA core.',
		docsHref: '/docs/framework-libraries/vue',
		sourcePath: 'packages/gatekeeper-vue'
	},
	{
		slug: 'angular',
		name: 'Angular SPA',
		framework: 'Angular',
		packageName: '@gatekeeper/angular',
		clientType: 'public',
		status: 'available',
		summary:
			'Angular standalone integration: a signal-based GateKeeperService, provideGateKeeper() with an APP_INITIALIZER callback handler, and an authGuard CanActivateFn.',
		docsHref: '/docs/framework-libraries/angular',
		sourcePath: 'packages/gatekeeper-angular'
	},
	{
		slug: 'angularjs',
		name: 'AngularJS SPA',
		framework: 'AngularJS',
		packageName: '@gatekeeper/angularjs',
		clientType: 'public',
		status: 'available',
		summary:
			'AngularJS 1.x module with a gatekeeper provider/service (digest-synced auth state) and a requireAuthenticated route-resolve helper, over the public SPA core.',
		docsHref: '/docs/framework-libraries/angularjs',
		sourcePath: 'packages/gatekeeper-angularjs'
	}
] satisfies FrameworkLibrary[];
