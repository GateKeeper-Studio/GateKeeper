#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::authentication::forgot_password::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
                user::User,
            },
            errors::DomainError,
        },
        testing::{
            fake_email_renderer::var_from_text,
            in_memory_repositories::{
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_password_reset_token_repository::InMemoryPasswordResetTokenRepository,
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
    const AUTH_UI: &str = "http://localhost:5173";
    const EMAIL: &str = "user@example.com";

    struct Ctx {
        use_case: UseCase,
        tokens: Arc<InMemoryPasswordResetTokenRepository>,
        email: Arc<RecordingEmailSender>,
        user_id: Uuid,
    }

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::new(
            tenant_id,
            Uuid::parse_str(CLIENT_ID).unwrap(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec![REDIRECT_URI.to_string()],
            vec![],
            vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            vec!["authorization_code".to_string()],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        )
    }

    // `can_forgot` gates the tenant feature flag; `with_user` seeds an active user.
    fn setup(can_forgot: bool, with_user: bool) -> Ctx {
        setup_ext(can_forgot, with_user, false)
    }

    fn setup_ext(can_forgot: bool, with_user: bool, external_login_only: bool) -> Ctx {
        let tenant_id = Uuid::now_v7();

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let mut client = make_client(tenant_id);
        client.set_external_login_only(external_login_only);
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client);

        let tenants = Arc::new(InMemoryTenantRepository::new());
        let tenant = Tenant::reconstruct(
            tenant_id,
            "Test Tenant".to_string(),
            None,
            "tenant_secret".to_string(),
            true,
            true,
            can_forgot,
            uuid::Uuid::now_v7(),
            Utc::now(),
            None,
        );
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant);

        let users = Arc::new(InMemoryUserRepository::new());
        let mut user_id = Uuid::now_v7();
        if with_user {
            let user = User::new(tenant_id, EMAIL.to_string(), None, None);
            user_id = user.id();
            users.memory.data.lock().unwrap().insert(user.id(), user);
        }

        let tokens = Arc::new(InMemoryPasswordResetTokenRepository::new());
        let (templated_email, email) = recording_templated_email_sender();

        let factory = InMemoryUnitOfWorkFactory::new(users, tenants)
            .with_oauth_client_repository(clients)
            .with_password_reset_token_repository(tokens.clone());

        let use_case = UseCase::new(Arc::new(factory), templated_email, AUTH_UI.to_string());

        Ctx {
            use_case,
            tokens,
            email,
            user_id,
        }
    }

    fn command() -> Command {
        Command {
            client_id: CLIENT_ID.to_string(),
            email: EMAIL.to_string(),
        }
    }

    #[tokio::test]
    async fn issues_token_and_emails_reset_link() {
        let ctx = setup(true, true);

        ctx.use_case.execute(command()).await.unwrap();

        // Exactly one token persisted, for this user, still active.
        let tokens = ctx.tokens.memory.data.lock().unwrap();
        assert_eq!(tokens.len(), 1);
        let token = tokens.values().next().unwrap();
        assert_eq!(token.user_id(), ctx.user_id);
        assert!(token.is_redeemable(Utc::now()));

        // One email sent to the user with a link carrying the raw token + id, and the
        // persisted hash matches that raw token.
        let sent = ctx.email.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, EMAIL);
        let link = var_from_text(&sent[0].text, "link").unwrap();
        assert!(link.starts_with(&format!(
            "{AUTH_UI}/auth/{CLIENT_ID}/change-password?token="
        )));
        assert!(link.contains(&format!("&id={}", token.id())));

        let raw = extract_query(&link, "token");
        assert_eq!(sha256_hex(&raw), token.token_hash());
    }

    // There is no password to reset on an external-login-only client. Unlike the
    // silent-Ok cases below this one errors: the flag is part of the client's PUBLIC
    // auth data, so refusing outright leaks nothing the caller couldn't already read.
    #[tokio::test]
    async fn refuses_for_an_external_login_only_client() {
        let ctx = setup_ext(true, true, true);

        let err = ctx.use_case.execute(command()).await.unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)));
        assert_eq!(ctx.tokens.memory.data.lock().unwrap().len(), 0);
        assert_eq!(ctx.email.sent.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn unknown_email_is_silent_no_enumeration() {
        let ctx = setup(true, false);

        ctx.use_case.execute(command()).await.unwrap();

        assert_eq!(ctx.tokens.memory.data.lock().unwrap().len(), 0);
        assert_eq!(ctx.email.sent.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn disabled_feature_is_silent() {
        let ctx = setup(false, true);

        ctx.use_case.execute(command()).await.unwrap();

        assert_eq!(ctx.tokens.memory.data.lock().unwrap().len(), 0);
        assert_eq!(ctx.email.sent.lock().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn second_request_invalidates_the_first_link() {
        let ctx = setup(true, true);

        ctx.use_case.execute(command()).await.unwrap();
        ctx.use_case.execute(command()).await.unwrap();

        let tokens = ctx.tokens.memory.data.lock().unwrap();
        // Two rows exist, but only one is still redeemable (single live token).
        assert_eq!(tokens.len(), 2);
        let active = tokens
            .values()
            .filter(|t| t.is_redeemable(Utc::now()))
            .count();
        assert_eq!(active, 1);
    }

    fn extract_query(url: &str, key: &str) -> String {
        let query = url.split_once('?').map(|(_, q)| q).unwrap_or("");
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix(&format!("{key}=")))
            .unwrap_or("")
            .to_string()
    }
}
