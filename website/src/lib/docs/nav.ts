import type { DocsNavSection } from './types';
import { frameworkLibraries } from './framework-libraries';

const frameworkLibraryItems = frameworkLibraries.map((library) => ({
	title: library.framework,
	href: library.docsHref
}));

/**
 * The docs table of contents. Single source of truth for the sidebar, the mobile Sheet, and
 * prev/next — a new page is one entry here plus one `+page.svx`.
 */
export const docsNav: DocsNavSection[] = [
	{
		title: 'Getting started',
		items: [
			{ title: 'Introduction', href: '/docs' },
			{ title: 'Before you begin', href: '/docs/before-you-begin' },
			{ title: 'Your first tenant', href: '/docs/first-tenant' },
			{ title: 'Integrate your first app', href: '/docs/integrate-your-app' }
		]
	},
	{
		title: 'Configuring a tenant',
		items: [
			{ title: 'OAuth clients', href: '/docs/oauth-clients' },
			{ title: 'Social login', href: '/docs/social-login' },
			{ title: 'Login page branding', href: '/docs/branding' },
			{ title: 'Users, groups & roles', href: '/docs/users-groups-roles' },
			{ title: 'Email', href: '/docs/email' },
			{ title: 'Multi-factor authentication', href: '/docs/mfa' }
		]
	},
	{
		title: 'Application libraries',
		items: [{ title: 'Overview', href: '/docs/framework-libraries' }, ...frameworkLibraryItems]
	},
	{
		title: 'Security',
		items: [
			{ title: 'How risk scoring works', href: '/docs/risk-engine' },
			{ title: 'Risk policy & monitoring', href: '/docs/risk-policy' }
		]
	},
	{
		title: 'Compliance',
		items: [
			{ title: 'Audit log', href: '/docs/audit-log' },
			{ title: 'Data subject requests', href: '/docs/data-subject-requests' },
			{ title: 'Legal pages', href: '/docs/legal-pages' }
		]
	},
	{
		title: 'Reference',
		items: [{ title: 'Troubleshooting', href: '/docs/troubleshooting' }]
	}
];

/** Flattened reading order, used for the prev/next footer links. */
export const flatDocsNav = docsNav.flatMap((section) => section.items);
