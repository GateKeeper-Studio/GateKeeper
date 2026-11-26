use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
use crate::application::shared::token_policy::AUTH_CODE_TTL_SECONDS;
use crate::{
    application::{
        ports::{
            external_identity_provider_client::{
                ExternalIdentityProviderClient, ExternalUserProfile,
            },
            risk_engine::{RiskEngine, RiskEvaluationInput},
            secret_cipher::SecretCipher,
            unit_of_work::{UnitOfWork, UnitOfWorkFactory},
            webauthn::WebauthnService,
        },
        services::templated_email_sender::TemplatedEmailSender,
        shared::token_policy::MFA_CODE_TTL_MINUTES,
        use_cases::{
            authentication::{
                external_login::callback::{
                    command::Command,
                    response::{Continuation, OidcParams, Response, build_continue_url},
                },
                login,
                login_flow::{
                    AuthorizationRequest, MfaGate, MfaHints, issue_change_password_code,
                    open_session,
                },
                session_policy::SESSION_IDLE_TTL_MINUTES,
            },
            webhook::user_event_payload::user_event,
        },
    },
    domain::{
        entities::{
            authorization_code::AuthorizationCode,
            consent_grant::ConsentGrant,
            domain_event_type::{DomainEventType, UserEventSource},
            external_identity_provider::ExternalProviderKind,
            login_event::RiskDecision,
            social_identity::SocialIdentity,
            user::User,
            user_profile::UserProfile,
        },
        errors::DomainError,
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

// Mirrors the credential flow's single-use authorization code TTL.

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    provider_client: Arc<dyn ExternalIdentityProviderClient>,
    secret_cipher: Arc<dyn SecretCipher>,
    // Same engine the password login uses, so a social sign-in is geo-enriched,
    // recorded as a login event, and refused on a critical risk decision.
    risk_engine: Arc<dyn RiskEngine>,
    mfa_gate: MfaGate,
    templated_email: Arc<TemplatedEmailSender>,
    auth_ui_base_url: String,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        provider_client: Arc<dyn ExternalIdentityProviderClient>,
        secret_cipher: Arc<dyn SecretCipher>,
        risk_engine: Arc<dyn RiskEngine>,
        webauthn: Arc<dyn WebauthnService>,
        templated_email: Arc<TemplatedEmailSender>,
        auth_ui_base_url: String,
    ) -> Self {
        Self {
            unit_of_work_factory,
            provider_client,
            secret_cipher,
            risk_engine,
            mfa_gate: MfaGate::new(webauthn),
            templated_email,
            auth_ui_base_url,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        let now = Utc::now();

        // 1. Resolve the pending flow by the echoed state. Generic failure so a
        //    forged/expired state can't be distinguished.
        let invalid = || DomainError::InvalidRequest("invalid or expired login flow".to_string());

        let flow = uow
            .external_login_flow_repository()
            .find_by_state_hash(&sha256_hex(&command.state))
            .await?
            .ok_or_else(invalid)?;

        if flow.is_expired(now) {
            return Err(invalid());
        }

        let tenant_id = flow.tenant_id();

        // 2. Load the provider config and exchange the code for the user's profile.
        let provider = uow
            .external_identity_provider_repository()
            .find_by_id(tenant_id, flow.external_provider_id())
            .await?
            .ok_or_else(|| DomainError::NotFound("external provider not found".into()))?;

        let secret = self.secret_cipher.decrypt(
            provider.provider_client_secret_encrypted(),
            SecretContext::new(SecretPurpose::ExternalIdpClientSecret, provider.tenant_id()),
        )?;

        let profile = self
            .provider_client
            .exchange_code(
                flow.provider(),
                provider.provider_client_id(),
                &secret,
                provider.redirect_uri(),
                &command.code,
            )
            .await?;

        // 3. Recognise, link, or provision the local user.
        let (user_id, user_email) = self
            .resolve_user(uow.as_ref(), tenant_id, flow.provider(), &profile)
            .await?;

        // 4. The client this login is for must still be active.
        let client = uow
            .oauth_client_repository()
            .find_by_id(tenant_id, flow.oauth_client_id())
            .await?
            .filter(|c| c.is_active())
            .ok_or_else(|| DomainError::InvalidClient("unknown or inactive client".to_string()))?;

        let scopes: Vec<String> = flow
            .client_scope()
            .split_whitespace()
            .map(str::to_string)
            .collect();

        // 4b. Risk evaluation (same engine as the password login): geo-enrich and
        //     record this social sign-in, and refuse it on a critical decision. Social
        //     login mints the code and 302s straight back with no MFA interstitial, so
        //     we don't step up — a temporary block simply refuses the sign-in.
        let risk = self
            .risk_engine
            .evaluate(RiskEvaluationInput {
                tenant_id,
                user_id: Some(user_id),
                client_id: Some(client.id()),
                username_normalized: profile.email.as_deref().map(|e| e.trim().to_lowercase()),
                ip_address: command.ip_address.clone(),
                user_agent: command.user_agent.clone(),
                credentials_valid: true,
                pre_auth_blocked: false,
                failure_reason: None,
                timestamp: now,
            })
            .await?;
        if risk.decision == RiskDecision::BlockTemporarily {
            return Err(DomainError::AuthenticationError(
                "sign-in blocked, please try again later".to_string(),
            ));
        }

        // 5. Record consent implicitly (the external provider authenticated the
        //    user for this first-party app). Idempotent union, as in authorize_session.
        // SECURITY: a social sign-in is a first factor like a password. The second one
        // is due here under the same rules (`MfaGate`): the client's policy, a factor
        // the user enrolled, or a risk step-up. Before this, the callback handed out
        // the authorization code directly, so "Sign in with Google" skipped two-step
        // sign-in altogether. When a factor is due no code is minted: the browser gets
        // an MFA-pending session and finishes on the hosted login page.
        let force_mfa = risk.decision == RiskDecision::AllowWithMfa;
        let (mut session, raw_session_token) = open_session(
            &client,
            &AuthorizationRequest {
                redirect_uri: flow.client_redirect_uri(),
                response_type: "code",
                scope: flow.client_scope(),
                state: flow.client_state(),
                code_challenge: flow.client_code_challenge(),
                code_challenge_method: flow.client_code_challenge_method(),
                nonce: flow.client_nonce(),
            },
            scopes.clone(),
            Duration::minutes(SESSION_IDLE_TTL_MINUTES),
        );
        session.authenticate(user_id, now);
        uow.auth_session_repository().add(&session).await?;
        let mfa = self
            .mfa_gate
            .engage(
                uow.as_ref(),
                &client,
                &mut session,
                user_id,
                &user_email,
                force_mfa,
                MfaHints::default(),
            )
            .await?;

        if session.is_mfa_pending() {
            if force_mfa {
                session.set_risk_login_event_id(risk.login_event_id);
            }
            // A pending forced password change must still be completed first.
            let must_change_password = uow
                .user_repository()
                .find_by_id_with_credentials(user_id)
                .await?
                .and_then(|u| u.credentials().map(|c| c.should_change_password()))
                .unwrap_or(false);
            let change_password_code = if must_change_password {
                Some(issue_change_password_code(uow.as_ref(), tenant_id, user_id).await?)
            } else {
                None
            };
            uow.auth_session_repository().update(&session).await?;
            uow.external_login_flow_repository()
                .delete_by_id(flow.id())
                .await?;
            uow.commit().await?;

            if let Some(otp) = &mfa.pending_email_otp {
                self.templated_email
                    .send_mfa_code(
                        tenant_id,
                        otp.provider.as_ref(),
                        &otp.email,
                        &otp.code,
                        client.name(),
                        &format!("{MFA_CODE_TTL_MINUTES} minutes"),
                    )
                    .await?;
            }

            let continuation = Continuation {
                email: user_email,
                login: login::response::Response {
                    session_code: raw_session_token,
                    user_id: Some(user_id.to_string()),
                    credentials_verified: true,
                    mfa_type: mfa.mfa_type,
                    mfa_id: mfa.mfa_id,
                    web_authn_options: mfa.web_authn_options,
                    available_mfa_methods: mfa.available_mfa_methods,
                    mfa_enrollment_required: mfa.mfa_enrollment_required,
                    enrollable_methods: mfa.enrollable_methods,
                    change_password_code,
                    session_expires_at: session.expires_at(),
                    session_absolute_expires_at: session.absolute_expires_at(),
                    message: "Second factor required".to_string(),
                },
                oidc: OidcParams {
                    redirect_uri: flow.client_redirect_uri().to_string(),
                    response_type: "code".to_string(),
                    scope: flow.client_scope().to_string(),
                    state: flow.client_state().to_string(),
                    code_challenge: flow.client_code_challenge().to_string(),
                    code_challenge_method: flow.client_code_challenge_method().to_string(),
                    nonce: flow.client_nonce().map(str::to_string),
                },
            };
            return Ok(Response::SecondFactor {
                continue_url: build_continue_url(
                    &self.auth_ui_base_url,
                    &client.client_id().to_string(),
                    &continuation,
                ),
            });
        }

        // No second factor is due: the session has done its job, the code is minted
        // right here.
        session.expire_now(now);
        uow.auth_session_repository().update(&session).await?;

        let existing = uow
            .consent_grant_repository()
            .find_by_user_and_client(user_id, client.id())
            .await?;
        let grant = match existing {
            Some(mut grant) => {
                grant.grant_scopes(&scopes);
                grant
            }
            None => ConsentGrant::new(tenant_id, user_id, client.id(), scopes.clone()),
        };
        uow.consent_grant_repository().upsert(&grant).await?;

        // 6. Mint the single-use authorization code from the frozen client request.
        let raw_code = generate_opaque_token();
        let code = AuthorizationCode::new(
            tenant_id,
            sha256_hex(&raw_code),
            client.id().to_string(),
            user_id,
            flow.client_redirect_uri().to_string(),
            scopes,
            flow.client_code_challenge().to_string(),
            flow.client_code_challenge_method().to_string(),
            flow.client_nonce().unwrap_or_default().to_string(),
            now + Duration::seconds(AUTH_CODE_TTL_SECONDS),
        );
        uow.authorization_code_repository().add(&code).await?;

        // 7. Consume the flow so the state can't be replayed.
        uow.external_login_flow_repository()
            .delete_by_id(flow.id())
            .await?;

        uow.commit().await?;

        Ok(Response::Code {
            redirect_uri: flow.client_redirect_uri().to_string(),
            authorization_code: raw_code,
            state: flow.client_state().to_string(),
        })
    }

    // Maps an external profile to a local user id: recognise a returning user by
    // provider subject; otherwise (requiring a verified email) link to an existing
    // account with that email, or provision a new passwordless user.
    async fn resolve_user(
        &self,
        uow: &dyn UnitOfWork,
        tenant_id: uuid::Uuid,
        provider: ExternalProviderKind,
        profile: &ExternalUserProfile,
    ) -> Result<(uuid::Uuid, String), DomainError> {
        let social_repo = uow.social_identity_repository();
        let user_repo = uow.user_repository();

        // 1. Returning user, recognised by the provider's stable subject.
        if let Some(identity) = social_repo
            .find_by_provider_subject(tenant_id, provider, &profile.subject)
            .await?
        {
            let user = user_repo
                .find_by_id(identity.user_id())
                .await?
                .filter(|u| u.tenant_id() == tenant_id && u.is_active())
                .ok_or_else(|| {
                    DomainError::AuthenticationError("account is not available".to_string())
                })?;
            return Ok((user.id(), user.email().to_string()));
        }

        // 2. First-time link/provision requires a provider-verified email — this is
        //    what makes linking to an existing account safe from takeover.
        let email = profile
            .email
            .as_deref()
            .filter(|e| !e.is_empty())
            .ok_or_else(|| DomainError::Upstream("the provider did not return an email".into()))?;
        if !profile.email_verified {
            return Err(DomainError::Upstream(
                "the provider email is not verified".into(),
            ));
        }

        // 2a. Link to an existing tenant user with this email.
        if let Some(existing) = user_repo.find_by_email(tenant_id, email).await? {
            if !existing.is_active() {
                return Err(DomainError::AuthenticationError(
                    "account is disabled".to_string(),
                ));
            }
            social_repo
                .add(&SocialIdentity::new(
                    existing.id(),
                    tenant_id,
                    provider,
                    profile.subject.clone(),
                    Some(email.to_lowercase()),
                ))
                .await?;
            return Ok((existing.id(), existing.email().to_string()));
        }

        // 2b. Otherwise provision a new passwordless user (email pre-verified).
        let mut user = User::new(tenant_id, email.to_string(), None, None);
        user.verify_email();
        user.set_profile(UserProfile::new(
            user.id(),
            profile.name.clone(),
            String::new(),
            String::new(),
        ));
        user_repo.add(&user).await?;
        social_repo
            .add(&SocialIdentity::new(
                user.id(),
                tenant_id,
                provider,
                profile.subject.clone(),
                Some(email.to_string()),
            ))
            .await?;

        // Only this branch announces a new user. Branch 2a above LINKS a social
        // identity to an account the tenant already had — from a subscriber's point
        // of view no user was created there, and emitting user.created would make
        // consumers insert a duplicate.
        uow.outbox_event_repository()
            .enqueue(&user_event(
                DomainEventType::UserCreated,
                &user,
                Some(UserEventSource::ExternalProvider),
            ))
            .await?;

        Ok((user.id(), user.email().to_string()))
    }
}
