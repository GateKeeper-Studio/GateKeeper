use async_trait::async_trait;

use crate::{
    application::ports::external_identity_provider_client::{
        ExternalIdentityProviderClient, ExternalUserProfile,
    },
    domain::{entities::external_identity_provider::ExternalProviderKind, errors::DomainError},
};

// Test double for the external provider HTTP client: `exchange_code` returns a
// canned profile (or a canned error) so use-case tests can drive the social
// login flow without real network calls.
pub struct FakeExternalIdentityProviderClient {
    result: Result<ExternalUserProfile, String>,
}

impl FakeExternalIdentityProviderClient {
    // Succeeds, returning `profile` from every `exchange_code`.
    pub fn returning(profile: ExternalUserProfile) -> Self {
        Self {
            result: Ok(profile),
        }
    }

    // Fails every `exchange_code` with an ExternalProviderError carrying `message`.
    pub fn failing(message: impl Into<String>) -> Self {
        Self {
            result: Err(message.into()),
        }
    }
}

#[async_trait]
impl ExternalIdentityProviderClient for FakeExternalIdentityProviderClient {
    fn authorize_url(
        &self,
        provider: ExternalProviderKind,
        client_id: &str,
        redirect_uri: &str,
        state: &str,
    ) -> String {
        format!(
            "https://provider.test/{}/authorize?client_id={client_id}&redirect_uri={redirect_uri}&state={state}",
            provider.as_str()
        )
    }

    async fn exchange_code(
        &self,
        _provider: ExternalProviderKind,
        _client_id: &str,
        _client_secret: &str,
        _redirect_uri: &str,
        _code: &str,
    ) -> Result<ExternalUserProfile, DomainError> {
        self.result
            .clone()
            .map_err(DomainError::ExternalProviderError)
    }
}
