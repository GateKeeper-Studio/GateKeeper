// After `vite build`: writes `<build>/_app/route-preloads.json`, the list of JS
// modules and stylesheets each route needs.
//
// The build is a single-page app, so the identity provider serves the same
// `index.html` for every route. On its own, that page only names the router's entry
// modules: the browser discovers the route's modules one import wave at a time, a
// round trip per wave (about 50 modules for a sign-in page). With this file the
// server adds `<link rel="modulepreload">` tags for the requested route to the page,
// so the browser fetches them all in parallel, the way SvelteKit's own server
// rendering did (server/src/presentation/ui.rs).
//
// It also records the i18n settings, so the server can predict the URL of the data a
// hosted login page fetches first and hint it as well.
//
// Inputs are SvelteKit's generated route dictionary and the Vite client manifest.
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const out = process.env.UI_BUILD_OUT ?? 'build';
const generated = '.svelte-kit/generated/client-optimized';
const manifest = JSON.parse(readFileSync('.svelte-kit/output/client/.vite/manifest.json', 'utf8'));

// `"/auth/[clientId]/sign-in": [13,[2]]` -> page node 13, layouts [2]. The root
// layout (node 0) is implied for every route.
const app = readFileSync(join(generated, 'app.js'), 'utf8');
const block = app.match(/export const dictionary = \{([\s\S]*?)\n\s*\};/);
if (!block) throw new Error('route-preloads: SvelteKit route dictionary not found');
const routes = [...block[1].matchAll(/"([^"]+)":\s*\[(-?\d+)(?:,\s*\[([^\]]*)\])?/g)].map(
	([, pattern, leaf, layouts]) => ({
		pattern,
		nodes: [
			0,
			...(layouts ? layouts.split(',').filter(Boolean).map(Number) : []),
			Number(leaf)
		].map(Math.abs)
	})
);

// Everything a manifest entry pulls in statically, depth first.
function collect(key, js, css, seen) {
	if (seen.has(key)) return;
	seen.add(key);
	const entry = manifest[key];
	if (!entry) return;
	js.add(`/${entry.file}`);
	for (const sheet of entry.css ?? []) css.add(`/${sheet}`);
	for (const dep of entry.imports ?? []) collect(dep, js, css, seen);
}

const result = routes.map(({ pattern, nodes }) => {
	const js = new Set();
	const css = new Set();
	const seen = new Set();
	for (const node of nodes) collect(`${generated}/nodes/${node}.js`, js, css, seen);
	return { pattern, js: [...js], css: [...css] };
});

// The i18n runtime settings that decide which locale a page renders in.
const runtime = readFileSync('src/lib/paraglide/runtime.js', 'utf8');
const pick = (name) => {
	const found = runtime.match(new RegExp(`export const ${name} = ([^;]+);`));
	if (!found) throw new Error(`route-preloads: ${name} not found in the paraglide runtime`);
	return JSON.parse(
		found[1]
			.replace(/\/\*[\s\S]*?\*\//g, '')
			.trim()
			.replace(/^\(|\)$/g, '')
	);
};

const file = join(out, '_app', 'route-preloads.json');
writeFileSync(
	file,
	JSON.stringify({
		routes: result,
		i18n: {
			locales: pick('locales'),
			baseLocale: pick('baseLocale'),
			cookieName: pick('cookieName')
		}
	})
);
console.log(`route-preloads: ${result.length} routes -> ${file}`);
