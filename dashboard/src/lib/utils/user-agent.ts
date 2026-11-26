/**
 * "Chrome on Windows" from a User-Agent string, for showing a person which device is
 * asking. Deliberately coarse: enough to recognise one's own computer, nothing more.
 * Returns null when nothing is recognised.
 */
export function describeUserAgent(userAgent: string | null | undefined): string | null {
	if (!userAgent) return null;
	const ua = userAgent;

	const browsers: [RegExp, string][] = [
		[/Edg(e|A|iOS)?\//, 'Edge'],
		[/OPR\/|Opera/, 'Opera'],
		[/SamsungBrowser\//, 'Samsung Internet'],
		[/Firefox\/|FxiOS\//, 'Firefox'],
		[/CriOS\/|Chrome\//, 'Chrome'],
		[/Version\/[\d.]+.*Safari\//, 'Safari']
	];
	const systems: [RegExp, string][] = [
		[/iPhone|iPad|iPod/, 'iOS'],
		[/Android/, 'Android'],
		[/Windows/, 'Windows'],
		[/CrOS/, 'ChromeOS'],
		[/Mac OS X|Macintosh/, 'macOS'],
		[/Linux/, 'Linux']
	];

	const browser = browsers.find(([pattern]) => pattern.test(ua))?.[1];
	const system = systems.find(([pattern]) => pattern.test(ua))?.[1];
	if (browser && system) return `${browser} · ${system}`;
	return browser ?? system ?? null;
}
