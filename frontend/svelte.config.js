import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    // SPA mode: this is a client-only public OAuth client (no server-side
    // rendering of tokens), so we emit a static fallback and hydrate in the browser.
    adapter: adapter({ fallback: 'index.html' })
  }
};

export default config;
