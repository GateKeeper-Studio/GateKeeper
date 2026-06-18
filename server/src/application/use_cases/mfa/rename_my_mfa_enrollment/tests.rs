#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::rename_my_mfa_enrollment::{
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
            use_case: UseCase::new(Arc::new(factory)),
            enrollments,
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        }
    }

    fn seed_webauthn(ctx: &Ctx, user_id: Uuid, name: Option<&str>) -> Uuid {
        let enrollment = MfaEnrollment::new_webauthn(
            ctx.tenant_id,
            user_id,
            json!({ "fake_passkey": true }),
            Uuid::now_v7().to_string(),
            name.map(str::to_string),
        );
        let id = enrollment.id();
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(id, enrollment);
        id
    }

    fn command(ctx: &Ctx, enrollment_id: Uuid, name: Option<&str>) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
            enrollment_id,
            name: name.map(str::to_string),
        }
    }

    fn stored_name(ctx: &Ctx, id: Uuid) -> Option<String> {
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .get(&id)
            .and_then(|e| e.name().map(str::to_string))
    }

    #[tokio::test]
    async fn renames_an_owned_enrollment() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, ctx.user_id, Some("old"));

        let response = ctx
            .use_case
            .execute(command(&ctx, id, Some("  Work laptop  ")))
            .await
            .unwrap();
        assert_eq!(response.name.as_deref(), Some("Work laptop"));
        assert_eq!(stored_name(&ctx, id).as_deref(), Some("Work laptop"));
    }

    #[tokio::test]
    async fn empty_name_clears_the_label() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, ctx.user_id, Some("old"));

        let response = ctx
            .use_case
            .execute(command(&ctx, id, Some("   ")))
            .await
            .unwrap();
        assert_eq!(response.name, None);
        assert_eq!(stored_name(&ctx, id), None);
    }

    #[tokio::test]
    async fn rejects_an_oversized_name() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, ctx.user_id, None);
        let long_name = "x".repeat(101);

        let err = ctx
            .use_case
            .execute(command(&ctx, id, Some(&long_name)))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn not_found_for_another_users_enrollment() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, Uuid::now_v7(), Some("theirs"));

        let err = ctx
            .use_case
            .execute(command(&ctx, id, Some("mine")))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::MfaEnrollmentNotFound(_)));
        assert_eq!(stored_name(&ctx, id).as_deref(), Some("theirs"));
    }
}
