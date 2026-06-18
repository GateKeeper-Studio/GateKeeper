#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::organization::list_organizations::{
            query::Query, use_case::UseCase,
        },
        domain::entities::organization::Organization,
        testing::in_memory_repositories::in_memory_organization_repository::InMemoryOrganizationRepository,
    };

    #[tokio::test]
    async fn lists_organizations() {
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        {
            let mut data = orgs.memory.data.lock().unwrap();
            for i in 0..2 {
                let org = Organization::new(format!("Org {i}"), None, true);
                data.insert(org.id(), org);
            }
        }

        let use_case = UseCase::new(orgs);
        let result = use_case
            .execute(Query {
                page: Some(1),
                page_size: Some(10),
            })
            .await
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
    }
}
