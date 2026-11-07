// Single source of truth for the languages offered in the UI.
//
// Adding a language is three steps, none of which touch the switcher/section code:
//   1. add its code to `locales` in project.inlang/settings.json
//   2. create messages/<code>.json (recompile: `paraglide-js compile`)
//   3. add one entry below with its display + native label
//
// `nativeLabel` is what users see in the picker (a language is always listed in its
// own language). `label` is the English name, handy for tooltips/aria in one language.

import { locales as runtimeLocales, type Locale } from '$lib/paraglide/runtime';

export type LocaleOption = {
	code: Locale;
	/** English name, e.g. for aria-labels. */
	label: string;
	/** The language's own name, shown in the picker. */
	nativeLabel: string;
};

const ALL_LOCALES: LocaleOption[] = [
	{ code: 'en', label: 'English', nativeLabel: 'English' },
	{ code: 'es', label: 'Spanish', nativeLabel: 'Español' },
	{ code: 'pt-br', label: 'Portuguese', nativeLabel: 'Português (Brasil)' }
];

// Only surface locales the Paraglide runtime actually knows about, so a stray entry
// here (or a locale removed from settings.json) never renders an option with no
// messages behind it. Order follows ALL_LOCALES.
export const LOCALES: LocaleOption[] = ALL_LOCALES.filter((option) =>
	(runtimeLocales as readonly string[]).includes(option.code)
);
