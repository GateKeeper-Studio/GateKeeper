use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::email_template::{EmailTemplate, EmailTemplateType},
        errors::DomainError,
        repositories::email_template_repository::EmailTemplateRepository,
    },
    infraestructure::database::{
        executor::{DbExecutor, SharedTx, run},
        repositories::legal_document_repository::map_sqlx_error,
    },
};

pub struct DbEmailTemplateRepository {
    executor: DbExecutor,
}

impl DbEmailTemplateRepository {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            executor: DbExecutor::Pool(db_pool),
        }
    }

    /// Builds a repository that runs every statement on a shared transaction, so
    /// its writes participate in the owning Unit of Work.
    pub fn with_tx(tx: SharedTx) -> Self {
        Self {
            executor: DbExecutor::Tx(tx),
        }
    }
}

#[async_trait]
impl EmailTemplateRepository for DbEmailTemplateRepository {
    async fn find_by_tenant_and_type(
        &self,
        tenant_id: Uuid,
        template_type: EmailTemplateType,
    ) -> Result<Option<EmailTemplate>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT subject, body_mjml, updated_at
        FROM email_templates
        WHERE tenant_id = $1 AND template_type = $2
        "#,
                tenant_id,
                template_type.as_str()
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        // The row was selected for this exact `template_type`, so reconstruct with
        // the known type rather than re-parsing the column.
        Ok(row.map(|r| {
            EmailTemplate::reconstruct(
                tenant_id,
                template_type,
                r.subject,
                r.body_mjml,
                r.updated_at,
            )
        }))
    }

    async fn upsert(&self, template: &EmailTemplate) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO email_templates (tenant_id, template_type, subject, body_mjml, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (tenant_id, template_type)
        DO UPDATE SET subject = EXCLUDED.subject, body_mjml = EXCLUDED.body_mjml, updated_at = EXCLUDED.updated_at
        "#,
                template.tenant_id(),
                template.template_type().as_str(),
                template.subject(),
                template.body_mjml(),
                template.updated_at()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete(
        &self,
        tenant_id: Uuid,
        template_type: EmailTemplateType,
    ) -> Result<(), DomainError> {
        // Idempotent: resetting a template that was never customized is a no-op.
        run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM email_templates WHERE tenant_id = $1 AND template_type = $2",
                tenant_id,
                template_type.as_str()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}
