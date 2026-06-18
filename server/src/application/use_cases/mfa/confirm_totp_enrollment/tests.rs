#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::confirm_totp_enrollment::{
            command::Command, use_case::UseCase,
        },
        domain::{entities::mfa_enrollment::MfaEnrollment, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
        },
        utils::totp,
    };

    const SECRET_BASE32: &str = "JBSWY3DPEHPK3PXP";

    struct Ctx {
        use_case: UseCase,
        enrollments: Arc<InMemoryMfaEnrollmentRepository>,
        tenant_id: Uuid,
        user_id: Uuid,
    }

    fn setup() -> Ctx {
        let users = Arc::new(InMemoryUserRepository::new());
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_mfa_enrollment_repository(enrollments.clone());
        Ctx {
            use_case: UseCase::new(Arc::new(factory), Arc::new(StubSecretCipher)),
            enrollments,
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        }
    }

    fn seed_pending(ctx: &Ctx) -> Uuid {
        // Stub cipher stores "enc:<plaintext>".
        let enrollment =
            MfaEnrollment::new_totp(ctx.tenant_id, ctx.user_id, format!("enc:{SECRET_BASE32}"));
        let id = enrollment.id();
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(id, enrollment);
        id
    }

    fn current_code() -> String {
        let secret =
            base32::decode(base32::Alphabet::Rfc4648 { padding: false }, SECRET_BASE32).unwrap();
        totp::generate(&secret, totp::step_for(Utc::now().timestamp()))
    }

    fn command(ctx: &Ctx, code: &str) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
            code: code.to_string(),
        }
    }

    #[tokio::test]
    async fn confirms_with_a_valid_code_and_seeds_replay_guard() {
        let ctx = setup();
        let id = seed_pending(&ctx);

        let response = ctx
            .use_case
            .execute(command(&ctx, &current_code()))
            .await
            .unwrap();
        assert_eq!(response.enrollment_id, id);

        let stored = ctx
            .enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .unwrap();
        assert!(stored.is_confirmed());
        // The confirmation step is recorded so the same code can't be replayed
        // at the next login.
        assert!(stored.totp_last_step().is_some());
    }

    #[tokio::test]
    async fn rejects_a_wrong_code() {
        let ctx = setup();
        let id = seed_pending(&ctx);

        let err = ctx
            .use_case
            .execute(command(&ctx, "000000"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::MfaError(_)));

        let stored = ctx
            .enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .unwrap();
        assert!(!stored.is_confirmed());
    }

    #[tokio::test]
    async fn errors_when_no_pending_enrollment_exists() {
        let ctx = setup();
        let err = ctx
            .use_case
            .execute(command(&ctx, "123456"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::MfaEnrollmentNotFound(_)));
    }

    #[tokio::test]
    async fn ignores_an_already_confirmed_enrollment() {
        let ctx = setup();
        let mut enrollment =
            MfaEnrollment::new_totp(ctx.tenant_id, ctx.user_id, format!("enc:{SECRET_BASE32}"));
        enrollment.confirm(Utc::now());
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);

        let err = ctx
            .use_case
            .execute(command(&ctx, &current_code()))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::MfaEnrollmentNotFound(_)));
    }
}
