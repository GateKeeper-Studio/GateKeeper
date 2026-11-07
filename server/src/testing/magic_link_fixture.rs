// In-memory end-to-end fixture for magic link login: the request use case, its
// background dispatcher (through a `ManualTaskSpawner`), and the redeem use case,
// all on one set of in-memory repositories.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    application::use_cases::authentication::{
        authorize_session, redeem_magic_link, request_magic_link,
        request_magic_link::dispatcher::MagicLinkDispatcher,
    },
    domain::entities::{
        auth_session::AuthSession,
        magic_link_token::MagicLinkToken,
        mfa_enrollment::MfaEnrollment,
        oauth_client::{ClientType, MfaPolicy, OAuthClient},
        tenant::Tenant,
        user::User,
        user_credentials::UserCredentials,
    },
    testing::{
        fake_email_renderer::var_from_text,
        fake_risk_engine::FakeRiskEngine,
        fake_webauthn_service::{FAKE_CREDENTIAL_ID, FakeWebauthnService},
        in_memory_repositories::{
            in_memory_audit_event_repository::InMemoryAuditEventRepository,
            in_memory_auth_session_repository::InMemoryAuthSessionRepository,
            in_memory_magic_link_token_repository::InMemoryMagicLinkTokenRepository,
            in_memory_mfa_challenge_repository::InMemoryMfaChallengeRepository,
            in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
            in_memory_password_change_token_repository::InMemoryPasswordChangeTokenRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        manual_task_spawner::ManualTaskSpawner,
        recording_email_sender::RecordingEmailSender,
        templated_email_support::recording_templated_email_sender,
        user_factory::{TEST_PASSWORD_SECRET, generate_user},
    },
};

pub const PUBLIC_CLIENT_ID: &str = "01890000-0000-7000-8000-000000000abc";
pub const REDIRECT_URI: &str = "https://app.example.com/callback";
pub const AUTH_UI: &str = "http://localhost:5173";
pub const UNKNOWN_EMAIL: &str = "nobody@example.com";

pub struct Options {
    pub magic_link_enabled: bool,
    pub external_login_only: bool,
    pub mfa_policy: MfaPolicy,
    pub user_verified: bool,
    pub user_active: bool,
    pub should_change_password: bool,
    pub risk: FakeRiskEngine,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            magic_link_enabled: true,
            external_login_only: false,
            mfa_policy: MfaPolicy::default(),
            user_verified: true,
            user_active: true,
            should_change_password: false,
            risk: FakeRiskEngine::allow(),
        }
    }
}

pub struct Fixture {
    pub request: request_magic_link::use_case::UseCase,
    pub redeem: redeem_magic_link::use_case::UseCase,
    pub authorize: authorize_session::use_case::UseCase,
    pub spawner: Arc<ManualTaskSpawner>,
    pub emails: Arc<RecordingEmailSender>,
    pub risk: Arc<FakeRiskEngine>,
    pub clients: Arc<InMemoryOAuthClientRepository>,
    pub users: Arc<InMemoryUserRepository>,
    pub sessions: Arc<InMemoryAuthSessionRepository>,
    pub tokens: Arc<InMemoryMagicLinkTokenRepository>,
    pub enrollments: Arc<InMemoryMfaEnrollmentRepository>,
    pub challenges: Arc<InMemoryMfaChallengeRepository>,
    pub change_tokens: Arc<InMemoryPasswordChangeTokenRepository>,
    pub audit: Arc<InMemoryAuditEventRepository>,
    pub client: OAuthClient,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
}

// A link as the user would get it: the token row id and the raw secret.
#[derive(Debug, Clone)]
pub struct Link {
    pub id: Uuid,
    pub token: String,
}

pub fn build(options: Options) -> Fixture {
    let tenant_id = Uuid::now_v7();

    let mut client = OAuthClient::new(
        tenant_id,
        Uuid::parse_str(PUBLIC_CLIENT_ID).unwrap(),
        None,
        ClientType::Public,
        "Test App".to_string(),
        vec![REDIRECT_URI.to_string()],
        vec![],
        vec!["openid".to_string(), "email".to_string()],
        vec!["authorization_code".to_string()],
        true,
        true,
        vec!["en".to_string()],
        "en".to_string(),
        options.mfa_policy,
    );
    client.set_magic_link_enabled(options.magic_link_enabled);
    client.set_external_login_only(options.external_login_only);
    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    clients
        .memory
        .data
        .lock()
        .unwrap()
        .insert(client.id(), client.clone());

    let tenants = Arc::new(InMemoryTenantRepository::new());
    let tenant = Tenant::reconstruct(
        tenant_id,
        "Test Tenant".to_string(),
        None,
        TEST_PASSWORD_SECRET.to_string(),
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
        .insert(tenant.id(), tenant);

    let users = Arc::new(InMemoryUserRepository::new());
    let generated = generate_user(tenant_id);
    let mut user = User::reconstruct(
        generated.id(),
        generated.email().to_string(),
        tenant_id,
        options.user_verified,
        generated.profile().cloned(),
        generated.credentials().cloned(),
        options.user_active,
        generated.created_at(),
        None,
    );
    if options.should_change_password {
        user.update_credentials(UserCredentials::new(
            user.id(),
            "hash".to_string(),
            "argon2id".to_string(),
            true,
        ));
    }
    let user_id = user.id();
    let email = user.email().to_string();
    users.memory.data.lock().unwrap().insert(user_id, user);

    let sessions = Arc::new(InMemoryAuthSessionRepository::new());
    let tokens = Arc::new(InMemoryMagicLinkTokenRepository::new());
    let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
    let challenges = Arc::new(InMemoryMfaChallengeRepository::new());
    let change_tokens = Arc::new(InMemoryPasswordChangeTokenRepository::new());
    let audit = Arc::new(InMemoryAuditEventRepository::new());

    let factory = Arc::new(
        InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients.clone())
            .with_auth_session_repository(sessions.clone())
            .with_magic_link_token_repository(tokens.clone())
            .with_mfa_enrollment_repository(enrollments.clone())
            .with_mfa_challenge_repository(challenges.clone())
            .with_password_change_token_repository(change_tokens.clone())
            .with_audit_event_repository(audit.clone()),
    );

    let (templated, emails) = recording_templated_email_sender();
    let risk = Arc::new(options.risk);
    let spawner = Arc::new(ManualTaskSpawner::new());
    let dispatcher = Arc::new(MagicLinkDispatcher::new(
        factory.clone(),
        templated,
        AUTH_UI.to_string(),
    ));

    Fixture {
        request: request_magic_link::use_case::UseCase::new(
            factory.clone(),
            risk.clone(),
            spawner.clone(),
            dispatcher,
        ),
        redeem: redeem_magic_link::use_case::UseCase::new(
            factory.clone(),
            Arc::new(FakeWebauthnService::new()),
            risk.clone(),
        ),
        authorize: authorize_session::use_case::UseCase::new(factory),
        spawner,
        emails,
        risk,
        clients,
        users,
        sessions,
        tokens,
        enrollments,
        challenges,
        change_tokens,
        audit,
        client,
        tenant_id,
        user_id,
        email,
    }
}

impl Fixture {
    pub fn request_command(&self, email: &str) -> request_magic_link::command::Command {
        request_magic_link::command::Command {
            email: email.to_string(),
            client_id: PUBLIC_CLIENT_ID.to_string(),
            redirect_uri: REDIRECT_URI.to_string(),
            response_type: "code".to_string(),
            scope: "openid email".to_string(),
            state: "xyz".to_string(),
            code_challenge: "challenge".to_string(),
            code_challenge_method: "S256".to_string(),
            nonce: Some("n-1".to_string()),
            ip_address: Some("203.0.113.7".to_string()),
            user_agent: Some("test-agent".to_string()),
        }
    }

    pub fn redeem_command(
        &self,
        link: &Link,
        session_code: &str,
    ) -> redeem_magic_link::command::Command {
        redeem_magic_link::command::Command {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            id: link.id,
            token: link.token.clone(),
            session_code: session_code.to_string(),
            mfa_methods: None,
            mfa_default_method: None,
            ip_address: Some("203.0.113.7".to_string()),
            user_agent: Some("test-agent".to_string()),
        }
    }

    pub fn authorize_command(&self, session_code: &str) -> authorize_session::command::Command {
        authorize_session::command::Command {
            session_code: session_code.to_string(),
            client_id: PUBLIC_CLIENT_ID.to_string(),
            email: self.email.clone(),
        }
    }

    // Request a link for the seeded user, run the background dispatch, and return
    // the session code and the link from the email.
    pub async fn request_link(&self) -> (String, Link) {
        let response = self
            .request
            .execute(self.request_command(&self.email))
            .await
            .expect("request accepted");
        self.spawner.run_all().await;
        let link = self.last_link().expect("a link was emailed");
        (response.session_code, link)
    }

    // The link in the most recent email.
    pub fn last_link(&self) -> Option<Link> {
        let sent = self.emails.sent.lock().unwrap();
        let text = &sent.last()?.text;
        let url = var_from_text(text, "link")?;
        let query = url.split_once('#')?.1;
        let mut id = None;
        let mut token = None;
        for pair in query.split('&') {
            match pair.split_once('=') {
                Some(("id", v)) => id = Uuid::parse_str(v).ok(),
                Some(("token", v)) => token = Some(v.to_string()),
                _ => {}
            }
        }
        Some(Link {
            id: id?,
            token: token?,
        })
    }

    pub fn session(&self, id: Uuid) -> AuthSession {
        self.sessions
            .memory
            .data
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .expect("session exists")
    }

    pub fn sessions_count(&self) -> usize {
        self.sessions.memory.data.lock().unwrap().len()
    }

    pub fn token(&self, id: Uuid) -> MagicLinkToken {
        self.tokens
            .all()
            .into_iter()
            .find(|t| t.id() == id)
            .expect("token exists")
    }

    pub fn audit_actions(&self) -> Vec<(String, Option<Uuid>, serde_json::Value)> {
        self.audit
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|e| (e.action().to_string(), e.actor_id(), e.metadata().clone()))
            .collect()
    }

    pub fn set_user(&self, update: impl FnOnce(&User) -> User) {
        let mut users = self.users.memory.data.lock().unwrap();
        let current = users.get(&self.user_id).cloned().unwrap();
        users.insert(self.user_id, update(&current));
    }

    pub fn set_client(&self, update: impl FnOnce(&mut OAuthClient)) {
        let mut clients = self.clients.memory.data.lock().unwrap();
        let client = clients.get_mut(&self.client.id()).unwrap();
        update(client);
    }

    pub fn seed_totp(&self) -> Uuid {
        let mut enrollment =
            MfaEnrollment::new_totp(self.tenant_id, self.user_id, "encrypted".to_string());
        enrollment.confirm(Utc::now());
        let id = enrollment.id();
        self.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(id, enrollment);
        id
    }

    pub fn seed_webauthn(&self) {
        let enrollment = MfaEnrollment::new_webauthn(
            self.tenant_id,
            self.user_id,
            serde_json::json!({ "fake_passkey": true, "cred_id": FAKE_CREDENTIAL_ID, "counter": 0 }),
            FAKE_CREDENTIAL_ID.to_string(),
            Some("Test key".to_string()),
        );
        self.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);
    }
}

// Rebuild `user` with the given activity/verification flags.
pub fn with_flags(user: &User, active: bool, verified: bool) -> User {
    User::reconstruct(
        user.id(),
        user.email().to_string(),
        user.tenant_id(),
        verified,
        user.profile().cloned(),
        user.credentials().cloned(),
        active,
        user.created_at(),
        user.updated_at(),
    )
}
