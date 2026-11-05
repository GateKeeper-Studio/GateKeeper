// Admin CRUD for a client's external (social) identity providers — the
// "Sign in with Google/GitHub" buttons. Distinct from the public sign-in flow in
// `authentication/external_login/`.
pub mod provider_view;

pub mod create_external_provider;
pub mod delete_external_provider;
pub mod list_external_providers;
pub mod update_external_provider;
