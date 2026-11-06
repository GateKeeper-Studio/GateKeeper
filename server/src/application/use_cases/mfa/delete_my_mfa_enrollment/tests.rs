#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;
    use uuid::Uuid;

    use crate::{
        application::use_cases::mfa::delete_my_mfa_enrollment::{
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

    fn seed_webauthn(ctx: &Ctx, user_id: Uuid, credential_id: &str) -> Uuid {
        let enrollment = MfaEnrollment::new_webauthn(
            ctx.tenant_id,
            user_id,
            json!({ "fake_passkey": true }),
            credential_id.to_string(),
            None,
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

    fn set_preferred(ctx: &Ctx, method: &str) {
        ctx.users
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .insert(ctx.user_id, method.to_string());
    }

    fn preferred(ctx: &Ctx) -> Option<String> {
        ctx.users
            .preferred_2fa_methods
            .lock()
            .unwrap()
            .get(&ctx.user_id)
            .cloned()
    }

    fn command(ctx: &Ctx, enrollment_id: Uuid) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
            enrollment_id,
        }
    }

    #[tokio::test]
    async fn deletes_an_owned_enrollment() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, ctx.user_id, "cred-1");

        ctx.use_case.execute(command(&ctx, id)).await.unwrap();
        assert!(
            !ctx.enrollments
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&id)
        );
    }

    #[tokio::test]
    async fn clears_preference_when_last_enrollment_of_method_is_deleted() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, ctx.user_id, "cred-1");
        set_preferred(&ctx, "webauthn");

        ctx.use_case.execute(command(&ctx, id)).await.unwrap();
        assert_eq!(preferred(&ctx), None);
    }

    #[tokio::test]
    async fn keeps_preference_while_a_sibling_enrollment_remains() {
        let ctx = setup();
        let first = seed_webauthn(&ctx, ctx.user_id, "cred-1");
        seed_webauthn(&ctx, ctx.user_id, "cred-2");
        set_preferred(&ctx, "webauthn");

        ctx.use_case.execute(command(&ctx, first)).await.unwrap();
        assert_eq!(preferred(&ctx).as_deref(), Some("webauthn"));
    }

    #[tokio::test]
    async fn keeps_preference_of_an_unrelated_method() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, ctx.user_id, "cred-1");
        set_preferred(&ctx, "email");

        ctx.use_case.execute(command(&ctx, id)).await.unwrap();
        assert_eq!(preferred(&ctx).as_deref(), Some("email"));
    }

    #[tokio::test]
    async fn not_found_for_another_users_enrollment() {
        let ctx = setup();
        let id = seed_webauthn(&ctx, Uuid::now_v7(), "cred-1");

        let err = ctx.use_case.execute(command(&ctx, id)).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
        assert!(
            ctx.enrollments
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&id)
        );
    }

    #[tokio::test]
    async fn not_found_for_a_missing_enrollment() {
        let ctx = setup();
        let err = ctx
            .use_case
            .execute(command(&ctx, Uuid::now_v7()))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    // ── Last-passkey guard (passwordless accounts) ───────────────────────────────
    // For an account with no password the passkey is the primary factor; removing
    // the last one must not leave the user with no way back in.
    mod last_passkey_guard {
        use super::*;

        use crate::{
            domain::entities::{
                external_identity_provider::ExternalProviderKind, social_identity::SocialIdentity,
                user::User,
            },
            testing::{
                in_memory_repositories::in_memory_social_identity_repository::InMemorySocialIdentityRepository,
                user_factory::generate_user,
            },
        };

        struct GuardCtx {
            ctx: Ctx,
            socials: Arc<InMemorySocialIdentityRepository>,
        }

        // `user` decides whether the account has a password.
        fn setup_with_user(user: User) -> GuardCtx {
            let tenant_id = user.tenant_id();
            let user_id = user.id();
            let users = Arc::new(InMemoryUserRepository::new());
            users.memory.data.lock().unwrap().insert(user_id, user);
            let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
            let socials = Arc::new(InMemorySocialIdentityRepository::new());
            let factory = InMemoryUnitOfWorkFactory::new(
                users.clone(),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_mfa_enrollment_repository(enrollments.clone())
            .with_social_identity_repository(socials.clone());
            GuardCtx {
                ctx: Ctx {
                    use_case: UseCase::new(Arc::new(factory)),
                    enrollments,
                    users,
                    tenant_id,
                    user_id,
                },
                socials,
            }
        }

        fn passwordless() -> GuardCtx {
            let tenant_id = Uuid::now_v7();
            setup_with_user(User::new(
                tenant_id,
                "passkey.user@example.com".to_string(),
                None,
                None,
            ))
        }

        fn still_there(ctx: &Ctx, id: Uuid) -> bool {
            ctx.enrollments
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&id)
        }

        #[tokio::test]
        async fn refuses_the_last_passkey_of_a_passwordless_account() {
            let g = passwordless();
            let id = seed_webauthn(&g.ctx, g.ctx.user_id, "cred-1");

            let err = g
                .ctx
                .use_case
                .execute(command(&g.ctx, id))
                .await
                .unwrap_err();

            assert!(matches!(err, DomainError::Conflict(_)));
            assert!(still_there(&g.ctx, id));
        }

        #[tokio::test]
        async fn allows_it_while_another_passkey_remains() {
            let g = passwordless();
            let first = seed_webauthn(&g.ctx, g.ctx.user_id, "cred-1");
            let second = seed_webauthn(&g.ctx, g.ctx.user_id, "cred-2");

            g.ctx
                .use_case
                .execute(command(&g.ctx, first))
                .await
                .unwrap();
            assert!(!still_there(&g.ctx, first));

            // ...but not the one that is now the last.
            let err = g
                .ctx
                .use_case
                .execute(command(&g.ctx, second))
                .await
                .unwrap_err();
            assert!(matches!(err, DomainError::Conflict(_)));
        }

        #[tokio::test]
        async fn allows_it_when_a_social_identity_is_linked() {
            let g = passwordless();
            let id = seed_webauthn(&g.ctx, g.ctx.user_id, "cred-1");
            let identity = SocialIdentity::new(
                g.ctx.user_id,
                g.ctx.tenant_id,
                ExternalProviderKind::Google,
                "google-subject".to_string(),
                None,
            );
            g.socials
                .memory
                .data
                .lock()
                .unwrap()
                .insert(identity.id(), identity);

            g.ctx.use_case.execute(command(&g.ctx, id)).await.unwrap();
            assert!(!still_there(&g.ctx, id));
        }

        #[tokio::test]
        async fn allows_it_for_an_account_with_a_password() {
            let g = setup_with_user(generate_user(Uuid::now_v7()));
            let id = seed_webauthn(&g.ctx, g.ctx.user_id, "cred-1");

            g.ctx.use_case.execute(command(&g.ctx, id)).await.unwrap();
            assert!(!still_there(&g.ctx, id));
        }

        #[tokio::test]
        async fn never_blocks_removing_totp() {
            let g = passwordless();
            seed_webauthn(&g.ctx, g.ctx.user_id, "cred-1");
            let mut totp =
                MfaEnrollment::new_totp(g.ctx.tenant_id, g.ctx.user_id, "enc".to_string());
            totp.confirm(chrono::Utc::now());
            let totp_id = totp.id();
            g.ctx
                .enrollments
                .memory
                .data
                .lock()
                .unwrap()
                .insert(totp_id, totp);

            g.ctx
                .use_case
                .execute(command(&g.ctx, totp_id))
                .await
                .unwrap();
            assert!(!still_there(&g.ctx, totp_id));
        }
    }
}
