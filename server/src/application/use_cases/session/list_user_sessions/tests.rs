#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use crate::{
        application::use_cases::session::list_user_sessions::query::Query,
        domain::errors::DomainError, testing::session_fixture::build,
    };

    #[tokio::test]
    async fn a_rotated_family_is_one_session_with_its_first_and_last_times() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(2), 3);

        let sessions = fx
            .list()
            .execute(Query {
                tenant_id: fx.tenant_id,
                user_id: fx.alice,
            })
            .await
            .unwrap();

        assert_eq!(sessions.len(), 1, "four rows, one session");
        let s = &sessions[0];
        assert_eq!(s.id, family.family_id);
        assert_eq!(s.signed_in_at, family.signed_in_at);
        assert_eq!(s.last_active_at, family.last_used_at);
        assert_eq!(
            s.client_id,
            fx.app.client_id().to_string(),
            "the PUBLIC client id"
        );
        assert_eq!(s.client_name.as_deref(), Some("Web App"));
        assert_eq!(
            s.scopes,
            vec!["openid".to_string(), "offline_access".to_string()]
        );
        assert!(s.expires_at > Utc::now());
    }

    #[tokio::test]
    async fn ended_and_expired_sessions_are_not_listed() {
        let fx = build();
        let live = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(5), 1);

        let revoked = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(4), 1);
        fx.update_family(revoked.family_id, |t| t.revoke(Utc::now()));

        // Rotated with no live successor (reuse detection left it like this).
        let rotated_only =
            fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(3), 0);
        fx.update_family(rotated_only.family_id, |t| {
            let mut successor = t.clone();
            let _ = successor.rotate("x".to_string(), Utc::now(), Utc::now());
            *t = successor;
        });

        let expired = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(40), 0);

        let sessions = fx
            .list()
            .execute(Query {
                tenant_id: fx.tenant_id,
                user_id: fx.alice,
            })
            .await
            .unwrap();

        let ids: Vec<_> = sessions.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![live.family_id]);
        assert!(!ids.contains(&expired.family_id));
    }

    #[tokio::test]
    async fn only_this_users_sessions_in_this_tenant_newest_activity_first() {
        let fx = build();
        let older = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(3), 0);
        let newer = fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(2),
            1,
        );
        fx.seed_family(fx.tenant_id, fx.bob, fx.app.id(), Duration::hours(1), 0);
        // Alice's id, but rows of another tenant: not hers here.
        fx.seed_family(
            fx.other_tenant_id,
            fx.alice,
            fx.app.id(),
            Duration::hours(1),
            0,
        );

        let sessions = fx
            .list()
            .execute(Query {
                tenant_id: fx.tenant_id,
                user_id: fx.alice,
            })
            .await
            .unwrap();

        let ids: Vec<_> = sessions.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![newer.family_id, older.family_id]);
        assert_eq!(sessions[0].client_name.as_deref(), Some("Mobile App"));
    }

    #[tokio::test]
    async fn a_user_of_another_tenant_is_not_found() {
        let fx = build();

        let err = fx
            .list()
            .execute(Query {
                tenant_id: fx.tenant_id,
                user_id: fx.carol,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)), "{err:?}");
    }

    // Nothing sensitive leaves the server: the view has no IP, user agent or token
    // material — the JSON shows exactly these fields.
    #[tokio::test]
    async fn the_view_exposes_only_the_minimal_fields() {
        let fx = build();
        fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        let sessions = fx
            .list()
            .execute(Query {
                tenant_id: fx.tenant_id,
                user_id: fx.alice,
            })
            .await
            .unwrap();
        let json = serde_json::to_value(&sessions[0]).unwrap();
        let mut keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
        keys.sort();

        assert_eq!(
            keys,
            vec![
                "client_id",
                "client_name",
                "expires_at",
                "id",
                "last_active_at",
                "scopes",
                "signed_in_at"
            ]
        );
    }
}
