use std::sync::Arc;

use crate::application::ports::{
    signing_key_generator::SigningKeyGenerator, unit_of_work::UnitOfWorkFactory,
};
use crate::application::use_cases::tenant::create_tenant::{command::Command, response::Response};
use crate::domain::{entities::tenant::Tenant, errors::DomainError};

pub struct UseCase {
    unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
    signing_key_generator: Arc<dyn SigningKeyGenerator>,
}

impl UseCase {
    pub fn new(
        unit_of_work_factory: Arc<dyn UnitOfWorkFactory>,
        signing_key_generator: Arc<dyn SigningKeyGenerator>,
    ) -> Self {
        Self {
            unit_of_work_factory,
            signing_key_generator,
        }
    }

    pub async fn execute(&self, command: Command) -> Result<Response, DomainError> {
        let uow = self.unit_of_work_factory.begin().await?;

        // The tenant must join an existing subscription.
        let subscription = uow
            .subscription_repository()
            .find_by_id(command.subscription_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionNotFound(format!(
                    "Subscription with id {} not found",
                    command.subscription_id
                ))
            })?;

        // Enforce the plan's tenant cap before adding another tenant.
        let plan = uow
            .subscription_plan_repository()
            .find_by_id(subscription.plan_id())
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionPlanNotFound(format!(
                    "Subscription plan with id {} not found",
                    subscription.plan_id()
                ))
            })?;

        let current_tenants = uow
            .subscription_repository()
            .count_tenants(subscription.id())
            .await?;

        if current_tenants >= plan.max_tenants() as u32 {
            return Err(DomainError::SubscriptionLimitReached(format!(
                "Subscription has reached its plan limit of {} tenants",
                plan.max_tenants()
            )));
        }

        let already_exists = uow
            .tenant_repository()
            .is_tenant_with_name_exists(&command.name)
            .await?;

        if already_exists {
            return Err(DomainError::TenantAlreadyExists(format!(
                "Tenant with name '{}' already exists",
                command.name
            )));
        }

        let tenant = Tenant::new(
            command.name,
            command.description,
            command.password_hash_secret,
            true, // is_active
            command.can_self_register,
            command.can_self_forgot_password,
            command.subscription_id,
            false, // is_default — only the auto-created master tenant is the default
        );

        uow.tenant_repository().add(&tenant).await?;

        // Provision the tenant's first signing key in the same transaction. Without an
        // active key the OAuth/OIDC token endpoint cannot mint JWTs, so a tenant is
        // never left in a state where it can register clients but not issue tokens.
        let signing_key = self.signing_key_generator.generate(tenant.id())?;
        uow.signing_key_repository().add(&signing_key).await?;

        uow.commit().await?;

        Ok(Response {
            tenant_id: tenant.id(),
            name: tenant.name().to_string(),
            description: tenant.description().map(str::to_string),
            is_active: tenant.is_active(),
            can_self_register: tenant.can_self_register(),
            can_self_forgot_password: tenant.can_self_forgot_password(),
            subscription_id: tenant.subscription_id(),
            is_default: tenant.is_default(),
            created_at: tenant.created_at(),
        })
    }
}
