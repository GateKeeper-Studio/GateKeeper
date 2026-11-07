// In-memory fixture for the session (refresh-token family) use cases: one tenant with
// two users and two apps, a second tenant, rotation chains seeded with real
// timestamps, and the refresh grant wired to the same repositories so a test can
// prove a revoked session can no longer refresh.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::{
    application::{
        shared::backchannel_logout::LogoutNotice,
        use_cases::{
            oauth::{end_session, token_refresh},
            session::{list_user_sessions, revoke_all_user_sessions, revoke_user_session},
        },
    },
    domain::entities::{
        audit_event::AuditEvent,
        domain_event_type::DomainEventType,
        oauth_client::{ClientType, MfaPolicy, OAuthClient},
        refresh_token::RefreshToken,
        signing_key::{SigningAlgorithm, SigningKey},
        tenant::Tenant,
        user::User,
    },
    testing::{
        fake_audit_logger::FakeAuditLogger,
        fake_risk_engine::FakeRiskEngine,
        in_memory_repositories::{
            in_memory_audit_event_repository::InMemoryAuditEventRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
            in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
            in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
            in_memory_signing_key_repository::InMemorySigningKeyRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        oauth_fakes::{FakeTokenSigner, FakeTokenVerifier},
    },
    utils::oauth_crypto::sha256_hex,
};

pub struct SessionFixture {
    pub factory: Arc<InMemoryUnitOfWorkFactory>,
    pub refresh: Arc<InMemoryRefreshTokenRepository>,
    pub clients: Arc<InMemoryOAuthClientRepository>,
    pub users: Arc<InMemoryUserRepository>,
    pub audit: Arc<InMemoryAuditEventRepository>,
    pub events: Arc<InMemoryOutboxEventRepository>,
    pub tenant_id: Uuid,
    pub other_tenant_id: Uuid,
    pub alice: Uuid,
    pub bob: Uuid,
    // A user of the other tenant.
    pub carol: Uuid,
    // Registered for back-channel logout.
    pub app: OAuthClient,
    // Not registered: it is never notified.
    pub other_app: OAuthClient,
}

pub const APP_LOGOUT_URI: &str = "https://app.example.com/backchannel-logout";
// Every fixture client registers this post_logout_redirect_uri.
pub const POST_LOGOUT_URI: &str = "https://app.example.com/signed-out";

// A seeded family: its id, and the raw value of its live (newest) token.
pub struct SeededFamily {
    pub family_id: Uuid,
    pub live_token: String,
    pub signed_in_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

fn client(tenant_id: Uuid, name: &str) -> OAuthClient {
    OAuthClient::new(
        tenant_id,
        Uuid::now_v7(),
        None,
        ClientType::Public,
        name.to_string(),
        vec!["https://app.example.com/callback".to_string()],
        vec![POST_LOGOUT_URI.to_string()],
        vec!["openid".to_string(), "offline_access".to_string()],
        vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        true,
        true,
        vec!["en".to_string()],
        "en".to_string(),
        MfaPolicy::default(),
    )
}

fn tenant(id: Uuid) -> Tenant {
    Tenant::reconstruct(
        id,
        "Tenant".to_string(),
        None,
        "pepper".to_string(),
        true,
        true,
        true,
        Uuid::now_v7(),
        Utc::now(),
        None,
    )
}

pub fn build() -> SessionFixture {
    let tenant_id = Uuid::now_v7();
    let other_tenant_id = Uuid::now_v7();

    let tenants = Arc::new(InMemoryTenantRepository::new());
    {
        let mut data = tenants.memory.data.lock().unwrap();
        data.insert(tenant_id, tenant(tenant_id));
        data.insert(other_tenant_id, tenant(other_tenant_id));
    }

    let users = Arc::new(InMemoryUserRepository::new());
    let alice = User::new(tenant_id, "alice@example.com".to_string(), None, None);
    let bob = User::new(tenant_id, "bob@example.com".to_string(), None, None);
    let carol = User::new(other_tenant_id, "carol@example.com".to_string(), None, None);
    let (alice_id, bob_id, carol_id) = (alice.id(), bob.id(), carol.id());
    {
        let mut data = users.memory.data.lock().unwrap();
        for u in [alice, bob, carol] {
            data.insert(u.id(), u);
        }
    }

    let mut app = client(tenant_id, "Web App");
    app.set_backchannel_logout(Some(APP_LOGOUT_URI.to_string()), true);
    let other_app = client(tenant_id, "Mobile App");
    let clients = Arc::new(InMemoryOAuthClientRepository::new());
    {
        let mut data = clients.memory.data.lock().unwrap();
        data.insert(app.id(), app.clone());
        data.insert(other_app.id(), other_app.clone());
    }

    let keys = Arc::new(InMemorySigningKeyRepository::new());
    let key = SigningKey::new(
        tenant_id,
        "kid-1".to_string(),
        SigningAlgorithm::RS256,
        "{}".to_string(),
        "secret".to_string(),
    );
    keys.memory.data.lock().unwrap().insert(key.id(), key);

    let refresh = Arc::new(InMemoryRefreshTokenRepository::new());
    let audit = Arc::new(InMemoryAuditEventRepository::new());
    let events = Arc::new(InMemoryOutboxEventRepository::new());
    let factory = Arc::new(
        InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients.clone())
            .with_refresh_token_repository(refresh.clone())
            .with_signing_key_repository(keys)
            .with_audit_event_repository(audit.clone())
            .with_outbox_event_repository(events.clone()),
    );

    SessionFixture {
        factory,
        refresh,
        clients,
        users,
        audit,
        events,
        tenant_id,
        other_tenant_id,
        alice: alice_id,
        bob: bob_id,
        carol: carol_id,
        app,
        other_app,
    }
}

impl SessionFixture {
    // The back-channel logout notices committed to the outbox so far.
    pub fn logout_notices(&self) -> Vec<LogoutNotice> {
        self.events
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .filter(|e| e.event_type() == DomainEventType::BackchannelLogout)
            .filter_map(|e| LogoutNotice::from_payload(e.payload()))
            .collect()
    }

    pub fn list(&self) -> list_user_sessions::use_case::UseCase {
        list_user_sessions::use_case::UseCase::new(
            self.refresh.clone(),
            self.clients.clone(),
            self.users.clone(),
        )
    }

    pub fn revoke(&self) -> revoke_user_session::use_case::UseCase {
        revoke_user_session::use_case::UseCase::new(self.factory.clone())
    }

    pub fn end_session(&self, verifier: FakeTokenVerifier) -> end_session::use_case::UseCase {
        end_session::use_case::UseCase::new(self.factory.clone(), Arc::new(verifier))
    }

    pub fn revoke_all(&self) -> revoke_all_user_sessions::use_case::UseCase {
        revoke_all_user_sessions::use_case::UseCase::new(self.factory.clone())
    }

    pub fn refresh_grant(&self) -> token_refresh::use_case::UseCase {
        token_refresh::use_case::UseCase::new(
            self.factory.clone(),
            Arc::new(FakeTokenSigner::new()),
            "https://auth.test".to_string(),
            Arc::new(FakeAuditLogger::new()),
            token_refresh::use_case::DEFAULT_REFRESH_REUSE_LEEWAY_SECONDS,
            Arc::new(FakeRiskEngine::allow()),
            crate::testing::password_hasher::test_password_hasher(),
        )
    }

    pub fn refresh_command(&self, raw: &str, app: &OAuthClient) -> token_refresh::command::Command {
        token_refresh::command::Command {
            tenant_id: app.tenant_id(),
            client_id: app.client_id().to_string(),
            client_secret: None,
            refresh_token: raw.to_string(),
            ip_address: None,
            user_agent: None,
        }
    }

    // A family signed in `signed_in_ago`, refreshed `refreshes` times since (one hour
    // apart), whose newest token is live unless the caller changes it afterwards.
    pub fn seed_family(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        client_id: Uuid,
        signed_in_ago: Duration,
        refreshes: usize,
    ) -> SeededFamily {
        let family_id = Uuid::now_v7();
        let signed_in_at = Utc::now() - signed_in_ago;
        let mut data = self.refresh.memory.data.lock().unwrap();
        let mut live_token = String::new();
        let mut last_used_at = signed_in_at;
        for i in 0..=refreshes {
            let created_at = signed_in_at + Duration::hours(i as i64);
            let raw = format!("raw-{family_id}-{i}");
            let is_last = i == refreshes;
            let token = RefreshToken::reconstruct(
                Uuid::now_v7(),
                tenant_id,
                sha256_hex(&raw),
                client_id,
                user_id,
                vec!["openid".to_string(), "offline_access".to_string()],
                family_id,
                (!is_last).then_some(created_at + Duration::hours(1)),
                None,
                created_at + Duration::days(30),
                created_at,
            );
            data.insert(token.id(), token);
            if is_last {
                live_token = raw;
                last_used_at = created_at;
            }
        }
        SeededFamily {
            family_id,
            live_token,
            signed_in_at,
            last_used_at,
        }
    }

    // Every committed token row of a family.
    pub fn family_rows(&self, family_id: Uuid) -> Vec<RefreshToken> {
        self.refresh
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .filter(|t| t.family_id() == family_id)
            .cloned()
            .collect()
    }

    // Apply `update` to every committed row of a family.
    pub fn update_family(&self, family_id: Uuid, mut update: impl FnMut(&mut RefreshToken)) {
        let mut data = self.refresh.memory.data.lock().unwrap();
        for token in data.values_mut().filter(|t| t.family_id() == family_id) {
            update(token);
        }
    }

    pub fn audit_events(&self) -> Vec<AuditEvent> {
        self.audit.events.lock().unwrap().clone()
    }
}
