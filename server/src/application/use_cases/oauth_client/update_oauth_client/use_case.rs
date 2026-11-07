use std::sync::Arc;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::oauth_client::{
            client_view::ClientView,
            update_oauth_client::{command::Command, response::Response},
            validation::{
                normalize_optional, require_non_empty, validate_backchannel_logout_uri,
                validate_client_languages, validate_initiate_login_uri, validate_legal_uri,
                validate_login_appearance, validate_post_logout_redirect_uris,
                validate_redirect_uris,
            },
        },
    },
    domain::{entities::oauth_client::MfaPolicy, errors::DomainError},
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    // WEBHOOK_ALLOW_PRIVATE_TARGETS: the back-channel logout URI is an outbound
    // target exactly like a webhook URL, so the same escape hatch applies.
    allow_private_targets: bool,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        allow_private_targets: bool,
    ) -> Self {
        Self {
            unit_of_work_factory,
            allow_private_targets,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        if uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::NotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        validate_redirect_uris(&command.redirect_uris)?;
        require_non_empty("allowed_scopes", &command.allowed_scopes)?;
        require_non_empty("allowed_grant_types", &command.allowed_grant_types)?;

        let tos_uri = normalize_optional(command.tos_uri);
        let policy_uri = normalize_optional(command.policy_uri);
        validate_legal_uri("tos_uri", &tos_uri)?;
        validate_legal_uri("policy_uri", &policy_uri)?;

        let initiate_login_uri = normalize_optional(command.initiate_login_uri);
        validate_initiate_login_uri(&initiate_login_uri)?;

        validate_post_logout_redirect_uris(&command.post_logout_redirect_uris)?;
        let backchannel_logout_uri = normalize_optional(command.backchannel_logout_uri);
        validate_backchannel_logout_uri(&backchannel_logout_uri, self.allow_private_targets)?;

        let login_appearance = validate_login_appearance(command.login_appearance)?;
        let (allowed_languages, default_language) =
            validate_client_languages(command.allowed_languages, command.default_language)?;

        let mut client = uow
            .oauth_client_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("OAuth client {} not found", command.id))
            })?;

        // Turning external-login-only on with nothing to sign in through would lock
        // every user out: the hosted login page would render zero sign-in methods and
        // every password path is refused. Require at least one ACTIVE provider first.
        if command.external_login_only {
            let providers = uow
                .external_identity_provider_repository()
                .find_active_by_oauth_client_id(client.id())
                .await?;
            if providers.is_empty() {
                return Err(DomainError::InvalidRequest(
                    "external_login_only requires at least one active external identity provider"
                        .to_string(),
                ));
            }
        }

        client.update_details(
            command.name,
            command.redirect_uris,
            command.post_logout_redirect_uris,
            command.allowed_scopes,
            command.allowed_grant_types,
            command.require_pkce,
            command.require_consent,
            command.email_provider_id,
            tos_uri,
            policy_uri,
            initiate_login_uri,
            login_appearance,
            command.external_login_only,
            allowed_languages,
            default_language,
            MfaPolicy {
                required: command.mfa_required,
                email_enabled: command.mfa_email_enabled,
                totp_enabled: command.mfa_totp_enabled,
                webauthn_enabled: command.mfa_webauthn_enabled,
            },
            command.is_active,
        );
        client.set_magic_link_enabled(command.magic_link_enabled);
        client.set_backchannel_logout(
            backchannel_logout_uri,
            command.backchannel_logout_session_required,
        );

        uow.oauth_client_repository().update(&client).await?;
        uow.commit().await?;

        Ok(ClientView::from_entity(&client))
    }
}
