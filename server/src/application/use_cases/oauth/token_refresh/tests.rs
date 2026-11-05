#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::{
            ports::risk_engine::RiskEngine,
            use_cases::oauth::token_refresh::{
                command::Command,
                use_case::{DEFAULT_REFRESH_REUSE_LEEWAY_SECONDS, UseCase},
            },
        },
        domain::{
            entities::{
                login_event::RiskDecision,
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                refresh_token::RefreshToken,
                signing_key::{SigningAlgorithm, SigningKey},
            },
            errors::DomainError,
        },
        testing::{
            fake_audit_logger::FakeAuditLogger,
            fake_risk_engine::FakeRiskEngine,
            in_memory_repositories::{
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_refresh_token_repository::InMemoryRefreshTokenRepository,
                in_memory_signing_key_repository::InMemorySigningKeyRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            oauth_fakes::FakeTokenSigner,
        },
        utils::oauth_crypto::sha256_hex,
    };

    const RAW_REFRESH: &str = "raw-refresh-token-value";

    fn client_id() -> Uuid {
        Uuid::parse_str("01890000-0000-7000-8000-000000000abc").unwrap()
    }

    struct Ctx {
        use_case: UseCase,
        refresh: Arc<InMemoryRefreshTokenRepository>,
        tenant_id: Uuid,
        client_internal_id: Uuid,
        family_id: Uuid,
    }

    fn build(seed: impl FnOnce(Uuid, Uuid) -> Vec<RefreshToken>) -> Ctx {
        build_with_risk(seed, Arc::new(FakeRiskEngine::allow()))
    }

    fn build_with_risk(
        seed: impl FnOnce(Uuid, Uuid) -> Vec<RefreshToken>,
        risk_engine: Arc<dyn RiskEngine>,
    ) -> Ctx {
        let tenant_id = Uuid::now_v7();

        let client = OAuthClient::new(
            tenant_id,
            client_id(),
            None,
            ClientType::Public,
            "Test App".to_string(),
            vec!["https://app.example.com/callback".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec![
                "authorization_code".to_string(),
                "refresh_token".to_string(),
            ],
            true,
            true,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
        );
        let client_internal_id = client.id();
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client);

        let refresh = Arc::new(InMemoryRefreshTokenRepository::new());
        let seeded = seed(tenant_id, client_internal_id);
        let family_id = seeded
            .first()
            .map(|t| t.family_id())
            .unwrap_or_else(Uuid::now_v7);
        {
            let mut data = refresh.memory.data.lock().unwrap();
            for token in seeded {
                data.insert(token.id(), token);
            }
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

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_oauth_client_repository(clients)
        .with_refresh_token_repository(refresh.clone())
        .with_signing_key_repository(keys);

        let use_case = UseCase::new(
            Arc::new(factory),
            Arc::new(FakeTokenSigner::new()),
            "https://auth.test".to_string(),
            Arc::new(FakeAuditLogger::new()),
            DEFAULT_REFRESH_REUSE_LEEWAY_SECONDS,
            risk_engine,
            crate::testing::password_hasher::test_password_hasher(),
        );

        Ctx {
            use_case,
            refresh,
            tenant_id,
            client_internal_id,
            family_id,
        }
    }

    fn command(tenant_id: Uuid, raw: &str) -> Command {
        Command {
            tenant_id,
            client_id: client_id().to_string(),
            client_secret: None,
            refresh_token: raw.to_string(),
            ip_address: None,
            user_agent: None,
        }
    }

    #[tokio::test]
    async fn rotates_and_revokes_the_old_token() {
        let ctx = build(|tenant_id, client_id| {
            vec![RefreshToken::new(
                tenant_id,
                sha256_hex(RAW_REFRESH),
                client_id,
                Uuid::now_v7(),
                vec!["openid".to_string()],
                Utc::now() + Duration::days(30),
            )]
        });

        let response = ctx
            .use_case
            .execute(command(ctx.tenant_id, RAW_REFRESH))
            .await
            .unwrap();

        let data = ctx.refresh.memory.data.lock().unwrap();
        // Old token rotated away; a brand-new active token exists in the same family.
        let old = data
            .values()
            .find(|t| t.token_hash() == sha256_hex(RAW_REFRESH))
            .unwrap();
        assert!(old.rotated_at().is_some());

        let new = data
            .values()
            .find(|t| t.token_hash() == sha256_hex(&response.refresh_token))
            .unwrap();
        assert!(new.is_active(Utc::now()));
        assert_eq!(new.family_id(), old.family_id());
        let _ = ctx.client_internal_id;
    }

    #[tokio::test]
    async fn replay_of_a_rotated_token_revokes_the_whole_family() {
        // Family with: an already-rotated token (the one we will replay) + the live
        // successor it was rotated into.
        let ctx = build(|tenant_id, client_id| {
            let family_id = Uuid::now_v7();
            let user_id = Uuid::now_v7();
            let rotated = RefreshToken::reconstruct(
                Uuid::now_v7(),
                tenant_id,
                sha256_hex(RAW_REFRESH),
                client_id,
                user_id,
                vec!["openid".to_string()],
                family_id,
                Some(Utc::now() - Duration::minutes(1)), // rotated_at set
                None,
                Utc::now() + Duration::days(30),
                Utc::now() - Duration::minutes(5),
            );
            let live_successor = RefreshToken::reconstruct(
                Uuid::now_v7(),
                tenant_id,
                sha256_hex("successor"),
                client_id,
                user_id,
                vec!["openid".to_string()],
                family_id,
                None,
                None,
                Utc::now() + Duration::days(30),
                Utc::now() - Duration::minutes(1),
            );
            vec![rotated, live_successor]
        });

        let err = ctx
            .use_case
            .execute(command(ctx.tenant_id, RAW_REFRESH))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidGrant(_)));

        // The whole family is now revoked, including the previously-live successor.
        let data = ctx.refresh.memory.data.lock().unwrap();
        let live_count = data
            .values()
            .filter(|t| t.family_id() == ctx.family_id && t.is_active(Utc::now()))
            .count();
        assert_eq!(live_count, 0);
    }

    #[tokio::test]
    async fn replay_within_leeway_reissues_without_revoking_the_family() {
        // A token rotated *just now* (inside the leeway window) presented again is a
        // legitimate concurrent refresh / retry, not theft: it must succeed and leave the
        // family intact instead of revoking it.
        let ctx = build(|tenant_id, client_id| {
            let family_id = Uuid::now_v7();
            let user_id = Uuid::now_v7();
            let rotated = RefreshToken::reconstruct(
                Uuid::now_v7(),
                tenant_id,
                sha256_hex(RAW_REFRESH),
                client_id,
                user_id,
                vec!["openid".to_string()],
                family_id,
                Some(Utc::now()), // rotated within the leeway window
                None,
                Utc::now() + Duration::days(30),
                Utc::now() - Duration::minutes(1),
            );
            let live_successor = RefreshToken::reconstruct(
                Uuid::now_v7(),
                tenant_id,
                sha256_hex("successor"),
                client_id,
                user_id,
                vec!["openid".to_string()],
                family_id,
                None,
                None,
                Utc::now() + Duration::days(30),
                Utc::now(),
            );
            vec![rotated, live_successor]
        });

        let response = ctx
            .use_case
            .execute(command(ctx.tenant_id, RAW_REFRESH))
            .await
            .unwrap();

        let data = ctx.refresh.memory.data.lock().unwrap();

        // The reissued token is active and joined the same family.
        let reissued = data
            .values()
            .find(|t| t.token_hash() == sha256_hex(&response.refresh_token))
            .unwrap();
        assert!(reissued.is_active(Utc::now()));
        assert_eq!(reissued.family_id(), ctx.family_id);

        // The family was NOT revoked: the pre-existing successor is still live, alongside
        // the freshly issued sibling.
        let live_count = data
            .values()
            .filter(|t| t.family_id() == ctx.family_id && t.is_active(Utc::now()))
            .count();
        assert_eq!(live_count, 2);
    }

    #[tokio::test]
    async fn refresh_blocked_by_risk_is_rejected_without_rotating() {
        // A refresh the Risk Engine decides to BLOCK (e.g. impossible travel or a
        // blocked IP) must be refused — and, because the refusal happens before the
        // mint step, the presented token stays active and un-rotated.
        let ctx = build_with_risk(
            |tenant_id, client_id| {
                vec![RefreshToken::new(
                    tenant_id,
                    sha256_hex(RAW_REFRESH),
                    client_id,
                    Uuid::now_v7(),
                    vec!["openid".to_string()],
                    Utc::now() + Duration::days(30),
                )]
            },
            Arc::new(FakeRiskEngine::with_decision(
                RiskDecision::BlockTemporarily,
            )),
        );

        let err = ctx
            .use_case
            .execute(command(ctx.tenant_id, RAW_REFRESH))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidGrant(_)));

        let data = ctx.refresh.memory.data.lock().unwrap();
        let presented = data
            .values()
            .find(|t| t.token_hash() == sha256_hex(RAW_REFRESH))
            .unwrap();
        assert!(presented.rotated_at().is_none());
        assert!(presented.is_active(Utc::now()));
    }

    #[tokio::test]
    async fn rejects_unknown_refresh_token() {
        let ctx = build(|tenant_id, client_id| {
            vec![RefreshToken::new(
                tenant_id,
                sha256_hex(RAW_REFRESH),
                client_id,
                Uuid::now_v7(),
                vec!["openid".to_string()],
                Utc::now() + Duration::days(30),
            )]
        });

        let err = ctx
            .use_case
            .execute(command(ctx.tenant_id, "nope"))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::InvalidGrant(_)));
    }
}
