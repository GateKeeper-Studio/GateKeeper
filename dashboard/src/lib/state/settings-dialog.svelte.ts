import { page } from '$app/state';
import { replaceState } from '$app/navigation';
import { SvelteURLSearchParams } from 'svelte/reactivity';

/**
 * App-wide Settings dialog.
 *
 * Account and Organization settings are consolidated into a single modal (Teams/Notion
 * style): a left rail of sections and a right content pane. This singleton holds whether
 * the dialog is open and which section is active, so any component — the sidebar user
 * menu, a deep-link redirect, the command palette — can pop it open without prop drilling.
 *
 * **URL as source of truth.** The open dialog + active section live in the URL as
 * `?settings=<section>`, so a reload (or a shared link) reopens the dialog on the same
 * section. User actions (`openTo`/`select`/`close`) mutate state *and* write the URL via
 * shallow `replaceState` — always safe because they only run from click handlers, long
 * after the router is initialized. The read direction (URL → state) is `syncFromUrl`,
 * called from the shell's `afterNavigate`; it never touches the router, so it's safe on
 * the initial hydration run (where `replaceState` would throw "before router is
 * initialized").
 *
 * `screen` is an optional nested screen within the active section.
 *
 * Import the singleton, never construct your own.
 */
class SettingsDialogStore {
	/** Whether the settings dialog is open. */
	open = $state(false);

	/** The active section id — must match an `id` in `settings-dialog/sections.ts`. */
	section = $state('account');

	/** Optional nested screen within the active section. */
	screen = $state<string | null>(null);

	/**
	 * Read direction: reflect the URL into state. Called from the shell on navigation
	 * (initial load, deep links, browser back/forward). Never writes the URL, so it's safe
	 * during hydration.
	 */
	syncFromUrl(section: string | null, screen: string | null) {
		if (section) {
			this.section = section;
			this.screen = screen;
			this.open = true;
		} else {
			this.open = false;
			this.screen = null;
		}
	}

	/** User action: open the dialog on `section` and reflect it in the URL. */
	openTo(section = 'account') {
		this.section = section;
		this.screen = null;
		this.open = true;
		this.#writeUrl();
	}

	/** User action: open on the current/previous section, initially the first rail item. */
	openCurrent() {
		this.open = true;
		this.#writeUrl();
	}

	/** User action: switch the active section. */
	select(section: string) {
		this.section = section;
		this.screen = null;
		this.#writeUrl();
	}

	/** User action: switch the nested screen inside the active section. */
	selectScreen(screen: string | null) {
		this.screen = screen;
		this.#writeUrl();
	}

	/** User action: close the dialog and strip the settings params from the URL. */
	close() {
		this.open = false;
		this.#writeUrl();
	}

	/**
	 * Mirror the current open/section state into the URL without a navigation. Preserves
	 * the current pathname and any unrelated query params.
	 */
	#writeUrl() {
		const params = new SvelteURLSearchParams(page.url.searchParams);
		if (this.open) {
			params.set('settings', this.section);
			if (this.screen) {
				params.set('screen', this.screen);
			} else {
				params.delete('view');
				params.delete('screen');
			}
		} else {
			params.delete('settings');
			params.delete('view');
			params.delete('screen');
		}
		const qs = params.toString();
		// Dynamic current path from the router; `resolve()` only accepts typed route literals.
		// eslint-disable-next-line svelte/no-navigation-without-resolve
		replaceState(qs ? `${page.url.pathname}?${qs}` : page.url.pathname, {});
	}
}

export const settingsDialog = new SettingsDialogStore();
