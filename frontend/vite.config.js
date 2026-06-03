import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    // Same default port the vanilla client documents, so a redirect_uri of
    // http://localhost:5173/ works for both.
    port: 5173,
    strictPort: false
  }
});
