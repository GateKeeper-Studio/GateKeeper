#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::legal_document::get_public_legal_document::{
            query::Query, use_case::UseCase,
        },
        domain::entities::legal_document::LegalDocumentType,
        testing::in_memory_repositories::{
            in_memory_legal_document_repository::InMemoryLegalDocumentRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
        },
    };

    #[tokio::test]
    async fn falls_back_to_default_for_unknown_client() {
        let legal = Arc::new(InMemoryLegalDocumentRepository::new());
        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        let use_case = UseCase::new(legal, clients);

        let response = use_case
            .execute(Query {
                client_id: uuid::Uuid::now_v7().to_string(),
                document_type: LegalDocumentType::TermsOfService,
            })
            .await
            .unwrap();

        assert!(!response.is_customized);
        assert!(!response.content.is_empty());
    }
}
