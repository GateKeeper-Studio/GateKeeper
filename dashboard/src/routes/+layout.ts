// A static single-page app, served by the identity provider itself from its UI_DIR
// (server/docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md). Nothing renders on a
// server: every page is built in the browser from the one fallback `index.html`.
export const ssr = false;
export const prerender = false;
