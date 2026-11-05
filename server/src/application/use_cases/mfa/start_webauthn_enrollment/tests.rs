#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::start_webauthn_enrollment::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                mfa_challenge::MfaChallengePurpose, mfa_enrollment::MfaEnrollment,
                mfa_method::MfaMethod,
            },
            errors::DomainError,
        },
        testing::{
            fake_webauthn_service::FakeWebauthnService,
            in_memory_repositories::{
                in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
                in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            user_factory::generate_user,
        },
    };

    struct Ctx {
        use_case: UseCase,
        enrollments: Arc<InMemoryMfaEnrollmentRepository>,
        challenges: Arc<InMemoryMfaChallengeRepository>,
        tenant_id: Uuid,
        user_id: Uuid,
    }

    fn setup() -> Ctx {
        let tenant_id = Uuid::now_v7();
        let users = Arc::new(InMemoryUserRepository::new());
        let user = generate_user(tenant_id);
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user_id, user);

        let tenants = Arc::new(InMemoryTenantRepository::new());
        let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
        let challenges = Arc::new(InMemoryMfaChallengeRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_mfa_enrollment_repository(enrollments.clone())
            .with_mfa_challenge_repository(challenges.clone());

        Ctx {
            use_case: UseCase::new(Arc::new(factory), Arc::new(FakeWebauthnService::new())),
            enrollments,
            challenges,
            tenant_id,
            user_id,
        }
    }

    fn command(ctx: &Ctx) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
        }
    }

    #[tokio::test]
    async fn starts_a_ceremony_and_stores_an_enroll_challenge() {
        let ctx = setup();
        let response = ctx.use_case.execute(command(&ctx)).await.unwrap();

        assert!(response.options.get("publicKey").is_some());

        let challenge = ctx
            .challenges
            .memory
            .data
            .lock()
            .unwrap()
            .get(&Uuid::parse_str(&response.mfa_id).unwrap())
            .cloned()
            .unwrap();
        assert_eq!(*challenge.purpose(), MfaChallengePurpose::Enroll);
        assert_eq!(*challenge.method(), MfaMethod::Webauthn);
        assert_eq!(challenge.auth_session_id(), None);
        assert_eq!(challenge.user_id(), ctx.user_id);
        // Ceremony state is persisted server-side, never returned.
        assert!(challenge.state_json().is_some());
    }

    #[tokio::test]
    async fn excludes_already_registered_credentials() {
        let ctx = setup();
        let enrollment = MfaEnrollment::new_webauthn(
            ctx.tenant_id,
            ctx.user_id,
            json!({ "fake_passkey": true }),
            "existing-cred".to_string(),
            None,
        );
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);

        let response = ctx.use_case.execute(command(&ctx)).await.unwrap();

        // FakeWebauthnService echoes the exclusion list into the stored state.
        let challenge = ctx
            .challenges
            .memory
            .data
            .lock()
            .unwrap()
            .get(&Uuid::parse_str(&response.mfa_id).unwrap())
            .cloned()
            .unwrap();
        let exclude = challenge.state_json().unwrap()["exclude"].clone();
        assert_eq!(exclude, json!(["existing-cred"]));
    }

    #[tokio::test]
    async fn replaces_a_stale_enrollment_ceremony() {
        let ctx = setup();
        let first = ctx.use_case.execute(command(&ctx)).await.unwrap();
        let second = ctx.use_case.execute(command(&ctx)).await.unwrap();
        assert_ne!(first.mfa_id, second.mfa_id);

        let now = Utc::now();
        let data = ctx.challenges.memory.data.lock().unwrap();
        let first_challenge = data.get(&Uuid::parse_str(&first.mfa_id).unwrap()).unwrap();
        let second_challenge = data.get(&Uuid::parse_str(&second.mfa_id).unwrap()).unwrap();
        assert!(!first_challenge.is_redeemable(now));
        assert!(second_challenge.is_redeemable(now));
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
