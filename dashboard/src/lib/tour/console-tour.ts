// The console's guided tour (driver.js).
//
// It runs once, the first time an admin opens the console, and again whenever the
// admin presses the help button in the header. The tour walks the console shell
// (tenants, navigation, search, preferences, account) and, on a page that has its own
// steps, explains that page too.
//
// "Seen" is kept on the server (`consoleTourVersion` of the admin's session identity,
// written by `PUT /admin/me/console-tour`), so the tour does not come back in another
// browser, and in localStorage, which covers the moment before the server answers and
// a failed write.
//
// Steps point at elements marked `data-tour="<id>"`. A step whose element is not on
// screen (a page has not loaded, a section is collapsed) is left out rather than shown
// floating over nothing. On mobile the sidebar is an off-canvas sheet, so its steps
// give way to one on the button that opens it.
//
// driver.js and its stylesheet load the first time the tour starts, so they stay out
// of the bundle of every console page.
//
// SECURITY: driver.js renders a step's title and description as HTML. Only the fixed,
// translated strings below may go into them, never data from the API (tenant names,
// emails…).
import type { Config, Driver, DriveStep, Side } from 'driver.js';

import { browser } from '$app/environment';
import { m } from '$lib/paraglide/messages.js';
import { recordConsoleTour } from '$lib/services/admin/account';

type TourStep = {
	// The `data-tour` id of the element; none for a centered step.
	target?: string;
	side?: Side;
	title: () => string;
	description: () => string;
};

// The admin the tour is for, as the console's session identity carries it.
export type TourAdmin = { id: string; consoleTourVersion?: number };

const WELCOME: TourStep = {
	title: m.tour_welcome_title,
	description: m.tour_welcome_description
};

// The sidebar, top to bottom.
const SIDEBAR: TourStep[] = [
	{
		target: 'tenant-switcher',
		side: 'right',
		title: m.tour_tenant_switcher_title,
		description: m.tour_tenant_switcher_description
	},
	{
		target: 'nav-home',
		side: 'right',
		title: m.tour_nav_home_title,
		description: m.tour_nav_home_description
	},
	{
		target: 'nav-general',
		side: 'right',
		title: m.tour_nav_general_title,
		description: m.tour_nav_general_description
	},
	{
		target: 'nav-security',
		side: 'right',
		title: m.tour_nav_security_title,
		description: m.tour_nav_security_description
	},
	{
		target: 'nav-compliance',
		side: 'right',
		title: m.tour_nav_compliance_title,
		description: m.tour_nav_compliance_description
	}
];

// In place of the sidebar steps when the sidebar is closed (mobile).
const SIDEBAR_CLOSED: TourStep = {
	target: 'sidebar-trigger',
	side: 'bottom',
	title: m.tour_sidebar_menu_title,
	description: m.tour_sidebar_menu_description
};

// The header and the account menu, ending on the button that replays the tour.
const SHELL_END: TourStep[] = [
	{
		target: 'search',
		side: 'bottom',
		title: m.tour_search_title,
		description: m.tour_search_description
	},
	{
		target: 'preferences',
		side: 'bottom',
		title: m.tour_preferences_title,
		description: m.tour_preferences_description
	},
	{
		target: 'user-menu',
		side: 'right',
		title: m.tour_user_menu_title,
		description: m.tour_user_menu_description
	},
	{
		target: 'tour-button',
		side: 'bottom',
		title: m.tour_replay_title,
		description: m.tour_replay_description
	}
];

// Page-specific steps, by SvelteKit route id.
const PAGE_STEPS: Record<string, TourStep[]> = {
	'/console/tenants/[tenantId]': [
		{
			target: 'tenant-overview',
			side: 'bottom',
			title: m.tour_tenant_overview_title,
			description: m.tour_tenant_overview_description
		},
		{
			target: 'tenant-stats',
			side: 'bottom',
			title: m.tour_tenant_stats_title,
			description: m.tour_tenant_stats_description
		},
		{
			target: 'tenant-actions',
			side: 'bottom',
			title: m.tour_tenant_actions_title,
			description: m.tour_tenant_actions_description
		}
	],
	'/console/tenants/[tenantId]/users': [
		{
			target: 'users-create',
			side: 'left',
			title: m.tour_users_create_title,
			description: m.tour_users_create_description
		},
		{
			target: 'users-stats',
			side: 'bottom',
			title: m.tour_users_stats_title,
			description: m.tour_users_stats_description
		},
		{
			target: 'users-table',
			side: 'top',
			title: m.tour_users_table_title,
			description: m.tour_users_table_description
		}
	],
	'/console/tenants/[tenantId]/users/[userId]': [
		{
			target: 'user-actions',
			side: 'bottom',
			title: m.tour_user_actions_title,
			description: m.tour_user_actions_description
		},
		{
			target: 'user-tabs',
			side: 'bottom',
			title: m.tour_user_tabs_title,
			description: m.tour_user_tabs_description
		}
	],
	'/console/tenants/[tenantId]/oauth-clients': [
		{
			target: 'oauth-clients-create',
			side: 'left',
			title: m.tour_oauth_clients_create_title,
			description: m.tour_oauth_clients_create_description
		},
		{
			target: 'oauth-clients-list',
			side: 'top',
			title: m.tour_oauth_clients_list_title,
			description: m.tour_oauth_clients_list_description
		}
	],
	'/console/tenants/[tenantId]/security': [
		{
			target: 'security-kpis',
			side: 'bottom',
			title: m.tour_security_kpis_title,
			description: m.tour_security_kpis_description
		},
		{
			target: 'security-insights',
			side: 'top',
			title: m.tour_security_insights_title,
			description: m.tour_security_insights_description
		}
	]
};

// Bump to show the tour again to everyone after it changes substantially. The server
// accepts 1–1000.
const TOUR_VERSION = 1;
const storageKey = (adminId: string) => `gatekeeper:console-tour:v${TOUR_VERSION}:${adminId}`;

let active: Driver | null = null;
// A start waiting for driver.js to load; `stopConsoleTour` cancels it.
let pending: { cancelled: boolean } | null = null;
// Admins who saw the tour since this page loaded: the session identity the console
// holds still has the old version, and localStorage may be blocked.
const seenHere = new Set<string>();
// Admins whose "seen" this page has already sent, so a replay does not send it again.
const recorded = new Set<string>();

export function hasSeenConsoleTour(admin: TourAdmin): boolean {
	if (!browser) return true;
	if ((admin.consoleTourVersion ?? 0) >= TOUR_VERSION || seenHere.has(admin.id)) return true;
	try {
		return localStorage.getItem(storageKey(admin.id)) !== null;
	} catch {
		// Storage blocked (private mode, policy) and the server says unseen: show it.
		// Closing it records it on the server, so it does not come back.
		return false;
	}
}

function markSeen(admin: TourAdmin) {
	seenHere.add(admin.id);
	try {
		localStorage.setItem(storageKey(admin.id), new Date().toISOString());
	} catch {
		// Best effort: the server keeps it too.
	}
	if ((admin.consoleTourVersion ?? 0) >= TOUR_VERSION || recorded.has(admin.id)) return;
	recorded.add(admin.id);
	// Best effort as well: if it fails, localStorage still stops the tour here.
	void recordConsoleTour(TOUR_VERSION).then(([error]) => {
		if (error) recorded.delete(admin.id);
	});
}

// Present, laid out and horizontally on screen: a hidden section has no boxes, and an
// off-canvas panel has its boxes outside the viewport. (Below the fold is fine:
// driver.js scrolls to it.)
function onScreen(target: string): boolean {
	const found = document.querySelector(`[data-tour="${target}"]`);
	if (!found) return false;
	return Array.from(found.getClientRects()).some(
		(rect) => rect.width > 0 && rect.height > 0 && rect.right > 0 && rect.left < window.innerWidth
	);
}

function present(steps: TourStep[]): TourStep[] {
	return steps.filter((step) => !step.target || onScreen(step.target));
}

function toDriveStep(step: TourStep): DriveStep {
	return {
		element: step.target ? `[data-tour="${step.target}"]` : undefined,
		popover: {
			title: step.title(),
			description: step.description(),
			side: step.side,
			align: 'start'
		}
	};
}

function stepsFor(routeId: string | null | undefined): DriveStep[] {
	const pageSteps = (routeId && PAGE_STEPS[routeId]) || [];
	const sidebar = present(SIDEBAR);
	const navigation = sidebar.length > 0 ? sidebar : present([SIDEBAR_CLOSED]);
	return [WELCOME, ...navigation, ...present(pageSteps), ...present(SHELL_END)].map(toDriveStep);
}

/**
 * Starts the tour for the current page (`routeId`). Finishing or closing it counts as
 * seen for `admin`, so the first-visit tour does not come back; the help button
 * replays it any time.
 */
export async function startConsoleTour(options: {
	admin?: TourAdmin;
	routeId?: string | null;
}): Promise<void> {
	if (!browser || active || pending) return;

	const start = { cancelled: false };
	pending = start;
	let driver: (config?: Config) => Driver;
	try {
		[{ driver }] = await Promise.all([import('driver.js'), import('driver.js/dist/driver.css')]);
	} catch {
		// A failed chunk load (offline, a deploy replaced the assets): no tour this time.
		return;
	} finally {
		pending = null;
	}
	// Navigated away while it loaded: the steps would describe the previous page.
	if (start.cancelled || active) return;

	const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
	const { admin } = options;

	active = driver({
		steps: stepsFor(options.routeId),
		showProgress: true,
		progressText: m.tour_progress({ current: '{{current}}', total: '{{total}}' }),
		nextBtnText: m.tour_next(),
		prevBtnText: m.tour_previous(),
		doneBtnText: m.tour_done(),
		popoverClass: 'gk-tour',
		animate: !reducedMotion,
		smoothScroll: !reducedMotion,
		overlayOpacity: 0.55,
		stagePadding: 6,
		stageRadius: 10,
		// The tour explains; it does not click through the console on the admin's behalf.
		disableActiveInteraction: true,
		onDestroyed: () => {
			active = null;
			if (admin) markSeen(admin);
		}
	});
	active.drive();
}

/**
 * Ends the tour, or a start still loading. The console calls it when the admin leaves
 * the page, whose elements the steps point at. Counts as seen, like closing it.
 */
export function stopConsoleTour() {
	if (pending) pending.cancelled = true;
	active?.destroy();
}
