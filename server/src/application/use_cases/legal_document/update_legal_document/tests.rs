#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::legal_document::update_legal_document::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::legal_document::LegalDocumentType,
            repositories::legal_document_repository::LegalDocumentRepository,
        },
        testing::in_memory_repositories::in_memory_legal_document_repository::InMemoryLegalDocumentRepository,
    };

    #[tokio::test]
    async fn stores_and_returns_customized_document() {
        let repo = Arc::new(InMemoryLegalDocumentRepository::new());
        let tenant_id = Uuid::now_v7();
        let use_case = UseCase::new(Arc::clone(&repo) as Arc<dyn LegalDocumentRepository>);

        let response = use_case
            .execute(Command {
                tenant_id,
                document_type: LegalDocumentType::TermsOfService,
                markdown_content: "# Our terms".to_string(),
            })
            .await
            .unwrap();

        assert!(response.is_customized);
        assert_eq!(response.content, "# Our terms");

        let stored = repo
            .find_by_tenant_and_type(tenant_id, LegalDocumentType::TermsOfService)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.markdown_content(), "# Our terms");
    }
}
