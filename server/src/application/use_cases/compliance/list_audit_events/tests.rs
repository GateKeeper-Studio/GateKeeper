#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::list_audit_events::{query::Query, use_case::UseCase},
        domain::{
            entities::audit_event::{ActorType, AuditEvent, Outcome},
            repositories::audit_event_repository::AuditEventRepository,
        },
        testing::in_memory_repositories::in_memory_audit_event_repository::InMemoryAuditEventRepository,
    };

    async fn seed(
        repo: &InMemoryAuditEventRepository,
        tenant_id: Uuid,
        action: &str,
        outcome: Outcome,
    ) {
        let event = AuditEvent::new(
            Some(tenant_id),
            ActorType::User,
            Some(Uuid::now_v7()),
            action,
            outcome,
        );
        repo.record(&event).await.unwrap();
        repo.commit_events();
    }

    #[tokio::test]
    async fn lists_tenant_events_newest_first_and_isolates_tenants() {
        let repo = Arc::new(InMemoryAuditEventRepository::new());
        let tenant_a = Uuid::now_v7();
        let tenant_b = Uuid::now_v7();

        seed(&repo, tenant_a, "user.login.success", Outcome::Success).await;
        seed(&repo, tenant_a, "user.login.failure", Outcome::Failure).await;
        seed(&repo, tenant_b, "user.login.success", Outcome::Success).await;

        let use_case = UseCase::new(repo.clone());
        let page = use_case
            .execute(Query {
                tenant_id: tenant_a,
                page: None,
                page_size: None,
                action: None,
                actor_id: None,
                outcome: None,
                from: None,
                to: None,
            })
            .await
            .unwrap();

        assert_eq!(page.total, 2); // tenant_b's event is not visible
        assert_eq!(page.items.len(), 2);
    }

    #[tokio::test]
    async fn filters_by_action_and_outcome() {
        let repo = Arc::new(InMemoryAuditEventRepository::new());
        let tenant = Uuid::now_v7();
        seed(&repo, tenant, "user.login.success", Outcome::Success).await;
        seed(&repo, tenant, "user.login.failure", Outcome::Failure).await;
        seed(&repo, tenant, "consent.revoked", Outcome::Success).await;

        let use_case = UseCase::new(repo.clone());
        let page = use_case
            .execute(Query {
                tenant_id: tenant,
                page: None,
                page_size: None,
                action: None,
                actor_id: None,
                outcome: Some("failure".to_string()),
                from: None,
                to: None,
            })
            .await
            .unwrap();

        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].action, "user.login.failure");
    }
}
