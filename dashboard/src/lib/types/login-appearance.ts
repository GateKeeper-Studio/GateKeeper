// Shared type + helpers for a client's login-page appearance. This mirrors the
// server's LoginAppearanceView (camelCase). It is the single source of truth used
// by BOTH the admin config UI and the hosted login pages.
//
// SECURITY: every customizable value is constrained by construction — layouts and
// fonts are closed unions, colors are validated hex, image fields are http(s) URLs
// rendered via <img>, and text is rendered as escaped text nodes. Fonts map to a
// bundled, safe stack here; a raw font-family string is never used.

export const LOGIN_LAYOUTS = ['centered', 'split_image', 'card', 'minimal'] as const;
export type LoginLayout = (typeof LOGIN_LAYOUTS)[number];

export const HERO_SIDES = ['left', 'right'] as const;
export type HeroSide = (typeof HERO_SIDES)[number];

export const LOGIN_FONTS = ['inter', 'system', 'roboto', 'georgia'] as const;
export type LoginFont = (typeof LOGIN_FONTS)[number];

// Fields that never differ by language.
type LoginAppearanceBase = {
	layout: LoginLayout;
	heroPosition: HeroSide;
	logoUri: string | null;
	backgroundImageUri: string | null;
	primaryColor: string | null;
	accentColor: string | null;
	borderRadius: string | null;
	fontFamily: LoginFont | null;
};

// The admin edit/wire shape: heading/subtext are per-language maps (locale ->
// text), since the admin fills in translations for every language the client
// allows. This is what the admin GET/PUT endpoints and the appearance-tab
// editor deal in.
export type LoginAppearance = LoginAppearanceBase & {
	heading: Record<string, string> | null;
	subtext: Record<string, string> | null;
};

// The resolved/public shape: heading/subtext are already resolved to a
// single string for one locale (server-side, via the fallback chain: the
// requested locale -> the client's default_language -> first available).
// This is what the hosted login/sign-up pages render.
export type ResolvedLoginAppearance = LoginAppearanceBase & {
	heading: string | null;
	subtext: string | null;
};

export const LAYOUT_LABELS: Record<LoginLayout, string> = {
	centered: 'Centered',
	split_image: 'Split image',
	card: 'Card',
	minimal: 'Minimal'
};

export const LAYOUT_DESCRIPTIONS: Record<LoginLayout, string> = {
	centered: 'Form centered on a plain background.',
	split_image: 'Brand image on one side, form on the other.',
	card: 'Form in a floating card over a background.',
	minimal: 'Compact — logo and form only.'
};

export const FONT_LABELS: Record<LoginFont, string> = {
	inter: 'Inter',
	system: 'System UI',
	roboto: 'Roboto',
	georgia: 'Georgia (serif)'
};

// Bundled, safe font stacks. The admin only ever picks a key from LOGIN_FONTS.
const FONT_STACKS: Record<LoginFont, string> = {
	inter: "'Inter Variable', Inter, ui-sans-serif, system-ui, sans-serif",
	system: "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
	roboto: "Roboto, ui-sans-serif, system-ui, sans-serif",
	georgia: "Georgia, 'Times New Roman', serif"
};

export function fontStackFor(font: LoginFont | null | undefined): string | undefined {
	return font ? FONT_STACKS[font] : undefined;
}

// Border-radius presets offered in the admin selector (value must satisfy the
// server's "<number>rem|px" validation).
export const RADIUS_PRESETS = [
	{ value: '0rem', label: 'Square' },
	{ value: '0.375rem', label: 'Small' },
	{ value: '0.625rem', label: 'Default' },
	{ value: '1rem', label: 'Large' },
	{ value: '1.5rem', label: 'Extra large' }
] as const;

const DEFAULT_APPEARANCE_BASE: LoginAppearanceBase = {
	layout: 'split_image',
	heroPosition: 'left',
	logoUri: null,
	backgroundImageUri: null,
	primaryColor: null,
	accentColor: null,
	borderRadius: null,
	fontFamily: null
};

// Used by the admin editor when a client has no appearance yet, and as the base
// the login pages merge over so a partial object always has every field.
export const DEFAULT_LOGIN_APPEARANCE: LoginAppearance = {
	...DEFAULT_APPEARANCE_BASE,
	heading: null,
	subtext: null
};

// Same as above, but for the resolved/public shape (hosted pages).
export const DEFAULT_RESOLVED_LOGIN_APPEARANCE: ResolvedLoginAppearance = {
	...DEFAULT_APPEARANCE_BASE,
	heading: null,
	subtext: null
};

// A #RGB or #RRGGBB string. Mirrors the server-side check so the admin gets
// instant feedback; the server remains the source of truth.
export function isHexColor(value: string): boolean {
	return /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(value);
}
