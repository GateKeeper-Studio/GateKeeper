use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    application::{
        ports::{
            risk_engine::{RiskEngine, RiskEvaluationInput},
            task_spawner::TaskSpawner,
            unit_of_work::UnitOfWorkFactory,
        },
        shared::token_policy::MAGIC_LINK_TTL_MINUTES,
        use_cases::authentication::{
            login_flow::{AuthorizationRequest, open_session},
            request_magic_link::{
                command::Command,
                dispatcher::{MagicLinkDispatcher, MagicLinkJob},
                response::Response,
            },
        },
    },
    domain::errors::DomainError,
};

// POST /v1/auth/magic-link — "email me a sign-in link".
//
// CONSTANT TIME (S6). This endpoint takes only an email, so it must answer the same,
// and in the same time, whether that email has an account or not. Everything here
// depends only on public inputs (the client, the authorization request, the caller's
// IP): it opens a session that binds NO user, and returns its code. Looking the
// account up, minting the link and sending the email happen in the background
// (`MagicLinkDispatcher`, through `TaskSpawner`) after this has returned.
//
// The returned `sessionCode` is the same-browser binding: the hosted login keeps it
// in this browser, and the emailed link only redeems together with it.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    risk_engine: Arc<dyn RiskEngine>,
    task_spawner: Arc<dyn TaskSpawner>,
    dispatcher: Arc<MagicLinkDispatcher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        risk_engine: Arc<dyn RiskEngine>,
        task_spawner: Arc<dyn TaskSpawner>,
        dispatcher: Arc<MagicLinkDispatcher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            risk_engine,
            task_spawner,
            dispatcher,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // 1. Resolve the client by its public id and derive the tenant.
        let client = uow
            .oauth_client_repository()
            .find_by_client_id_unscoped(&command.client_id)
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;
        let tenant_id = client.tenant_id();

        // 1b. Not an enumeration risk: whether a client offers magic links is part of
        //     its PUBLIC auth data, so refusing outright tells nothing new.
        if !client.allows_magic_link_login() {
            return Err(DomainError::AccessDenied(
                "this application does not offer sign-in links".to_string(),
            ));
        }

        // 2. Validate the authorization request the session will freeze.
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

        // 3. Pre-authentication IP gate, the same one login uses. Depends on the IP
        //    only, never on the email, so it is safe on the request path.
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
                    username_normalized: Some(command.email.trim().to_lowercase()),
                    ip_address: command.ip_address.clone(),
                    user_agent: command.user_agent.clone(),
                    credentials_valid: false,
                    pre_auth_blocked: true,
                    failure_reason: Some("ip_blocked".to_string()),
                    timestamp: Utc::now(),
                })
                .await;
            return Err(DomainError::AuthenticationError(
                "sign-in is temporarily unavailable, please try again later".to_string(),
            ));
        }

        // 4. Open the session that waits for the click. It lives as long as the link.
        let (mut session, raw_session_token) = open_session(
            &client,
            &request,
            scopes,
            Duration::minutes(MAGIC_LINK_TTL_MINUTES),
        );
        session.await_magic_link();
        uow.auth_session_repository().add(&session).await?;
        uow.commit().await?;

        // 5. Everything that depends on the email, off the request path.
        let job = MagicLinkJob {
            tenant_id,
            client_id: client.id(),
            auth_session_id: session.id(),
            email: command.email.trim().to_string(),
            ip_address: command.ip_address.clone(),
            user_agent: command.user_agent.clone(),
        };
        let dispatcher = Arc::clone(&self.dispatcher);
        self.task_spawner.spawn(
            "magic_link_dispatch",
            Box::pin(async move { dispatcher.dispatch(job).await }),
        );

        Ok(Response {
            session_code: raw_session_token,
            session_expires_at: session.expires_at(),
            session_absolute_expires_at: session.absolute_expires_at(),
        })
    }
}
