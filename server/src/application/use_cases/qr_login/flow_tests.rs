// QR code sign-in end to end on in-memory repositories: the browser's start and poll,
// the scanning device's scan and decision, for both the hosted approval page and a
// signed-in app.

use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::{
    application::use_cases::{
        authentication::authorize_session,
        qr_login::{
            decide::{
                self,
                command::{Decider, Decision},
            },
            poll::{self, response::Status},
            scan::{self, command::Scanner},
            start,
        },
    },
    domain::{
        entities::{
            audit_action::{
                USER_LOGIN_FAILURE, USER_LOGIN_SUCCESS, USER_QR_LOGIN_APPROVED,
                USER_QR_LOGIN_DENIED,
            },
            auth_session::AuthSession,
            login_event::RiskDecision,
            mfa_enrollment::MfaEnrollment,
            mfa_method::MfaMethod,
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            qr_login_request::{QrApprover, QrLoginStatus},
            tenant::Tenant,
            user::User,
            user_credentials::UserCredentials,
        },
        errors::DomainError,
    },
    testing::{
        fake_geoip_resolver::FakeGeoIpResolver,
        fake_risk_engine::FakeRiskEngine,
        fake_webauthn_service::FakeWebauthnService,
        in_memory_repositories::{
            in_memory_audit_event_repository::InMemoryAuditEventRepository,
            in_memory_auth_session_repository::InMemoryAuthSessionRepository,
            in_memory_mfa_enrollment_repository::InMemoryMfaEnrollmentRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
            in_memory_qr_login_request_repository::InMemoryQrLoginRequestRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
        in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        templated_email_support::recording_templated_email_sender,
        user_factory::{TEST_PASSWORD_SECRET, generate_user},
    },
    utils::oauth_crypto::{generate_opaque_token, sha256_hex},
};

const PUBLIC_CLIENT_ID: &str = "01890000-0000-7000-8000-000000000def";
const REDIRECT_URI: &str = "https://app.example.com/callback";
const BROWSER_IP: &str = "203.0.113.7";

struct Options {
    qr_enabled: bool,
    external_login_only: bool,
    mfa_policy: MfaPolicy,
    risk: FakeRiskEngine,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            qr_enabled: true,
            external_login_only: false,
            mfa_policy: MfaPolicy::default(),
            risk: FakeRiskEngine::allow(),
        }
    }
}

struct Fixture {
    start: start::use_case::UseCase,
    poll: poll::use_case::UseCase,
    scan: scan::use_case::UseCase,
    decide: decide::use_case::UseCase,
    authorize: authorize_session::use_case::UseCase,
    risk: Arc<FakeRiskEngine>,
    clients: Arc<InMemoryOAuthClientRepository>,
    users: Arc<InMemoryUserRepository>,
    sessions: Arc<InMemoryAuthSessionRepository>,
    requests: Arc<InMemoryQrLoginRequestRepository>,
    enrollments: Arc<InMemoryMfaEnrollmentRepository>,
    audit: Arc<InMemoryAuditEventRepository>,
    client: OAuthClient,
    tenant_id: Uuid,
    user_id: Uuid,
    email: String,
}

// What the browser holds after `start`.
#[derive(Debug)]
struct Shown {
    request_id: Uuid,
    token: String,
    session_code: String,
}

fn build(options: Options) -> Fixture {
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
    client.set_qr_login_enabled(options.qr_enabled);
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
    let user = generate_user(tenant_id);
    let user_id = user.id();
    let email = user.email().to_string();
    users.memory.data.lock().unwrap().insert(user_id, user);

    let sessions = Arc::new(InMemoryAuthSessionRepository::new());
    let requests = Arc::new(InMemoryQrLoginRequestRepository::new());
    let enrollments = Arc::new(InMemoryMfaEnrollmentRepository::new());
    let audit = Arc::new(InMemoryAuditEventRepository::new());
    let factory = Arc::new(
        InMemoryUnitOfWorkFactory::new(users.clone(), tenants)
            .with_oauth_client_repository(clients.clone())
            .with_auth_session_repository(sessions.clone())
            .with_qr_login_request_repository(requests.clone())
            .with_mfa_enrollment_repository(enrollments.clone())
            .with_audit_event_repository(audit.clone()),
    );
    let (templated, _emails) = recording_templated_email_sender();
    let risk = Arc::new(options.risk);
    let geoip = Arc::new(FakeGeoIpResolver::new().with(
        BROWSER_IP,
        crate::application::ports::geoip_resolver::GeoLocation {
            country_code: Some("BR".to_string()),
            region: Some("SP".to_string()),
            city: Some("São Paulo".to_string()),
            ..Default::default()
        },
    ));

    Fixture {
        start: start::use_case::UseCase::new(factory.clone(), risk.clone(), geoip),
        poll: poll::use_case::UseCase::new(
            factory.clone(),
            templated,
            Arc::new(FakeWebauthnService::new()),
            risk.clone(),
        ),
        scan: scan::use_case::UseCase::new(factory.clone()),
        decide: decide::use_case::UseCase::new(factory.clone()),
        authorize: authorize_session::use_case::UseCase::new(factory),
        risk,
        clients,
        users,
        sessions,
        requests,
        enrollments,
        audit,
        client,
        tenant_id,
        user_id,
        email,
    }
}

impl Fixture {
    async fn show(&self) -> Result<Shown, DomainError> {
        let started = self
            .start
            .execute(start::command::Command {
                client_id: PUBLIC_CLIENT_ID.to_string(),
                redirect_uri: REDIRECT_URI.to_string(),
                response_type: "code".to_string(),
                scope: "openid email".to_string(),
                state: "xyz".to_string(),
                code_challenge: "challenge".to_string(),
                code_challenge_method: "S256".to_string(),
                nonce: None,
                ip_address: Some(BROWSER_IP.to_string()),
                user_agent: Some("Mozilla/5.0 (X11; Linux x86_64) Chrome/130".to_string()),
            })
            .await?;
        Ok(Shown {
            request_id: started.request_id,
            token: started.token,
            session_code: started.session_code,
        })
    }

    async fn poll(&self, shown: &Shown) -> Result<poll::response::Response, DomainError> {
        self.poll_with(shown, &shown.session_code).await
    }

    async fn poll_with(
        &self,
        shown: &Shown,
        session_code: &str,
    ) -> Result<poll::response::Response, DomainError> {
        self.poll
            .execute(poll::command::Command {
                client_id: PUBLIC_CLIENT_ID.to_string(),
                request_id: shown.request_id,
                session_code: session_code.to_string(),
                mfa_methods: None,
                mfa_default_method: None,
                ip_address: Some(BROWSER_IP.to_string()),
                user_agent: Some("browser".to_string()),
            })
            .await
    }

    async fn scan_as(
        &self,
        shown: &Shown,
        scanner: Scanner,
    ) -> Result<scan::response::Response, DomainError> {
        self.scan
            .execute(scan::command::Command {
                request_id: shown.request_id,
                token: shown.token.clone(),
                scanner,
            })
            .await
    }

    async fn scan_on_phone(&self, shown: &Shown) -> Result<scan::response::Response, DomainError> {
        self.scan_as(
            shown,
            Scanner::HostedPage {
                client_id: PUBLIC_CLIENT_ID.to_string(),
            },
        )
        .await
    }

    fn app(&self, user_id: Uuid) -> Scanner {
        Scanner::SignedInUser {
            tenant_id: self.tenant_id,
            user_id,
        }
    }

    async fn decide_as(
        &self,
        shown: &Shown,
        claim_code: &str,
        decision: Decision,
        decider: Decider,
    ) -> Result<decide::response::Response, DomainError> {
        self.decide
            .execute(decide::command::Command {
                request_id: shown.request_id,
                claim_code: claim_code.to_string(),
                decision,
                decider,
                ip_address: Some("198.51.100.9".to_string()),
                user_agent: Some("phone".to_string()),
            })
            .await
    }

    fn hosted(&self, session_code: Option<&str>) -> Decider {
        Decider::HostedPage {
            client_id: PUBLIC_CLIENT_ID.to_string(),
            session_code: session_code.map(str::to_string),
        }
    }

    // A sign-in completed on the phone for this client (what the hosted sign-in page
    // leaves behind), returned as its raw code.
    fn phone_session(&self, shape: impl FnOnce(&mut AuthSession)) -> String {
        let now = Utc::now();
        let raw = generate_opaque_token();
        let mut session = AuthSession::new(
            self.tenant_id,
            sha256_hex(&raw),
            self.client.id(),
            REDIRECT_URI.to_string(),
            vec!["openid".to_string()],
            "phone-state".to_string(),
            "phone-challenge".to_string(),
            "S256".to_string(),
            None,
            now + Duration::minutes(10),
            now + Duration::minutes(30),
        );
        session.authenticate(self.user_id, now);
        shape(&mut session);
        self.sessions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(session.id(), session);
        raw
    }

    fn session_by_code(&self, raw: &str) -> AuthSession {
        let hash = sha256_hex(raw);
        self.sessions
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .find(|s| s.session_token_hash() == hash)
            .cloned()
            .expect("session exists")
    }

    fn request(&self, shown: &Shown) -> crate::domain::entities::qr_login_request::QrLoginRequest {
        self.requests.get(shown.request_id).expect("request exists")
    }

    fn add_user(&self) -> Uuid {
        let user = generate_user(self.tenant_id);
        let id = user.id();
        self.users.memory.data.lock().unwrap().insert(id, user);
        id
    }

    fn seed_totp(&self) {
        let mut enrollment =
            MfaEnrollment::new_totp(self.tenant_id, self.user_id, "encrypted".to_string());
        enrollment.confirm(Utc::now());
        self.enrollments
            .memory
            .data
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment);
    }

    fn audit_actions(&self) -> Vec<(String, Option<Uuid>, serde_json::Value)> {
        self.audit
            .events
            .lock()
            .unwrap()
            .iter()
            .map(|e| (e.action().to_string(), e.actor_id(), e.metadata().clone()))
            .collect()
    }

    fn authorize_command(&self, session_code: &str) -> authorize_session::command::Command {
        authorize_session::command::Command {
            session_code: session_code.to_string(),
            client_id: PUBLIC_CLIENT_ID.to_string(),
            email: self.email.clone(),
        }
    }
}

fn is_auth_error(result: Result<impl std::fmt::Debug, DomainError>) -> bool {
    matches!(result, Err(DomainError::AuthenticationError(_)))
}

#[tokio::test]
async fn a_phone_that_signs_in_on_the_hosted_page_signs_the_browser_in() {
    let fx = build(Options::default());
    let shown = fx.show().await.unwrap();

    // The browser's session binds nobody while it waits.
    let waiting = fx.session_by_code(&shown.session_code);
    assert!(waiting.is_awaiting_qr_approval());
    assert_eq!(fx.poll(&shown).await.unwrap().status, Status::Pending);

    // The phone scans: it is told what is asking, and how to sign in for it.
    let scanned = fx.scan_on_phone(&shown).await.unwrap();
    assert_eq!(scanned.client_name, "Test App");
    assert_eq!(
        scanned.requester.location.as_deref(),
        Some("São Paulo, SP, BR")
    );
    assert!(
        scanned
            .requester
            .user_agent
            .as_deref()
            .unwrap()
            .contains("Linux")
    );
    let sign_in = scanned.sign_in.expect("the hosted page signs in first");
    assert_eq!(sign_in.redirect_uri, REDIRECT_URI);
    assert_eq!(sign_in.scope, "openid email");
    assert_eq!(fx.poll(&shown).await.unwrap().status, Status::Scanned);

    // It signs in there, then approves with that sign-in.
    let phone = fx.phone_session(|_| {});
    let decided = fx
        .decide_as(
            &shown,
            &scanned.claim_code,
            Decision::Approve,
            fx.hosted(Some(&phone)),
        )
        .await
        .unwrap();
    assert_eq!(decided.status, "approved");
    assert!(
        fx.session_by_code(&phone).is_expired(Utc::now()),
        "the phone's sign-in was spent as proof"
    );

    // The browser's next poll is its sign-in, with a new session code.
    let approved = fx.poll(&shown).await.unwrap();
    assert_eq!(approved.status, Status::Approved);
    assert_eq!(approved.email.as_deref(), Some(fx.email.as_str()));
    let login = approved.login.expect("a sign-in");
    assert!(login.mfa_type.is_none());
    assert_ne!(login.session_code, shown.session_code, "rotated");
    assert_eq!(fx.request(&shown).status(), QrLoginStatus::Completed);
    assert_eq!(
        fx.request(&shown).approved_via(),
        Some(QrApprover::HostedPage)
    );

    // The old code is dead; the new one is spent for an authorization code.
    assert!(is_auth_error(fx.poll(&shown).await));
    let code = fx
        .authorize
        .execute(fx.authorize_command(&login.session_code))
        .await
        .unwrap();
    assert!(!code.authorization_code.is_empty());
    assert_eq!(code.state, "xyz", "the browser's own authorization request");

    // Scored as the browser, and audited on both sides.
    let evaluated = fx.risk.recorded();
    assert!(
        evaluated
            .iter()
            .any(|e| e.user_id == Some(fx.user_id) && e.credentials_valid)
    );
    let actions = fx.audit_actions();
    assert!(
        actions
            .iter()
            .any(|(a, actor, _)| a == USER_QR_LOGIN_APPROVED && *actor == Some(fx.user_id))
    );
    let success = actions
        .iter()
        .find(|(a, _, _)| a == USER_LOGIN_SUCCESS)
        .expect("the browser's sign-in");
    assert_eq!(success.2["method"], "qr_code");
    assert_eq!(success.2["approved_via"], "hosted_page");
}

#[tokio::test]
async fn a_code_is_claimed_once_and_only_with_its_token() {
    let fx = build(Options::default());
    let shown = fx.show().await.unwrap();

    let wrong = Shown {
        request_id: shown.request_id,
        token: "not-the-token".to_string(),
        session_code: String::new(),
    };
    assert!(is_auth_error(fx.scan_on_phone(&wrong).await));

    fx.scan_on_phone(&shown).await.unwrap();
    // A second device (a photo of the same screen) gets nothing.
    assert!(is_auth_error(fx.scan_on_phone(&shown).await));
    assert!(is_auth_error(fx.scan_as(&shown, fx.app(fx.user_id)).await));
}

#[tokio::test]
async fn an_unscanned_code_expires() {
    let fx = build(Options::default());
    let shown = fx.show().await.unwrap();
    {
        // Age the code past its two minutes.
        let mut data = fx.requests.memory.data.lock().unwrap();
        let request = data.get(&shown.request_id).unwrap().clone();
        let aged = crate::domain::entities::qr_login_request::QrLoginRequest::reconstruct(
            request.id(),
            request.tenant_id(),
            request.client_id(),
            request.auth_session_id(),
            request.token_hash().to_string(),
            request.status(),
            None,
            None,
            None,
            false,
            None,
            request.requester().clone(),
            None,
            None,
            None,
            Utc::now() - Duration::seconds(1),
            request.created_at(),
        );
        data.insert(aged.id(), aged);
    }
    assert!(is_auth_error(fx.scan_on_phone(&shown).await));
    assert_eq!(fx.poll(&shown).await.unwrap().status, Status::Expired);
}

#[tokio::test]
async fn only_the_claim_code_decides_and_a_denial_ends_the_wait() {
    let fx = build(Options::default());
    let shown = fx.show().await.unwrap();
    let scanned = fx.scan_on_phone(&shown).await.unwrap();
    let phone = fx.phone_session(|_| {});

    assert!(is_auth_error(
        fx.decide_as(&shown, "guess", Decision::Approve, fx.hosted(Some(&phone)))
            .await
    ));
    assert!(
        !fx.session_by_code(&phone).is_expired(Utc::now()),
        "a refusal spends nothing"
    );

    let denied = fx
        .decide_as(&shown, &scanned.claim_code, Decision::Deny, fx.hosted(None))
        .await
        .unwrap();
    assert_eq!(denied.status, "denied");
    // Decided once: approving afterwards fails.
    assert!(is_auth_error(
        fx.decide_as(
            &shown,
            &scanned.claim_code,
            Decision::Approve,
            fx.hosted(Some(&phone))
        )
        .await
    ));

    assert_eq!(fx.poll(&shown).await.unwrap().status, Status::Denied);
    assert!(
        fx.session_by_code(&shown.session_code)
            .is_expired(Utc::now()),
        "nothing left to wait for"
    );
    assert!(
        fx.audit_actions()
            .iter()
            .any(|(a, _, _)| a == USER_QR_LOGIN_DENIED)
    );
}

#[tokio::test]
async fn the_hosted_page_needs_a_finished_sign_in_for_this_client() {
    let fx = build(Options::default());
    let shown = fx.show().await.unwrap();
    let scanned = fx.scan_on_phone(&shown).await.unwrap();
    let approve = |code: Option<String>| {
        fx.decide_as(
            &shown,
            &scanned.claim_code,
            Decision::Approve,
            fx.hosted(code.as_deref()),
        )
    };

    // No sign-in at all.
    assert!(is_auth_error(approve(None).await));
    // A sign-in still waiting for its second factor.
    let pending = fx.phone_session(|s| s.require_mfa());
    assert!(is_auth_error(approve(Some(pending)).await));
    // An expired one.
    let expired = fx.phone_session(|s| s.expire_now(Utc::now()));
    assert!(is_auth_error(approve(Some(expired)).await));
    // The browser's own waiting session is not a sign-in.
    assert!(is_auth_error(
        approve(Some(shown.session_code.clone())).await
    ));

    // A user who must change a temporary password first.
    {
        let mut users = fx.users.memory.data.lock().unwrap();
        let mut user: User = users.get(&fx.user_id).cloned().unwrap();
        user.update_credentials(UserCredentials::new(
            fx.user_id,
            "hash".to_string(),
            "argon2id".to_string(),
            true,
        ));
        users.insert(fx.user_id, user);
    }
    let must_change = fx.phone_session(|_| {});
    assert!(is_auth_error(approve(Some(must_change)).await));
    assert_eq!(fx.request(&shown).status(), QrLoginStatus::Scanned);
}

#[tokio::test]
async fn a_signed_in_app_approves_but_the_browser_still_owes_the_clients_second_factor() {
    let fx = build(Options {
        mfa_policy: MfaPolicy {
            required: true,
            email_enabled: true,
            totp_enabled: true,
            webauthn_enabled: true,
        },
        ..Options::default()
    });
    fx.seed_totp();
    let shown = fx.show().await.unwrap();

    let scanned = fx.scan_as(&shown, fx.app(fx.user_id)).await.unwrap();
    assert!(scanned.sign_in.is_none(), "an app is signed in already");
    fx.decide_as(
        &shown,
        &scanned.claim_code,
        Decision::Approve,
        Decider::SignedInUser {
            tenant_id: fx.tenant_id,
            user_id: fx.user_id,
        },
    )
    .await
    .unwrap();

    let login = fx.poll(&shown).await.unwrap().login.unwrap();
    assert_eq!(login.mfa_type.as_deref(), Some("totp"));
    assert!(fx.session_by_code(&login.session_code).is_mfa_pending());
    assert!(
        fx.authorize
            .execute(fx.authorize_command(&login.session_code))
            .await
            .is_err()
    );
    assert_eq!(
        fx.request(&shown).approved_via(),
        Some(QrApprover::SignedInApp)
    );
}

#[tokio::test]
async fn a_second_factor_passed_on_the_phone_is_not_asked_again() {
    let fx = build(Options {
        mfa_policy: MfaPolicy {
            required: true,
            email_enabled: true,
            totp_enabled: true,
            webauthn_enabled: true,
        },
        ..Options::default()
    });
    fx.seed_totp();
    let shown = fx.show().await.unwrap();
    let scanned = fx.scan_on_phone(&shown).await.unwrap();
    let phone = fx.phone_session(|s| {
        s.require_mfa();
        s.verify_mfa(&MfaMethod::Totp, Utc::now());
    });
    fx.decide_as(
        &shown,
        &scanned.claim_code,
        Decision::Approve,
        fx.hosted(Some(&phone)),
    )
    .await
    .unwrap();
    assert!(fx.request(&shown).approver_mfa_verified());

    let login = fx.poll(&shown).await.unwrap().login.unwrap();
    assert!(login.mfa_type.is_none());
    fx.authorize
        .execute(fx.authorize_command(&login.session_code))
        .await
        .expect("spendable at once");
}

#[tokio::test]
async fn an_apps_claim_is_decided_by_its_own_user_in_its_own_tenant() {
    let fx = build(Options::default());
    let shown = fx.show().await.unwrap();

    // Another tenant's token cannot even scan.
    assert!(is_auth_error(
        fx.scan_as(
            &shown,
            Scanner::SignedInUser {
                tenant_id: Uuid::now_v7(),
                user_id: fx.user_id,
            },
        )
        .await
    ));

    let scanned = fx.scan_as(&shown, fx.app(fx.user_id)).await.unwrap();
    let other = fx.add_user();
    let as_other = Decider::SignedInUser {
        tenant_id: fx.tenant_id,
        user_id: other,
    };
    assert!(is_auth_error(
        fx.decide_as(
            &shown,
            &scanned.claim_code,
            Decision::Approve,
            as_other.clone()
        )
        .await
    ));
    assert!(is_auth_error(
        fx.decide_as(&shown, &scanned.claim_code, Decision::Deny, as_other)
            .await
    ));
    // Nor can the hosted page decide what an app claimed: a denial there names no
    // user, and a sign-in there as someone else is not the scanning user.
    let phone = fx.phone_session(|s| s.authenticate(other, Utc::now()));
    assert!(is_auth_error(
        fx.decide_as(&shown, &scanned.claim_code, Decision::Deny, fx.hosted(None))
            .await
    ));
    assert!(is_auth_error(
        fx.decide_as(
            &shown,
            &scanned.claim_code,
            Decision::Approve,
            fx.hosted(Some(&phone))
        )
        .await
    ));
    assert_eq!(fx.request(&shown).status(), QrLoginStatus::Scanned);
}

#[tokio::test]
async fn the_feature_is_per_client_and_switching_it_off_stops_shown_codes() {
    for options in [
        Options {
            qr_enabled: false,
            ..Options::default()
        },
        Options {
            external_login_only: true,
            ..Options::default()
        },
    ] {
        let fx = build(options);
        assert!(matches!(fx.show().await, Err(DomainError::AccessDenied(_))));
    }

    let fx = build(Options::default());
    let shown = fx.show().await.unwrap();
    fx.clients
        .memory
        .data
        .lock()
        .unwrap()
        .get_mut(&fx.client.id())
        .unwrap()
        .set_qr_login_enabled(false);
    assert!(is_auth_error(fx.scan_on_phone(&shown).await));
    assert_eq!(fx.poll(&shown).await.unwrap().status, Status::Expired);
}

#[tokio::test]
async fn the_browser_is_risk_scored_and_a_block_refuses_it() {
    let fx = build(Options {
        risk: FakeRiskEngine::with_decision(RiskDecision::BlockTemporarily),
        ..Options::default()
    });
    let shown = fx.show().await.unwrap();
    let scanned = fx.scan_on_phone(&shown).await.unwrap();
    let phone = fx.phone_session(|_| {});
    fx.decide_as(
        &shown,
        &scanned.claim_code,
        Decision::Approve,
        fx.hosted(Some(&phone)),
    )
    .await
    .unwrap();

    assert!(is_auth_error(fx.poll(&shown).await));
    assert!(
        fx.session_by_code(&shown.session_code)
            .is_expired(Utc::now())
    );
    let failure = fx
        .audit_actions()
        .into_iter()
        .find(|(a, _, _)| a == USER_LOGIN_FAILURE)
        .expect("recorded");
    assert_eq!(failure.2["reason"], "risk_blocked");
}

#[tokio::test]
async fn a_risk_step_up_on_the_browser_asks_for_a_second_factor() {
    let fx = build(Options {
        risk: FakeRiskEngine::with_decision(RiskDecision::AllowWithMfa),
        ..Options::default()
    });
    fx.seed_totp();
    let shown = fx.show().await.unwrap();
    let scanned = fx.scan_as(&shown, fx.app(fx.user_id)).await.unwrap();
    fx.decide_as(
        &shown,
        &scanned.claim_code,
        Decision::Approve,
        Decider::SignedInUser {
            tenant_id: fx.tenant_id,
            user_id: fx.user_id,
        },
    )
    .await
    .unwrap();

    let login = fx.poll(&shown).await.unwrap().login.unwrap();
    assert_eq!(login.mfa_type.as_deref(), Some("totp"));
    let session = fx.session_by_code(&login.session_code);
    assert!(session.is_mfa_pending());
    assert!(
        session.risk_login_event_id().is_some(),
        "the step-up labels its event"
    );
}

#[tokio::test]
async fn a_blocked_ip_gets_no_code() {
    let fx = build(Options {
        risk: FakeRiskEngine::blocking_pre_auth(),
        ..Options::default()
    });
    assert!(is_auth_error(fx.show().await));
    assert!(fx.requests.all().is_empty());
}
