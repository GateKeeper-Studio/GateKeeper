#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::legal_document::get_legal_document::{query::Query, use_case::UseCase},
        domain::{
            entities::legal_document::{LegalDocument, LegalDocumentType},
            repositories::legal_document_repository::LegalDocumentRepository,
        },
        testing::in_memory_repositories::in_memory_legal_document_repository::InMemoryLegalDocumentRepository,
    };

    #[tokio::test]
    async fn returns_default_when_not_customized() {
        let repo = Arc::new(InMemoryLegalDocumentRepository::new());
        let use_case = UseCase::new(repo);

        let response = use_case
            .execute(Query {
                tenant_id: Uuid::now_v7(),
                document_type: LegalDocumentType::TermsOfService,
            })
            .await
            .unwrap();

        assert!(!response.is_customized);
        assert!(response.updated_at.is_none());
        assert!(!response.content.is_empty());
    }

    #[tokio::test]
    async fn returns_stored_document_when_customized() {
        let repo = Arc::new(InMemoryLegalDocumentRepository::new());
        let tenant_id = Uuid::now_v7();
        repo.upsert(&LegalDocument::new(
            tenant_id,
            LegalDocumentType::PrivacyPolicy,
            "# Custom policy".to_string(),
        ))
        .await
        .unwrap();

        let use_case = UseCase::new(repo);
        let response = use_case
            .execute(Query {
                tenant_id,
                document_type: LegalDocumentType::PrivacyPolicy,
            })
            .await
            .unwrap();

        assert!(response.is_customized);
        assert!(response.updated_at.is_some());
        assert_eq!(response.content, "# Custom policy");
    }
}
