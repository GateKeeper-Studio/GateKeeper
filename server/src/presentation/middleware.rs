// Presentation-layer middleware. `auth` provides bearer-token authentication,
// RBAC enforcement (WI-08), and per-request audit context capture (WI-02);
// `client_ip` resolves the client address behind trusted reverse proxies.
pub mod auth;
pub mod client_ip;
pub mod metrics;
