#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::email_provider::delete_email_provider::{
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
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    struct Ctx {
        use_case: UseCase,
        providers: Arc<InMemoryEmailProviderRepository>,
        tenant_id: Uuid,
        provider_id: Uuid,
    }

    fn setup() -> Ctx {
        let tenant = Tenant::reconstruct(
            Uuid::now_v7(),
            "T".to_string(),
            None,
            "s".to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
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
            .insert(tenant.id(), tenant);

        let providers = Arc::new(InMemoryEmailProviderRepository::new());
        let provider = EmailProvider::new(
            tenant_id,
            "P".to_string(),
            "h".to_string(),
            587,
            "u".to_string(),
            "enc:x".to_string(),
            "from@example.com".to_string(),
            None,
            SmtpEncryption::StartTls,
            false,
        );
        let provider_id = provider.id();
        providers
            .memory
            .data
            .lock()
            .unwrap()
            .insert(provider.id(), provider);

        let users = Arc::new(InMemoryUserRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_email_provider_repository(providers.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory)),
            providers,
            tenant_id,
            provider_id,
        }
    }

    #[tokio::test]
    async fn deletes_existing_provider() {
        let ctx = setup();
        ctx.use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: ctx.tenant_id,
                    id: ctx.provider_id,
                },
            )
            .await
            .unwrap();
        assert!(
            ctx.providers
                .memory
                .data
                .lock()
                .unwrap()
                .get(&ctx.provider_id)
                .is_none()
        );
    }

    #[tokio::test]
    async fn unknown_provider_is_not_found() {
        let ctx = setup();
        let err = ctx
            .use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id: ctx.tenant_id,
                    id: Uuid::now_v7(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
