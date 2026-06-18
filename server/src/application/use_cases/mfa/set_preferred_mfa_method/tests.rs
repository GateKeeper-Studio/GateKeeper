#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::set_preferred_mfa_method::{
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
        },
    };

    struct Ctx {
        use_case: UseCase,
        enrollments: Arc<InMemoryMfaEnrollmentRepository>,
        users: Arc<InMemoryUserRepository>,
        tenant_id: Uuid,
        user_id: Uuid,
    }

    fn setup() -> Ctx {
        let users = Arc::new(InMemoryUserRepository::new());
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_mfa_enrollment_repository(enrollments.clone());
        Ctx {
            use_case: UseCase::new(Arc::new(factory)),
            enrollments,
            users,
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        }
    }

    fn command(ctx: &Ctx, method: Option<&str>) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
            method: method.map(str::to_string),
        }
    }

    fn preferred(ctx: &Ctx) -> Option<String> {
        ctx.users
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .get(&ctx.user_id)
            .cloned()
    }

    #[tokio::test]
    async fn email_is_always_settable() {
        let ctx = setup();
        ctx.use_case
            .execute(command(&ctx, Some("email")))
            .await
            .unwrap();
        assert_eq!(preferred(&ctx).as_deref(), Some("email"));
    }

    #[tokio::test]
    async fn none_clears_the_preference() {
        let ctx = setup();
        ctx.users
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .insert(ctx.user_id, "email".to_string());

        ctx.use_case.execute(command(&ctx, None)).await.unwrap();
        assert_eq!(preferred(&ctx), None);
    }

    #[tokio::test]
    async fn rejects_unknown_method() {
        let ctx = setup();
        let err = ctx
            .use_case
            .execute(command(&ctx, Some("sms")))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn rejects_totp_without_confirmed_enrollment() {
        let ctx = setup();
        let err = ctx
            .use_case
            .execute(command(&ctx, Some("totp")))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
        assert_eq!(preferred(&ctx), None);
    }

    #[tokio::test]
    async fn rejects_totp_with_only_pending_enrollment() {
        let ctx = setup();
        let enrollment =
            MfaEnrollment::new_totp(ctx.tenant_id, ctx.user_id, "enc:secret".to_string());
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);

        let err = ctx
            .use_case
            .execute(command(&ctx, Some("totp")))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn accepts_totp_with_confirmed_enrollment() {
        let ctx = setup();
        let mut enrollment =
            MfaEnrollment::new_totp(ctx.tenant_id, ctx.user_id, "enc:secret".to_string());
        enrollment.confirm(Utc::now());
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);

        ctx.use_case
            .execute(command(&ctx, Some("totp")))
            .await
            .unwrap();
        assert_eq!(preferred(&ctx).as_deref(), Some("totp"));
    }

    #[tokio::test]
    async fn rejects_webauthn_enrolled_under_another_tenant() {
        let ctx = setup();
        let enrollment = MfaEnrollment::new_webauthn(
            Uuid::now_v7(),
            ctx.user_id,
            json!({ "fake_passkey": true }),
            "cred-1".to_string(),
            None,
        );
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);

        let err = ctx
            .use_case
            .execute(command(&ctx, Some("webauthn")))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }
}
