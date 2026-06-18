#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::email_provider::update_email_provider::{
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
            stub_secret_cipher::StubSecretCipher,
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
        // Seed an existing provider whose password is stored as "enc:old".
        let provider = EmailProvider::new(
            tenant_id,
            "Old".to_string(),
            "smtp.old.com".to_string(),
            587,
            "u".to_string(),
            "enc:old".to_string(),
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
            use_case: UseCase::new(Arc::new(factory), Arc::new(StubSecretCipher)),
            providers,
            tenant_id,
            provider_id,
        }
    }

    fn cmd(ctx: &Ctx, password: Option<String>) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            id: ctx.provider_id,
            name: "SMTP Gmail".to_string(),
            host: "smtp.gmail.com".to_string(),
            port: 465,
            username: "user@gmail.com".to_string(),
            password,
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Example".to_string()),
            encryption: "tls".to_string(),
            is_default: false,
            is_active: true,
        }
    }

    fn stored_password(ctx: &Ctx) -> String {
        ctx.providers
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.provider_id)
            .unwrap()
            .password_encrypted()
            .to_string()
    }

    #[tokio::test]
    async fn updates_fields_and_keeps_password_when_omitted() {
        let ctx = setup();
        let view = ctx.use_case.execute(cmd(&ctx, None)).await.unwrap();

        assert_eq!(view.name, "SMTP Gmail");
        assert_eq!(view.host, "smtp.gmail.com");
        assert_eq!(view.encryption, "tls");
        // Password unchanged.
        assert_eq!(stored_password(&ctx), "enc:old");
    }

    #[tokio::test]
    async fn replaces_password_when_provided() {
        let ctx = setup();
        ctx.use_case
            .execute(cmd(&ctx, Some("brand-new".to_string())))
            .await
            .unwrap();
        assert_eq!(stored_password(&ctx), "enc:brand-new");
    }

    #[tokio::test]
    async fn unknown_provider_is_rejected() {
        let ctx = setup();
        let mut c = cmd(&ctx, None);
        c.id = Uuid::now_v7();
        let err = ctx.use_case.execute(c).await.unwrap_err();
        assert!(matches!(err, DomainError::EmailProviderNotFound(_)));
    }
}
