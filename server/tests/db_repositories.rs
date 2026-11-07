// Repository tests against a real Postgres.
//
// The unit suite runs every use case against in-memory fakes, which cannot tell
// whether the SQL itself is right: a missing `WHERE tenant_id = $1`, a broken
// sealed-column round trip or a trigger that rejects a legitimate write would all
// pass there. These tests run the real repositories on a throwaway database that
// `#[sqlx::test]` creates per test (with every migration applied) and drops after.
//
// Opt-in, because they need a server: `cargo test --features db-tests` with
// DATABASE_URL pointing at a Postgres whose user may CREATE DATABASE. CI runs
// them (`--all-features`).
#![cfg(feature = "db-tests")]

use std::sync::Arc;

use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use tokio::sync::Mutex;
use uuid::Uuid;
use web_api::{
    application::ports::secret_cipher::SecretCipher,
    domain::{
        entities::{
            auth_session::{AuthSession, PrimaryFactor},
            domain_event_type::DomainEventType,
            magic_link_token::MagicLinkToken,
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            organization::Organization,
            outbox_event::OutboxEvent,
            refresh_token::RefreshToken,
            tenant::Tenant,
            user::User,
            webhook_delivery::WebhookDelivery,
            webhook_subscription::WebhookSubscription,
        },
        repositories::{
            auth_session_repository::AuthSessionRepository,
            magic_link_token_repository::MagicLinkTokenRepository,
            mfa_challenge_repository::MfaChallengeRepository,
            oauth_client_repository::OAuthClientRepository,
            organization_repository::OrganizationRepository,
            outbox_event_repository::OutboxEventRepository,
            refresh_token_repository::RefreshTokenRepository, tenant_repository::TenantRepository,
            user_repository::UserRepository,
            webhook_delivery_repository::WebhookDeliveryRepository,
            webhook_subscription_repository::WebhookSubscriptionRepository,
        },
    },
    infraestructure::{
        crypto::{aes_gcm_secret_cipher::AesGcmSecretCipher, secret_reseal::reseal_legacy_secrets},
        database::{
            executor::SharedTx,
            repositories::{
                auth_session_repository::DbAuthSessionRepository,
                magic_link_token_repository::DbMagicLinkTokenRepository,
                mfa_challenge_repository::DbMfaChallengeRepository,
                oauth_client_repository::DbOAuthClientRepository,
                organization_repository::DbOrganizationRepository,
                outbox_event_repository::DbOutboxEventRepository,
                refresh_token_repository::DbRefreshTokenRepository,
                tenant_repository::DbTenantRepository, user_repository::DbUserRepository,
                webhook_delivery_repository::DbWebhookDeliveryRepository,
                webhook_subscription_repository::DbWebhookSubscriptionRepository,
            },
        },
    },
};

fn cipher() -> Arc<dyn SecretCipher> {
    Arc::new(AesGcmSecretCipher::new([42u8; 32]))
}

async fn seed_tenant(pool: &PgPool, cipher: &Arc<dyn SecretCipher>, pepper: &str) -> Tenant {
    let organization = Organization::new(format!("org-{}", Uuid::now_v7()), None, true);
    DbOrganizationRepository::new(pool.clone())
        .add(&organization)
        .await
        .expect("insert organization");
    let tenant = Tenant::new(
        format!("tenant-{}", Uuid::now_v7()),
        None,
        pepper.to_string(),
        true,
        true,
        true,
        organization.id(),
    );
    DbTenantRepository::new(pool.clone(), Arc::clone(cipher))
        .add(&tenant)
        .await
        .expect("insert tenant");
    tenant
}

#[sqlx::test(migrations = "./migrations")]
async fn the_tenant_pepper_is_sealed_at_rest_and_opened_on_read(pool: PgPool) {
    let cipher = cipher();
    let pepper = "a-very-secret-pepper-value-0123456789";
    let tenant = seed_tenant(&pool, &cipher, pepper).await;

    let row = sqlx::query(
        "SELECT password_hash_secret, password_hash_secret_sealed FROM tenants WHERE id = $1",
    )
    .bind(tenant.id())
    .fetch_one(&pool)
    .await
    .unwrap();
    let stored: String = row.get("password_hash_secret");
    let sealed: bool = row.get("password_hash_secret_sealed");
    assert!(sealed);
    assert!(stored.starts_with("v1:"), "stored as {stored}");
    assert!(
        !stored.contains(pepper),
        "the pepper must not be readable in the row"
    );

    let loaded = DbTenantRepository::new(pool.clone(), Arc::clone(&cipher))
        .find_by_id(tenant.id())
        .await
        .unwrap()
        .expect("tenant");
    assert_eq!(loaded.password_hash_secret(), pepper);
}

#[sqlx::test(migrations = "./migrations")]
async fn the_reseal_job_upgrades_legacy_secrets_once(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "placeholder").await;

    // Simulate a row written before sealing existed.
    sqlx::query(
        "UPDATE tenants SET password_hash_secret = 'legacy-plain-pepper', \
         password_hash_secret_sealed = FALSE WHERE id = $1",
    )
    .bind(tenant.id())
    .execute(&pool)
    .await
    .unwrap();

    let first = reseal_legacy_secrets(&pool, cipher.as_ref()).await.unwrap();
    assert!(first.resealed >= 1);
    assert_eq!(first.unreadable, 0);

    let loaded = DbTenantRepository::new(pool.clone(), Arc::clone(&cipher))
        .find_by_id(tenant.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.password_hash_secret(), "legacy-plain-pepper");

    // Idempotent: nothing left to do on the second run.
    let second = reseal_legacy_secrets(&pool, cipher.as_ref()).await.unwrap();
    assert_eq!(second.resealed, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn users_are_isolated_per_tenant(pool: PgPool) {
    let cipher = cipher();
    let tenant_a = seed_tenant(&pool, &cipher, "pepper-a").await;
    let tenant_b = seed_tenant(&pool, &cipher, "pepper-b").await;

    let user = User::new(tenant_a.id(), "alice@example.com".to_string(), None, None);
    let users = DbUserRepository::new(pool.clone());
    users.add(&user).await.expect("insert user");

    assert!(
        users
            .find_by_email(tenant_a.id(), "alice@example.com")
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        users
            .find_by_email(tenant_b.id(), "alice@example.com")
            .await
            .unwrap()
            .is_none(),
        "a lookup scoped to tenant B must not see tenant A's user"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn audit_events_are_append_only(pool: PgPool) {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_events (id, actor_type, action, outcome) \
         VALUES ($1, 'system', 'test.event', 'success')",
    )
    .bind(id)
    .execute(&pool)
    .await
    .expect("insert audit event");

    let update = sqlx::query("UPDATE audit_events SET action = 'tampered' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;
    assert!(update.is_err(), "UPDATE on audit_events must be refused");

    // The sanctioned erasure path can still pseudonymise.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL gatekeeper.audit_erasure = 'on'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE audit_events SET actor_id = NULL WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .expect("erasure update is allowed");
    tx.commit().await.unwrap();

    // Retention may still delete.
    sqlx::query("DELETE FROM audit_events WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("delete is allowed");
}

async fn seed_client(pool: &PgPool, tenant: &Tenant) -> OAuthClient {
    let client = OAuthClient::new(
        tenant.id(),
        Uuid::now_v7(),
        None,
        ClientType::Public,
        "DB test client".to_string(),
        vec!["https://app.example.com/callback".to_string()],
        vec![],
        vec!["openid".to_string()],
        vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        true,
        false,
        vec!["en".to_string()],
        "en".to_string(),
        MfaPolicy::default(),
    );
    DbOAuthClientRepository::new(pool.clone())
        .add(&client)
        .await
        .expect("insert oauth client");
    client
}

async fn seed_user(pool: &PgPool, tenant: &Tenant) -> User {
    let user = User::new(
        tenant.id(),
        format!("{}@example.com", Uuid::now_v7()),
        None,
        None,
    );
    DbUserRepository::new(pool.clone())
        .add(&user)
        .await
        .expect("insert user");
    user
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_token_rotation_and_family_revocation(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;
    let other_tenant = seed_tenant(&pool, &cipher, "pepper-2").await;
    let client = seed_client(&pool, &tenant).await;
    let user = seed_user(&pool, &tenant).await;
    let repo = DbRefreshTokenRepository::new(pool.clone());
    let now = Utc::now();

    let mut first = RefreshToken::new(
        tenant.id(),
        "hash-1".to_string(),
        client.id(),
        user.id(),
        vec!["openid".to_string()],
        now + Duration::days(30),
    );
    repo.add(&first).await.unwrap();

    // Rotate: the old token is stamped, the new one joins the same family.
    let second = first.rotate("hash-2".to_string(), now + Duration::days(30), now);
    repo.rotate(&first, &second).await.unwrap();

    let old = repo
        .find_by_token_hash(tenant.id(), "hash-1")
        .await
        .unwrap()
        .expect("old token row");
    let new = repo
        .find_by_token_hash(tenant.id(), "hash-2")
        .await
        .unwrap()
        .expect("new token row");
    assert!(old.rotated_at().is_some());
    assert!(!old.is_active(now));
    assert!(new.is_active(now));
    assert_eq!(old.family_id(), new.family_id());

    // A lookup scoped to another tenant finds nothing.
    assert!(
        repo.find_by_token_hash(other_tenant.id(), "hash-2")
            .await
            .unwrap()
            .is_none()
    );

    // Reuse detection: revoking the family kills every member.
    repo.revoke_family(tenant.id(), new.family_id())
        .await
        .unwrap();
    for hash in ["hash-1", "hash-2"] {
        let token = repo
            .find_by_token_hash(tenant.id(), hash)
            .await
            .unwrap()
            .unwrap();
        assert!(token.revoked_at().is_some(), "{hash} must be revoked");
        assert!(!token.is_active(now));
    }
}

// The session list is one SQL query over the rotation rows: one row per live family,
// with its first and last created_at. Ending a session is scoped to its owner.
#[sqlx::test(migrations = "./migrations")]
async fn sessions_are_live_refresh_token_families_scoped_to_their_user(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;
    let client = seed_client(&pool, &tenant).await;
    let alice = seed_user(&pool, &tenant).await;
    let bob = seed_user(&pool, &tenant).await;
    let repo = DbRefreshTokenRepository::new(pool.clone());
    let now = Utc::now();
    let token = |hash: &str, user: &User, at: chrono::DateTime<Utc>| {
        RefreshToken::reconstruct(
            Uuid::now_v7(),
            tenant.id(),
            hash.to_string(),
            client.id(),
            user.id(),
            vec!["openid".to_string()],
            Uuid::now_v7(),
            None,
            None,
            at + Duration::days(30),
            at,
        )
    };

    // A family signed in two hours ago, refreshed once an hour ago, plus a sibling
    // reissued within the reuse leeway (both newest rows live).
    let signed_in_at = now - Duration::hours(2);
    let mut first = token("s-1", &alice, signed_in_at);
    repo.add(&first).await.unwrap();
    let refreshed_at = now - Duration::hours(1);
    let second = first.rotate(
        "s-2".to_string(),
        refreshed_at + Duration::days(30),
        refreshed_at,
    );
    repo.rotate(&first, &second).await.unwrap();
    let sibling = first.reissue_within_family(
        "s-3".to_string(),
        refreshed_at + Duration::days(30),
        refreshed_at - Duration::seconds(1),
    );
    repo.add(&sibling).await.unwrap();
    let family_id = first.family_id();

    // A second, more recent session of Alice on the same app.
    let recent = token("r-1", &alice, now - Duration::minutes(5));
    repo.add(&recent).await.unwrap();

    // Not listed: revoked, expired, and Bob's.
    let revoked = token("v-1", &alice, now - Duration::minutes(30));
    repo.add(&revoked).await.unwrap();
    repo.revoke_family(tenant.id(), revoked.family_id())
        .await
        .unwrap();
    let expired = token("e-1", &alice, now - Duration::days(31));
    repo.add(&expired).await.unwrap();
    let bobs = token("b-1", &bob, now - Duration::minutes(1));
    repo.add(&bobs).await.unwrap();

    let sessions = repo
        .find_active_families_by_user(tenant.id(), alice.id(), now)
        .await
        .unwrap();
    let ids: Vec<_> = sessions.iter().map(|s| s.family_id).collect();
    assert_eq!(
        ids,
        vec![recent.family_id(), family_id],
        "newest activity first"
    );
    let rotated = &sessions[1];
    let micros = |t: chrono::DateTime<Utc>| t.timestamp_micros();
    assert_eq!(micros(rotated.signed_in_at), micros(signed_in_at));
    assert_eq!(micros(rotated.last_used_at), micros(refreshed_at));
    assert_eq!(rotated.client_id, client.id());
    assert_eq!(rotated.scopes, vec!["openid".to_string()]);

    // Bob cannot end Alice's session by its id: not his, nothing changes.
    assert!(
        !repo
            .revoke_family_of_user(tenant.id(), bob.id(), family_id)
            .await
            .unwrap()
    );
    assert_eq!(
        repo.find_active_families_by_user(tenant.id(), alice.id(), now)
            .await
            .unwrap()
            .len(),
        2
    );

    // Alice ends it: every row of the family is revoked; ending it again still
    // reports that it is hers.
    assert!(
        repo.revoke_family_of_user(tenant.id(), alice.id(), family_id)
            .await
            .unwrap()
    );
    for hash in ["s-1", "s-2", "s-3"] {
        let row = repo
            .find_by_token_hash(tenant.id(), hash)
            .await
            .unwrap()
            .unwrap();
        assert!(row.revoked_at().is_some(), "{hash} must be revoked");
    }
    assert!(
        repo.revoke_family_of_user(tenant.id(), alice.id(), family_id)
            .await
            .unwrap()
    );
    let left: Vec<_> = repo
        .find_active_families_by_user(tenant.id(), alice.id(), now)
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.family_id)
        .collect();
    assert_eq!(left, vec![recent.family_id()]);
}

fn login_session(tenant: &Tenant, client: &OAuthClient, token_hash: &str) -> AuthSession {
    let now = Utc::now();
    AuthSession::new(
        tenant.id(),
        token_hash.to_string(),
        client.id(),
        "https://app.example.com/callback".to_string(),
        vec!["openid".to_string()],
        "state".to_string(),
        "challenge".to_string(),
        "S256".to_string(),
        None,
        now + Duration::minutes(10),
        now + Duration::minutes(30),
    )
}

// A passkey (passwordless) session must read back as one: the lock that makes only
// WebAuthn able to open it lives in `primary_factor`, so losing it on the way through
// the database would quietly turn the session into a password session.
#[sqlx::test(migrations = "./migrations")]
async fn auth_sessions_round_trip_their_primary_factor(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;
    let client = seed_client(&pool, &tenant).await;
    let user = seed_user(&pool, &tenant).await;
    let repo = DbAuthSessionRepository::new(pool.clone());

    let mut password_session = login_session(&tenant, &client, "hash-password");
    password_session.authenticate(user.id(), Utc::now());
    repo.add(&password_session).await.unwrap();

    let mut passkey_session = login_session(&tenant, &client, "hash-passkey");
    passkey_session.identify_for_passkey(user.id());
    repo.add(&passkey_session).await.unwrap();

    let read = repo
        .find_by_token_hash(tenant.id(), "hash-passkey")
        .await
        .unwrap()
        .expect("passkey session");
    assert_eq!(read.primary_factor(), PrimaryFactor::Passkey);
    assert!(read.is_mfa_pending());
    assert!(read.authenticated_at().is_none());
    assert!(!read.accepts_mfa_method(&MfaMethod::Totp));

    let mut read = read;
    read.verify_mfa(&MfaMethod::Webauthn, Utc::now());
    repo.update(&read).await.unwrap();
    let verified = repo
        .find_by_id(tenant.id(), read.id())
        .await
        .unwrap()
        .expect("passkey session");
    assert_eq!(verified.primary_factor(), PrimaryFactor::Passkey);
    assert!(!verified.is_mfa_pending());
    assert!(verified.authenticated_at().is_some());

    let password = repo
        .find_by_token_hash(tenant.id(), "hash-password")
        .await
        .unwrap()
        .expect("password session");
    assert_eq!(password.primary_factor(), PrimaryFactor::Password);

    // Rows written without the column (an older binary mid-rollout) are password
    // sessions, and the CHECK refuses anything else.
    let legacy: String = sqlx::query_scalar(
        "INSERT INTO auth_sessions (id, tenant_id, session_token_hash, pending_client_id,
            pending_redirect_uri, pending_scopes, pending_state, pending_code_challenge,
            pending_code_challenge_method, expires_at, absolute_expires_at, created_at)
         VALUES ($1, $2, 'hash-legacy', $3, 'https://app.example.com/callback', '{openid}',
            's', 'c', 'S256', now() + interval '10 minutes', now() + interval '30 minutes', now())
         RETURNING primary_factor",
    )
    .bind(Uuid::now_v7())
    .bind(tenant.id())
    .bind(client.id())
    .fetch_one(&pool)
    .await
    .expect("legacy insert");
    assert_eq!(legacy, "password");
    let bogus = sqlx::query("UPDATE auth_sessions SET primary_factor = 'magic' WHERE id = $1")
        .bind(read.id())
        .execute(&pool)
        .await;
    assert!(
        bogus.is_err(),
        "the CHECK constraint must refuse unknown factors"
    );
}

// Magic link login: the waiting session (no user bound) and its link survive the trip
// through the database, the client flag persists, and deleting the session — which
// is how the retention purge expires it — takes its links with it.
#[sqlx::test(migrations = "./migrations")]
async fn magic_link_tokens_round_trip_and_die_with_their_session(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;
    let mut client = seed_client(&pool, &tenant).await;
    let user = seed_user(&pool, &tenant).await;

    let clients = DbOAuthClientRepository::new(pool.clone());
    client.set_magic_link_enabled(true);
    clients.update(&client).await.unwrap();
    let read_client = clients
        .find_by_id(tenant.id(), client.id())
        .await
        .unwrap()
        .expect("client");
    assert!(read_client.magic_link_enabled());

    let sessions = DbAuthSessionRepository::new(pool.clone());
    let mut session = login_session(&tenant, &client, "hash-magic");
    session.await_magic_link();
    sessions.add(&session).await.unwrap();
    let read = sessions
        .find_by_token_hash(tenant.id(), "hash-magic")
        .await
        .unwrap()
        .expect("magic-link session");
    assert_eq!(read.primary_factor(), PrimaryFactor::MagicLink);
    assert!(read.is_awaiting_magic_link());
    assert!(!read.accepts_mfa_method(&MfaMethod::Email));

    let tokens = DbMagicLinkTokenRepository::new(pool.clone());
    assert_eq!(
        tokens.latest_created_at_for_user(user.id()).await.unwrap(),
        None
    );
    let mut token = MagicLinkToken::new(
        tenant.id(),
        user.id(),
        session.id(),
        "a".repeat(64),
        Utc::now() + Duration::minutes(15),
    );
    tokens.add(&token).await.unwrap();
    let stored = tokens.find_by_id(token.id()).await.unwrap().expect("token");
    assert_eq!(stored.auth_session_id(), session.id());
    assert_eq!(stored.token_hash(), "a".repeat(64));
    assert!(!stored.is_consumed());
    assert!(
        tokens
            .latest_created_at_for_user(user.id())
            .await
            .unwrap()
            .is_some()
    );

    token.consume(Utc::now());
    tokens.update(&token).await.unwrap();
    assert!(
        tokens
            .find_by_id(token.id())
            .await
            .unwrap()
            .unwrap()
            .is_consumed()
    );

    sqlx::query("DELETE FROM auth_sessions WHERE id = $1")
        .bind(session.id())
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        tokens.find_by_id(token.id()).await.unwrap().is_none(),
        "the link is deleted with its session"
    );
}

// Restarting a ceremony replaces only that session's challenge, never one running in
// another session of the same user.
#[sqlx::test(migrations = "./migrations")]
async fn mfa_challenges_are_invalidated_per_session(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;
    let client = seed_client(&pool, &tenant).await;
    let user = seed_user(&pool, &tenant).await;
    let sessions = DbAuthSessionRepository::new(pool.clone());
    let challenges = DbMfaChallengeRepository::new(pool.clone());

    let mut ids = Vec::new();
    for hash in ["hash-a", "hash-b"] {
        let mut session = login_session(&tenant, &client, hash);
        session.identify_for_passkey(user.id());
        sessions.add(&session).await.unwrap();
        let challenge = MfaChallenge::new_webauthn(
            tenant.id(),
            user.id(),
            Some(session.id()),
            MfaChallengePurpose::Login,
            serde_json::json!({ "state": hash }),
            Utc::now() + Duration::minutes(5),
        );
        challenges.add(&challenge).await.unwrap();
        ids.push((session.id(), challenge.id()));
    }

    challenges
        .invalidate_active_by_session(ids[0].0, &MfaChallengePurpose::Login, &MfaMethod::Webauthn)
        .await
        .unwrap();

    let first = challenges
        .find_by_id(tenant.id(), ids[0].1)
        .await
        .unwrap()
        .unwrap();
    let second = challenges
        .find_by_id(tenant.id(), ids[1].1)
        .await
        .unwrap()
        .unwrap();
    assert!(first.is_consumed());
    assert!(!second.is_consumed());
}

async fn begin(pool: &PgPool) -> SharedTx {
    Arc::new(Mutex::new(Some(pool.begin().await.expect("begin"))))
}

#[sqlx::test(migrations = "./migrations")]
async fn webhook_claims_skip_rows_another_worker_holds(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;

    let subscription = WebhookSubscription::new(
        tenant.id(),
        "hook".to_string(),
        "https://hooks.example.com/in".to_string(),
        "v1:test:sealed".to_string(),
        vec!["user.created".to_string()],
        true,
    );
    DbWebhookSubscriptionRepository::new(pool.clone())
        .add(&subscription)
        .await
        .unwrap();

    let outbox = DbOutboxEventRepository::new(pool.clone());
    let deliveries = DbWebhookDeliveryRepository::new(pool.clone());
    let now = Utc::now();
    let mut rows = Vec::new();
    for _ in 0..3 {
        let event = OutboxEvent::new(
            tenant.id(),
            DomainEventType::UserCreated,
            serde_json::json!({ "id": Uuid::now_v7() }),
        );
        outbox.enqueue(&event).await.unwrap();
        rows.push(WebhookDelivery::new(
            event.id(),
            subscription.id(),
            tenant.id(),
            now - Duration::seconds(1),
        ));
    }
    deliveries
        .insert_many_ignoring_conflicts(&rows)
        .await
        .unwrap();
    // Fan-out is idempotent: inserting the same (event, subscription) again is a no-op.
    deliveries
        .insert_many_ignoring_conflicts(&rows)
        .await
        .unwrap();
    assert_eq!(deliveries.count_by_tenant_id(tenant.id()).await.unwrap(), 3);

    // Worker A claims two rows and keeps its transaction open (row locks held).
    let tx_a = begin(&pool).await;
    let claimed_a = DbWebhookDeliveryRepository::with_tx(tx_a.clone())
        .claim_due(Utc::now(), 2)
        .await
        .unwrap();
    assert_eq!(claimed_a.len(), 2);

    // Worker B, concurrently, gets only the row A does not hold — never a duplicate.
    let tx_b = begin(&pool).await;
    let claimed_b = DbWebhookDeliveryRepository::with_tx(tx_b.clone())
        .claim_due(Utc::now(), 10)
        .await
        .unwrap();
    assert_eq!(claimed_b.len(), 1);
    assert!(
        claimed_a.iter().all(|a| a.id() != claimed_b[0].id()),
        "SKIP LOCKED must keep the two workers on disjoint rows"
    );

    // When A's transaction ends without an update, its rows become claimable again.
    drop(tx_b.lock().await.take());
    drop(tx_a.lock().await.take());
    let reclaimed = deliveries.claim_due(Utc::now(), 10).await.unwrap();
    assert_eq!(reclaimed.len(), 3);
}

// OIDC back-channel logout shares the outbox and the delivery table with the user
// webhooks: a delivery targets a subscription OR an OAuth client, each kind with its
// own idempotency key; both are claimed by the same SKIP LOCKED query; the admin
// webhook log shows only webhook deliveries. The client registration round-trips.
#[sqlx::test(migrations = "./migrations")]
async fn logout_deliveries_share_the_webhook_delivery_table(pool: PgPool) {
    let cipher = cipher();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;

    let mut client = seed_client(&pool, &tenant).await;
    let clients = DbOAuthClientRepository::new(pool.clone());
    client.set_backchannel_logout(Some("https://rp.example.com/bcl".to_string()), true);
    clients.update(&client).await.unwrap();
    let reloaded = clients
        .find_by_id(tenant.id(), client.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        reloaded.backchannel_logout_uri(),
        Some("https://rp.example.com/bcl")
    );
    assert!(reloaded.backchannel_logout_session_required());

    let subscription = WebhookSubscription::new(
        tenant.id(),
        "hook".to_string(),
        "https://hooks.example.com/in".to_string(),
        "v1:test:sealed".to_string(),
        vec![],
        true,
    );
    DbWebhookSubscriptionRepository::new(pool.clone())
        .add(&subscription)
        .await
        .unwrap();

    let outbox = DbOutboxEventRepository::new(pool.clone());
    let webhook_event = OutboxEvent::new(
        tenant.id(),
        DomainEventType::UserCreated,
        serde_json::json!({ "data": {} }),
    );
    let logout_event = OutboxEvent::new(
        tenant.id(),
        DomainEventType::BackchannelLogout,
        serde_json::json!({
            "client_id": client.id().to_string(),
            "sub": Uuid::now_v7().to_string(),
            "sid": Uuid::now_v7().to_string(),
        }),
    );
    outbox.enqueue(&webhook_event).await.unwrap();
    outbox.enqueue(&logout_event).await.unwrap();
    // The internal event type reads back (the claim skips unknown discriminators).
    let claimed_events = outbox.claim_unpublished(10).await.unwrap();
    assert!(
        claimed_events
            .iter()
            .any(|e| e.event_type() == DomainEventType::BackchannelLogout)
    );

    let deliveries = DbWebhookDeliveryRepository::new(pool.clone());
    let now = Utc::now() - Duration::seconds(1);
    let rows = vec![
        WebhookDelivery::new(webhook_event.id(), subscription.id(), tenant.id(), now),
        WebhookDelivery::for_backchannel_logout(logout_event.id(), client.id(), tenant.id(), now),
    ];
    deliveries
        .insert_many_ignoring_conflicts(&rows)
        .await
        .unwrap();
    // A replayed fan-out inserts nothing, for either kind.
    let replay = vec![
        WebhookDelivery::new(webhook_event.id(), subscription.id(), tenant.id(), now),
        WebhookDelivery::for_backchannel_logout(logout_event.id(), client.id(), tenant.id(), now),
    ];
    deliveries
        .insert_many_ignoring_conflicts(&replay)
        .await
        .unwrap();

    let due = deliveries.claim_due(Utc::now(), 10).await.unwrap();
    assert_eq!(due.len(), 2);
    let logout = due
        .iter()
        .find(|d| d.client_id().is_some())
        .expect("the logout delivery is claimed like any other");
    assert_eq!(logout.client_id(), Some(client.id()));
    assert_eq!(logout.subscription_id(), None);

    // The webhook delivery log does not list logout deliveries.
    assert_eq!(deliveries.count_by_tenant_id(tenant.id()).await.unwrap(), 1);
    let listed = deliveries
        .find_by_tenant_id(tenant.id(), 1, 10)
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].subscription_id(), Some(subscription.id()));

    // Deleting the client removes its pending logout deliveries (ON DELETE CASCADE).
    sqlx::query("DELETE FROM oauth_clients WHERE id = $1")
        .bind(client.id())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(deliveries.claim_due(Utc::now(), 10).await.unwrap().len(), 1);
}

// End to end, against real Postgres, real ES256 keys and a real HTTP listener:
// "sign out everywhere" commits a notice; the dispatcher turns it into a
// form-encoded logout_token POST; the token verifies against the tenant's published
// key exactly as an RP would check it (Back-Channel Logout 1.0 §2.6).
#[sqlx::test(migrations = "./migrations")]
async fn a_signed_out_session_reaches_the_app_as_a_verifiable_logout_token(pool: PgPool) {
    use axum::{Router, extract::State, http::HeaderMap, routing::post};
    use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::Jwk};
    use web_api::{
        application::{
            ports::signing_key_generator::SigningKeyGenerator,
            shared::{actor::Actor, backchannel_logout::BACKCHANNEL_LOGOUT_EVENT},
            use_cases::{session::revoke_all_user_sessions, webhook::dispatch_pending_events},
        },
        domain::repositories::signing_key_repository::SigningKeyRepository,
        infraestructure::{
            crypto::{
                ecdsa_key_generator::EcdsaKeyGenerator, jwt_signing_service::JwtSigningService,
            },
            database::{
                repositories::signing_key_repository::DbSigningKeyRepository,
                unit_of_work::DbUnitOfWorkFactory,
            },
            webhooks::reqwest_webhook_sender::ReqwestWebhookSender,
        },
    };

    // The RP's back-channel logout endpoint: records what it received.
    type Received = Arc<Mutex<Vec<(Option<String>, String)>>>;
    async fn receive(State(received): State<Received>, headers: HeaderMap, body: String) {
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        received.lock().await.push((content_type, body));
    }
    let received: Received = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new()
        .route("/bcl", post(receive))
        .with_state(received.clone());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let cipher = cipher();
    let issuer = "https://idp.example.com".to_string();
    let tenant = seed_tenant(&pool, &cipher, "pepper").await;
    let key = EcdsaKeyGenerator::new(Arc::clone(&cipher))
        .generate(tenant.id())
        .unwrap();
    DbSigningKeyRepository::new(pool.clone())
        .add(&key)
        .await
        .unwrap();

    let mut client = seed_client(&pool, &tenant).await;
    client.set_backchannel_logout(Some(format!("http://{address}/bcl")), false);
    DbOAuthClientRepository::new(pool.clone())
        .update(&client)
        .await
        .unwrap();
    let user = seed_user(&pool, &tenant).await;
    let session = RefreshToken::new(
        tenant.id(),
        format!("hash-{}", Uuid::now_v7()),
        client.id(),
        user.id(),
        vec!["openid".to_string()],
        Utc::now() + Duration::days(30),
    );
    DbRefreshTokenRepository::new(pool.clone())
        .add(&session)
        .await
        .unwrap();

    let factory = Arc::new(DbUnitOfWorkFactory::new(pool.clone(), Arc::clone(&cipher)));
    revoke_all_user_sessions::use_case::UseCase::new(factory.clone())
        .execute(
            &Actor::System,
            revoke_all_user_sessions::command::Command {
                tenant_id: tenant.id(),
                user_id: user.id(),
                ip_address: None,
                user_agent: None,
            },
        )
        .await
        .unwrap();

    let tick = dispatch_pending_events::use_case::UseCase::new(
        factory,
        Arc::new(ReqwestWebhookSender::new(5)),
        Arc::clone(&cipher),
        Arc::new(JwtSigningService::new(Arc::clone(&cipher))),
        issuer.clone(),
        10,
    )
    .execute()
    .await
    .unwrap();
    assert_eq!(tick.deliveries_succeeded, 1, "{tick:?}");

    let received = received.lock().await;
    assert_eq!(received.len(), 1);
    let (content_type, body) = &received[0];
    assert_eq!(
        content_type.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    let logout_token = body
        .strip_prefix("logout_token=")
        .expect("a single logout_token form field");

    // Verify as the RP does: typ, signature by the published key, iss, aud, exp.
    let header = decode_header(logout_token).unwrap();
    assert_eq!(header.typ.as_deref(), Some("logout+jwt"));
    assert_eq!(header.kid.as_deref(), Some(key.kid()));
    let jwk: Jwk = serde_json::from_str(key.public_jwk()).unwrap();
    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_issuer(&[issuer.as_str()]);
    validation.set_audience(&[client.client_id().to_string()]);
    let claims = decode::<serde_json::Value>(
        logout_token,
        &DecodingKey::from_jwk(&jwk).unwrap(),
        &validation,
    )
    .expect("the logout token verifies against the tenant's published key")
    .claims;
    assert_eq!(claims["sub"], user.id().to_string());
    assert_eq!(claims["sid"], session.family_id().to_string());
    assert!(claims["events"].get(BACKCHANNEL_LOGOUT_EVENT).is_some());
    assert!(claims.get("nonce").is_none());
}
