/**
 * Dashboard boot progress — the real signal behind the splash screen.
 *
 * A full page load has a handful of milestones that must finish before the app is
 * usable and, crucially, before the organization theme has been applied (otherwise
 * the default theme flashes first). Each milestone carries a weight; the splash reads
 * `target` (0..1) to drive its progress bar off actual load events, not a timer.
 *
 * The store is a module singleton, so once the splash has revealed the app it stays
 * revealed for the rest of the SPA session — client-side navigations don't re-trigger
 * it. A full reload re-imports the module and starts fresh (i.e. "on restart").
 */

export type BootStep = 'app' | 'session' | 'tenants' | 'content';

// Weights sum to 1. `tenants` dominates because it's the network gate that resolves
// the active organization and its saved appearance.
const WEIGHTS: Record<BootStep, number> = {
	app: 0.1, // dashboard shell mounted + hydrated
	session: 0.15, // admin session resolved from the stored token
	tenants: 0.5, // tenant list fetched → appearance known & applied
	content: 0.25 // the landing route's queries settled
};

class BootProgressStore {
	done = $state<Record<BootStep, boolean>>({
		app: false,
		session: false,
		tenants: false,
		content: false
	});

	/** True once the splash has finished and handed the app to the user. */
	revealed = $state(false);

	/** Weighted 0..1 progress from completed milestones. */
	readonly target = $derived(
		(Object.keys(WEIGHTS) as BootStep[]).reduce(
			(sum, step) => sum + (this.done[step] ? WEIGHTS[step] : 0),
			0
		)
	);

	/** Every milestone finished. */
	readonly complete = $derived(this.target >= 0.999);

	mark(step: BootStep) {
		this.done[step] = true;
	}

	/** Force-finish every milestone (hard-cap fallback so the splash can't hang). */
	finishAll() {
		this.done = { app: true, session: true, tenants: true, content: true };
	}

	reveal() {
		this.revealed = true;
	}
}

export const bootProgress = new BootProgressStore();
