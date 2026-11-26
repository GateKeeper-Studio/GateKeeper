import adapter from '@sveltejs/adapter-static';

// SvelteKit names the page's bootstrap global after `version.name`, which defaults to
// `Date.now()` evaluated each time this file is loaded. If it is loaded twice for one
// build, the client chunks and the fallback index.html can disagree and the page
// fails to start (`__sveltekit_<version>` undefined). Fix the value once per process
// (child processes inherit it); BUILD_VERSION pins it.
process.env.BUILD_VERSION ??= Date.now().toString();

/** @type {import('@sveltejs/kit').Config} */
const config = {
	compilerOptions: {
		// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
		runes: ({ filename }) => (filename.split(/[/\\]/).includes('node_modules') ? undefined : true)
	},
	kit: {
		// adapter-static: this app is a single-page app served by the identity
		// provider itself (UI_DIR; see server/docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md).
		// There is no Node server: `npm run build` writes plain files to `build/`,
		// with `index.html` as the entry page for every route (`ssr = false` in the
		// root layout). The server serves that page only on its SPA paths and keeps
		// everything else for the API.
		//
		// Server-only SvelteKit files (`+page.server.ts`, `+server.ts`,
		// `hooks.server.ts`) cannot work here; `npm run check` refuses them
		// (scripts/assert-static.mjs).
		adapter: adapter({
			// `npm run build:watch` builds into a temporary directory and swaps it in.
			pages: process.env.UI_BUILD_OUT ?? 'build',
			assets: process.env.UI_BUILD_OUT ?? 'build',
			fallback: 'index.html',
			precompress: true,
			strict: true
		}),
		version: { name: process.env.BUILD_VERSION },
		alias: {
			'@/*': './path/to/lib/*'
		}
	}
};

export default config;
