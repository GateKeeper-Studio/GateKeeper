// This is a client-only public OAuth client: it touches window/localStorage and
// holds tokens in memory, so server-side rendering would be both pointless and a
// place to accidentally leak. Render entirely in the browser.
export const ssr = false;
export const prerender = false;
