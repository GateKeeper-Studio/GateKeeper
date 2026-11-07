use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory, use_cases::security::views::LoginEventLabelView,
    },
    domain::{
        entities::{
            audit_action::SECURITY_LOGIN_EVENT_LABELED,
            audit_event::{ActorType, AuditEvent, Outcome},
            login_event_label::{LoginEventLabel, RiskLabel},
        },
        errors::DomainError,
    },
};

const MAX_NOTE_CHARS: usize = 500;

// An admin's verdict on a login event: "legitimate" (e.g. a false positive) or
// "fraudulent" (a confirmed attack). The statistical risk model trains on these
// (docs/RISK-SCORING.md); an admin label always outranks the weak labels taken from
// step-up MFA outcomes. `label: None` clears it.
pub struct Command {
    pub tenant_id: Uuid,
    pub login_event_id: Uuid,
    pub label: Option<String>,
    pub note: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
    pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
        Self {
            unit_of_work_factory,
        }
    }

    pub async fn execute(
        &self,
        actor: &Actor,
        command: Command,
    ) -> Result<Option<LoginEventLabelView>, DomainError> {
        let label = match command.label.as_deref() {
            Some(value) => Some(RiskLabel::parse(value).ok_or_else(|| {
                DomainError::InvalidRequest(
                    "label must be \"legitimate\" or \"fraudulent\"".to_string(),
                )
            })?),
            None => None,
        };
        let note = command
            .note
            .map(|n| n.trim().chars().take(MAX_NOTE_CHARS).collect::<String>())
            .filter(|n| !n.is_empty());

        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        uow.login_event_repository()
            .find_by_id(command.tenant_id, command.login_event_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("Login event not found".to_string()))?;

        let previous = uow
            .login_event_label_repository()
            .find(command.tenant_id, command.login_event_id)
            .await?;

        let saved = match label {
            Some(label) => {
                let labeled = LoginEventLabel::by_admin(
                    command.tenant_id,
                    command.login_event_id,
                    label,
                    actor.id(),
                    note,
                );
                uow.login_event_label_repository().upsert(&labeled).await?;
                Some(labeled)
            }
            None => {
                uow.login_event_label_repository()
                    .delete(command.tenant_id, command.login_event_id)
                    .await?;
                None
            }
        };

        uow.audit_event_repository()
            .record(
                &AuditEvent::new(
                    Some(command.tenant_id),
                    ActorType::Admin,
                    actor.id(),
                    SECURITY_LOGIN_EVENT_LABELED,
                    Outcome::Success,
                )
                .with_resource("login_event", command.login_event_id)
                .with_request_context(command.ip_address, command.user_agent)
                .with_metadata(json!({
                    "label": saved.as_ref().map(|l| l.label().as_str()),
                    "previous_label": previous.as_ref().map(|l| l.label().as_str()),
                    "previous_source": previous.as_ref().map(|l| l.source().as_str()),
                })),
            )
            .await?;
        uow.commit().await?;

        Ok(saved.map(LoginEventLabelView::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            entities::{
                login_event::LoginEvent,
                login_event_label::{LabelSource, LoginEventLabel},
            },
            repositories::{
                login_event_label_repository::LoginEventLabelRepository,
                login_event_repository::LoginEventRepository,
            },
        },
        testing::{
            in_memory_repositories::{
                in_memory_audit_event_repository::InMemoryAuditEventRepository,
                in_memory_login_event_label_repository::InMemoryLoginEventLabelRepository,
                in_memory_login_event_repository::InMemoryLoginEventRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    struct Fx {
        use_case: UseCase,
        labels: Arc<InMemoryLoginEventLabelRepository>,
        audit: Arc<InMemoryAuditEventRepository>,
        tenant_id: Uuid,
        event_id: Uuid,
    }

    async fn fixture() -> Fx {
        let tenant_id = Uuid::now_v7();
        let events = Arc::new(InMemoryLoginEventRepository::new());
        let event = LoginEvent::new(tenant_id, Some(Uuid::now_v7()), None, true, true);
        events.add(&event, &[]).await.unwrap();
        let labels = Arc::new(InMemoryLoginEventLabelRepository::with_events(
            events.clone(),
        ));
        let audit = Arc::new(InMemoryAuditEventRepository::new());
        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_login_event_repository(events)
        .with_login_event_label_repository(labels.clone())
        .with_audit_event_repository(audit.clone());
        Fx {
            use_case: UseCase::new(Arc::new(factory)),
            labels,
            audit,
            tenant_id,
            event_id: event.id(),
        }
    }

    fn admin(fx: &Fx) -> Actor {
        Actor::TenantAdmin {
            tenant_id: fx.tenant_id,
            user_id: Uuid::now_v7(),
        }
    }

    fn command(fx: &Fx, label: Option<&str>) -> Command {
        Command {
            tenant_id: fx.tenant_id,
            login_event_id: fx.event_id,
            label: label.map(str::to_string),
            note: Some("  checked with the user  ".to_string()),
            ip_address: Some("203.0.113.7".to_string()),
            user_agent: None,
        }
    }

    #[tokio::test]
    async fn an_admin_labels_an_event_and_it_is_audited() {
        let fx = fixture().await;
        let actor = admin(&fx);

        let view = fx
            .use_case
            .execute(&actor, command(&fx, Some("legitimate")))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(view.label, "legitimate");
        assert_eq!(view.source, "admin");
        assert_eq!(view.note.as_deref(), Some("checked with the user"));
        assert_eq!(view.labeled_by, actor.id());
        let events = fx.audit.events.lock().unwrap().clone();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action(), SECURITY_LOGIN_EVENT_LABELED);
        assert_eq!(events[0].resource_id(), Some(fx.event_id));
        assert_eq!(events[0].metadata()["label"], "legitimate");
    }

    // An admin's verdict replaces a weak MFA-outcome label.
    #[tokio::test]
    async fn an_admin_label_replaces_a_weak_one() {
        let fx = fixture().await;
        fx.labels
            .upsert(&LoginEventLabel::from_mfa(fx.tenant_id, fx.event_id, false))
            .await
            .unwrap();

        fx.use_case
            .execute(&admin(&fx), command(&fx, Some("legitimate")))
            .await
            .unwrap();

        let label = fx
            .labels
            .find(fx.tenant_id, fx.event_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(label.source(), LabelSource::Admin);
        assert_eq!(
            fx.audit.events.lock().unwrap().clone()[0].metadata()["previous_source"],
            "mfa_failed"
        );
    }

    #[tokio::test]
    async fn clearing_removes_the_label() {
        let fx = fixture().await;
        fx.use_case
            .execute(&admin(&fx), command(&fx, Some("fraudulent")))
            .await
            .unwrap();

        let cleared = fx
            .use_case
            .execute(&admin(&fx), command(&fx, None))
            .await
            .unwrap();

        assert!(cleared.is_none());
        assert!(
            fx.labels
                .find(fx.tenant_id, fx.event_id)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn refuses_a_bad_label_an_unknown_event_and_a_non_admin() {
        let fx = fixture().await;

        let bad = fx
            .use_case
            .execute(&admin(&fx), command(&fx, Some("suspicious")))
            .await
            .unwrap_err();
        assert!(matches!(bad, DomainError::InvalidRequest(_)), "{bad:?}");

        let mut other_tenant = command(&fx, Some("legitimate"));
        other_tenant.login_event_id = Uuid::now_v7();
        let missing = fx
            .use_case
            .execute(&admin(&fx), other_tenant)
            .await
            .unwrap_err();
        assert!(matches!(missing, DomainError::NotFound(_)), "{missing:?}");

        let user = Actor::User {
            tenant_id: fx.tenant_id,
            user_id: Uuid::now_v7(),
        };
        let denied = fx
            .use_case
            .execute(&user, command(&fx, Some("legitimate")))
            .await
            .unwrap_err();
        assert!(matches!(denied, DomainError::AccessDenied(_)), "{denied:?}");

        assert!(fx.labels.all().is_empty());
        assert!(fx.audit.events.lock().unwrap().clone().is_empty());
    }
}
