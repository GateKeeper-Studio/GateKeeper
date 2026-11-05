#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::legal_document::reset_legal_document::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::legal_document::{LegalDocument, LegalDocumentType},
            repositories::legal_document_repository::LegalDocumentRepository,
        },
        testing::in_memory_repositories::in_memory_legal_document_repository::InMemoryLegalDocumentRepository,
    };

    #[tokio::test]
    async fn removes_override_and_returns_default() {
        let repo = Arc::new(InMemoryLegalDocumentRepository::new());
        let tenant_id = Uuid::now_v7();
        repo.upsert(&LegalDocument::new(
            tenant_id,
            LegalDocumentType::TermsOfService,
            "# Custom".to_string(),
        ))
        .await
        .unwrap();

        let use_case = UseCase::new(Arc::clone(&repo) as Arc<dyn LegalDocumentRepository>, Arc::new(crate::testing::in_memory_repositories::in_memory_tenant_repository::InMemoryTenantRepository::new()));
        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                Command {
                    tenant_id,
                    document_type: LegalDocumentType::TermsOfService,
                },
            )
            .await
            .unwrap();

        assert!(!response.is_customized);
        assert!(
            repo.find_by_tenant_and_type(tenant_id, LegalDocumentType::TermsOfService)
                .await
                .unwrap()
                .is_none()
        );
    }
}
