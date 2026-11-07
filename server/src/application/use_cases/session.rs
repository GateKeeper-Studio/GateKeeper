// Signed-in sessions: the user's active refresh-token families, as a view over the
// rows token rotation already keeps (no table of their own). A user manages their own
// from /me/sessions; an admin manages anyone's in the tenant. Ending a session revokes
// its family, so the app can no longer refresh; an access token already issued stays
// valid until it expires (ACCESS_TOKEN_TTL_SECONDS).
pub mod list_user_sessions;
pub mod revoke_all_user_sessions;
pub mod revoke_user_session;
pub mod session_audit;
pub mod session_view;
