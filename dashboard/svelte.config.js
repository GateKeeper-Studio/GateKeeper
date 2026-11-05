import adapter from '@sveltejs/adapter-node';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	compilerOptions: {
		// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
		runes: ({ filename }) => (filename.split(/[/\\]/).includes('node_modules') ? undefined : true)
	},
	kit: {
		// adapter-node: this dashboard is deployed to a VPS and run as a long-lived
		// Node process behind a reverse proxy (`node build`). It was previously
		// adapter-auto, which is a getting-started convenience — it picks a target by
		// sniffing the CI environment and simply fails when it recognises none, so it
		// is not something to deploy on.
		//
		// A persistent server is also what the server-side session in
		// `src/lib/server/session.ts` assumes.
		adapter: adapter(),
		alias: {
			'@/*': './path/to/lib/*'
		}
	}
};

export default config;
