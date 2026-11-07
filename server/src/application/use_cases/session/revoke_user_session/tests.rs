#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use crate::{
        application::{
            shared::actor::Actor,
            use_cases::session::{
                list_user_sessions::query::Query, revoke_user_session::command::Command,
            },
        },
        domain::{
            entities::{audit_action::USER_SESSION_REVOKED, audit_event::ActorType},
            errors::DomainError,
        },
        testing::session_fixture::{SessionFixture, build},
    };

    fn command(fx: &SessionFixture, user_id: Uuid, session_id: Uuid) -> Command {
        Command {
            tenant_id: fx.tenant_id,
            user_id,
            session_id,
            ip_address: Some("203.0.113.9".to_string()),
            user_agent: Some("test-agent".to_string()),
        }
    }

    fn alice(fx: &SessionFixture) -> Actor {
        Actor::User {
            tenant_id: fx.tenant_id,
            user_id: fx.alice,
        }
    }

    fn all_revoked(fx: &SessionFixture, family_id: Uuid) -> bool {
        fx.family_rows(family_id)
            .iter()
            .all(|t| t.revoked_at().is_some())
    }

    #[tokio::test]
    async fn a_user_ends_one_of_their_sessions_and_only_that_one() {
        let fx = build();
        let ended = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(1), 2);
        let kept = fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(1),
            0,
        );

        fx.revoke()
            .execute(&alice(&fx), command(&fx, fx.alice, ended.family_id))
            .await
            .unwrap();

        assert!(all_revoked(&fx, ended.family_id), "every row of the family");
        assert!(!all_revoked(&fx, kept.family_id));
        let listed: Vec<_> = fx
            .list()
            .execute(Query {
                tenant_id: fx.tenant_id,
                user_id: fx.alice,
            })
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.id)
            .collect();
        assert_eq!(listed, vec![kept.family_id]);

        let events = fx.audit_events();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.action(), USER_SESSION_REVOKED);
        assert_eq!(event.actor_type(), ActorType::User);
        assert_eq!(event.actor_id(), Some(fx.alice));
        assert_eq!(event.resource_id(), Some(ended.family_id));
        assert_eq!(event.ip_address(), Some("203.0.113.9"));
    }

    // The app holding the session is told (OIDC back-channel logout), and only
    // about that session.
    #[tokio::test]
    async fn the_app_is_told_its_session_ended() {
        let fx = build();
        let ended = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(1), 1);
        fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        fx.revoke()
            .execute(&alice(&fx), command(&fx, fx.alice, ended.family_id))
            .await
            .unwrap();

        let notices = fx.logout_notices();
        assert_eq!(notices.len(), 1);
        assert_eq!(notices[0].sid, ended.family_id);
        assert_eq!(notices[0].user_id, fx.alice);
        assert_eq!(notices[0].client_id, fx.app.id());
    }

    #[tokio::test]
    async fn an_app_without_a_back_channel_uri_is_not_notified() {
        let fx = build();
        let family = fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(1),
            0,
        );

        fx.revoke()
            .execute(&alice(&fx), command(&fx, fx.alice, family.family_id))
            .await
            .unwrap();

        assert!(fx.logout_notices().is_empty());
    }

    // Already over, so already announced: nothing new to say.
    #[tokio::test]
    async fn ending_an_ended_session_announces_nothing() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        fx.update_family(family.family_id, |t| t.revoke(Utc::now()));

        fx.revoke()
            .execute(&alice(&fx), command(&fx, fx.alice, family.family_id))
            .await
            .unwrap();

        assert!(fx.logout_notices().is_empty());
    }

    // The point of ending a session: the app cannot refresh any more.
    #[tokio::test]
    async fn an_ended_session_can_no_longer_refresh() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        fx.revoke()
            .execute(&alice(&fx), command(&fx, fx.alice, family.family_id))
            .await
            .unwrap();
        let err = fx
            .refresh_grant()
            .execute(fx.refresh_command(&family.live_token, &fx.app))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::InvalidGrant(_)), "{err:?}");
    }

    #[tokio::test]
    async fn a_live_session_still_refreshes() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        fx.refresh_grant()
            .execute(fx.refresh_command(&family.live_token, &fx.app))
            .await
            .expect("the fixture's refresh grant works before any revocation");
    }

    #[tokio::test]
    async fn an_admin_ends_a_users_session() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        let admin_id = Uuid::now_v7();

        fx.revoke()
            .execute(
                &Actor::TenantAdmin {
                    tenant_id: fx.tenant_id,
                    user_id: admin_id,
                },
                command(&fx, fx.alice, family.family_id),
            )
            .await
            .unwrap();

        assert!(all_revoked(&fx, family.family_id));
        let event = &fx.audit_events()[0];
        assert_eq!(event.actor_type(), ActorType::Admin);
        assert_eq!(event.actor_id(), Some(admin_id));
    }

    #[tokio::test]
    async fn another_end_user_is_refused() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        let err = fx
            .revoke()
            .execute(
                &Actor::User {
                    tenant_id: fx.tenant_id,
                    user_id: fx.bob,
                },
                command(&fx, fx.alice, family.family_id),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::AccessDenied(_)), "{err:?}");
        assert!(!all_revoked(&fx, family.family_id));
        assert!(fx.audit_events().is_empty());
    }

    // Bob cannot end Alice's session through his own account by passing her family
    // id: the revoke is scoped to the user, so her session is simply not his.
    #[tokio::test]
    async fn another_users_session_id_is_not_found_and_untouched() {
        let fx = build();
        let alices = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        for session_id in [alices.family_id, Uuid::now_v7()] {
            let err = fx
                .revoke()
                .execute(
                    &Actor::User {
                        tenant_id: fx.tenant_id,
                        user_id: fx.bob,
                    },
                    command(&fx, fx.bob, session_id),
                )
                .await
                .unwrap_err();
            assert!(matches!(err, DomainError::NotFound(_)), "{err:?}");
        }
        assert!(!all_revoked(&fx, alices.family_id));
        assert!(fx.audit_events().is_empty());
        assert!(
            fx.logout_notices().is_empty(),
            "nothing about Alice's session"
        );
    }

    #[tokio::test]
    async fn ending_an_already_ended_session_is_fine() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);
        fx.update_family(family.family_id, |t| t.revoke(Utc::now()));

        fx.revoke()
            .execute(&alice(&fx), command(&fx, fx.alice, family.family_id))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_user_of_another_tenant_is_not_found() {
        let fx = build();

        let err = fx
            .revoke()
            .execute(&Actor::System, command(&fx, fx.carol, Uuid::now_v7()))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)), "{err:?}");
    }
}
