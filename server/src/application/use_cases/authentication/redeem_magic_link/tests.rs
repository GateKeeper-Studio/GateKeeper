#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::authentication::login_flow::{AuthorizationRequest, open_session},
        domain::{
            entities::{
                audit_action::{USER_LOGIN_FAILURE, USER_LOGIN_SUCCESS},
                auth_session::PrimaryFactor,
                login_event::RiskDecision,
                magic_link_token::MagicLinkToken,
                mfa_method::MfaMethod,
                oauth_client::MfaPolicy,
            },
            errors::DomainError,
        },
        testing::{
            fake_risk_engine::FakeRiskEngine,
            magic_link_fixture::{Fixture, Link, Options, REDIRECT_URI, build, with_flags},
        },
        utils::oauth_crypto::{generate_opaque_token, sha256_hex},
    };

    fn is_generic_failure(err: &DomainError) -> bool {
        matches!(err, DomainError::AuthenticationError(m) if m == "this sign-in link is invalid or has expired")
    }

    // Seed a waiting session and its link directly, for fixtures whose risk engine
    // would refuse the request itself.
    fn seed_link(fx: &Fixture) -> (String, Link) {
        let request = AuthorizationRequest {
            redirect_uri: REDIRECT_URI,
            response_type: "code",
            scope: "openid email",
            state: "xyz",
            code_challenge: "challenge",
            code_challenge_method: "S256",
            nonce: None,
        };
        let (mut session, session_code) = open_session(
            &fx.client,
            &request,
            vec!["openid".to_string()],
            Duration::minutes(15),
        );
        session.await_magic_link();
        let raw = generate_opaque_token();
        let token = MagicLinkToken::new(
            fx.tenant_id,
            fx.user_id,
            session.id(),
            sha256_hex(&raw),
            Utc::now() + Duration::minutes(15),
        );
        let link = Link {
            id: token.id(),
            token: raw,
        };
        fx.sessions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(session.id(), session);
        fx.tokens
            .memory
            .data
            .lock()
            .unwrap()
            .insert(token.id(), token);
        (session_code, link)
    }

    fn last_failure_reason(fx: &Fixture) -> Option<String> {
        fx.risk
            .recorded()
            .last()
            .and_then(|r| r.failure_reason.clone())
    }

    fn failure_audits(fx: &Fixture) -> Vec<serde_json::Value> {
        fx.audit_actions()
            .into_iter()
            .filter(|(a, _, _)| a == USER_LOGIN_FAILURE)
            .map(|(_, _, m)| m)
            .collect()
    }

    #[tokio::test]
    async fn redeeming_in_the_requesting_browser_signs_the_user_in() {
        let fx = build(Options::default());
        let (session_code, link) = fx.request_link().await;

        let response = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();

        assert_eq!(response.user_id, Some(fx.user_id.to_string()));
        assert!(response.mfa_type.is_none(), "no MFA policy");
        assert!(response.change_password_code.is_none());
        assert_ne!(response.session_code, session_code, "the code rotates");

        let token = fx.token(link.id);
        assert!(token.is_consumed(), "single use");
        let session = fx.session(token.auth_session_id());
        assert_eq!(session.primary_factor(), PrimaryFactor::MagicLink);
        assert_eq!(session.user_id(), Some(fx.user_id));
        assert!(session.authenticated_at().is_some());
        assert_eq!(
            session.session_token_hash(),
            sha256_hex(&response.session_code)
        );

        let recorded = fx.risk.recorded();
        assert_eq!(recorded.len(), 1);
        assert!(recorded[0].credentials_valid);
        assert_eq!(recorded[0].user_id, Some(fx.user_id));

        let success: Vec<_> = fx
            .audit_actions()
            .into_iter()
            .filter(|(a, _, _)| a == USER_LOGIN_SUCCESS)
            .collect();
        assert_eq!(success.len(), 1);
        assert_eq!(success[0].1, Some(fx.user_id));
        assert_eq!(success[0].2["method"], "magic_link");
    }

    // The whole point of the session: before the click it cannot be spent, after it
    // the normal authorize step mints a code — with the rotated code only.
    #[tokio::test]
    async fn the_session_is_spendable_only_after_redemption_and_only_with_the_new_code() {
        let fx = build(Options::default());
        let (session_code, link) = fx.request_link().await;

        let err = fx
            .authorize
            .execute(fx.authorize_command(&session_code))
            .await
            .unwrap_err();
        assert!(
            matches!(err, DomainError::AuthenticationError(_)),
            "{err:?}"
        );

        let response = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();

        assert!(
            fx.authorize
                .execute(fx.authorize_command(&session_code))
                .await
                .is_err(),
            "the old code stopped resolving"
        );
        let code = fx
            .authorize
            .execute(fx.authorize_command(&response.session_code))
            .await
            .unwrap();
        assert_eq!(code.state, "xyz");
        assert_eq!(code.redirect_uri, REDIRECT_URI);
    }

    // Racing defence: a forwarded copy, or a link scanner, has the token but not the
    // session code. It is refused, and the real user's link is NOT burned.
    #[tokio::test]
    async fn a_link_opened_without_the_requesting_session_is_refused_and_not_burned() {
        let fx = build(Options::default());
        let (session_code, link) = fx.request_link().await;

        // Another browser: its own session code, or a made-up one.
        let other_browser = fx
            .request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap()
            .session_code;
        for code in [other_browser.as_str(), "not-a-session"] {
            let err = fx
                .redeem
                .execute(fx.redeem_command(&link, code))
                .await
                .unwrap_err();
            assert!(is_generic_failure(&err), "{err:?}");
            assert_eq!(
                last_failure_reason(&fx).as_deref(),
                Some("magic_link_wrong_browser")
            );
        }
        assert!(!fx.token(link.id).is_consumed(), "the link survives");
        assert_eq!(failure_audits(&fx).len(), 2);

        // ...and still works in the browser that asked for it.
        fx.redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_link_works_once() {
        let fx = build(Options::default());
        let (session_code, link) = fx.request_link().await;

        fx.redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();
        let err = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap_err();

        assert!(is_generic_failure(&err), "{err:?}");
    }

    #[tokio::test]
    async fn a_tampered_token_is_refused() {
        let fx = build(Options::default());
        let (session_code, mut link) = fx.request_link().await;
        link.token.push('x');

        let err = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap_err();

        assert!(is_generic_failure(&err), "{err:?}");
        assert_eq!(
            last_failure_reason(&fx).as_deref(),
            Some("invalid_magic_link")
        );
        assert!(fx.risk.recorded().last().unwrap().user_id.is_none());
    }

    #[tokio::test]
    async fn an_expired_link_or_session_is_refused() {
        // Expired token.
        let fx = build(Options::default());
        let (session_code, link) = fx.request_link().await;
        {
            let mut data = fx.tokens.memory.data.lock().unwrap();
            let t = data.get(&link.id).cloned().unwrap();
            data.insert(
                t.id(),
                MagicLinkToken::reconstruct(
                    t.id(),
                    t.tenant_id(),
                    t.user_id(),
                    t.auth_session_id(),
                    t.token_hash().to_string(),
                    None,
                    Utc::now() - Duration::seconds(1),
                    t.created_at(),
                ),
            );
        }
        let err = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap_err();
        assert!(is_generic_failure(&err), "{err:?}");

        // Expired session.
        let fx = build(Options::default());
        let (session_code, link) = fx.request_link().await;
        {
            let session_id = fx.token(link.id).auth_session_id();
            let mut data = fx.sessions.memory.data.lock().unwrap();
            let session = data.get_mut(&session_id).unwrap();
            session.expire_now(Utc::now() - Duration::seconds(1));
        }
        let err = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap_err();
        assert!(is_generic_failure(&err), "{err:?}");
        assert!(!fx.token(link.id).is_consumed());
    }

    #[tokio::test]
    async fn an_account_that_changed_since_the_email_is_refused() {
        for (active, verified) in [(false, true), (true, false)] {
            let fx = build(Options::default());
            let (session_code, link) = fx.request_link().await;
            fx.set_user(|u| with_flags(u, active, verified));

            let err = fx
                .redeem
                .execute(fx.redeem_command(&link, &session_code))
                .await
                .unwrap_err();

            assert!(is_generic_failure(&err), "{err:?}");
            assert_eq!(
                last_failure_reason(&fx).as_deref(),
                Some("account_unavailable")
            );
            assert!(!fx.token(link.id).is_consumed());
        }
    }

    #[tokio::test]
    async fn a_client_that_switched_magic_links_off_refuses_old_links() {
        let fx = build(Options::default());
        let (session_code, link) = fx.request_link().await;
        fx.set_client(|c| c.set_magic_link_enabled(false));

        let err = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)), "{err:?}");
        assert!(!fx.token(link.id).is_consumed());
    }

    #[tokio::test]
    async fn a_blocked_ip_cannot_redeem() {
        let fx = build(Options {
            risk: FakeRiskEngine::blocking_pre_auth(),
            ..Options::default()
        });
        let (session_code, link) = seed_link(&fx);

        let err = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap_err();

        assert!(is_generic_failure(&err), "{err:?}");
        assert!(fx.risk.recorded().last().unwrap().pre_auth_blocked);
        assert!(!fx.token(link.id).is_consumed());
    }

    #[tokio::test]
    async fn a_risk_block_burns_the_link_and_the_session() {
        let fx = build(Options {
            risk: FakeRiskEngine::with_decision(RiskDecision::BlockTemporarily),
            ..Options::default()
        });
        let (session_code, link) = fx.request_link().await;

        let err = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap_err();

        assert!(
            matches!(&err, DomainError::AuthenticationError(m) if m.contains("blocked")),
            "{err:?}"
        );
        let token = fx.token(link.id);
        assert!(token.is_consumed());
        assert!(fx.session(token.auth_session_id()).is_expired(Utc::now()));
        assert_eq!(failure_audits(&fx)[0]["reason"], "risk_blocked");
        assert_eq!(fx.risk.recorded().len(), 1, "evaluated once, as valid");
    }

    // After a magic link the mailbox is the primary factor: the second factor may be
    // TOTP or a passkey, never an email code.
    #[tokio::test]
    async fn mfa_after_a_magic_link_never_offers_an_email_code() {
        let required = MfaPolicy {
            required: true,
            email_enabled: true,
            totp_enabled: true,
            webauthn_enabled: true,
        };
        let fx = build(Options {
            mfa_policy: required,
            ..Options::default()
        });
        fx.seed_totp();
        let (session_code, link) = fx.request_link().await;
        let emails_before = fx.emails.recipients().len();

        let response = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();

        assert_eq!(response.mfa_type.as_deref(), Some("totp"));
        assert_eq!(response.available_mfa_methods, vec!["totp".to_string()]);
        assert!(
            !response
                .available_mfa_methods
                .contains(&"email".to_string())
        );
        assert_eq!(fx.emails.recipients().len(), emails_before, "no OTP mailed");
        let session = fx.session(fx.token(link.id).auth_session_id());
        assert!(session.is_mfa_pending());
        assert!(!session.accepts_mfa_method(&MfaMethod::Email));
    }

    #[tokio::test]
    async fn risk_step_up_is_satisfied_by_a_passkey_not_email() {
        let fx = build(Options {
            risk: FakeRiskEngine::with_decision(RiskDecision::AllowWithMfa),
            ..Options::default()
        });
        fx.seed_webauthn();
        let (session_code, link) = fx.request_link().await;

        let response = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();

        assert_eq!(response.mfa_type.as_deref(), Some("webauthn"));
        assert!(response.web_authn_options.is_some());
        assert!(response.mfa_id.is_some());
    }

    #[tokio::test]
    async fn a_user_without_a_second_factor_is_sent_to_enroll_one() {
        let fx = build(Options {
            mfa_policy: MfaPolicy {
                required: true,
                email_enabled: true,
                totp_enabled: true,
                webauthn_enabled: false,
            },
            ..Options::default()
        });
        let (session_code, link) = fx.request_link().await;

        let response = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();

        assert!(response.mfa_enrollment_required);
        assert_eq!(response.enrollable_methods, vec!["totp".to_string()]);
        assert!(
            response.available_mfa_methods.is_empty(),
            "email is not offered"
        );
    }

    // A client whose MFA policy only an email code could satisfy cannot complete a
    // magic-link sign-in at all, so it does not offer one.
    #[tokio::test]
    async fn a_client_that_only_allows_email_mfa_does_not_offer_magic_links() {
        let fx = build(Options {
            mfa_policy: MfaPolicy {
                required: true,
                email_enabled: true,
                totp_enabled: false,
                webauthn_enabled: false,
            },
            ..Options::default()
        });

        let err = fx
            .request
            .execute(fx.request_command(&fx.email))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)), "{err:?}");
    }

    #[tokio::test]
    async fn a_forced_password_change_still_applies() {
        let fx = build(Options {
            should_change_password: true,
            ..Options::default()
        });
        let (session_code, link) = fx.request_link().await;

        let response = fx
            .redeem
            .execute(fx.redeem_command(&link, &session_code))
            .await
            .unwrap();

        assert!(response.change_password_code.is_some());
    }
}
