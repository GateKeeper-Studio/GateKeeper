import { z } from 'zod';

// Client-side mirror of the server's login-appearance validation, for instant
// feedback on the free-text fields. Layout / hero / font / radius come from
// constrained selects, so only URLs, hex colors and text lengths need checking
// here. The server re-validates everything and remains the source of truth.

const httpUrl = z
	.string()
	.trim()
	.refine((v) => v === '' || /^https?:\/\/.+/i.test(v), 'Must be an http(s) URL');

const hex = z
	.string()
	.trim()
	.refine(
		(v) => v === '' || /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(v),
		'Must be a hex color like #1a2b3c'
	);

export const appearanceSchema = z.object({
	logoUri: httpUrl,
	backgroundImageUri: httpUrl,
	primaryColor: hex,
	accentColor: hex
});

// heading/subtext are per-language maps now, so they're validated separately
// from the flat schema above: every present translation must fit the same
// length limit the server enforces. Returns the first violation found (or
// null), matching this form's one-error-per-field display convention.
export function validateLocalizedText(
	map: Record<string, string>,
	maxLen: number
): string | null {
	for (const value of Object.values(map)) {
		if (value.trim().length > maxLen) {
			return `Must be at most ${maxLen} characters`;
		}
	}
	return null;
}
