use std::sync::Arc;

use crate::{
  application::{
    ports::unit_of_work::UnitOfWorkFactory,
    use_cases::oauth_client::{
      client_view::ClientView,
      create_oauth_client::{command::Command, response::Response},
      validation::{parse_client_type, require_non_empty, validate_redirect_uris},
    },
  },
  domain::{
    entities::oauth_client::{ClientType, OAuthClient},
    errors::DomainError,
  },
  utils::{oauth_crypto::generate_opaque_token, password_hasher::hash_password},
};

pub struct UseCase {
  unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
}

impl UseCase {
  pub fn new(unit_of_work_factory: Arc<dyn UnitOfWorkFactory>) -> Self {
    Self { unit_of_work_factory }
  }

  pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
    let uow = self.unit_of_work_factory.begin().await?;

    let tenant = uow.tenant_repository()
      .find_by_id(command.tenant_id)
      .await?
      .ok_or_else(|| DomainError::TenantNotFound(format!("Tenant with id {} not found", command.tenant_id)))?;

    let client_type = parse_client_type(&command.client_type)?;
    validate_redirect_uris(&command.redirect_uris)?;
    require_non_empty("allowed_scopes", &command.allowed_scopes)?;
    require_non_empty("allowed_grant_types", &command.allowed_grant_types)?;

    // The public client_id is an opaque random value.
    let client_id = generate_opaque_token();

    // Confidential clients receive a secret; we return it once and store only its
    // Argon2 hash (with the tenant pepper, matching token-endpoint verification).
    let (client_secret_hash, raw_secret) = match client_type {
      ClientType::Confidential => {
        let raw = generate_opaque_token();
        let secret = raw.clone();
        let pepper = tenant.password_hash_secret().to_string();
        // Argon2 is CPU/memory heavy — keep it off the async worker.
        let hash = tokio::task::spawn_blocking(move || hash_password(&secret, &pepper))
          .await
          .map_err(|_| DomainError::PasswordHashingError("failed to hash client secret".to_string()))?
          .map_err(|_| DomainError::PasswordHashingError("failed to hash client secret".to_string()))?;
        (Some(hash), Some(raw))
      }
      ClientType::Public => (None, None),
    };

    let client = OAuthClient::new(
      command.tenant_id,
      client_id,
      client_secret_hash,
      client_type,
      command.name,
      command.redirect_uris,
      command.post_logout_redirect_uris,
      command.allowed_scopes,
      command.allowed_grant_types,
      command.require_pkce,
      command.require_consent,
    );

    uow.oauth_client_repository().add(&client).await?;
    uow.commit().await?;

    Ok(Response {
      client: ClientView::from_entity(&client),
      client_secret: raw_secret,
    })
  }
}
