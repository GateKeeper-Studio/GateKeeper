#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::organization::get_organization_by_id::{
            query::Query, use_case::UseCase,
        },
        domain::{entities::organization::Organization, errors::DomainError},
        testing::in_memory_repositories::in_memory_organization_repository::InMemoryOrganizationRepository,
    };

    #[tokio::test]
    async fn returns_organization_when_found() {
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let org = Organization::new("Acme".to_string(), None, true);
        let id = org.id();
        orgs.memory.data.lock().unwrap().insert(id, org);

        let use_case = UseCase::new(orgs);
        let response = use_case
            .execute(Query {
                organization_id: id,
            })
            .await
            .unwrap();

        assert_eq!(response.organization_id, id);
        assert_eq!(response.name, "Acme");
    }

    #[tokio::test]
    async fn fails_when_not_found() {
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let use_case = UseCase::new(orgs);

        let err = use_case
            .execute(Query {
                organization_id: Uuid::now_v7(),
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::OrganizationNotFound(_)));
    }
}
