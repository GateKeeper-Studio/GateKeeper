#[cfg(test)]
mod tests {
    use chrono::Duration;
    use uuid::Uuid;

    use crate::{
        application::{
            shared::actor::Actor, use_cases::session::revoke_all_user_sessions::command::Command,
        },
        domain::{
            entities::{audit_action::USER_SESSION_REVOKED, audit_event::ActorType},
            errors::DomainError,
        },
        testing::session_fixture::{SessionFixture, build},
    };

    fn command(fx: &SessionFixture, user_id: Uuid) -> Command {
        Command {
            tenant_id: fx.tenant_id,
            user_id,
            ip_address: None,
            user_agent: None,
        }
    }

    fn live(fx: &SessionFixture, family_id: Uuid) -> bool {
        fx.family_rows(family_id)
            .iter()
            .any(|t| t.revoked_at().is_none())
    }

    #[tokio::test]
    async fn signs_the_user_out_of_every_app_and_nobody_else() {
        let fx = build();
        let a1 = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::days(1), 2);
        let a2 = fx.seed_family(
            fx.tenant_id,
            fx.alice,
            fx.other_app.id(),
            Duration::hours(1),
            0,
        );
        let bobs = fx.seed_family(fx.tenant_id, fx.bob, fx.app.id(), Duration::hours(1), 0);

        fx.revoke_all()
            .execute(
                &Actor::User {
                    tenant_id: fx.tenant_id,
                    user_id: fx.alice,
                },
                command(&fx, fx.alice),
            )
            .await
            .unwrap();

        assert!(!live(&fx, a1.family_id));
        assert!(!live(&fx, a2.family_id));
        assert!(live(&fx, bobs.family_id), "other users keep their sessions");

        let events = fx.audit_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action(), USER_SESSION_REVOKED);
        assert_eq!(events[0].actor_type(), ActorType::User);
        assert_eq!(events[0].metadata()["all"], true);
    }

    #[tokio::test]
    async fn an_admin_may_another_end_user_may_not() {
        let fx = build();
        let family = fx.seed_family(fx.tenant_id, fx.alice, fx.app.id(), Duration::hours(1), 0);

        let err = fx
            .revoke_all()
            .execute(
                &Actor::User {
                    tenant_id: fx.tenant_id,
                    user_id: fx.bob,
                },
                command(&fx, fx.alice),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)), "{err:?}");
        assert!(live(&fx, family.family_id));

        fx.revoke_all()
            .execute(
                &Actor::TenantAdmin {
                    tenant_id: fx.tenant_id,
                    user_id: Uuid::now_v7(),
                },
                command(&fx, fx.alice),
            )
            .await
            .unwrap();
        assert!(!live(&fx, family.family_id));
        assert_eq!(fx.audit_events()[0].actor_type(), ActorType::Admin);
    }

    #[tokio::test]
    async fn a_user_with_no_sessions_is_fine() {
        let fx = build();

        fx.revoke_all()
            .execute(&Actor::System, command(&fx, fx.alice))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn a_user_of_another_tenant_is_not_found() {
        let fx = build();

        let err = fx
            .revoke_all()
            .execute(&Actor::System, command(&fx, fx.carol))
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)), "{err:?}");
    }
}
