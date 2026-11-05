use std::sync::Arc;

use crate::application::ports::password_hasher::PasswordHasher;
use crate::application::shared::actor::Actor;
use crate::{
    application::{
        ports::unit_of_work::UnitOfWorkFactory,
        use_cases::{
            tenant_user::create_tenant_user::{command::Command, response::Response},
            webhook::user_event_payload::user_event,
        },
    },
    domain::{
        entities::{
            domain_event_type::{DomainEventType, UserEventSource},
            user::User,
            user_credentials::UserCredentials,
            user_profile,
        },
        errors::DomainError,
    },
};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    password_hasher: Arc<dyn PasswordHasher>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        password_hasher: Arc<dyn PasswordHasher>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            password_hasher,
        }
    }

    pub async fn execute(&self, actor: &Actor, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;
        // Authorized here, not only by the HTTP guard: see `application/shared/actor.rs`.
        actor
            .authorize_tenant_admin_for(uow.tenant_repository().as_ref(), command.tenant_id)
            .await?;

        let tenant = uow
            .tenant_repository()
            .find_by_id(command.tenant_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Tenant with id {} not found", command.tenant_id))
            })?;

        let is_user_already_exists = uow
            .user_repository()
            .is_user_with_email_exists(command.tenant_id, &command.email)
            .await?;

        if is_user_already_exists {
            return Err(DomainError::Conflict(format!(
                "User with email {} already exists",
                command.email
            )));
        }

        let mut user = User::new(command.tenant_id, command.email.clone(), None, None);

        if command.is_email_verified {
            user.verify_email();
        }

        // Argon2id runs behind the `PasswordHasher` port: bounded and off the async
        // workers.
        let hashed_password = self
            .password_hasher
            .hash(&command.temporary_password, tenant.password_hash_secret())
            .await?;

        let user_credentials = UserCredentials::new(
            user.id(),
            hashed_password,
            "argon2".to_string(),
            // Temporary password: the user must change it on first use.
            true,
        );

        let user_profile = user_profile::UserProfile::new(
            user.id(),
            command.display_name,
            command.first_name.clone(),
            command.last_name.clone(),
        );

        user.set_credentials(user_credentials);
        user.set_profile(user_profile);

        uow.user_repository().add(&user).await?;

        // Same transaction as the insert: the event and the user commit together.
        uow.outbox_event_repository()
            .enqueue(&user_event(
                DomainEventType::UserCreated,
                &user,
                Some(UserEventSource::Admin),
            ))
            .await?;

        uow.commit().await?;

        Ok(Response {
            user_id: user.id(),
            display_name: user.profile().and_then(|p| p.display_name().cloned()),
            first_name: user
                .profile()
                .map(|p| p.first_name().to_string())
                .unwrap_or(command.first_name),
            last_name: user
                .profile()
                .map(|p| p.last_name().to_string())
                .unwrap_or(command.last_name),
            email: command.email,
        })
    }
}
