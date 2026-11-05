import { browser } from '$app/environment';
import { setMode } from 'mode-watcher';

import type { TenantAppearance } from '$lib/services/admin/tenants';

/**
 * Runtime application theming.
 *
 * The base theme lives statically in `src/routes/layout.css` (OKLCH tokens wired to
 * Tailwind via `@theme inline`). This store lets the active organization override a
 * small, safe subset of those tokens at runtime by writing inline custom properties
 * on `<html>` — Tailwind v4 resolves `--color-primary: var(--primary)` to any CSS
 * color, so the raw hex can be assigned directly with no OKLCH conversion.
 *
 * The organization's appearance is the SINGLE source of truth: it lives in the
 * database (`tenants.appearance`) and is applied once the tenant list resolves. It is
 * intentionally NOT cached in the browser — the SSR splash screen covers the whole
 * load and only reveals the app after the DB theme has been applied, so there is no
 * flash to hide. Persisting a copy locally would risk a stale browser value winning
 * over the database; keeping the DB authoritative avoids that entirely.
 *
 * Import the singleton; never construct your own.
 */

// Radius presets → the single `--radius` token (a `calc()` scale in layout.css
// derives every `rounded-*` step from it).
const RADIUS_REM: Record<TenantAppearance['radius'], string> = {
	none: '0rem',
	sm: '0.35rem',
	md: '0.625rem', // matches the layout.css default
	lg: '0.9rem'
};

// mode-watcher's default localStorage key. Presence means the user has explicitly
// picked a theme, so the org default must not override it.
const MODE_STORAGE_KEY = 'mode-watcher-mode';

// Every custom property this store may set, so reset() can remove them wholesale.
const MANAGED_VARS = [
	'--primary',
	'--primary-foreground',
	'--sidebar-primary',
	'--sidebar-primary-foreground',
	'--radius'
];

// Pick a readable foreground for a solid background color: dark ink on light colors,
// light ink on dark colors, via WCAG relative luminance. Values mirror layout.css.
function foregroundFor(hex: string): string {
	const c = hex.replace('#', '');
	const full =
		c.length === 3
			? c
					.split('')
					.map((ch) => ch + ch)
					.join('')
			: c;
	const channel = (i: number) => parseInt(full.slice(i, i + 2), 16) / 255;
	const lin = (v: number) => (v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4));
	const luminance = 0.2126 * lin(channel(0)) + 0.7152 * lin(channel(2)) + 0.0722 * lin(channel(4));
	return luminance > 0.5 ? 'oklch(0.205 0 0)' : 'oklch(0.985 0 0)';
}

class AppThemeStore {
	// The org default mode is a first-run fallback only — applied at most once.
	#defaultModeApplied = false;

	/** Apply an org appearance (or clear overrides when null/undefined). */
	apply(appearance: TenantAppearance | null | undefined) {
		if (!browser) return;
		if (!appearance) {
			this.reset();
			return;
		}
		const root = document.documentElement;
		const fg = foregroundFor(appearance.primaryColor);
		root.style.setProperty('--primary', appearance.primaryColor);
		root.style.setProperty('--primary-foreground', fg);
		root.style.setProperty('--sidebar-primary', appearance.primaryColor);
		root.style.setProperty('--sidebar-primary-foreground', fg);
		root.style.setProperty('--radius', RADIUS_REM[appearance.radius] ?? RADIUS_REM.md);
	}

	/** Live-preview a draft appearance while the settings form is open. */
	preview(appearance: TenantAppearance) {
		this.apply(appearance);
	}

	/** Drop every override so the static layout.css theme takes over again. */
	reset() {
		if (!browser) return;
		const root = document.documentElement;
		for (const v of MANAGED_VARS) root.style.removeProperty(v);
	}

	/**
	 * Apply the org's default light/dark mode as a first-run fallback — only when the
	 * user has never chosen a theme themselves, and only once per session.
	 */
	applyDefaultMode(mode: TenantAppearance['defaultMode']) {
		if (!browser || this.#defaultModeApplied) return;
		this.#defaultModeApplied = true;
		if (!localStorage.getItem(MODE_STORAGE_KEY)) setMode(mode);
	}
}

export const appTheme = new AppThemeStore();
