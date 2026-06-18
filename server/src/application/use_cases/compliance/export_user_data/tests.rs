#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::compliance::export_user_data::{query::Query, use_case::UseCase},
        domain::entities::{
            consent_grant::ConsentGrant, group::Group, role::Role, user::User,
            user_profile::UserProfile,
        },
        testing::in_memory_repositories::{
            in_memory_consent_grant_repository::InMemoryConsentGrantRepository,
            in_memory_group_repository::InMemoryGroupRepository,
            in_memory_role_repository::InMemoryRoleRepository,
            in_memory_user_repository::InMemoryUserRepository,
        },
    };

    fn seeded_user(tenant_id: Uuid, user_id: Uuid) -> User {
        let profile = UserProfile::new(
            user_id,
            Some("Ada L.".into()),
            "Ada".into(),
            "Lovelace".into(),
        );
        User::reconstruct(
            user_id,
            "ada@example.com".into(),
            tenant_id,
            true,
            Some(profile),
            None,
            true,
            Utc::now(),
            None,
        )
    }

    #[tokio::test]
    async fn exports_profile_consents_roles_and_groups_scoped_to_tenant() {
        let tenant_id = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let users = Arc::new(InMemoryUserRepository::new());
        users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user_id, seeded_user(tenant_id, user_id));

        let roles = Arc::new(InMemoryRoleRepository::new(users.clone()));
        let role = Role::new(tenant_id, "admin".into(), None);
        let role_id = role.id();
        roles.memory.data.lock().unwrap().insert(role_id, role);
        roles.assignments.lock().unwrap().insert((role_id, user_id));

        let groups = Arc::new(InMemoryGroupRepository::new(users.clone()));
        let group = Group::new(tenant_id, "engineering".into(), None);
        let group_id = group.id();
        groups.memory.data.lock().unwrap().insert(group_id, group);
        groups.members.lock().unwrap().insert((group_id, user_id));

        let consents = Arc::new(InMemoryConsentGrantRepository::new());
        let client_id = Uuid::now_v7();
        let grant = ConsentGrant::new(
            tenant_id,
            user_id,
            client_id,
            vec!["openid".into(), "email".into()],
        );
        consents
            .memory
            .data
            .lock()
            .unwrap()
            .insert(grant.id(), grant);

        let use_case = UseCase::new(
            users.clone(),
            consents.clone(),
            roles.clone(),
            groups.clone(),
        );
        let bundle = use_case
            .execute(Query { tenant_id, user_id })
            .await
            .unwrap();

        assert_eq!(bundle.user.email, "ada@example.com");
        assert_eq!(bundle.user.first_name.as_deref(), Some("Ada"));
        assert_eq!(bundle.roles, vec!["admin"]);
        assert_eq!(bundle.groups, vec!["engineering"]);
        assert_eq!(bundle.consents.len(), 1);
        assert_eq!(bundle.consents[0].scopes, vec!["openid", "email"]);
    }

    #[tokio::test]
    async fn rejects_user_from_another_tenant() {
        let tenant_id = Uuid::now_v7();
        let other_tenant = Uuid::now_v7();
        let user_id = Uuid::now_v7();

        let users = Arc::new(InMemoryUserRepository::new());
        users
            .memory
            .data
            .lock()
            .unwrap()
            .insert(user_id, seeded_user(tenant_id, user_id));
        let roles = Arc::new(InMemoryRoleRepository::new(users.clone()));
        let groups = Arc::new(InMemoryGroupRepository::new(users.clone()));
        let consents = Arc::new(InMemoryConsentGrantRepository::new());

        let use_case = UseCase::new(users.clone(), consents, roles, groups);
        let result = use_case
            .execute(Query {
                tenant_id: other_tenant,
                user_id,
            })
            .await;
        assert!(result.is_err());
    }
}
