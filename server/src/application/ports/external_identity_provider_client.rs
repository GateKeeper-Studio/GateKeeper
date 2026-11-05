use async_trait::async_trait;

use crate::domain::{
    entities::external_identity_provider::ExternalProviderKind, errors::DomainError,
};

// The normalized profile extracted from an external provider after exchanging an
// authorization code. `subject` is the provider's stable, unique user id ("sub").
#[derive(Debug, Clone)]
pub struct ExternalUserProfile {
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
}

// Talks to an external OAuth/OIDC provider (Google, GitHub). The concrete
// endpoints and HTTP client live in infrastructure; use cases depend only on
// this port so they stay provider-agnostic and unit-testable with a fake.
#[async_trait]
pub trait ExternalIdentityProviderClient: Send + Sync {
    // Build the provider's authorization URL the browser is redirected to (leg 1).
    // `redirect_uri` must be the callback registered at the provider, and `state`
    // the opaque CSRF value the provider will echo back.
    fn authorize_url(
        &self,
        provider: ExternalProviderKind,
        client_id: &str,
        redirect_uri: &str,
        state: &str,
    ) -> String;

    // Exchange the authorization code from the provider's callback for the user's
    // profile — token endpoint then userinfo — behind a single call (leg 2).
    async fn exchange_code(
        &self,
        provider: ExternalProviderKind,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        code: &str,
    ) -> Result<ExternalUserProfile, DomainError>;
}
