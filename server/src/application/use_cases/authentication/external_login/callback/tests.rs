#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::{
            ports::{
                external_identity_provider_client::ExternalUserProfile, risk_engine::RiskEngine,
            },
            use_cases::authentication::external_login::callback::{
                command::Command, use_case::UseCase,
            },
        },
        domain::entities::{
            external_identity_provider::{ExternalIdentityProvider, ExternalProviderKind},
            external_login_flow::ExternalLoginFlow,
            login_event::RiskDecision,
            oauth_client::{ClientType, MfaPolicy, OAuthClient},
            social_identity::SocialIdentity,
            user::User,
        },
        testing::{
            fake_external_identity_provider_client::FakeExternalIdentityProviderClient,
            fake_risk_engine::FakeRiskEngine,
            in_memory_repositories::{
                in_memory_authorization_code_repository::InMemoryAuthorizationCodeRepository,
                in_memory_external_identity_provider_repository::InMemoryExternalIdentityProviderRepository,
                in_memory_external_login_flow_repository::InMemoryExternalLoginFlowRepository,
                in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
                in_memory_outbox_event_repository::InMemoryOutboxEventRepository,
                in_memory_social_identity_repository::InMemorySocialIdentityRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
        },
        utils::oauth_crypto::sha256_hex,
    };

    const RAW_STATE: &str = "raw-state-token";

    struct Harness {
        users: Arc<InMemoryUserRepository>,
        socials: Arc<InMemorySocialIdentityRepository>,
        codes: Arc<InMemoryAuthorizationCodeRepository>,
        flows: Arc<InMemoryExternalLoginFlowRepository>,
        outbox: Arc<InMemoryOutboxEventRepository>,
        tenant_id: Uuid,
        use_case: UseCase,
    }

    fn make_client(tenant_id: Uuid) -> OAuthClient {
        OAuthClient::reconstruct(
            Uuid::now_v7(),
            tenant_id,
            Uuid::now_v7(),
            None,
            ClientType::Public,
            "App".to_string(),
            vec!["https://app.example.com/cb".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            None,
            None,
            None,
            None,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
            true,
            chrono::Utc::now(),
            None,
        )
    }

    fn setup(
        profile: ExternalUserProfile,
        seed_users: Vec<User>,
        seed_socials: Vec<SocialIdentity>,
    ) -> Harness {
        setup_with_risk(
            profile,
            seed_users,
            seed_socials,
            Arc::new(FakeRiskEngine::allow()),
        )
    }

    fn setup_with_risk(
        profile: ExternalUserProfile,
        seed_users: Vec<User>,
        seed_socials: Vec<SocialIdentity>,
        risk_engine: Arc<dyn RiskEngine>,
    ) -> Harness {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        let provider = ExternalIdentityProvider::new(
            client.id(),
            tenant_id,
            ExternalProviderKind::Google,
            "g-id".to_string(),
            "enc:g-secret".to_string(),
            "https://idp.example.com/cb/google".to_string(),
            true,
        );
        providers
            .memory
            .data
            .lock()
            .unwrap()
            .insert(provider.id(), provider.clone());

        let flows = Arc::new(InMemoryExternalLoginFlowRepository::new());
        let flow = ExternalLoginFlow::new(
            sha256_hex(RAW_STATE),
            provider.id(),
            client.id(),
            tenant_id,
            ExternalProviderKind::Google,
            "https://app.example.com/cb".to_string(),
            "code".to_string(),
            "openid".to_string(),
            "client-state".to_string(),
            None,
            "challenge".to_string(),
            "S256".to_string(),
            chrono::Utc::now() + chrono::Duration::minutes(10),
        );
        flows.memory.data.lock().unwrap().insert(flow.id(), flow);

        let users = Arc::new(InMemoryUserRepository::new());
        for u in seed_users {
            users.memory.data.lock().unwrap().insert(u.id(), u);
        }

        let socials = Arc::new(InMemorySocialIdentityRepository::new());
        for s in seed_socials {
            socials.memory.data.lock().unwrap().insert(s.id(), s);
        }

        let codes = Arc::new(InMemoryAuthorizationCodeRepository::new());
        let outbox = Arc::new(InMemoryOutboxEventRepository::new());

        let factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::clone(&users),
                Arc::new(InMemoryTenantRepository::new()),
            )
            .with_oauth_client_repository(clients)
            .with_external_identity_provider_repository(providers)
            .with_external_login_flow_repository(Arc::clone(&flows))
            .with_social_identity_repository(Arc::clone(&socials))
            .with_authorization_code_repository(Arc::clone(&codes))
            .with_outbox_event_repository(Arc::clone(&outbox)),
        );

        let use_case = UseCase::new(
            factory,
            Arc::new(FakeExternalIdentityProviderClient::returning(profile)),
            Arc::new(StubSecretCipher),
            risk_engine,
        );

        Harness {
            users,
            socials,
            codes,
            flows,
            outbox,
            tenant_id,
            use_case,
        }
    }

    fn command() -> Command {
        Command {
            provider_name: "google".to_string(),
            code: "auth-code".to_string(),
            state: RAW_STATE.to_string(),
            ip_address: None,
            user_agent: None,
        }
    }

    #[tokio::test]
    async fn provisions_new_passwordless_user() {
        let profile = ExternalUserProfile {
            subject: "google-sub-1".to_string(),
            email: Some("new@example.com".to_string()),
            email_verified: true,
            name: Some("New User".to_string()),
        };
        let h = setup(profile, vec![], vec![]);

        let response = h.use_case.execute(command()).await.unwrap();

        assert_eq!(response.redirect_uri, "https://app.example.com/cb");
        assert_eq!(response.state, "client-state");
        assert!(!response.authorization_code.is_empty());

        // A new verified user + its social link were created, the code minted, and
        // the flow consumed.
        let users = h.users.memory.data.lock().unwrap();
        assert_eq!(users.len(), 1);
        let user = users.values().next().unwrap();
        assert_eq!(user.email(), "new@example.com");
        assert!(user.is_email_verified());

        assert_eq!(h.socials.memory.data.lock().unwrap().len(), 1);
        assert_eq!(h.codes.memory.data.lock().unwrap().len(), 1);
        assert!(h.flows.memory.data.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn reuses_returning_user_by_subject() {
        // The provider returns no email at all; the user must still be recognised
        // by their previously-linked subject.
        let profile = ExternalUserProfile {
            subject: "google-sub-1".to_string(),
            email: None,
            email_verified: false,
            name: None,
        };
        let h = setup(profile, vec![], vec![]);

        // Seed a pre-existing user + social link (under the harness's tenant).
        let user = User::new(h.tenant_id, "returning@example.com".to_string(), None, None);
        let user_id = user.id();
        let identity = SocialIdentity::new(
            user_id,
            h.tenant_id,
            ExternalProviderKind::Google,
            "google-sub-1".to_string(),
            Some("returning@example.com".to_string()),
        );
        h.users.memory.data.lock().unwrap().insert(user_id, user);
        h.socials
            .memory
            .data
            .lock()
            .unwrap()
            .insert(identity.id(), identity);

        let response = h.use_case.execute(command()).await.unwrap();
        assert!(!response.authorization_code.is_empty());

        // No new user created; the existing one is reused, and a code is minted.
        assert_eq!(h.users.memory.data.lock().unwrap().len(), 1);
        assert_eq!(h.codes.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn links_to_existing_user_by_verified_email() {
        let profile = ExternalUserProfile {
            subject: "google-sub-2".to_string(),
            email: Some("existing@example.com".to_string()),
            email_verified: true,
            name: None,
        };
        let h = setup(profile, vec![], vec![]);
        let existing = User::new(h.tenant_id, "existing@example.com".to_string(), None, None);
        let existing_id = existing.id();
        h.users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing_id, existing);

        let response = h.use_case.execute(command()).await.unwrap();
        assert!(!response.authorization_code.is_empty());

        // No new user; a social link to the existing user was added.
        assert_eq!(h.users.memory.data.lock().unwrap().len(), 1);
        let socials = h.socials.memory.data.lock().unwrap();
        assert_eq!(socials.len(), 1);
        assert_eq!(socials.values().next().unwrap().user_id(), existing_id);
    }

    #[tokio::test]
    async fn links_to_existing_user_when_email_case_differs() {
        // Regression: a social login whose email differs only in letter case from a
        // native account must LINK to it, not provision a second duplicate user.
        let profile = ExternalUserProfile {
            subject: "google-sub-4".to_string(),
            email: Some("Gabriel.Sobral@Example.com".to_string()),
            email_verified: true,
            name: None,
        };
        let h = setup(profile, vec![], vec![]);
        // Native account stored canonically (User::new lowercases) as
        // "gabriel.sobral@example.com".
        let existing = User::new(
            h.tenant_id,
            "gabriel.sobral@example.com".to_string(),
            None,
            None,
        );
        let existing_id = existing.id();
        h.users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing_id, existing);

        let response = h.use_case.execute(command()).await.unwrap();
        assert!(!response.authorization_code.is_empty());

        // Linked, not duplicated: still one user, and the social link points at it.
        assert_eq!(h.users.memory.data.lock().unwrap().len(), 1);
        let socials = h.socials.memory.data.lock().unwrap();
        assert_eq!(socials.len(), 1);
        assert_eq!(socials.values().next().unwrap().user_id(), existing_id);
        // The linked identity email is normalized to lowercase.
        assert_eq!(
            socials.values().next().unwrap().email(),
            Some("gabriel.sobral@example.com")
        );
    }

    #[tokio::test]
    async fn blocked_by_risk_is_rejected_without_minting_a_code() {
        // The Risk Engine decides to BLOCK this social sign-in (e.g. impossible travel).
        // The login must be refused and no authorization code minted / flow consumed.
        let profile = ExternalUserProfile {
            subject: "google-sub-1".to_string(),
            email: Some("blocked@example.com".to_string()),
            email_verified: true,
            name: Some("Blocked User".to_string()),
        };
        let h = setup_with_risk(
            profile,
            vec![],
            vec![],
            Arc::new(FakeRiskEngine::with_decision(
                RiskDecision::BlockTemporarily,
            )),
        );

        let err = h.use_case.execute(command()).await.unwrap_err();
        assert!(matches!(
            err,
            crate::domain::errors::DomainError::AuthenticationError(_)
        ));

        // Nothing was committed: no code minted and the flow is still present.
        assert!(h.codes.memory.data.lock().unwrap().is_empty());
        assert_eq!(h.flows.memory.data.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn rejects_unverified_email_for_new_user() {
        let profile = ExternalUserProfile {
            subject: "google-sub-3".to_string(),
            email: Some("unverified@example.com".to_string()),
            email_verified: false,
            name: None,
        };
        let h = setup(profile, vec![], vec![]);

        let err = h.use_case.execute(command()).await.unwrap_err();
        assert!(matches!(
            err,
            crate::domain::errors::DomainError::ExternalProviderError(_)
        ));
    }

    // ── Outbound events ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn provisioning_a_new_user_enqueues_a_user_created_event() {
        let profile = ExternalUserProfile {
            subject: "google-sub-events-1".to_string(),
            email: Some("brand-new@example.com".to_string()),
            email_verified: true,
            name: Some("Brand New".to_string()),
        };
        let h = setup(profile, vec![], vec![]);

        h.use_case.execute(command()).await.unwrap();

        let stored = h.outbox.memory.data.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let event = stored.values().next().unwrap();
        assert_eq!(event.event_type().as_str(), "user.created");

        let data = &event.payload()["data"];
        assert_eq!(data["email"], "brand-new@example.com");
        assert_eq!(data["source"], "external_provider");
        // The provider vouched for the address, so the account arrives verified.
        assert_eq!(data["email_verified"], true);
    }

    // The subtle one: branch 2a LINKS a social identity to an account the tenant
    // already had. No user was created, so emitting user.created here would make
    // every subscriber insert a duplicate of a user they already know about.
    #[tokio::test]
    async fn linking_to_an_existing_user_enqueues_no_event() {
        let profile = ExternalUserProfile {
            subject: "google-sub-events-2".to_string(),
            email: Some("already-here@example.com".to_string()),
            email_verified: true,
            name: None,
        };
        let h = setup(profile, vec![], vec![]);
        let existing = User::new(
            h.tenant_id,
            "already-here@example.com".to_string(),
            None,
            None,
        );
        h.users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        h.use_case.execute(command()).await.unwrap();

        // The link was made, but nothing was announced.
        assert_eq!(h.socials.memory.data.lock().unwrap().len(), 1);
        assert_eq!(h.outbox.memory.data.lock().unwrap().len(), 0);
    }

    // A returning user recognised by provider subject is neither created nor
    // changed, so it announces nothing either.
    #[tokio::test]
    async fn a_returning_social_user_enqueues_no_event() {
        let profile = ExternalUserProfile {
            subject: "google-sub-events-3".to_string(),
            email: Some("returning@example.com".to_string()),
            email_verified: true,
            name: None,
        };
        let h = setup(profile, vec![], vec![]);

        // Seed the user under the harness tenant and link it up front, so the
        // callback recognises it by provider subject rather than provisioning.
        let user = User::new(h.tenant_id, "returning@example.com".to_string(), None, None);
        let user_id = user.id();
        h.users.memory.data.lock().unwrap().insert(user_id, user);
        let identity = SocialIdentity::new(
            user_id,
            h.tenant_id,
            ExternalProviderKind::Google,
            "google-sub-events-3".to_string(),
            Some("returning@example.com".to_string()),
        );
        h.socials
            .memory
            .data
            .lock()
            .unwrap()
            .insert(identity.id(), identity);

        h.use_case.execute(command()).await.unwrap();

        assert_eq!(h.outbox.memory.data.lock().unwrap().len(), 0);
    }
}
