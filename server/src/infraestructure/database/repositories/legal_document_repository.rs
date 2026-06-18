use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::legal_document::{LegalDocument, LegalDocumentType},
        errors::DomainError,
        repositories::legal_document_repository::LegalDocumentRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbLegalDocumentRepository {
    executor: DbExecutor,
}

impl DbLegalDocumentRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    /// Builds a repository that runs every statement on a shared transaction,
    /// so its writes participate in the owning Unit of Work.
    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

#[async_trait]
impl LegalDocumentRepository for DbLegalDocumentRepository {
    async fn find_by_tenant_and_type(
        &self,
        tenant_id: Uuid,
        document_type: LegalDocumentType,
    ) -> Result<Option<LegalDocument>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT markdown_content, updated_at
        FROM legal_documents
        WHERE tenant_id = $1 AND document_type = $2
        "#,
                tenant_id,
                document_type.as_str()
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        // The row was selected for this exact `document_type`, so reconstruct with
        // the known type rather than re-parsing the column.
        Ok(row.map(|r| {
            LegalDocument::reconstruct(tenant_id, document_type, r.markdown_content, r.updated_at)
        }))
    }

    async fn upsert(&self, document: &LegalDocument) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO legal_documents (tenant_id, document_type, markdown_content, updated_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (tenant_id, document_type)
        DO UPDATE SET markdown_content = EXCLUDED.markdown_content, updated_at = EXCLUDED.updated_at
        "#,
                document.tenant_id(),
                document.document_type().as_str(),
                document.markdown_content(),
                document.updated_at()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete(
        &self,
        tenant_id: Uuid,
        document_type: LegalDocumentType,
    ) -> Result<(), DomainError> {
        // Idempotent: resetting a document that was never customized is a no-op.
        run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM legal_documents WHERE tenant_id = $1 AND document_type = $2",
                tenant_id,
                document_type.as_str()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}

// Translates sqlx errors at the infrastructure boundary into domain errors.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);
    DomainError::InfrastructureError("Database error".to_string())
}
