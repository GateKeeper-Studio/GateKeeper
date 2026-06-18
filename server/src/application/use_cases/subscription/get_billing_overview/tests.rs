#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::subscription::get_billing_overview::{
            query::Query, use_case::UseCase,
        },
        domain::entities::{
            billing_interval::BillingInterval, subscription::Subscription,
            subscription_plan::SubscriptionPlan, tenant::Tenant, user::User,
        },
        testing::in_memory_repositories::{
            in_memory_subscription_plan_repository::InMemorySubscriptionPlanRepository,
            in_memory_subscription_repository::InMemorySubscriptionRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
    };

    #[tokio::test]
    async fn returns_subscription_plan_and_usage() {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        let users = Arc::new(InMemoryUserRepository::new());
        let subscriptions = Arc::new(InMemorySubscriptionRepository::new(
            tenants.clone(),
            users.clone(),
        ));
        let plans = Arc::new(InMemorySubscriptionPlanRepository::new());

        let organization_id = Uuid::now_v7();
        let plan = SubscriptionPlan::new(
            "Pro".into(),
            Some("Growing teams".into()),
            1900,
            BillingInterval::Monthly,
            5,
            10,
            true,
        );
        let plan_id = plan.id();
        plans.memory.data.lock().unwrap().insert(plan_id, plan);

        let sub = Subscription::new(organization_id, plan_id, true);
        let sub_id = sub.id();
        subscriptions
            .memory
            .data
            .lock()
            .unwrap()
            .insert(sub_id, sub);

        let tenant = Tenant::new(
            "master".into(),
            None,
            "secret".into(),
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
        {
            let mut u = users.memory.data.lock().unwrap();
            for email in ["a@x.com", "b@x.com"] {
                let user = User::new(tenant_id, email.into(), None, None);
                u.insert(user.id(), user);
            }
        }

        let use_case = UseCase::new(subscriptions, plans);
        let resp = use_case
            .execute(Query { organization_id })
            .await
            .unwrap();

        assert_eq!(resp.subscription_id, sub_id);
        assert_eq!(resp.plan.plan_id, plan_id);
        assert_eq!(resp.plan.price_cents, 1900);
        assert_eq!(resp.usage.tenants, 1);
        assert_eq!(resp.usage.max_tenants, 5);
        assert_eq!(resp.usage.max_users_in_any_tenant, 2);
        assert_eq!(resp.usage.max_users, 10);
    }
}
