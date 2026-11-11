use std::sync::Arc;

use chrono::Utc;

use crate::{
    application::{
        ports::{
            geoip_resolver::GeoIpResolver,
            risk_engine::{RiskEngine, RiskEvaluationInput},
            unit_of_work::UnitOfWorkFactory,
        },
        use_cases::{
            authentication::login_flow::{AuthorizationRequest, open_session},
            qr_login::{
                shared::{
                    POLL_INTERVAL_SECONDS, approximate_location, code_lifetime, session_lifetime,
                },
                start::{command::Command, response::Response},
            },
        },
    },
    domain::{
        entities::qr_login_request::{QrLoginRequest, QrRequester},
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// POST /v1/auth/qr-login — the hosted sign-in page asks for a QR code.
//
// Opens a login session that freezes the authorization request and binds NO user
// (`AuthSession::await_qr_approval`), like a magic-link session, and a QR request
// tied to it. Nothing here depends on who will scan it, so there is nothing to
// enumerate. The caller's IP goes through the same pre-authentication gate as a
// password login.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    risk_engine: Arc<dyn RiskEngine>,
    geoip: Arc<dyn GeoIpResolver>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        risk_engine: Arc<dyn RiskEngine>,
        geoip: Arc<dyn GeoIpResolver>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            risk_engine,
            geoip,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        // 1. The client, and whether it offers QR sign-in (public auth data, so
        //    refusing outright reveals nothing).
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;
        if !client.allows_qr_login() {
            return Err(DomainError::AccessDenied(
                "this application does not offer QR code sign-in".to_string(),
            ));
        }
        let tenant_id = client.tenant_id();

        // 2. The authorization request the session freezes.
        let request = AuthorizationRequest {
            redirect_uri: &command.redirect_uri,
            response_type: &command.response_type,
            scope: &command.scope,
            state: &command.state,
            code_challenge: &command.code_challenge,
            code_challenge_method: &command.code_challenge_method,
            nonce: command.nonce.as_deref(),
        };
        let scopes = request.validate(&client)?;

        uow.tenant_repository()
            .find_by_id(tenant_id)
            .await?
            .filter(|t| t.is_active())
            .ok_or_else(|| DomainError::NotFound(format!("tenant {tenant_id} not found")))?;

        // 3. Pre-authentication IP gate, as on login.
        if self
            .risk_engine
            .check_pre_auth(tenant_id, command.ip_address.as_deref())
            .await
            .unwrap_or(false)
        {
            let _ = self
                .risk_engine
                .evaluate(RiskEvaluationInput {
                    tenant_id,
                    user_id: None,
                    client_id: Some(client.id()),
                    username_normalized: None,
                    ip_address: command.ip_address.clone(),
                    user_agent: command.user_agent.clone(),
                    credentials_valid: false,
                    pre_auth_blocked: true,
                    failure_reason: Some("ip_blocked".to_string()),
                    timestamp: now,
                })
                .await;
            return Err(DomainError::AuthenticationError(
                "sign-in is temporarily unavailable, please try again later".to_string(),
            ));
        }

        // 4. The waiting session lives as long as a scan plus its decision can take.
        let (mut session, raw_session_token) =
            open_session(&client, &request, scopes, session_lifetime());
        session.await_qr_approval();
        uow.auth_session_repository().add(&session).await?;

        // 5. The code. What the scanning device will be shown about this browser is
        //    captured now, from this request.
        let raw_token = generate_opaque_token();
        let qr = QrLoginRequest::new(
            tenant_id,
            client.id(),
            session.id(),
            sha256_hex(&raw_token),
            QrRequester {
                location: approximate_location(self.geoip.as_ref(), command.ip_address.as_deref()),
                ip_address: command.ip_address.clone(),
                user_agent: command.user_agent.clone(),
            },
            now,
            (now + code_lifetime()).min(session.absolute_expires_at()),
        );
        uow.qr_login_request_repository().add(&qr).await?;
        uow.commit().await?;

        Ok(Response {
            request_id: qr.id(),
            token: raw_token,
            session_code: raw_session_token,
            expires_at: qr.expires_at(),
            poll_interval_seconds: POLL_INTERVAL_SECONDS,
            session_expires_at: session.expires_at(),
            session_absolute_expires_at: session.absolute_expires_at(),
        })
    }
}
