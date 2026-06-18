#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::email_provider::list_email_providers::{
            query::Query, use_case::UseCase,
        },
        domain::entities::{
            email_provider::{EmailProvider, SmtpEncryption},
            tenant::Tenant,
        },
        testing::in_memory_repositories::{
            in_memory_email_provider_repository::InMemoryEmailProviderRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
        },
    };

    #[tokio::test]
    async fn lists_tenant_providers_paged() {
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
        for i in 0..2 {
            let p = EmailProvider::new(
                tenant_id,
                format!("P{i}"),
                "h".to_string(),
                587,
                "u".to_string(),
                "enc:x".to_string(),
                "from@example.com".to_string(),
                None,
                SmtpEncryption::StartTls,
                false,
            );
            providers.memory.data.lock().unwrap().insert(p.id(), p);
        }

        let use_case = UseCase::new(providers, tenants);
        let page = use_case
            .execute(Query {
                tenant_id,
                page: None,
                page_size: None,
            })
            .await
            .unwrap();

        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.page, 1);
    }
}
