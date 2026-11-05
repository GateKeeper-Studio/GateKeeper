#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::start_totp_enrollment::{command::Command, use_case::UseCase},
        domain::{
            entities::{mfa_enrollment::MfaEnrollment, tenant::Tenant},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
            user_factory::generate_user,
        },
    };

    struct Ctx {
        use_case: UseCase,
        enrollments: Arc<InMemoryMfaEnrollmentRepository>,
        tenant_id: Uuid,
        user_id: Uuid,
        email: String,
    }

    fn setup() -> Ctx {
        let tenant_id = Uuid::now_v7();
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "Test Tenant".to_string(),
            None,
            "secret".to_string(),
            true,
            true,
            true,
            Uuid::now_v7(),
            Utc::now(),
            None,
        );
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, tenant);

        let users = Arc::new(InMemoryUserRepository::new());
        let user = generate_user(tenant_id);
        let user_id = user.id();
        let email = user.email().to_string();
        users.memory.data.lock().unwrap().insert(user_id, user);

        let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_mfa_enrollment_repository(enrollments.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory), Arc::new(StubSecretCipher)),
            enrollments,
            tenant_id,
            user_id,
            email,
        }
    }

    fn command(ctx: &Ctx) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
        }
    }

    #[tokio::test]
    async fn creates_pending_enrollment_with_encrypted_secret() {
        let ctx = setup();
        let response = ctx.use_case.execute(command(&ctx)).await.unwrap();

        assert!(!response.secret_base32.is_empty());
        assert!(response.otpauth_url.starts_with("otpauth://totp/"));
        assert!(response.otpauth_url.contains(&response.secret_base32));
        assert!(response.otpauth_url.contains(&ctx.email));

        let stored = ctx
            .enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .get(&response.enrollment_id)
            .cloned()
            .unwrap();
        assert!(!stored.is_confirmed());
        // Stored encrypted, never the raw base32 secret.
        assert_eq!(
            stored.secret_enc(),
            Some(format!("enc:{}", response.secret_base32).as_str())
        );
    }

    #[tokio::test]
    async fn replaces_an_abandoned_pending_enrollment() {
        let ctx = setup();
        let first = ctx.use_case.execute(command(&ctx)).await.unwrap();
        let second = ctx.use_case.execute(command(&ctx)).await.unwrap();

        assert_ne!(first.enrollment_id, second.enrollment_id);
        assert_ne!(first.secret_base32, second.secret_base32);
        let data = ctx.enrollments.memory.data.lock().unwrap();
        assert!(!data.contains_key(&first.enrollment_id));
        assert!(data.contains_key(&second.enrollment_id));
    }

    #[tokio::test]
    async fn conflicts_when_a_confirmed_authenticator_exists() {
        let ctx = setup();
        let mut existing =
            MfaEnrollment::new_totp(ctx.tenant_id, ctx.user_id, "enc:old".to_string());
        existing.confirm(Utc::now());
        let existing_id = existing.id();
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing_id, existing);

        let err = ctx.use_case.execute(command(&ctx)).await.unwrap_err();
        assert!(matches!(err, DomainError::Conflict(_)));
        // The working factor is untouched.
        assert!(
            ctx.enrollments
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&existing_id)
        );
    }

    #[tokio::test]
    async fn rejects_a_user_from_another_tenant() {
        let ctx = setup();
        let err = ctx
            .use_case
            .execute(Command {
                tenant_id: Uuid::now_v7(),
                user_id: ctx.user_id,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
