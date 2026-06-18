#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::finish_webauthn_enrollment::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                mfa_challenge::{MfaChallenge, MfaChallengePurpose},
                mfa_enrollment::MfaEnrollment,
            },
            errors::DomainError,
        },
        testing::{
            fake_webauthn_service::{FAKE_CREDENTIAL_ID, FakeWebauthnService},
            in_memory_repositories::{
                in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
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
        challenges: Arc<InMemoryMfaChallengeRepository>,
        tenant_id: Uuid,
        user_id: Uuid,
    }

    fn setup() -> Ctx {
        let users = Arc::new(InMemoryUserRepository::new());
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
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        }
    }

    fn seed_challenge(ctx: &Ctx, purpose: MfaChallengePurpose) -> Uuid {
        let challenge = MfaChallenge::new_webauthn(
            ctx.tenant_id,
            ctx.user_id,
            None,
            purpose,
            json!({ "ceremony": "registration" }),
            Utc::now() + Duration::minutes(5),
        );
        let id = challenge.id();
        ctx.challenges
            .memory
            .data
            .lock()
            .unwrap()
            .insert(id, challenge);
        id
    }

    fn command(ctx: &Ctx, mfa_id: Uuid, name: Option<&str>) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
            mfa_id,
            name: name.map(str::to_string),
            attestation: FakeWebauthnService::valid_client_payload(),
        }
    }

    #[tokio::test]
    async fn registers_a_confirmed_passkey_and_consumes_the_challenge() {
        let ctx = setup();
        let mfa_id = seed_challenge(&ctx, MfaChallengePurpose::Enroll);

        let response = ctx
            .use_case
            .execute(command(&ctx, mfa_id, Some("  YubiKey 5C  ")))
            .await
            .unwrap();

        assert_eq!(response.method, "webauthn");
        assert_eq!(response.name.as_deref(), Some("YubiKey 5C"));

        let enrollment = ctx
            .enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .get(&response.id)
            .cloned()
            .unwrap();
        assert!(enrollment.is_confirmed());
        assert_eq!(enrollment.credential_id(), Some(FAKE_CREDENTIAL_ID));

        let challenge = ctx
            .challenges
            .memory
            .data
            .lock()
            .unwrap()
            .get(&mfa_id)
            .cloned()
            .unwrap();
        assert!(challenge.is_consumed());
    }

    #[tokio::test]
    async fn rejects_a_login_purpose_challenge() {
        let ctx = setup();
        let mfa_id = seed_challenge(&ctx, MfaChallengePurpose::Login);

        let err = ctx
            .use_case
            .execute(command(&ctx, mfa_id, None))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn rejects_a_consumed_or_expired_challenge() {
        let ctx = setup();
        let mfa_id = seed_challenge(&ctx, MfaChallengePurpose::Enroll);
        ctx.use_case
            .execute(command(&ctx, mfa_id, None))
            .await
            .unwrap();

        // Second redemption of the same challenge must fail.
        let err = ctx
            .use_case
            .execute(command(&ctx, mfa_id, None))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn rejects_another_users_challenge() {
        let ctx = setup();
        let challenge = MfaChallenge::new_webauthn(
            ctx.tenant_id,
            Uuid::now_v7(),
            None,
            MfaChallengePurpose::Enroll,
            json!({ "ceremony": "registration" }),
            Utc::now() + Duration::minutes(5),
        );
        let mfa_id = challenge.id();
        ctx.challenges
            .memory
            .data
            .lock()
            .unwrap()
            .insert(mfa_id, challenge);

        let err = ctx
            .use_case
            .execute(command(&ctx, mfa_id, None))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn failed_attestation_counts_an_attempt() {
        let ctx = setup();
        let mfa_id = seed_challenge(&ctx, MfaChallengePurpose::Enroll);

        let mut bad = command(&ctx, mfa_id, None);
        bad.attestation = json!({ "fake": "garbage" });
        let err = ctx.use_case.execute(bad).await.unwrap_err();
        assert!(matches!(err, DomainError::MfaError(_)));

        let challenge = ctx
            .challenges
            .memory
            .data
            .lock()
            .unwrap()
            .get(&mfa_id)
            .cloned()
            .unwrap();
        assert_eq!(challenge.attempts(), 1);
        assert!(!challenge.is_consumed());
    }

    #[tokio::test]
    async fn rejects_a_credential_already_registered_elsewhere() {
        let ctx = setup();
        let existing = MfaEnrollment::new_webauthn(
            ctx.tenant_id,
            Uuid::now_v7(),
            json!({ "fake_passkey": true }),
            FAKE_CREDENTIAL_ID.to_string(),
            None,
        );
        ctx.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let mfa_id = seed_challenge(&ctx, MfaChallengePurpose::Enroll);
        let err = ctx
            .use_case
            .execute(command(&ctx, mfa_id, None))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::MfaEnrollmentConflict(_)));
    }

    #[tokio::test]
    async fn rejects_an_oversized_name() {
        let ctx = setup();
        let mfa_id = seed_challenge(&ctx, MfaChallengePurpose::Enroll);
        let long_name = "x".repeat(101);

        let err = ctx
            .use_case
            .execute(command(&ctx, mfa_id, Some(&long_name)))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidRequest(_)));
    }
}
