import adapter from '@sveltejs/adapter-auto';
import { mdsvex } from 'mdsvex';
import mdsvexConfig from './mdsvex.config.js';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// `.svx` files are the docs pages under src/routes/docs. Registering the extension
	// here is what lets SvelteKit treat `+page.svx` as a routable page.
	extensions: ['.svelte', '.svx'],

	preprocess: [mdsvex(mdsvexConfig)],

	compilerOptions: {
		// Force runes mode for the project, except for libraries. Can be removed in svelte 6.
		//
		// `.svx` is the exception: when mdsvex wraps a page in its layout it emits
		// `<Layout {...$$props}>`, and `$$props` is illegal in runes mode. Compiling
		// markdown in legacy mode is harmless — the generated component is static markup,
		// and the runes components it renders (Callout, Mermaid, …) work fine as children.
		runes: ({ filename }) => {
			if (filename.split(/[/\\]/).includes('node_modules')) return undefined;
			if (filename.endsWith('.svx')) return false;
			return true;
		}
	},
	kit: {
		// adapter-auto only supports some environments, see https://svelte.dev/docs/kit/adapter-auto for a list.
		// If your environment is not supported, or you settled on a specific environment, switch out the adapter.
		// See https://svelte.dev/docs/kit/adapters for more information about adapters.
		adapter: adapter(),
		alias: {
			'@/*': './path/to/lib/*'
		}
	}
};

export default config;
