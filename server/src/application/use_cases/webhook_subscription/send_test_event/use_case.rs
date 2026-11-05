use std::sync::Arc;

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::application::ports::secret_cipher::{SecretContext, SecretPurpose};
use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::{
            secret_cipher::SecretCipher,
            unit_of_work::UnitOfWorkFactory,
            webhook_sender::{WebhookRequest, WebhookSender},
        },
        use_cases::webhook_subscription::send_test_event::{command::Command, response::Response},
    },
    domain::errors::DomainError,
};

// Delivers a synthetic event straight to one subscription, right now.
//
// Deliberately NOT routed through the outbox: this is a configuration check, not a
// fact about a user. Enqueuing it would mean fake data reaching every OTHER
// subscription of the tenant too, and polluting the event stream consumers dedupe
// against. It also bypasses the retry machinery on purpose — the admin is standing
// there waiting for an answer.
pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    sender: Arc<dyn WebhookSender>,
    secret_cipher: Arc<dyn SecretCipher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        sender: Arc<dyn WebhookSender>,
        secret_cipher: Arc<dyn SecretCipher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            sender,
            secret_cipher,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;
        let subscription = uow
            .webhook_subscription_repository()
            .find_by_id(command.tenant_id, command.id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Webhook subscription {} not found", command.id))
            })?;

        let secret = self.secret_cipher.decrypt(
            subscription.secret_encrypted(),
            SecretContext::new(SecretPurpose::WebhookSecret, subscription.tenant_id()),
        )?;

        // `test: true` and an obviously-synthetic identity, so a consumer that wires
        // this straight into its user table cannot mistake it for a real person.
        let event_id = Uuid::now_v7();
        let body = json!({
            "id": event_id.to_string(),
            "type": "user.created",
            "occurred_at": Utc::now().to_rfc3339(),
            "tenant_id": command.tenant_id.to_string(),
            "test": true,
            "data": {
                "user_id": Uuid::nil().to_string(),
                "email": "webhook-test@example.invalid",
                "email_verified": true,
                "display_name": "Webhook Test",
                "first_name": "Webhook",
                "last_name": "Test",
                "is_active": true,
                "source": "admin",
            },
        })
        .to_string();

        let outcome = self
            .sender
            .send(WebhookRequest {
                url: subscription.url(),
                secret: &secret,
                event_id,
                event_type: "user.created",
                body,
            })
            .await;

        Ok(Response {
            success: outcome.success,
            status: outcome.status,
            error: outcome.error,
        })
    }
}
