use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domain::{
        entities::custom_field_definition::{CustomFieldDefinition, CustomFieldType},
        errors::DomainError,
        repositories::custom_field_definition_repository::CustomFieldDefinitionRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};

pub struct DbCustomFieldDefinitionRepository {
    executor: DbExecutor,
}

impl DbCustomFieldDefinitionRepository {
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

// field_type is stored as a plain lowercase discriminator string, validated at
// the application layer — see CustomFieldType::as_str()/parse(). A row whose
// field_type fails to parse indicates the store was written by a version that
// no longer matches this binary; treat it as an infrastructure error rather
// than silently coercing it.
fn parse_field_type(raw: &str) -> Result<CustomFieldType, DomainError> {
    CustomFieldType::parse(raw).ok_or_else(|| {
        DomainError::InfrastructureError(format!("Unknown custom field type: {raw}"))
    })
}

#[allow(clippy::too_many_arguments)]
fn to_entity(
    id: Uuid,
    tenant_id: Uuid,
    field_key: String,
    label: sqlx::types::Json<HashMap<String, String>>,
    description: Option<sqlx::types::Json<HashMap<String, String>>>,
    field_type: String,
    options: Option<sqlx::types::Json<Vec<HashMap<String, String>>>>,
    is_required: bool,
    display_order: i32,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
) -> Result<CustomFieldDefinition, DomainError> {
    Ok(CustomFieldDefinition::reconstruct(
        id,
        tenant_id,
        field_key,
        label.0,
        description.map(|j| j.0),
        parse_field_type(&field_type)?,
        options.map(|j| j.0),
        is_required,
        display_order,
        is_active,
        created_at,
        updated_at,
    ))
}

#[async_trait]
impl CustomFieldDefinitionRepository for DbCustomFieldDefinitionRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<CustomFieldDefinition>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, tenant_id, field_key,
          label as "label!: sqlx::types::Json<HashMap<String, String>>",
          description as "description?: sqlx::types::Json<HashMap<String, String>>",
          field_type,
          options as "options?: sqlx::types::Json<Vec<HashMap<String, String>>>",
          is_required, display_order, is_active, created_at, updated_at
        FROM tenant_custom_fields
        WHERE tenant_id = $1 AND id = $2
        "#,
                tenant_id,
                id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        row.map(|r| {
            to_entity(
                r.id,
                r.tenant_id,
                r.field_key,
                r.label,
                r.description,
                r.field_type,
                r.options,
                r.is_required,
                r.display_order,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        })
        .transpose()
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<CustomFieldDefinition>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, tenant_id, field_key,
          label as "label!: sqlx::types::Json<HashMap<String, String>>",
          description as "description?: sqlx::types::Json<HashMap<String, String>>",
          field_type,
          options as "options?: sqlx::types::Json<Vec<HashMap<String, String>>>",
          is_required, display_order, is_active, created_at, updated_at
        FROM tenant_custom_fields
        WHERE tenant_id = $1
        ORDER BY display_order ASC, created_at ASC
        "#,
                tenant_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|r| {
                to_entity(
                    r.id,
                    r.tenant_id,
                    r.field_key,
                    r.label,
                    r.description,
                    r.field_type,
                    r.options,
                    r.is_required,
                    r.display_order,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect()
    }

    async fn find_active_by_tenant_id(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<CustomFieldDefinition>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, tenant_id, field_key,
          label as "label!: sqlx::types::Json<HashMap<String, String>>",
          description as "description?: sqlx::types::Json<HashMap<String, String>>",
          field_type,
          options as "options?: sqlx::types::Json<Vec<HashMap<String, String>>>",
          is_required, display_order, is_active, created_at, updated_at
        FROM tenant_custom_fields
        WHERE tenant_id = $1 AND is_active = TRUE
        ORDER BY display_order ASC, created_at ASC
        "#,
                tenant_id
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        rows.into_iter()
            .map(|r| {
                to_entity(
                    r.id,
                    r.tenant_id,
                    r.field_key,
                    r.label,
                    r.description,
                    r.field_type,
                    r.options,
                    r.is_required,
                    r.display_order,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect()
    }

    async fn find_by_tenant_id_and_key(
        &self,
        tenant_id: Uuid,
        field_key: &str,
    ) -> Result<Option<CustomFieldDefinition>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
        SELECT id, tenant_id, field_key,
          label as "label!: sqlx::types::Json<HashMap<String, String>>",
          description as "description?: sqlx::types::Json<HashMap<String, String>>",
          field_type,
          options as "options?: sqlx::types::Json<Vec<HashMap<String, String>>>",
          is_required, display_order, is_active, created_at, updated_at
        FROM tenant_custom_fields
        WHERE tenant_id = $1 AND field_key = $2
        "#,
                tenant_id,
                field_key
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        row.map(|r| {
            to_entity(
                r.id,
                r.tenant_id,
                r.field_key,
                r.label,
                r.description,
                r.field_type,
                r.options,
                r.is_required,
                r.display_order,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        })
        .transpose()
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                "SELECT COUNT(*) as count FROM tenant_custom_fields WHERE tenant_id = $1",
                tenant_id
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(row.count.unwrap_or(0) as u32)
    }

    async fn add(&self, definition: &CustomFieldDefinition) -> Result<(), DomainError> {
        let options = definition.options().map(|o| sqlx::types::Json(o.to_vec()));
        run!(
            self.executor,
            sqlx::query!(
                r#"
        INSERT INTO tenant_custom_fields
          (id, tenant_id, field_key, label, description, field_type, options,
           is_required, display_order, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
                definition.id(),
                definition.tenant_id(),
                definition.field_key(),
                sqlx::types::Json(definition.label()) as _,
                definition.description().map(sqlx::types::Json) as _,
                definition.field_type().as_str(),
                options as _,
                definition.is_required(),
                definition.display_order(),
                definition.is_active(),
                definition.created_at(),
                definition.updated_at()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn update(&self, definition: &CustomFieldDefinition) -> Result<(), DomainError> {
        let options = definition.options().map(|o| sqlx::types::Json(o.to_vec()));
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
        UPDATE tenant_custom_fields
        SET label = $1, description = $2, options = $3, is_required = $4,
          display_order = $5, is_active = $6, updated_at = $7
        WHERE tenant_id = $8 AND id = $9
        "#,
                sqlx::types::Json(definition.label()) as _,
                definition.description().map(sqlx::types::Json) as _,
                options as _,
                definition.is_required(),
                definition.display_order(),
                definition.is_active(),
                definition.updated_at(),
                definition.tenant_id(),
                definition.id()
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::CustomFieldNotFound(
                "Custom field not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                "DELETE FROM tenant_custom_fields WHERE tenant_id = $1 AND id = $2",
                tenant_id,
                id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            Err(DomainError::CustomFieldNotFound(
                "Custom field not found".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => {
            DomainError::CustomFieldNotFound("Custom field not found".to_string())
        }
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::CustomFieldKeyAlreadyExists(
                "A custom field with this key already exists".to_string(),
            )
        }
        _ => DomainError::InfrastructureError("Database error".to_string()),
    }
}
