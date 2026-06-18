#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription::update_subscription::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{
                billing_interval::BillingInterval, organization::Organization,
                subscription::Subscription, subscription_plan::SubscriptionPlan, tenant::Tenant,
                user::User,
            },
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_organization_repository::InMemoryOrganizationRepository,
                in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
                in_memory_subscription_repository::InMemorySubscriptionRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
        },
    };

    fn seed_organization(orgs: &InMemoryOrganizationRepository) -> Uuid {
        let org = Organization::new("Acme Inc".to_string(), None, true);
        let id = org.id();
        orgs.memory.data.lock().unwrap().insert(id, org);
        id
    }

    #[tokio::test]
    async fn updates_subscription_and_changes_plan() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let users = Arc::new(InMemoryUserRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            users.clone(),
        ));
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);

        let old_plan =
            SubscriptionPlan::new("Old".into(), None, 0, BillingInterval::Monthly, 5, 5, true);
        let new_plan =
            SubscriptionPlan::new("New".into(), None, 0, BillingInterval::Yearly, 5, 5, true);
        let new_plan_id = new_plan.id();
        {
            let mut p = plans.memory.data.lock().unwrap();
            p.insert(old_plan.id(), old_plan.clone());
            p.insert(new_plan.id(), new_plan);
        }

        let sub = Subscription::new(organization_id, old_plan.id(), true);
        let sub_id = sub.id();
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub_id, sub);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(users, tenants)
                .with_subscription_repository(subscriptions.clone())
                .with_subscription_plan_repository(plans)
                .with_organization_repository(orgs),
        );
        let use_case = UseCase::new(uow_factory);

        let response = use_case
            .execute(Command {
                organization_id,
                plan_id: new_plan_id,
                is_active: false,
            })
            .await
            .unwrap();

        assert_eq!(response.organization_id, organization_id);
        assert_eq!(response.plan_id, new_plan_id);
        assert!(!response.is_active);
    }

    #[tokio::test]
    async fn fails_when_new_plan_missing() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let users = Arc::new(InMemoryUserRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            users.clone(),
        ));
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);

        let sub = Subscription::new(organization_id, Uuid::now_v7(), true);
        let sub_id = sub.id();
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub_id, sub);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(users, tenants)
                .with_subscription_repository(subscriptions.clone())
                .with_subscription_plan_repository(plans)
                .with_organization_repository(orgs),
        );
        let use_case = UseCase::new(uow_factory);

        let err = use_case
            .execute(Command {
                organization_id,
                plan_id: Uuid::now_v7(),
                is_active: true,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionPlanNotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_new_plan_is_inactive() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let users = Arc::new(InMemoryUserRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            users.clone(),
        ));
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);

        let old_plan =
            SubscriptionPlan::new("Old".into(), None, 0, BillingInterval::Monthly, 5, 5, true);
        let inactive_plan = SubscriptionPlan::new(
            "Inactive".into(),
            None,
            0,
            BillingInterval::Monthly,
            5,
            5,
            false,
        );
        let inactive_plan_id = inactive_plan.id();
        {
            let mut p = plans.memory.data.lock().unwrap();
            p.insert(old_plan.id(), old_plan.clone());
            p.insert(inactive_plan_id, inactive_plan);
        }

        let sub = Subscription::new(organization_id, old_plan.id(), true);
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub.id(), sub);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(users, tenants)
                .with_subscription_repository(subscriptions)
                .with_subscription_plan_repository(plans)
                .with_organization_repository(orgs),
        );
        let use_case = UseCase::new(uow_factory);

        let err = use_case
            .execute(Command {
                organization_id,
                plan_id: inactive_plan_id,
                is_active: true,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::SubscriptionPlanInactive(_)));
    }

    #[tokio::test]
    async fn blocks_downgrade_below_current_tenant_usage() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let users = Arc::new(InMemoryUserRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            users.clone(),
        ));
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);

        // Current plan allows 5 tenants; target plan allows only 1.
        let big_plan =
            SubscriptionPlan::new("Big".into(), None, 0, BillingInterval::Monthly, 5, 100, true);
        let small_plan =
            SubscriptionPlan::new("Small".into(), None, 0, BillingInterval::Monthly, 1, 100, true);
        let small_plan_id = small_plan.id();
        {
            let mut p = plans.memory.data.lock().unwrap();
            p.insert(big_plan.id(), big_plan.clone());
            p.insert(small_plan_id, small_plan);
        }

        let sub = Subscription::new(organization_id, big_plan.id(), true);
        let sub_id = sub.id();
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub_id, sub);

        // Two tenants under the subscription — exceeds the target plan's max_tenants (1).
        {
            let mut t = tenants.memory.data.lock().unwrap();
            for name in ["t1", "t2"] {
                let tenant = Tenant::new(
                    name.to_string(),
                    None,
                    "secret".to_string(),
                    true,
                    true,
                    true,
                    sub_id,
                    false,
                );
                t.insert(tenant.id(), tenant);
            }
        }

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(users, tenants)
                .with_subscription_repository(subscriptions)
                .with_subscription_plan_repository(plans)
                .with_organization_repository(orgs),
        );
        let use_case = UseCase::new(uow_factory);

        let err = use_case
            .execute(Command {
                organization_id,
                plan_id: small_plan_id,
                is_active: true,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::PlanDowngradeNotAllowed(_)));
    }

    #[tokio::test]
    async fn blocks_downgrade_below_current_user_usage() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let users = Arc::new(InMemoryUserRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            users.clone(),
        ));
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());
        let orgs = Arc::new(InMemoryOrganizationRepository::new());
        let organization_id = seed_organization(&orgs);

        // Current plan allows 100 users/tenant; target plan only 1.
        let big_plan =
            SubscriptionPlan::new("Big".into(), None, 0, BillingInterval::Monthly, 5, 100, true);
        let small_plan =
            SubscriptionPlan::new("Small".into(), None, 0, BillingInterval::Monthly, 5, 1, true);
        let small_plan_id = small_plan.id();
        {
            let mut p = plans.memory.data.lock().unwrap();
            p.insert(big_plan.id(), big_plan.clone());
            p.insert(small_plan_id, small_plan);
        }

        let sub = Subscription::new(organization_id, big_plan.id(), true);
        let sub_id = sub.id();
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub_id, sub);

        let tenant = Tenant::new(
            "t1".to_string(),
            None,
            "secret".to_string(),
            true,
            true,
            true,
            sub_id,
            true,
        );
        let tenant_id = tenant.id();
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant_id, tenant);

        // Two users in the tenant — exceeds the target plan's max_users (1).
        {
            let mut u = users.memory.data.lock().unwrap();
            for email in ["a@x.com", "b@x.com"] {
                let user = User::new(tenant_id, email.to_string(), None, None);
                u.insert(user.id(), user);
            }
        }

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(users, tenants)
                .with_subscription_repository(subscriptions)
                .with_subscription_plan_repository(plans)
                .with_organization_repository(orgs),
        );
        let use_case = UseCase::new(uow_factory);

        let err = use_case
            .execute(Command {
                organization_id,
                plan_id: small_plan_id,
                is_active: true,
            })
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::PlanDowngradeNotAllowed(_)));
    }
}
