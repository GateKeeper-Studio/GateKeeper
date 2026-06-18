use std::sync::Arc;

use crate::application::ports::{
    signing_key_generator::SigningKeyGenerator, unit_of_work::UnitOfWorkFactory,
};
use crate::application::use_cases::subscription::create_subscription::{
    command::Command, response::Response,
};
use crate::domain::{
    entities::{subscription::Subscription, tenant::Tenant},
    errors::DomainError,
};
use crate::utils::oauth_crypto::generate_opaque_token;

// Name of the default tenant every subscription is created with.
const DEFAULT_TENANT_NAME: &str = "master";

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

        // The owning organization must exist.
        uow.organization_repository()
            .find_by_id(command.organization_id)
            .await?
            .ok_or_else(|| {
                DomainError::OrganizationNotFound(format!(
                    "Organization with id {} not found",
                    command.organization_id
                ))
            })?;

        // An organization can own at most one subscription.
        if uow
            .subscription_repository()
            .find_by_organization_id(command.organization_id)
            .await?
            .is_some()
        {
            return Err(DomainError::OrganizationAlreadyHasSubscription(format!(
                "Organization {} already has a subscription",
                command.organization_id
            )));
        }

        // The plan must exist before we can bind a subscription to it.
        let plan = uow
            .subscription_plan_repository()
            .find_by_id(command.plan_id)
            .await?
            .ok_or_else(|| {
                DomainError::SubscriptionPlanNotFound(format!(
                    "Subscription plan with id {} not found",
                    command.plan_id
                ))
            })?;

        // The subscription always starts with its single default ("master") tenant,
        // so the plan must allow at least one tenant.
        if plan.max_tenants() < 1 {
            return Err(DomainError::SubscriptionLimitReached(
                "Plan does not allow any tenants".to_string(),
            ));
        }
        if !plan.is_active() {
            return Err(DomainError::SubscriptionPlanInactive(format!(
                "Subscription plan {} is inactive",
                plan.id()
            )));
        }

        let subscription = Subscription::new(command.organization_id, plan.id(), true);
        uow.subscription_repository().add(&subscription).await?;

        // Auto-provision the default "master" tenant. Its password-hash secret is
        // generated here (the caller never supplies it) so the subscription is born
        // with a usable default tenant.
        let master_tenant = Tenant::new(
            DEFAULT_TENANT_NAME.to_string(),
            None,
            generate_opaque_token(),
            true,  // is_active
            false, // can_self_register
            false, // can_self_forgot_password
            subscription.id(),
            true, // is_default
        );
        uow.tenant_repository().add(&master_tenant).await?;

        // Provision the master tenant's first signing key in the same transaction —
        // same rationale as create_tenant: a tenant must be able to mint tokens.
        let signing_key = self.signing_key_generator.generate(master_tenant.id())?;
        uow.signing_key_repository().add(&signing_key).await?;

        uow.commit().await?;

        Ok(Response {
            subscription_id: subscription.id(),
            organization_id: subscription.organization_id(),
            plan_id: subscription.plan_id(),
            is_active: subscription.is_active(),
            created_at: subscription.created_at(),
            default_tenant_id: master_tenant.id(),
        })
    }
}
