#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use chrono::{Duration, Utc};
    use serde_json::{Value, json};
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth::end_session::command::Command,
        domain::{
            entities::{
                audit_action::USER_LOGOUT, audit_event::ActorType, oauth_client::OAuthClient,
            },
            errors::DomainError,
        },
        testing::{
            oauth_fakes::FakeTokenVerifier,
            session_fixture::{POST_LOGOUT_URI, SessionFixture, build},
        },
    };

    // A JWT-shaped hint whose payload carries `claims`, and a verifier that accepts it
    // with `verified` claims (normally the same).
    fn hint(claims: &Value) -> String {
        format!(
            "eyJhbGciOiJFUzI1NiIsInR5cCI6IkpXVCJ9.{}.signature",
            URL_SAFE_NO_PAD.encode(claims.to_string())
        )
    }

    fn id_token_claims(fx: &SessionFixture, client: &OAuthClient, sid: Option<Uuid>) -> Value {
        let mut claims = json!({
            "iss": "https://auth.test",
            "sub": fx.alice.to_string(),
            "aud": client.client_id().to_string(),
            "iat": Utc::now().timestamp() - 7200,
            // Long expired: a hint is still good.
            "exp": Utc::now().timestamp() - 3600,
        });
        if let Some(sid) = sid {
            claims["sid"] = json!(sid.to_string());
        }
        claims
    }

    fn verifying(token: &str, claims: Value) -> FakeTokenVerifier {
        FakeTokenVerifier::new().with_valid_id_token_hint(token, claims)
    }

    fn command(token: &str) -> Command {
        Command {
            id_token_hint: Some(token.to_string()),
            ip_address: Some("203.0.113.9".to_string()),
            ..Command::default()
        }
    }

    fn live_families(fx: &SessionFixture, user_id: Uuid) -> usize {
        let mut families: Vec<Uuid> = fx
            .refresh
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .filter(|t| t.user_id() == user_id && t.is_active(Utc::now()))
            .map(|t| t.family_id())
            .collect();
        families.sort_unstable();
        families.dedup();
        families.len()
    }

    #[tokio::test]
    async fn signs_the_user_out_of_every_app_and_tells_the_others() {
        let fx = build();
        // Signing out from the unregistered app; Alice also has two sessions with the
        // registered app (two devices), and Bob has one.
        let asking = fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(2),
            1,
        );
        let laptop = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(3), 2);
        let phone = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(5), 0);
        let bobs = fx.seed_family(fx.tenant_id, fx.bob, fx.app.id(), Duration::hours(1), 0);
        let claims = id_token_claims(&fx, &fx.other_app, Some(asking.family_id));
        let token = hint(&claims);

        let response = fx
            .end_session(verifying(&token, claims))
            .execute(command(&token))
            .await
            .unwrap();

        assert_eq!(response.sessions_ended, 3);
        assert_eq!(live_families(&fx, fx.alice), 0);
        assert_eq!(live_families(&fx, fx.bob), 1, "nobody else is signed out");

        let mut notified: Vec<Uuid> = fx.logout_notices().iter().map(|n| n.sid).collect();
        notified.sort_unstable();
        let mut expected = vec![laptop.family_id, phone.family_id];
        expected.sort_unstable();
        assert_eq!(
            notified, expected,
            "the registered app hears about both devices"
        );
        assert!(!notified.contains(&bobs.family_id));

        let events = fx.audit_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action(), USER_LOGOUT);
        assert_eq!(events[0].actor_type(), ActorType::User);
        assert_eq!(events[0].actor_id(), Some(fx.alice));
        assert_eq!(events[0].metadata()["sessions_ended"], 3);
    }

    // The asking app already ended its own session; telling it again is noise.
    #[tokio::test]
    async fn the_asking_app_is_not_notified_about_its_own_session() {
        let fx = build();
        let asking = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(2), 0);
        let other_device =
            fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        let claims = id_token_claims(&fx, &fx.app, Some(asking.family_id));
        let token = hint(&claims);

        fx.end_session(verifying(&token, claims))
            .execute(command(&token))
            .await
            .unwrap();

        let notified: Vec<Uuid> = fx.logout_notices().iter().map(|n| n.sid).collect();
        assert_eq!(notified, vec![other_device.family_id]);
    }

    #[tokio::test]
    async fn redirects_to_a_registered_uri_with_the_state() {
        let fx = build();
        let asking = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(2), 0);
        let claims = id_token_claims(&fx, &fx.app, Some(asking.family_id));
        let token = hint(&claims);

        let response = fx
            .end_session(verifying(&token, claims))
            .execute(Command {
                post_logout_redirect_uri: Some(POST_LOGOUT_URI.to_string()),
                state: Some("xyz".to_string()),
                ..command(&token)
            })
            .await
            .unwrap();

        assert_eq!(
            response.post_logout_redirect_uri.as_deref(),
            Some(POST_LOGOUT_URI)
        );
        assert_eq!(response.state.as_deref(), Some("xyz"));
    }

    // An unregistered redirect is refused outright, before anything is ended: the
    // endpoint must not become an open redirect, nor half-succeed.
    #[tokio::test]
    async fn an_unregistered_redirect_is_refused_and_ends_nothing() {
        let fx = build();
        let asking = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(2), 0);
        let claims = id_token_claims(&fx, &fx.app, Some(asking.family_id));
        let token = hint(&claims);

        let err = fx
            .end_session(verifying(&token, claims))
            .execute(Command {
                post_logout_redirect_uri: Some("https://evil.example.com/".to_string()),
                ..command(&token)
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)), "{err:?}");
        assert_eq!(live_families(&fx, fx.alice), 1);
        assert!(fx.logout_notices().is_empty());
    }

    // A replayed hint from a session that is already over cannot sign the user out
    // of the sessions they still have.
    #[tokio::test]
    async fn a_hint_from_an_ended_session_ends_nothing_else() {
        let fx = build();
        let ended = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(2), 0);
        fx.update_family(ended.family_id, |t| t.revoke(Utc::now()));
        fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(1),
            0,
        );
        let claims = id_token_claims(&fx, &fx.app, Some(ended.family_id));
        let token = hint(&claims);

        let response = fx
            .end_session(verifying(&token, claims))
            .execute(Command {
                post_logout_redirect_uri: Some(POST_LOGOUT_URI.to_string()),
                ..command(&token)
            })
            .await
            .unwrap();

        assert_eq!(response.sessions_ended, 0);
        assert_eq!(
            response.post_logout_redirect_uri.as_deref(),
            Some(POST_LOGOUT_URI)
        );
        assert_eq!(live_families(&fx, fx.alice), 1);
        assert!(fx.logout_notices().is_empty());
        assert!(fx.audit_events().is_empty());
    }

    // A session id that is live but belongs to ANOTHER app does not count: the hint
    // must come from a session of the client it was issued to.
    #[tokio::test]
    async fn a_sid_from_another_app_ends_nothing() {
        let fx = build();
        let elsewhere = fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(1),
            0,
        );
        let claims = id_token_claims(&fx, &fx.app, Some(elsewhere.family_id));
        let token = hint(&claims);

        let response = fx
            .end_session(verifying(&token, claims))
            .execute(command(&token))
            .await
            .unwrap();

        assert_eq!(response.sessions_ended, 0);
        assert_eq!(live_families(&fx, fx.alice), 1);
    }

    // ID tokens issued before `sid` existed name no session.
    #[tokio::test]
    async fn a_hint_without_a_sid_ends_nothing() {
        let fx = build();
        fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        let claims = id_token_claims(&fx, &fx.app, None);
        let token = hint(&claims);

        let response = fx
            .end_session(verifying(&token, claims))
            .execute(command(&token))
            .await
            .unwrap();

        assert_eq!(response.sessions_ended, 0);
        assert_eq!(live_families(&fx, fx.alice), 1);
    }

    #[tokio::test]
    async fn a_hint_is_required_and_must_verify() {
        let fx = build();
        let asking = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        let token = hint(&id_token_claims(&fx, &fx.app, Some(asking.family_id)));

        for cmd in [
            Command::default(),
            command("not-a-jwt"),
            // Well-formed, but no verifier accepts it (bad signature, wrong issuer...).
            command(&token),
        ] {
            let err = fx
                .end_session(FakeTokenVerifier::new())
                .execute(cmd)
                .await
                .unwrap_err();
            assert!(matches!(err, DomainError::InvalidRequest(_)), "{err:?}");
        }
        assert_eq!(live_families(&fx, fx.alice), 1);
    }

    #[tokio::test]
    async fn a_client_id_that_is_not_the_hints_audience_is_refused() {
        let fx = build();
        let asking = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        let claims = id_token_claims(&fx, &fx.app, Some(asking.family_id));
        let token = hint(&claims);

        let err = fx
            .end_session(verifying(&token, claims))
            .execute(Command {
                client_id: Some(fx.other_app.client_id().to_string()),
                ..command(&token)
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)), "{err:?}");
        assert_eq!(live_families(&fx, fx.alice), 1);
    }

    // The unverified audience only picks the client; the VERIFIED one must agree.
    #[tokio::test]
    async fn the_verified_audience_must_be_the_client() {
        let fx = build();
        let asking = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        let token = hint(&id_token_claims(&fx, &fx.app, Some(asking.family_id)));
        let verified = id_token_claims(&fx, &fx.other_app, Some(asking.family_id));

        let err = fx
            .end_session(verifying(&token, verified))
            .execute(command(&token))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidRequest(_)), "{err:?}");
        assert_eq!(live_families(&fx, fx.alice), 1);
    }
}
