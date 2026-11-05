#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        application::use_cases::authentication::confirm_current_password::{
            command::Command, use_case::UseCase,
        },
        domain::{
            entities::{tenant::Tenant, user::User},
            errors::DomainError,
        },
        testing::{
            in_memory_repositories::{
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            password_hasher::test_password_hasher,
            user_factory::{TEST_PASSWORD, TEST_PASSWORD_SECRET, generate_users},
        },
    };

    fn setup(with_password: bool) -> (UseCase, Tenant, User) {
        let tenant = Tenant::new(
            "Acme".to_string(),
            None,
            TEST_PASSWORD_SECRET.to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
        );
        let user = if with_password {
            generate_users(tenant.id(), 1).remove(0)
        } else {
            User::new(tenant.id(), "social@example.com".to_string(), None, None)
        };

        let users = Arc::new(InMemoryUserRepository::new());
        users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user.id(), user.clone());
        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant.clone());

        (
            UseCase::new(users, tenants, test_password_hasher()),
            tenant,
            user,
        )
    }

    fn command(tenant: &Tenant, user: &User, password: Option<&str>) -> Command {
        Command {
            tenant_id: tenant.id(),
            user_id: user.id(),
            current_password: password.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn accepts_the_right_password() {
        let (use_case, tenant, user) = setup(true);
        assert!(
            use_case
                .execute(command(&tenant, &user, Some(TEST_PASSWORD)))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn refuses_a_wrong_or_missing_password() {
        let (use_case, tenant, user) = setup(true);
        for password in [Some("wrong"), Some(""), None] {
            let err = use_case
                .execute(command(&tenant, &user, password))
                .await
                .unwrap_err();
            assert!(
                matches!(err, DomainError::StepUpRequired(_)),
                "{password:?}"
            );
        }
    }

    #[tokio::test]
    async fn an_account_without_a_password_has_nothing_to_confirm() {
        let (use_case, tenant, user) = setup(false);
        assert!(
            use_case
                .execute(command(&tenant, &user, None))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn refuses_a_user_from_another_tenant() {
        let (use_case, _, user) = setup(true);
        let other = Tenant::new(
            "Other".to_string(),
            None,
            "x".to_string(),
            true,
            true,
            true,
            uuid::Uuid::now_v7(),
        );
        let err = use_case
            .execute(command(&other, &user, Some(TEST_PASSWORD)))
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::AccessDenied(_)));
    }
}
