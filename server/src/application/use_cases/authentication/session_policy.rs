// Lifetime policy for the pre-authorization `AuthSession` — the short-lived thing
// that exists between "password accepted" and "authorization code issued" (MFA
// screens, forced password change).
//
// Two clocks, because one is not enough:
//
//   * IDLE — how long the session survives without the user doing anything. Short,
//     since the SPA normally chains login → MFA → authorize in seconds. A user who
//     is still there can push it forward via POST /v1/auth/session/refresh.
//
//   * ABSOLUTE — the ceiling, fixed when the session is created. An idle window you
//     can renew without limit is not a timeout at all, so refreshing clamps to this
//     and the session dies here regardless of activity.
//
// Both are consumed by `login` (which sets them) and `refresh_session` (which slides
// the idle one), so they live here rather than in either use case.
pub const SESSION_IDLE_TTL_MINUTES: i64 = 10;
pub const SESSION_ABSOLUTE_TTL_MINUTES: i64 = 30;
