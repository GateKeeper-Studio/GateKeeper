use crate::{
    domain::{
        entities::{user::User, user_credentials::UserCredentials, user_profile::UserProfile},
        errors::DomainError,
        repositories::user_repository::UserRepository,
    },
    infraestructure::database::executor::{DbExecutor, SharedTx, run},
};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub struct DbUserRepository {
    executor: DbExecutor,
}

impl DbUserRepository {
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
impl UserRepository for DbUserRepository {
    async fn find_by_id(&self, user_id: Uuid) -> Result<Option<User>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id as "id!", email as "email!", tenant_id as "tenant_id!", 
            is_email_verified as "is_email_verified!", is_active as "is_active!", 
            created_at as "created_at!", updated_at
          FROM users
          WHERE id = $1
        "#,
                user_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            User::reconstruct(
                r.id,
                r.email,
                r.tenant_id,
                r.is_email_verified,
                None,
                None,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_id_with_profile(&self, user_id: Uuid) -> Result<Option<User>, DomainError> {
        let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT u.id as "id!", u.email as "email!", u.tenant_id as "tenant_id!", u.is_email_verified as "is_email_verified!",
            p.first_name as "first_name?", p.last_name as "last_name?", p.display_name as "display_name?",
            p.custom_field_values as "custom_field_values?: sqlx::types::Json<serde_json::Value>",
            u.created_at as "created_at!", u.updated_at, u.is_active as "is_active!"
          FROM users u
          LEFT JOIN user_profiles p ON p.user_id = u.id
          WHERE u.id = $1
        "#,
        user_id
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            let profile = r
                .first_name
                .zip(r.last_name)
                .map(|(first_name, last_name)| {
                    let mut profile = UserProfile::new(r.id, r.display_name, first_name, last_name);
                    profile.hydrate_custom_field_values(r.custom_field_values.map(|j| j.0));
                    profile
                });

            User::reconstruct(
                r.id,
                r.email,
                r.tenant_id,
                r.is_email_verified,
                profile,
                None,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_id_with_credentials(
        &self,
        user_id: Uuid,
    ) -> Result<Option<User>, DomainError> {
        let row = run!(
      self.executor,
      sqlx::query!(
        r#"
          SELECT u.id as "id!", u.email as "email!", u.tenant_id as "tenant_id!", u.is_email_verified as "is_email_verified!",
            c.password_hash, c.password_algorithm, c.should_change_pass, u.is_active,
            u.created_at, u.updated_at
          FROM users u
          LEFT JOIN user_credentials c ON c.user_id = u.id
          WHERE u.id = $1
        "#,
        user_id
      ),
      fetch_optional
    )
    .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            let credentials = r.password_hash.map(|password_hash| {
                UserCredentials::new(
                    r.id,
                    password_hash,
                    r.password_algorithm.unwrap(),
                    r.should_change_pass.unwrap_or(false),
                )
            });

            User::reconstruct(
                r.id,
                r.email,
                r.tenant_id,
                r.is_email_verified,
                None,
                credentials,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_email(
        &self,
        tenant_id: Uuid,
        email: &str,
    ) -> Result<Option<User>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT id as "id!", email as "email!", tenant_id as "tenant_id!", 
            is_email_verified as "is_email_verified!", is_active as "is_active!", 
            created_at as "created_at!", updated_at
          FROM users
          WHERE tenant_id = $1 AND lower(email) = lower($2)
        "#,
                tenant_id,
                email
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.map(|r| {
            User::reconstruct(
                r.id,
                r.email,
                r.tenant_id,
                r.is_email_verified,
                None,
                None,
                r.is_active,
                r.created_at,
                r.updated_at,
            )
        }))
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<User>, DomainError> {
        let rows = run!(
            self.executor,
            sqlx::query!(
                r#"
          SELECT u.id as "id!", u.email as "email!", u.tenant_id as "tenant_id!",
            u.is_email_verified as "is_email_verified!", p.first_name as "first_name?",
            p.last_name as "last_name?", p.display_name as "display_name?",
            p.custom_field_values as "custom_field_values?: sqlx::types::Json<serde_json::Value>",
            u.created_at, u.updated_at, u.is_active
          FROM users u
          LEFT JOIN user_profiles p ON p.user_id = u.id
          WHERE u.tenant_id = $1
          ORDER BY u.created_at DESC
          LIMIT $2 OFFSET $3
        "#,
                tenant_id,
                page_size as i64,
                ((page - 1) * page_size) as i64
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                // Only build a profile when the joined row actually has one.
                let profile = r
                    .first_name
                    .zip(r.last_name)
                    .map(|(first_name, last_name)| {
                        UserProfile::reconstruct(
                            r.id,
                            r.display_name,
                            first_name,
                            last_name,
                            None,
                            r.custom_field_values.map(|j| j.0),
                        )
                    });

                User::reconstruct(
                    r.id,
                    r.email,
                    r.tenant_id,
                    r.is_email_verified,
                    profile,
                    None,
                    r.is_active,
                    r.created_at,
                    r.updated_at,
                )
            })
            .collect())
    }

    async fn quantity_of_users_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let count = run!(
            self.executor,
            sqlx::query_scalar!("SELECT COUNT(*) FROM users WHERE tenant_id = $1", tenant_id),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(count.unwrap_or(0) as u32)
    }

    async fn is_user_with_email_exists(
        &self,
        tenant_id: Uuid,
        email: &str,
    ) -> Result<bool, DomainError> {
        let exists = run!(
            self.executor,
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM users WHERE tenant_id = $1 AND lower(email) = lower($2))",
                tenant_id,
                email
            ),
            fetch_one
        )
        .map_err(map_sqlx_error)?;

        Ok(exists.unwrap_or(false))
    }

    async fn find_user_ids_with_password(
        &self,
        user_ids: &[Uuid],
    ) -> Result<Vec<Uuid>, DomainError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = run!(
            self.executor,
            sqlx::query_scalar!(
                "SELECT user_id FROM user_credentials WHERE user_id = ANY($1)",
                user_ids
            ),
            fetch_all
        )
        .map_err(map_sqlx_error)?;

        Ok(rows)
    }

    async fn delete_by_id(&self, user_id: Uuid) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!("DELETE FROM users WHERE id = $1", user_id),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn anonymize(&self, user_id: Uuid) -> Result<(), DomainError> {
        // Scrub the user row — but only when not under a legal hold. rows_affected
        // distinguishes "blocked by hold" from "no such user" below.
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE users
          SET email = 'deleted+' || id::text || '@anonymized.invalid',
              is_active = false,
              is_email_verified = false,
              anonymized_at = now(),
              updated_at = now()
          WHERE id = $1 AND legal_hold = false AND anonymized_at IS NULL
        "#,
                user_id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            // Either the user does not exist, is under legal hold, or is already
            // anonymized. Disambiguate so the caller gets a precise error.
            let existing = run!(
                self.executor,
                sqlx::query!(
                    r#"SELECT legal_hold, anonymized_at FROM users WHERE id = $1"#,
                    user_id
                ),
                fetch_optional
            )
            .map_err(map_sqlx_error)?;

            return match existing {
                None => Err(DomainError::NotFound(format!("user {user_id} not found"))),
                Some(r) if r.legal_hold => Err(DomainError::AccessDenied(
                    "user is under legal hold; erasure refused".to_string(),
                )),
                // Already anonymized — treat as success (idempotent).
                Some(_) => Ok(()),
            };
        }

        // Clear profile names, tenant-collected custom field values, and drop credentials.
        run!(
      self.executor,
      sqlx::query!(
        r#"UPDATE user_profiles SET first_name = '', last_name = '', display_name = '', custom_field_values = NULL WHERE user_id = $1"#,
        user_id
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

        run!(
            self.executor,
            sqlx::query!(
                r#"DELETE FROM user_credentials WHERE user_id = $1"#,
                user_id
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn get_preferred_2fa_method(&self, user_id: Uuid) -> Result<Option<String>, DomainError> {
        let row = run!(
            self.executor,
            sqlx::query!(
                r#"SELECT preferred_2fa_method FROM users WHERE id = $1"#,
                user_id
            ),
            fetch_optional
        )
        .map_err(map_sqlx_error)?;

        Ok(row.and_then(|r| r.preferred_2fa_method))
    }

    async fn set_preferred_2fa_method(
        &self,
        user_id: Uuid,
        method: Option<&str>,
    ) -> Result<(), DomainError> {
        let result = run!(
            self.executor,
            sqlx::query!(
                r#"UPDATE users SET preferred_2fa_method = $2, updated_at = now() WHERE id = $1"#,
                user_id,
                method
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound(format!("user {user_id} not found")));
        }
        Ok(())
    }

    async fn add(&self, user: &User) -> Result<(), DomainError> {
        run!(
      self.executor,
      sqlx::query!(
        r#"
          INSERT INTO users (id, tenant_id, email, is_email_verified, is_active, created_at, updated_at)
          VALUES ($1, $2, $3, $4, TRUE, $5, $6)
        "#,
        user.id(),
        user.tenant_id(),
        user.email(),
        user.is_email_verified(),
        user.created_at(),
        user.updated_at(),
      ),
      execute
    )
    .map_err(map_sqlx_error)?;

        if let Some(profile) = user.profile() {
            run!(
                self.executor,
                sqlx::query!(
                    r#"
            INSERT INTO user_profiles (user_id, display_name, first_name, last_name, custom_field_values)
            VALUES ($1, $2, $3, $4, $5)
          "#,
                    profile.user_id(),
                    profile.display_name().map(|s| s.as_str()).unwrap_or(""),
                    profile.first_name(),
                    profile.last_name(),
                    profile.custom_field_values().map(sqlx::types::Json)
                        as Option<sqlx::types::Json<&serde_json::Value>>,
                ),
                execute
            )
            .map_err(map_sqlx_error)?;
        }

        if let Some(credentials) = user.credentials() {
            run!(
        self.executor,
        sqlx::query!(
          r#"
            INSERT INTO user_credentials (id, user_id, password_hash, password_algorithm, should_change_pass, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
          "#,
          Uuid::now_v7(),
          credentials.user_id(),
          credentials.password_hash(),
          credentials.password_hash_algorithm(),
          credentials.should_change_password(),
          credentials.created_at(),
        ),
        execute
      )
      .map_err(map_sqlx_error)?;
        }

        Ok(())
    }

    async fn update(&self, user: &User) -> Result<(), DomainError> {
        run!(
            self.executor,
            sqlx::query!(
                r#"
          UPDATE users
          SET email = $2, is_email_verified = $3, updated_at = $4
          WHERE id = $1
        "#,
                user.id(),
                user.email(),
                user.is_email_verified(),
                user.updated_at(),
            ),
            execute
        )
        .map_err(map_sqlx_error)?;

        if let Some(profile) = user.profile() {
            run!(
                self.executor,
                sqlx::query!(
                    r#"
            INSERT INTO user_profiles (user_id, display_name, first_name, last_name, custom_field_values)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id) DO UPDATE
            SET display_name = EXCLUDED.display_name,
                first_name = EXCLUDED.first_name,
                last_name = EXCLUDED.last_name,
                custom_field_values = EXCLUDED.custom_field_values
          "#,
                    profile.user_id(),
                    profile.display_name().map(|s| s.as_str()).unwrap_or(""),
                    profile.first_name(),
                    profile.last_name(),
                    profile.custom_field_values().map(sqlx::types::Json)
                        as Option<sqlx::types::Json<&serde_json::Value>>,
                ),
                execute
            )
            .map_err(map_sqlx_error)?;
        }

        if let Some(credentials) = user.credentials() {
            // Upsert so updating a user whose credentials row is missing doesn't
            // silently no-op (relies on the UNIQUE (user_id) constraint).
            run!(
        self.executor,
        sqlx::query!(
          r#"
            INSERT INTO user_credentials (id, user_id, password_hash, password_algorithm, should_change_pass, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (user_id) DO UPDATE
            SET password_hash = EXCLUDED.password_hash,
                password_algorithm = EXCLUDED.password_algorithm,
                should_change_pass = EXCLUDED.should_change_pass,
                updated_at = EXCLUDED.updated_at
          "#,
          Uuid::now_v7(),
          credentials.user_id(),
          credentials.password_hash(),
          credentials.password_hash_algorithm(),
          credentials.should_change_password(),
          credentials.created_at(),
          credentials.updated_at(),
        ),
        execute
      )
      .map_err(map_sqlx_error)?;
        }

        Ok(())
    }
}

// WHY a separate mapping function: sqlx errors are infrastructure concerns.
// The domain knows nothing about sqlx. This function translates the boundary.
pub fn map_sqlx_error(e: sqlx::Error) -> DomainError {
    tracing::error!("Database error: {:?}", e);

    match e {
        sqlx::Error::RowNotFound => DomainError::NotFound("User not found".to_string()),

        // A unique-constraint race (e.g. two requests creating the same email in a
        // tenant) should surface as a conflict, not an opaque 500.
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            DomainError::Conflict("User already exists".to_string())
        }

        _ => DomainError::InfrastructureError("Database error".to_string()), // expand as needed
    }
}
