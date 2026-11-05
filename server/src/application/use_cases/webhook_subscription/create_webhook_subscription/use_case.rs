use std::sync::Arc;

use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::webhook_subscription::{
            create_webhook_subscription::{command::Command, response::Response},
            subscription_view::SubscriptionView,
            validation::{validate_event_types, validate_webhook_url},
        },
    },
    domain::{entities::webhook_subscription::WebhookSubscription, errors::DomainError},
    utils::oauth_crypto::generate_opaque_token,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    secret_cipher: Arc<dyn SecretCipher>,
    // Mirrors WEBHOOK_ALLOW_PRIVATE_TARGETS. Injected rather than read from the
    // environment here so the trust boundary stays testable.
    allow_private_targets: bool,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        secret_cipher: Arc<dyn SecretCipher>,
        allow_private_targets: bool,
    ) -> Self {
        Self {
            unit_of_work_factory,
            secret_cipher,
            allow_private_targets,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        if uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .is_none()
        {
            return Err(DomainError::TenantNotFound(format!(
                "Tenant with id {} not found",
                command.tenant_id
            )));
        }

        let url = command.url.trim().to_string();
        validate_webhook_url(&url, self.allow_private_targets)?;
        let event_types = validate_event_types(command.event_types)?;

        // A fresh high-entropy secret per subscription: one compromised consumer
        // must not let an attacker forge deliveries to any other.
        let raw_secret = generate_opaque_token();
        let secret_encrypted = self.secret_cipher.encrypt(&raw_secret)?;

        let subscription = WebhookSubscription::new(
            command.tenant_id,
            command.name.trim().to_string(),
            url,
            secret_encrypted,
            event_types,
            command.is_active,
        );

        uow.webhook_subscription_repository()
            .add(&subscription)
            .await?;
        uow.commit().await?;

        Ok(Response {
            subscription: SubscriptionView::from_entity(&subscription),
            secret: raw_secret,
        })
    }
}
