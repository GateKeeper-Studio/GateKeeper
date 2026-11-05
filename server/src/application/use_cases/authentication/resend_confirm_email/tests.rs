#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::authentication::resend_confirm_email::{
            command::Command, use_case::UseCase,
        },
        domain::entities::{
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            tenant::Tenant,
            user::User,
        },
        testing::{
            fake_email_renderer::var_from_text,
            in_memory_repositories::{
                in_memory_email_verification_token_repository::InMemoryEmailVerificationTokenRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            recording_email_sender::RecordingEmailSender,
            templated_email_support::recording_templated_email_sender,
        },
        utils::oauth_crypto::sha256_hex,
    };

    const CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";
    const REDIRECT_URI: &str = "https://app.example.com/callback";

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            Uuid::parse_str(CLIENT_ID).unwrap(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec![REDIRECT_URI.to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    fn make_tenant(tenant_id: Uuid, can_self_register: bool) -> Tenant {
        Tenant::reconstruct(
            tenant_id,
            "Test Tenant".to_string(),
            None,
            "secret".to_string(),
            true,
            can_self_register,
            true,
            uuid::Uuid::now_v7(),
            chrono::Utc::now(),
            None,
        )
    }

    fn make_command() -> Command {
        Command {
            client_id: CLIENT_ID.to_string(),
            email: "user@example.com".to_string(),
        }
    }

    struct Ctx {
        use_case: UseCase,
        tokens: Arc<InMemoryEmailVerificationTokenRepository>,
        emails: Arc<RecordingEmailSender>,
        user_id: Uuid,
    }

    fn setup_with_unverified_user(tenant_id: Uuid) -> Ctx {
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(Uuid::parse_str(CLIENT_ID).unwrap(), make_client(tenant_id));

        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, make_tenant(tenant_id, true));

        let users = Arc::new(InMemoryUserRepository::new());
        let user = User::new(tenant_id, "user@example.com".to_string(), None, None);
        let user_id = user.id();
        users.memory.data.lock().unwrap().insert(user_id, user);

        let tokens = Arc::new(InMemoryEmailVerificationTokenRepository::new());
        let (templated_email, emails) = recording_templated_email_sender();

        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_oauth_client_repository(clients)
            .with_email_verification_token_repository(tokens.clone());

        let use_case = UseCase::new(Arc::new(factory), templated_email);

        Ctx {
            use_case,
            tokens,
            emails,
            user_id,
        }
    }

    #[tokio::test]
    async fn sends_fresh_code_to_unverified_user() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup_with_unverified_user(tenant_id);

        ctx.use_case.execute(make_command()).await.unwrap();

        let sent = ctx.emails.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "user@example.com");
        let code = var_from_text(&sent[0].text, "code").unwrap();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()));

        let tokens = ctx.tokens.memory.data.lock().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens.values().next().unwrap().token_hash(),
            sha256_hex(&code)
        );
    }

    #[tokio::test]
    async fn consumes_old_code_before_issuing_new_one() {
        use crate::domain::entities::email_verification_token::EmailVerificationToken;
        use chrono::{Duration, Utc};

        let tenant_id = Uuid::now_v7();
        let ctx = setup_with_unverified_user(tenant_id);

        let old_token = EmailVerificationToken::new(
            tenant_id,
            ctx.user_id,
            sha256_hex("999999"),
            Utc::now() + Duration::minutes(15),
        );
        let old_id = old_token.id();
        ctx.tokens
            .memory
            .data
            .lock()
            .unwrap()
            .insert(old_id, old_token);

        ctx.use_case.execute(make_command()).await.unwrap();

        let tokens = ctx.tokens.memory.data.lock().unwrap();
        assert!(tokens.get(&old_id).unwrap().is_consumed());
        assert_eq!(tokens.len(), 2);
    }

    #[tokio::test]
    async fn does_nothing_for_unknown_email() {
        let tenant_id = Uuid::now_v7();
        let ctx = setup_with_unverified_user(tenant_id);

        let mut cmd = make_command();
        cmd.email = "nobody@example.com".to_string();

        ctx.use_case.execute(cmd).await.unwrap();

        assert_eq!(ctx.emails.sent.lock().unwrap().len(), 0);
    }
}
