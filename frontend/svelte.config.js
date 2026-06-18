import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    // Backend-for-Frontend: the authorization code is exchanged for tokens on the
    // server, tokens live only in an encrypted httpOnly cookie, and pages render
    // with SSR. That requires a Node server runtime, not a static fallback.
    adapter: adapter()
  }
};

export default config;
