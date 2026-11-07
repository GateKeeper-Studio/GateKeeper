// The platform-admin console's browser session, used when this server serves the
// dashboard itself (UI_DIR). It replaces the dashboard's former Node BFF: the
// browser holds only an httpOnly cookie, and the `/api` cookie bridge turns a live
// session into a short-lived admin access token per request.
// See docs/SERVING-THE-DASHBOARD-FROM-THE-SERVER.md.
use chrono::Duration;

pub mod end;
pub mod identity;
pub mod resolve;
pub mod start;

// Absolute lifetime, fixed at sign-in. Matches the former dashboard cookie.
pub fn session_lifetime() -> Duration {
    Duration::hours(12)
}

// A session unused for this long stops working before its absolute lifetime.
pub fn idle_timeout() -> Duration {
    Duration::hours(2)
}

// `last_seen_at` is written at most this often per session.
pub fn touch_granularity() -> Duration {
    Duration::minutes(1)
}
