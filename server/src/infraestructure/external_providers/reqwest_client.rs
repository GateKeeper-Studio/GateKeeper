use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

use crate::{
    application::ports::external_identity_provider_client::{
        ExternalIdentityProviderClient, ExternalUserProfile,
    },
    domain::{entities::external_identity_provider::ExternalProviderKind, errors::DomainError},
};

// Talks to Google and GitHub over HTTPS to complete an external login: builds the
// authorization URL (leg 1) and exchanges the returned code for the user's
// profile (leg 2). One shared, connection-pooling client is reused for all calls.
pub struct ReqwestExternalIdentityProviderClient {
    http: Client,
}

// Ceiling on a single provider call. Google and GitHub normally answer in well
// under a second; this is the point at which we stop waiting, not a target.
const REQUEST_TIMEOUT_SECONDS: u64 = 10;

// Ceiling on establishing the TCP+TLS connection specifically, so a black-holed
// route fails fast instead of consuming the whole request budget.
const CONNECT_TIMEOUT_SECONDS: u64 = 5;

impl ReqwestExternalIdentityProviderClient {
    pub fn new() -> Self {
        // GitHub's API rejects requests without a User-Agent, so set one globally.
        let http = Client::builder()
            .user_agent("Gatekeeper")
            // Without a timeout a stalled Google/GitHub endpoint hangs the external-
            // login callback forever: the handler never returns, so the request holds
            // its connection and its database connection from a pool of ten. A few of
            // those and the whole service stops serving. reqwest has NO default
            // timeout, so this has to be set explicitly.
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECONDS))
            .build()
            .expect("failed to build reqwest client");
        Self { http }
    }

    async fn exchange_google(
        &self,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        code: &str,
    ) -> Result<ExternalUserProfile, DomainError> {
        let token: TokenResponse = self
            .http
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("redirect_uri", redirect_uri),
            ])
            .send()
            .await
            .map_err(|e| provider_err(format!("Google token request failed: {e}")))?
            .error_for_status()
            .map_err(|e| provider_err(format!("Google token endpoint returned an error: {e}")))?
            .json()
            .await
            .map_err(|e| provider_err(format!("Invalid Google token response: {e}")))?;

        let user: serde_json::Value = self
            .http
            .get("https://openidconnect.googleapis.com/v1/userinfo")
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|e| provider_err(format!("Google userinfo request failed: {e}")))?
            .error_for_status()
            .map_err(|e| provider_err(format!("Google userinfo returned an error: {e}")))?
            .json()
            .await
            .map_err(|e| provider_err(format!("Invalid Google userinfo response: {e}")))?;

        let subject = user
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or_else(|| provider_err("Google userinfo is missing the subject"))?
            .to_string();

        Ok(ExternalUserProfile {
            subject,
            email: user
                .get("email")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            email_verified: as_bool(user.get("email_verified")),
            name: user
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
    }

    async fn exchange_github(
        &self,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        code: &str,
    ) -> Result<ExternalUserProfile, DomainError> {
        let token: TokenResponse = self
            .http
            .post("https://github.com/login/oauth/access_token")
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("code", code),
                ("redirect_uri", redirect_uri),
            ])
            .send()
            .await
            .map_err(|e| provider_err(format!("GitHub token request failed: {e}")))?
            .error_for_status()
            .map_err(|e| provider_err(format!("GitHub token endpoint returned an error: {e}")))?
            .json()
            .await
            .map_err(|e| provider_err(format!("Invalid GitHub token response: {e}")))?;

        // Surface the actually-granted scopes so a missing `user:email` is obvious
        // in the logs (the usual reason /user/emails 403s).
        let granted_scope = token.scope.clone().unwrap_or_default();
        if !granted_scope.contains("user:email") {
            tracing::warn!(
                "GitHub did not grant the `user:email` scope (granted: '{granted_scope}'). \
                 Revoke the app at github.com/settings/applications and sign in again to re-consent."
            );
        }

        let user: serde_json::Value = self
            .http
            .get("https://api.github.com/user")
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|e| provider_err(format!("GitHub user request failed: {e}")))?
            .error_for_status()
            .map_err(|e| provider_err(format!("GitHub user endpoint returned an error: {e}")))?
            .json()
            .await
            .map_err(|e| provider_err(format!("Invalid GitHub user response: {e}")))?;

        let subject = user
            .get("id")
            .and_then(|v| v.as_i64())
            .map(|id| id.to_string())
            .ok_or_else(|| provider_err("GitHub user is missing the id"))?;

        let name = user
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| user.get("login").and_then(|v| v.as_str()))
            .map(str::to_string);

        // The user's public profile email (if any). GitHub only lets a *verified*
        // address be set as the public email, so we can trust it as verified.
        let mut email = user
            .get("email")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let mut email_verified = email.is_some();

        // Prefer the authoritative primary verified address from /user/emails. That
        // endpoint needs the `user:email` scope and 403s when the OAuth-app grant
        // doesn't include it. Treat it as best-effort: on any failure, fall back to
        // the public profile email above rather than failing outright.
        let emails_response = self
            .http
            .get("https://api.github.com/user/emails")
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .bearer_auth(&token.access_token)
            .send()
            .await;

        // Captured for the diagnostic error below.
        let mut emails_body = String::new();
        let emails_status = match emails_response {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if status.is_success() {
                    let emails: Vec<GithubEmail> = serde_json::from_str(&body).unwrap_or_default();
                    if let Some(primary) = emails.iter().find(|e| e.primary && e.verified) {
                        email = Some(primary.email.clone());
                        email_verified = true;
                    } else if let Some(verified) = emails.iter().find(|e| e.verified) {
                        email = Some(verified.email.clone());
                        email_verified = true;
                    }
                } else {
                    // Keep the (truncated) body so the cause is visible in the response.
                    emails_body = body.chars().take(200).collect();
                    tracing::warn!("GitHub /user/emails returned {status}: {emails_body}");
                }
                status.as_u16().to_string()
            }
            Err(e) => format!("request failed: {e}"),
        };

        // If we still have no email, fail with everything we learned so the operator
        // can see *why* (granted scope + the emails-endpoint result) instead of a
        // bare "no email".
        if email.is_none() {
            return Err(provider_err(format!(
                "GitHub returned no email. Granted scope: '{granted_scope}'. \
                 /user/emails -> {emails_status} {emails_body}. \
                 Fix: ensure the OAuth app requests `user:email`, revoke it at \
                 github.com/settings/applications, and sign in again; or set a public \
                 email on your GitHub profile."
            )));
        }

        Ok(ExternalUserProfile {
            subject,
            email,
            email_verified,
            name,
        })
    }
}

impl Default for ReqwestExternalIdentityProviderClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ExternalIdentityProviderClient for ReqwestExternalIdentityProviderClient {
    fn authorize_url(
        &self,
        provider: ExternalProviderKind,
        client_id: &str,
        redirect_uri: &str,
        state: &str,
    ) -> String {
        let (endpoint, scope) = match provider {
            ExternalProviderKind::Google => (
                "https://accounts.google.com/o/oauth2/v2/auth",
                "openid email profile",
            ),
            ExternalProviderKind::Github => (
                "https://github.com/login/oauth/authorize",
                "read:user user:email",
            ),
        };

        // Build the query manually with RFC 3986 encoding (space -> %20). We avoid
        // `query_pairs_mut`, which form-encodes spaces as `+`: GitHub can then
        // misparse `read:user+user:email` as a single unknown scope and never grant
        // `user:email`, breaking the /user/emails lookup.
        let mut query = format!(
            "client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            enc(client_id),
            enc(redirect_uri),
            enc(scope),
            enc(state),
        );
        if matches!(provider, ExternalProviderKind::Google) {
            query.push_str("&access_type=online");
        }
        format!("{endpoint}?{query}")
    }

    async fn exchange_code(
        &self,
        provider: ExternalProviderKind,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
        code: &str,
    ) -> Result<ExternalUserProfile, DomainError> {
        match provider {
            ExternalProviderKind::Google => {
                self.exchange_google(client_id, client_secret, redirect_uri, code)
                    .await
            }
            ExternalProviderKind::Github => {
                self.exchange_github(client_id, client_secret, redirect_uri, code)
                    .await
            }
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    // The scopes the provider actually granted (GitHub returns e.g.
    // "read:user,user:email"). Used only for diagnostics.
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Deserialize, Default)]
struct GithubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

fn provider_err(msg: impl Into<String>) -> DomainError {
    DomainError::ExternalProviderError(msg.into())
}

// RFC 3986 percent-encoding for query values (space -> %20, ':' -> %3A). Used to
// build provider authorize URLs unambiguously.
fn enc(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

// Google returns `email_verified` as a JSON bool on userinfo but as a string in
// some token contexts — accept both.
fn as_bool(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}
