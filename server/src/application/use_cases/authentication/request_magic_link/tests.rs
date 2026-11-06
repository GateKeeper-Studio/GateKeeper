#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            entities::{audit_action::USER_MAGIC_LINK_SENT, auth_session::PrimaryFactor},
            errors::DomainError,
        },
        testing::{
            fake_risk_engine::FakeRiskEngine,
            magic_link_fixture::{
                AUTH_UI, Options, PUBLIC_CLIENT_ID, UNKNOWN_EMAIL, build, with_flags,
            },
        },
        utils::oauth_crypto::sha256_hex,
    };

    // S6: the request answers before ANY account-dependent work. Nothing was looked
    // up, minted or sent when the response came back — it all waits in the spawner.
    #[tokio::test]
    async fn answers_before_any_account_dependent_work() {
        let fx = build(Options::default());

        let response = fx
            .request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap();

        assert!(!response.session_code.is_empty());
        assert_eq!(fx.spawner.pending(), 1, "the dispatch is queued, not run");
        assert!(fx.emails.recipients().is_empty(), "no email yet");
        assert!(fx.tokens.all().is_empty(), "no token yet");
    }

    // Unknown, inactive and unverified accounts get the same kind of answer as a
    // real one: a fresh waiting session. Only an eligible account ever gets an email.
    #[tokio::test]
    async fn every_email_gets_the_same_answer_but_only_an_eligible_account_gets_a_link() {
        for (label, email, active, verified, expect_email) in [
            ("eligible", None, true, true, true),
            ("unknown", Some(UNKNOWN_EMAIL), true, true, false),
            ("inactive", None, false, true, false),
            ("unverified", None, true, false, false),
        ] {
            let fx = build(Options::default());
            fx.set_user(|u| with_flags(u, active, verified));
            let email = email.unwrap_or(&fx.email).to_string();

            let response = fx
                .request
                .execute(fx.request_command(&email))
                .await
                .unwrap_or_else(|e| panic!("{label}: {e:?}"));
            assert!(!response.session_code.is_empty(), "{label}");
            assert_eq!(fx.sessions_count(), 1, "{label}: a session either way");

            fx.spawner.run_all().await;
            assert_eq!(
                fx.emails.recipients().len(),
                usize::from(expect_email),
                "{label}"
            );
            assert_eq!(fx.tokens.all().len(), usize::from(expect_email), "{label}");
        }
    }

    #[tokio::test]
    async fn opens_a_waiting_session_that_binds_nobody_and_freezes_the_request() {
        let fx = build(Options::default());

        fx.request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap();

        let session = fx
            .sessions
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .next()
            .cloned()
            .unwrap();
        assert_eq!(session.primary_factor(), PrimaryFactor::MagicLink);
        assert!(session.is_awaiting_magic_link());
        assert!(session.user_id().is_none(), "the email was not looked up");
        assert_eq!(session.pending_client_id(), fx.client.id());
        assert_eq!(session.pending_state(), "xyz");
        assert_eq!(session.pending_code_challenge(), "challenge");
    }

    #[tokio::test]
    async fn emails_a_link_tied_to_the_waiting_session() {
        let fx = build(Options::default());

        let (session_code, link) = fx.request_link().await;

        assert_eq!(fx.emails.recipients(), vec![fx.email.clone()]);
        let sent = fx.emails.sent.lock().unwrap().last().cloned().unwrap();
        assert!(
            sent.text
                .contains(&format!("{AUTH_UI}/auth/{PUBLIC_CLIENT_ID}/magic-link#id=")),
            "link points at the hosted magic-link page: {}",
            sent.text
        );

        let token = fx.token(link.id);
        assert_eq!(
            token.token_hash(),
            sha256_hex(&link.token),
            "only the hash is stored"
        );
        assert_ne!(token.token_hash(), link.token);
        assert_eq!(token.user_id(), fx.user_id);
        let session = fx
            .sessions
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .find(|s| s.session_token_hash() == sha256_hex(&session_code))
            .cloned()
            .unwrap();
        assert_eq!(token.auth_session_id(), session.id());

        let sent_events: Vec<_> = fx
            .audit_actions()
            .into_iter()
            .filter(|(action, _, _)| action == USER_MAGIC_LINK_SENT)
            .collect();
        assert_eq!(sent_events.len(), 1);
        assert_eq!(sent_events[0].1, Some(fx.user_id));
    }

    #[tokio::test]
    async fn a_second_request_within_the_cooldown_sends_nothing() {
        let fx = build(Options::default());

        fx.request_link().await;
        fx.request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap();
        fx.spawner.run_all().await;

        assert_eq!(
            fx.emails.recipients().len(),
            1,
            "one email per cooldown window"
        );
        assert_eq!(fx.tokens.all().len(), 1);
    }

    // A new request must not cancel a link that is already on its way to the user:
    // anyone who knows the email can make one.
    #[tokio::test]
    async fn a_request_does_not_invalidate_another_sessions_link() {
        let fx = build(Options::default());
        let (_, first) = fx.request_link().await;

        // Past the cooldown: age the first token.
        {
            let mut data = fx.tokens.memory.data.lock().unwrap();
            let t = data.get(&first.id).cloned().unwrap();
            let aged = crate::domain::entities::magic_link_token::MagicLinkToken::reconstruct(
                t.id(),
                t.tenant_id(),
                t.user_id(),
                t.auth_session_id(),
                t.token_hash().to_string(),
                None,
                t.expires_at(),
                t.created_at() - chrono::Duration::minutes(5),
            );
            data.insert(t.id(), aged);
        }
        fx.request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap();
        fx.spawner.run_all().await;

        assert_eq!(fx.emails.recipients().len(), 2);
        assert!(!fx.token(first.id).is_consumed(), "first link still live");
    }

    #[tokio::test]
    async fn a_client_without_magic_links_refuses_outright() {
        for options in [
            Options {
                magic_link_enabled: false,
                ..Options::default()
            },
            Options {
                external_login_only: true,
                ..Options::default()
            },
        ] {
            let fx = build(options);
            let err = fx
                .request
                .execute(fx.request_command(&fx.email))
                .await
                .unwrap_err();
            assert!(matches!(err, DomainError::AccessDenied(_)), "{err:?}");
            assert_eq!(fx.sessions_count(), 0);
            assert_eq!(fx.spawner.pending(), 0);
        }
    }

    #[tokio::test]
    async fn a_malformed_authorization_request_is_refused() {
        let fx = build(Options::default());

        let mut bad_redirect = fx.request_command(&fx.email);
        bad_redirect.redirect_uri = "https://evil.example/cb".to_string();
        let mut bad_pkce = fx.request_command(&fx.email);
        bad_pkce.code_challenge_method = "plain".to_string();

        for command in [bad_redirect, bad_pkce] {
            let err = fx.request.execute(command).await.unwrap_err();
            assert!(matches!(err, DomainError::InvalidRequest(_)), "{err:?}");
        }
        assert_eq!(fx.sessions_count(), 0);
    }

    #[tokio::test]
    async fn a_blocked_ip_gets_no_session_and_no_email() {
        let fx = build(Options {
            risk: FakeRiskEngine::blocking_pre_auth(),
            ..Options::default()
        });

        let err = fx
            .request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap_err();

        assert!(
            matches!(err, DomainError::AuthenticationError(_)),
            "{err:?}"
        );
        assert_eq!(fx.sessions_count(), 0);
        assert_eq!(fx.spawner.pending(), 0);
        assert!(fx.risk.recorded()[0].pre_auth_blocked);
    }

    // The admin switched the feature off between the request and the dispatch.
    #[tokio::test]
    async fn the_dispatch_rechecks_the_client() {
        let fx = build(Options::default());
        fx.request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap();

        fx.set_client(|c| c.set_magic_link_enabled(false));
        fx.spawner.run_all().await;

        assert!(fx.emails.recipients().is_empty());
        assert!(fx.tokens.all().is_empty());
    }
}
