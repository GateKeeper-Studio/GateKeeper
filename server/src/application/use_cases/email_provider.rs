// Admin CRUD for per-tenant SMTP email providers (the catalog an OAuth client
// picks from to send its users' emails). Mirrors the oauth_client admin CRUD.
pub mod provider_view;
pub mod validation;

pub mod create_email_provider;
pub mod delete_email_provider;
pub mod get_email_provider_by_id;
pub mod list_email_providers;
pub mod send_test_email;
pub mod update_email_provider;
