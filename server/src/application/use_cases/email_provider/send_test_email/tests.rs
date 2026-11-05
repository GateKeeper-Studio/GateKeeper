#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::ports::email_sender::EmailSender,
        application::use_cases::email_provider::send_test_email::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                email_provider::{EmailProvider, SmtpEncryption},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_email_provider_repository::InMemoryEmailProviderRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
            },
            recording_email_sender::RecordingEmailSender,
        },
    };

    fn seed(is_active: bool) -> (UseCase, Arc<RecordingEmailSender>, Uuid, Uuid) {
        let tenant = Tenant::reconstruct(
            Uuid::now_v7(),
            "T".to_string(),
            None,
            "s".to_string(),
            true,
            true,
            true,
            Uuid::now_v7(),
            Utc::now(),
            None,
        );
        let tenant_id = tenant.id();
        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, tenant);

        let providers = Arc::new(InMemoryEmailProviderRepository::new());
        let provider = EmailProvider::reconstruct(
            Uuid::now_v7(),
            tenant_id,
            "P".to_string(),
            "smtp.example.com".to_string(),
            587,
            "user".to_string(),
            "enc:x".to_string(),
            "from@example.com".to_string(),
            None,
            SmtpEncryption::StartTls,
            false,
            is_active,
            Utc::now(),
            None,
        );
        let provider_id = provider.id();
        providers
            .memory
            .data
            .lock()
            .unwrap()
            .insert(provider_id, provider);

        let emails = Arc::new(RecordingEmailSender::new());
        let use_case = UseCase::new(providers, tenants, emails.clone() as Arc<dyn EmailSender>);
        (use_case, emails, tenant_id, provider_id)
    }

    #[tokio::test]
    async fn sends_test_email_to_recipient() {
        let (use_case, emails, tenant_id, provider_id) = seed(true);
        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    id: provider_id,
                    to: "dest@example.com".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(
            emails.test_emails.lock().unwrap().as_slice(),
            ["dest@example.com"]
        );
    }

    #[tokio::test]
    async fn can_test_an_inactive_provider() {
        // Testing must work before a provider is activated, so credentials can be
        // validated up front.
        let (use_case, emails, tenant_id, provider_id) = seed(false);
        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    id: provider_id,
                    to: "dest@example.com".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(emails.test_emails.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn unknown_provider_is_not_found() {
        let (use_case, _emails, tenant_id, _) = seed(true);
        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    id: Uuid::now_v7(),
                    to: "dest@example.com".to_string(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
