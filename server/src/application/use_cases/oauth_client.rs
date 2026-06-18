// Admin CRUD for OAuth client (relying party) registrations — the equivalent of
// "apps in a developer console". Distinct from the runtime OAuth flow in `oauth/`.
pub mod client_view;
pub mod validation;

pub mod create_oauth_client;
pub mod delete_oauth_client;
pub mod get_client_auth_data;
pub mod get_oauth_client_by_id;
pub mod list_oauth_clients;
pub mod rotate_oauth_client_secret;
pub mod update_oauth_client;
