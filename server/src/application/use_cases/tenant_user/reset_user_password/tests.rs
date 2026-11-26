#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::{
            shared::{actor::Actor, token_policy::TEMPORARY_PASSWORD_TTL_HOURS},
            use_cases::tenant_user::reset_user_password::{command::Command, use_case::UseCase},
        },
        domain::{
            entities::{
                audit_action::USER_PASSWORD_ADMIN_RESET,
                audit_event::ActorType,
                auth_session::AuthSession,
                email_provider::{EmailProvider, SmtpEncryption},
                password_change_token::PasswordChangeToken,
                password_reset_token::PasswordResetToken,
                refresh_token::RefreshToken,
                tenant::Tenant,
                user::User,
                user_credentials::UserCredentials,
            },
            errors::DomainError,
        },
        testing::{
            fake_email_renderer::var_from_text,
            in_memory_repositories::{
                in_memory_audit_event_repository::InMemoryAuditEventRepository,
                in_memory_auth_session_repository::InMemoryAuthSessionRepository,
                in_memory_email_provider_repository::InMemoryEmailProviderRepository,
                in_memory_password_change_token_repository::InMemoryPasswordChangeTokenRepository,
                in_memory_password_reset_token_repository::InMemoryPasswordResetTokenRepository,
                in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            recording_email_sender::RecordingEmailSender,
            templated_email_support::recording_templated_email_sender,
        },
        utils::{
            oauth_crypto::sha256_hex,
            password_hasher::{compare_password, hash_password},
        },
    };

    const SECRET: &str = "tenant_secret";
    const EMAIL: &str = "user@example.com";
    const OLD_PASSWORD: &str = "old-password-123";

    struct Ctx {
        use_case: UseCase,
        tenant_id: Uuid,
        user_id: Uuid,
        users: Arc<InMemoryUserRepository>,
        sessions: Arc<InMemoryAuthSessionRepository>,
        refresh: Arc<InMemoryRefreshTokenRepository>,
        change_tokens: Arc<InMemoryPasswordChangeTokenRepository>,
        reset_tokens: Arc<InMemoryPasswordResetTokenRepository>,
        audit: Arc<InMemoryAuditEventRepository>,
        email: Arc<RecordingEmailSender>,
        refresh_id: Uuid,
        session_id: Uuid,
        change_token_id: Uuid,
        reset_token_id: Uuid,
    }

    struct Options {
        with_provider: bool,
        require_email_provider: bool,
        with_password: bool,
        active: bool,
    }

    impl Default for Options {
        fn default() -> Self {
            Self {
                with_provider: true,
                require_email_provider: true,
                with_password: true,
                active: true,
            }
        }
    }

    fn setup(options: Options) -> Ctx {
        let tenant_id = Uuid::now_v7();
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "Acme".to_string(),
            None,
            SECRET.to_string(),
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

        let user_id = Uuid::now_v7();
        let credentials = options.with_password.then(|| {
            UserCredentials::new(
                user_id,
                hash_password(OLD_PASSWORD, SECRET).unwrap(),
                "argon2".to_string(),
                false,
            )
        });
        let user = User::reconstruct(
            user_id,
            EMAIL.to_string(),
            tenant_id,
            true,
            None,
            credentials,
            options.active,
            Utc::now(),
            None,
        );
        let users = Arc::new(InMemoryUserRepository::new());
        users.memory.data.lock().unwrap().insert(user_id, user);

        let providers = Arc::new(InMemoryEmailProviderRepository::new());
        if options.with_provider {
            let provider = EmailProvider::new(
                tenant_id,
                "Main".to_string(),
                "smtp.example.com".to_string(),
                587,
                "mailer".to_string(),
                "v1:sealed".to_string(),
                "no-reply@example.com".to_string(),
                None,
                SmtpEncryption::StartTls,
                true,
            );
            providers
                .memory
                .data
                .lock()
                .unwrap()
                .insert(provider.id(), provider);
        }

        // What the old password opened: a refresh token, a sign-in in progress, an
        // earlier temporary password's change code and a forgot-password link.
        let refresh = Arc::new(InMemoryRefreshTokenRepository::new());
        let rt = RefreshToken::new(
            tenant_id,
            sha256_hex("rt"),
            Uuid::now_v7(),
            user_id,
            vec![],
            Utc::now() + Duration::days(30),
        );
        let refresh_id = rt.id();
        refresh.memory.data.lock().unwrap().insert(refresh_id, rt);

        let sessions = Arc::new(InMemoryAuthSessionRepository::new());
        let mut session = AuthSession::new(
            tenant_id,
            sha256_hex("session"),
            Uuid::now_v7(),
            "https://app.example.com/callback".to_string(),
            vec!["openid".to_string()],
            "state".to_string(),
            "challenge".to_string(),
            "S256".to_string(),
            None,
            Utc::now() + Duration::minutes(30),
            Utc::now() + Duration::hours(12),
        );
        session.authenticate(user_id, Utc::now());
        let session_id = session.id();
        sessions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(session_id, session);

        let change_tokens = Arc::new(InMemoryPasswordChangeTokenRepository::new());
        let change = PasswordChangeToken::new(
            tenant_id,
            user_id,
            sha256_hex("change"),
            Utc::now() + Duration::hours(1),
        );
        let change_token_id = change.id();
        change_tokens
            .memory
            .data
            .lock()
            .unwrap()
            .insert(change_token_id, change);

        let reset_tokens = Arc::new(InMemoryPasswordResetTokenRepository::new());
        let reset = PasswordResetToken::new(
            tenant_id,
            user_id,
            sha256_hex("reset"),
            Utc::now() + Duration::minutes(30),
        );
        let reset_token_id = reset.id();
        reset_tokens
            .memory
            .data
            .lock()
            .unwrap()
            .insert(reset_token_id, reset);

        let audit = Arc::new(InMemoryAuditEventRepository::new());
        let (templated_email, email) = recording_templated_email_sender();

        let factory = InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_email_provider_repository(providers)
            .with_refresh_token_repository(refresh.clone())
            .with_auth_session_repository(sessions.clone())
            .with_password_change_token_repository(change_tokens.clone())
            .with_password_reset_token_repository(reset_tokens.clone())
            .with_audit_event_repository(audit.clone());

        Ctx {
            use_case: UseCase::new(
                Arc::new(factory),
                crate::testing::password_hasher::test_password_hasher(),
                templated_email,
                options.require_email_provider,
            ),
            tenant_id,
            user_id,
            users,
            sessions,
            refresh,
            change_tokens,
            reset_tokens,
            audit,
            email,
            refresh_id,
            session_id,
            change_token_id,
            reset_token_id,
        }
    }

    fn command(ctx: &Ctx) -> Command {
        Command {
            tenant_id: ctx.tenant_id,
            user_id: ctx.user_id,
            ip_address: Some("203.0.113.7".to_string()),
            user_agent: Some("test".to_string()),
        }
    }

    fn admin(ctx: &Ctx) -> Actor {
        Actor::TenantAdmin {
            tenant_id: ctx.tenant_id,
            user_id: Uuid::now_v7(),
        }
    }

    fn stored_credentials(ctx: &Ctx) -> Option<UserCredentials> {
        ctx.users
            .memory
            .data
            .lock()
            .unwrap()
            .get(&ctx.user_id)
            .and_then(|u| u.credentials().cloned())
    }

    #[tokio::test]
    async fn emails_a_single_use_temporary_password_and_closes_everything_the_old_one_opened() {
        let ctx = setup(Options::default());
        let actor = admin(&ctx);

        let response = ctx.use_case.execute(&actor, command(&ctx)).await.unwrap();

        // The password went to the user, by email, and only there.
        let sent = ctx.email.sent.lock().unwrap().clone();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, EMAIL);
        let password = var_from_text(&sent[0].text, "password").expect("password in the email");
        assert_eq!(password.len(), 19);
        let response_json = serde_json::to_string(&response).unwrap();
        assert!(!response_json.contains(&password));
        assert_eq!(response.email, EMAIL);

        // Stored hashed, flagged for a change, expiring in a day; the old one is gone.
        let credentials = stored_credentials(&ctx).unwrap();
        assert!(compare_password(
            credentials.password_hash(),
            &password,
            SECRET
        ));
        assert!(!compare_password(
            credentials.password_hash(),
            OLD_PASSWORD,
            SECRET
        ));
        assert!(credentials.should_change_password());
        let expires_at = credentials.temporary_password_expires_at().unwrap();
        assert_eq!(expires_at, response.temporary_password_expires_at);
        let ttl = expires_at - Utc::now();
        assert!(ttl <= Duration::hours(TEMPORARY_PASSWORD_TTL_HOURS));
        assert!(ttl > Duration::hours(TEMPORARY_PASSWORD_TTL_HOURS) - Duration::minutes(1));

        // Signed out everywhere; nothing else can set a password any more.
        let rt = ctx.refresh.memory.data.lock().unwrap()[&ctx.refresh_id].clone();
        assert!(rt.revoked_at().is_some());
        assert!(
            !ctx.sessions
                .memory
                .data
                .lock()
                .unwrap()
                .contains_key(&ctx.session_id)
        );
        let change = ctx.change_tokens.memory.data.lock().unwrap()[&ctx.change_token_id].clone();
        assert!(!change.is_redeemable(Utc::now()));
        let reset = ctx.reset_tokens.memory.data.lock().unwrap()[&ctx.reset_token_id].clone();
        assert!(!reset.is_redeemable(Utc::now()));

        // Audited as the admin's act, without the password.
        let events = ctx.audit.events.lock().unwrap().clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action(), USER_PASSWORD_ADMIN_RESET);
        assert_eq!(events[0].actor_type(), ActorType::Admin);
        assert_eq!(events[0].actor_id(), actor.id());
        assert!(!format!("{:?}", events[0]).contains(&password));
    }

    #[tokio::test]
    async fn each_reset_makes_a_new_password() {
        let ctx = setup(Options::default());
        let actor = admin(&ctx);
        ctx.use_case.execute(&actor, command(&ctx)).await.unwrap();
        ctx.use_case.execute(&actor, command(&ctx)).await.unwrap();

        let sent = ctx.email.sent.lock().unwrap().clone();
        let first = var_from_text(&sent[0].text, "password").unwrap();
        let second = var_from_text(&sent[1].text, "password").unwrap();
        assert_ne!(first, second);
        let credentials = stored_credentials(&ctx).unwrap();
        assert!(!compare_password(
            credentials.password_hash(),
            &first,
            SECRET
        ));
        assert!(compare_password(
            credentials.password_hash(),
            &second,
            SECRET
        ));
    }

    #[tokio::test]
    async fn refuses_before_changing_anything_when_the_email_cannot_go_out() {
        let ctx = setup(Options {
            with_provider: false,
            ..Options::default()
        });

        let err = ctx
            .use_case
            .execute(&admin(&ctx), command(&ctx))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Validation(_)));
        let credentials = stored_credentials(&ctx).unwrap();
        assert!(compare_password(
            credentials.password_hash(),
            OLD_PASSWORD,
            SECRET
        ));
        assert!(ctx.email.sent.lock().unwrap().is_empty());
        let rt = ctx.refresh.memory.data.lock().unwrap()[&ctx.refresh_id].clone();
        assert!(rt.revoked_at().is_none());
    }

    #[tokio::test]
    async fn development_without_a_provider_still_hands_the_email_to_the_transport() {
        // The development transport prints undelivered mail to the log.
        let ctx = setup(Options {
            with_provider: false,
            require_email_provider: false,
            ..Options::default()
        });
        ctx.use_case
            .execute(&admin(&ctx), command(&ctx))
            .await
            .unwrap();
        assert_eq!(ctx.email.recipients(), vec![EMAIL.to_string()]);
    }

    #[tokio::test]
    async fn an_admin_of_another_tenant_is_refused() {
        let ctx = setup(Options::default());
        let outsider = Actor::TenantAdmin {
            tenant_id: Uuid::now_v7(),
            user_id: Uuid::now_v7(),
        };
        let err = ctx
            .use_case
            .execute(&outsider, command(&ctx))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert!(ctx.email.sent.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_user_of_another_tenant_is_not_found() {
        let ctx = setup(Options::default());
        let other = setup(Options::default());
        let mut cmd = command(&ctx);
        cmd.user_id = other.user_id;
        let err = ctx.use_case.execute(&admin(&ctx), cmd).await.unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn accounts_without_a_password_or_deactivated_are_refused() {
        for options in [
            Options {
                with_password: false,
                ..Options::default()
            },
            Options {
                active: false,
                ..Options::default()
            },
        ] {
            let ctx = setup(options);
            let err = ctx
                .use_case
                .execute(&admin(&ctx), command(&ctx))
                .await
                .unwrap_err();
            assert!(matches!(err, DomainError::Validation(_)));
            assert!(ctx.email.sent.lock().unwrap().is_empty());
        }
    }
}
