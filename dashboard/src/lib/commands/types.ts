// Registry types for the command palette (Cmd+K).
//
// The palette component is a thin, generic renderer: everything it can navigate to
// or search is declared here and in `navigation.ts` / `search-providers.ts`. Adding a
// new destination or a new searchable entity is a one-object change in those files —
// the component never needs to be touched.

// Lucide icon component. Typed loosely to match the repo convention (see
// `app-sidebar/nav-main.svelte`): `@lucide/svelte`'s component types aren't
// structurally compatible with svelte's `Component` yet.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type IconComponent = any;

/** Context passed to a navigation command when building its href. */
export type CommandContext = {
	/** The active tenant id from the URL, or undefined on org-level pages. */
	tenantId?: string;
};

/** A static navigation target shown in the palette's "Navigation" group. */
export type NavCommand = {
	/** Stable id, unique within the registry. */
	id: string;
	/** Primary label shown to the user. */
	label: string;
	icon: IconComponent;
	/** Extra terms to match against (e.g. synonyms like "apps" for OAuth Clients). */
	keywords?: string[];
	/**
	 * Builds the destination href for the given context. Returns `null` when a
	 * required tenant is missing, in which case the item is hidden (avoids emitting
	 * `/tenants/undefined/...`).
	 */
	href: (ctx: CommandContext) => string | null;
};

/** One matched entity, ready to render and navigate to. */
export type SearchResult = {
	/** Stable id, used as the list key and to disambiguate identical labels. */
	id: string;
	/** Primary label (e.g. a user's name). */
	label: string;
	/** Secondary line (e.g. an email or description). */
	sublabel?: string;
	/** Destination when selected. */
	href: string;
};

/**
 * A searchable entity source. Add one of these to make a new entity type searchable
 * in the palette. `load` fetches a page of the entity and maps it to `SearchResult`s;
 * the palette filters/ranks the results client-side via the command's built-in filter.
 */
export type SearchProvider = {
	/** Stable id, e.g. 'users'. */
	key: string;
	/** Group heading shown in the palette. */
	heading: string;
	icon: IconComponent;
	/** When true, `load` is only called if a tenant is selected. */
	requiresTenant: boolean;
	load: (tenantId: string | undefined) => Promise<SearchResult[]>;
};
