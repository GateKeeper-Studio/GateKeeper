import { paraglideVitePlugin } from '@inlang/paraglide-js';
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

import { docsSearchIndex } from './docs-tooling/search-index-plugin.js';

export default defineConfig({
	server: {
		port: 5174,
		strictPort: true
	},
	plugins: [
		tailwindcss(),
		sveltekit(),
		paraglideVitePlugin({ project: './project.inlang', outdir: './src/lib/paraglide' }),
		docsSearchIndex()
	]
});
