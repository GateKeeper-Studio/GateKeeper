use std::sync::Arc;

use crate::{
    application::{
        ports::{secret_cipher::SecretCipher, unit_of_work::UnitOfWorkFactory},
        use_cases::webhook_subscription::{
            subscription_view::SubscriptionView,
            update_webhook_subscription::{command::Command, response::Response},
            validation::{validate_event_types, validate_webhook_url},
        },
    },
    domain::errors::DomainError,
    utils::oauth_crypto::generate_opaque_token,
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    secret_cipher: Arc<dyn SecretCipher>,
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

        let mut subscription = uow
            .webhook_subscription_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::WebhookSubscriptionNotFound(format!(
                    "Webhook subscription {} not found",
                    command.id
                ))
            })?;

        let url = command.url.trim().to_string();
        // Re-validated on every edit: the SSRF check must not be bypassable by
        // creating a benign subscription and then editing the URL.
        validate_webhook_url(&url, self.allow_private_targets)?;
        let event_types = validate_event_types(command.event_types)?;

        subscription.update_details(
            command.name.trim().to_string(),
            url,
            event_types,
            command.is_active,
        );

        let raw_secret = if command.rotate_secret {
            let secret = generate_opaque_token();
            subscription.rotate_secret(self.secret_cipher.encrypt(&secret)?);
            Some(secret)
        } else {
            None
        };

        uow.webhook_subscription_repository()
            .update(&subscription)
            .await?;
        uow.commit().await?;

        Ok(Response {
            subscription: SubscriptionView::from_entity(&subscription),
            secret: raw_secret,
        })
    }
}
