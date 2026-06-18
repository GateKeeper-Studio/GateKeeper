#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::{
            ports::secret_cipher::SecretCipher,
            use_cases::email_provider::create_email_provider::{
                command::Command, use_case::UseCase,
            },
        },
        domain::{entities::tenant::Tenant, errors::DomainError},
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

        let users = Arc::new(InMemoryUserRepository::new());
        let providers = Arc::new(InMemoryEmailProviderRepository::new());

        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_email_provider_repository(providers.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory), Arc::new(StubSecretCipher)),
            providers,
            tenant_id,
        }
    }

    fn cmd(tenant_id: Uuid, is_default: bool) -> Command {
        Command {
            tenant_id,
            name: "SMTP Gmail".to_string(),
            host: "smtp.gmail.com".to_string(),
            port: 587,
            username: "user@gmail.com".to_string(),
            password: "app-password".to_string(),
            from_email: "noreply@example.com".to_string(),
            from_name: Some("Example".to_string()),
            encryption: "starttls".to_string(),
            is_default,
        }
    }

    #[tokio::test]
    async fn creates_and_encrypts_password() {
        let ctx = setup();

        let view = ctx
            .use_case
            .execute(cmd(ctx.tenant_id, true))
            .await
            .unwrap();
        assert_eq!(view.encryption, "starttls");
        assert!(view.is_default);

        let stored = ctx.providers.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let p = stored.values().next().unwrap();
        // Stored password is encrypted (not plaintext) but reversible.
        assert_ne!(p.password_encrypted(), "app-password");
        assert_eq!(
            StubSecretCipher.decrypt(p.password_encrypted()).unwrap(),
            "app-password"
        );
    }

    #[tokio::test]
    async fn invalid_encryption_is_rejected() {
        let ctx = setup();
        let mut c = cmd(ctx.tenant_id, false);
        c.encryption = "ssl".to_string();

        let err = ctx.use_case.execute(c).await.unwrap_err();
        assert!(matches!(err, DomainError::EmailProviderError(_)));
    }

    #[tokio::test]
    async fn second_default_clears_the_first() {
        let ctx = setup();
        ctx.use_case
            .execute(cmd(ctx.tenant_id, true))
            .await
            .unwrap();
        ctx.use_case
            .execute(cmd(ctx.tenant_id, true))
            .await
            .unwrap();

        let stored = ctx.providers.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored.values().filter(|p| p.is_default()).count(), 1);
    }

    #[tokio::test]
    async fn unknown_tenant_is_rejected() {
        let ctx = setup();
        let err = ctx
            .use_case
            .execute(cmd(Uuid::now_v7(), false))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::TenantNotFound(_)));
    }
}
