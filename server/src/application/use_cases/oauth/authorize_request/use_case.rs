use std::sync::Arc;

use crate::{
    application::use_cases::oauth::authorize_request::{query::Query, response::Response},
    domain::{errors::DomainError, repositories::oauth_client_repository::OAuthClientRepository},
};

pub struct UseCase {
    oauth_client_repository: Arc<dyn OAuthClientRepository>,
}

impl UseCase {
    pub fn new(oauth_client_repository: Arc<dyn OAuthClientRepository>) -> Self {
        Self {
            oauth_client_repository,
        }
    }

    // Validate in this EXACT order — what you do with an error depends on whether
    // you can trust the redirect_uri yet (open-redirect guard).
    pub async fn execute(&self, query: Query) -> Result<Response, DomainError> {
        // 1. Client must exist and be active. Unknown/inactive → error PAGE (the
        //    handler must NOT redirect, the redirect_uri isn't trusted yet).
        let client = self
            .oauth_client_repository
            .find_by_client_id(query.tenant_id, &query.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        // 2. redirect_uri must EXACTLY match a registered URI (no wildcards). Mismatch
        //    → error PAGE, never a redirect.
        if !client.is_redirect_uri_allowed(&query.redirect_uri) {
            return Err(DomainError::InvalidRequest(
                "redirect_uri mismatch".to_string(),
            ));
        }

        // From here the redirect_uri is trusted; remaining errors may be returned as
        // an OAuth redirect (?error=...&state=...) by the handler.

        // 3. response_type MUST be exactly "code" (implicit flow is forbidden).
        if query.response_type != "code" {
            return Err(DomainError::InvalidRequest(
                "unsupported_response_type".to_string(),
            ));
        }

        // 4. PKCE is mandatory: code_challenge present and method == "S256".
        if client.require_pkce()
            && (query.code_challenge.is_empty() || query.code_challenge_method != "S256")
        {
            return Err(DomainError::InvalidRequest(
                "PKCE with S256 is required".to_string(),
            ));
        }

        // 5. state is required (CSRF protection for the browser flow).
        if query.state.is_empty() {
            return Err(DomainError::InvalidRequest("state is required".to_string()));
        }

        // 6. Scope: intersect with the client's allow-list (never widen). Empty after
        //    filtering → invalid_scope.
        let requested: Vec<String> = query
            .scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let scopes = client.filter_scopes(&requested);
        if scopes.is_empty() {
            return Err(DomainError::InvalidScope(
                "no requested scope is allowed for this client".to_string(),
            ));
        }

        Ok(Response {
            tenant_id: query.tenant_id,
            client_internal_id: client.id(),
            client_id: client.client_id().to_string(),
            client_name: client.name().to_string(),
            redirect_uri: query.redirect_uri,
            scopes,
            state: query.state,
            code_challenge: query.code_challenge,
            code_challenge_method: query.code_challenge_method,
            nonce: query.nonce,
            requires_consent: client.require_consent(),
        })
    }
}
