import { paraglideVitePlugin } from '@inlang/paraglide-js';
import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Usual UI development needs no dev server: run `npm run build:watch`, which rebuilds
// `build/` on every save, and the Rust server (UI_DIR) serves the result; refresh the
// browser to see a change.
//
// `npm run dev` also works, for hot reload: Vite serves the app on :5173 and forwards
// the identity provider's paths to the Rust server, so the browser still sees one
// origin. Start the server with AUTH_UI_BASE_URL=http://localhost:5173 so its
// console-session origin check accepts this origin.
const IDP = process.env.GATEKEEPER_SERVER_URL ?? 'http://localhost:8080';
const IDP_PATHS = ['/api', '/session', '/v1', '/oauth', '/.well-known', '/authentication'];

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit(),
		paraglideVitePlugin({ project: './project.inlang', outdir: './src/lib/paraglide' })
	],
	server: {
		proxy: Object.fromEntries(IDP_PATHS.map((path) => [path, { target: IDP }]))
	}
});
