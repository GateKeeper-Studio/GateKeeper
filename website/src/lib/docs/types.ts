import type { Pathname } from '$app/types';

/** A heading collected at build time by `docs-tooling/rehype-headings.js`. */
export type DocsHeading = {
	depth: number;
	id: string;
	text: string;
};

/** One searchable record emitted by the `virtual:docs-search-index` Vite plugin. */
export type DocsSearchEntry = {
	id: string;
	url: string;
	/** Page title, from frontmatter. */
	title: string;
	/** Nav section the page belongs to, e.g. "Security". */
	section: string;
	/** Set when the entry points at a heading rather than the page as a whole. */
	heading: string | null;
	text: string;
};

// `Pathname` is SvelteKit's generated union of every real route in the app, so a typo in the
// nav is a type error rather than a 404 discovered at prerender time.
export type DocsNavItem = { title: string; href: Pathname };
export type DocsNavSection = { title: string; items: DocsNavItem[] };
