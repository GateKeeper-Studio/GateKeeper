// Public "Sign in with Google/GitHub" flow. `start` redirects the browser to the
// external provider; `callback` exchanges the returned code, links/provisions the
// user, and mints our own authorization code so the normal OAuth flow continues.
pub mod callback;
pub mod start;
