// Resolves a per-language text map down to a single value: exact locale
// match, else the tenant/client's own default locale, else whatever
// translation happens to exist. Mirrors the server's
// `application::shared::localization::resolve_localized`. Used only in
// admin-side preview/editor code — hosted pages always receive
// already-resolved strings from the server.
export function resolveLocalized(
	map: Record<string, string> | null | undefined,
	locale: string,
	defaultLocale: string
): string {
	if (!map) return '';
	return map[locale] || map[defaultLocale] || Object.values(map)[0] || '';
}
