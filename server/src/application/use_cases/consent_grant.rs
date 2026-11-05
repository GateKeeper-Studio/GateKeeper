// Consent grants are NOT a generic admin CRUD: they are created/widened only
// through the authorization flow (authorize_session and the external-login
// callback), so there is no standalone create/update here. What a user
// legitimately needs is the "connected apps" surface: LIST what they have
// authorized, and REVOKE an app's access.
pub mod consent_view;
pub mod list_user_consents;
pub mod revoke_consent;
