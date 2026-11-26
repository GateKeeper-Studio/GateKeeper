import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    // Must match OAUTH_REDIRECT_URI exactly, so fail instead of silently moving to
    // another port (the dashboard's optional `npm run dev` also defaults to 5173).
    port: 5173,
    strictPort: true
  }
});
