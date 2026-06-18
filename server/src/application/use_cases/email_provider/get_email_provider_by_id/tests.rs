#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::email_provider::get_email_provider_by_id::{
            query::Query, use_case::UseCase,
        },
        domain::{
            entities::{
                email_provider::{EmailProvider, SmtpEncryption},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::in_memory_repositories::{
            in_memory_email_provider_repository::InMemoryEmailProviderRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
        },
    };

    fn seed() -> (UseCase, Uuid, Uuid) {
        let tenant = Tenant::reconstruct(
            Uuid::now_v7(),
            "T".to_string(),
            None,
            "s".to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
            false,
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

        (UseCase::new(providers, tenants), tenant_id, provider_id)
    }

    #[tokio::test]
    async fn returns_provider_without_password() {
        let (use_case, tenant_id, provider_id) = seed();
        let view = use_case
            .execute(Query {
                tenant_id,
                id: provider_id,
            })
            .await
            .unwrap();
        assert_eq!(view.id, provider_id);
        assert_eq!(view.from_email, "from@example.com");
    }

    #[tokio::test]
    async fn unknown_provider_is_not_found() {
        let (use_case, tenant_id, _) = seed();
        let err = use_case
            .execute(Query {
                tenant_id,
                id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::EmailProviderNotFound(_)));
    }
}
