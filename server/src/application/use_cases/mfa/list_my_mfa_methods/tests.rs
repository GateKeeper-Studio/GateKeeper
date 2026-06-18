#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::list_my_mfa_methods::{query::Query, use_case::UseCase},
        domain::entities::mfa_enrollment::MfaEnrollment,
        testing::in_memory_repositories::{
            in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
    };

    struct Ctx {
        use_case: UseCase,
        enrollments: Arc<InMemoryMfaEnrollmentRepository>,
        users: Arc<InMemoryUserRepository>,
    }

    fn setup() -> Ctx {
        let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
        let users = Arc::new(InMemoryUserRepository::new());
        Ctx {
            use_case: UseCase::new(enrollments.clone(), users.clone()),
            enrollments,
            users,
        }
    }

    fn seed_totp(ctx: &Ctx, tenant_id: Uuid, user_id: Uuid, confirmed: bool) -> Uuid {
        let mut enrollment = MfaEnrollment::new_totp(tenant_id, user_id, "enc:secret".to_string());
        if confirmed {
            enrollment.confirm(Utc::now());
        }
        let id = enrollment.id();
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(id, enrollment);
        id
    }

    fn seed_webauthn(ctx: &Ctx, tenant_id: Uuid, user_id: Uuid, name: &str) -> Uuid {
        let enrollment = MfaEnrollment::new_webauthn(
            tenant_id,
            user_id,
            json!({ "fake_passkey": true }),
            Uuid::now_v7().to_string(),
            Some(name.to_string()),
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

    #[tokio::test]
    async fn lists_implicit_email_even_without_enrollments() {
        let ctx = setup();
        let response = ctx
            .use_case
            .execute(Query {
                tenant_id: Uuid::now_v7(),
                user_id: Uuid::now_v7(),
            })
            .await
            .unwrap();

        assert_eq!(response.preferred_method, None);
        assert_eq!(response.methods.len(), 1);
        assert_eq!(response.methods[0].method, "email");
        assert_eq!(response.methods[0].id, None);
        assert!(response.methods[0].confirmed);
        assert!(!response.methods[0].is_preferred);
    }

    #[tokio::test]
    async fn lists_enrollments_including_pending_totp() {
        let ctx = setup();
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let totp_id = seed_totp(&ctx, tenant_id, user_id, false);
        let passkey_id = seed_webauthn(&ctx, tenant_id, user_id, "YubiKey");

        let response = ctx
            .use_case
            .execute(Query { tenant_id, user_id })
            .await
            .unwrap();

        assert_eq!(response.methods.len(), 3);
        let totp = response
            .methods
            .iter()
            .find(|m| m.id == Some(totp_id))
            .unwrap();
        assert_eq!(totp.method, "totp");
        assert!(!totp.confirmed);

        let passkey = response
            .methods
            .iter()
            .find(|m| m.id == Some(passkey_id))
            .unwrap();
        assert_eq!(passkey.method, "webauthn");
        assert!(passkey.confirmed);
        assert_eq!(passkey.name.as_deref(), Some("YubiKey"));
    }

    #[tokio::test]
    async fn marks_preferred_method() {
        let ctx = setup();
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let totp_id = seed_totp(&ctx, tenant_id, user_id, true);
        ctx.users
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .insert(user_id, "totp".to_string());

        let response = ctx
            .use_case
            .execute(Query { tenant_id, user_id })
            .await
            .unwrap();

        assert_eq!(response.preferred_method.as_deref(), Some("totp"));
        let totp = response
            .methods
            .iter()
            .find(|m| m.id == Some(totp_id))
            .unwrap();
        assert!(totp.is_preferred);
        let email = response.methods.iter().find(|m| m.id.is_none()).unwrap();
        assert!(!email.is_preferred);
    }

    #[tokio::test]
    async fn pending_enrollment_is_never_preferred() {
        let ctx = setup();
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        let totp_id = seed_totp(&ctx, tenant_id, user_id, false);
        ctx.users
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .insert(user_id, "totp".to_string());

        let response = ctx
            .use_case
            .execute(Query { tenant_id, user_id })
            .await
            .unwrap();

        let totp = response
            .methods
            .iter()
            .find(|m| m.id == Some(totp_id))
            .unwrap();
        assert!(!totp.is_preferred);
    }

    #[tokio::test]
    async fn hides_enrollments_from_other_tenants_and_users() {
        let ctx = setup();
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();
        seed_webauthn(&ctx, Uuid::now_v7(), user_id, "other-tenant");
        seed_webauthn(&ctx, tenant_id, Uuid::now_v7(), "other-user");

        let response = ctx
            .use_case
            .execute(Query { tenant_id, user_id })
            .await
            .unwrap();

        // Only the synthetic email entry remains.
        assert_eq!(response.methods.len(), 1);
        assert_eq!(response.methods[0].method, "email");
    }
}
