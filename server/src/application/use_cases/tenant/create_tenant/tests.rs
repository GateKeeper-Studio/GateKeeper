#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::tenant::create_tenant::{command::Command, use_case::UseCase},
        domain::{entities::organization::Organization, errors::DomainError},
        infraestructure::crypto::ecdsa_key_generator::EcdsaKeyGenerator,
        testing::{
            in_memory_repositories::{
                in_memory_organization_repository::InMemoryOrganizationRepository,
                in_memory_signing_key_repository::InMemorySigningKeyRepository,
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
        },
    };

    fn setup() -> (
        Arc<InMemoryTenantRepository>,
        Arc<InMemorySigningKeyRepository>,
        Uuid,
        UseCase,
    ) {
        let tenant_repo = Arc::new(InMemoryTenantRepository::new());
        let signing_keys = Arc::new(InMemorySigningKeyRepository::new());
        let organizations = Arc::new(InMemoryOrganizationRepository::new());
        let organization = Organization::new("Test Organization".to_string(), None, true);
        let organization_id = organization.id();
        organizations
            .memory
            .data
            .lock()
            .unwrap()
            .insert(organization_id, organization);

        let uow_factory = Arc::new(
            InMemoryUnitOfWorkFactory::new(
                Arc::new(InMemoryUserRepository::new()),
                tenant_repo.clone(),
            )
            .with_signing_key_repository(signing_keys.clone())
            .with_organization_repository(organizations),
        );
        let use_case = UseCase::new(
            uow_factory,
            Arc::new(EcdsaKeyGenerator::new(Arc::new(StubSecretCipher))),
        );
        (tenant_repo, signing_keys, organization_id, use_case)
    }

    fn make_command(name: &str, organization_id: Uuid) -> Command {
        Command {
            organization_id,
            name: name.to_string(),
            description: Some("A test tenant".to_string()),
            can_self_register: true,
            can_self_forgot_password: true,
        }
    }

    #[tokio::test]
    async fn creates_tenant_for_organization_and_signing_key() {
        let (tenant_repo, signing_keys, organization_id, use_case) = setup();

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command("Acme Corp", organization_id),
            )
            .await
            .unwrap();

        assert_eq!(response.organization_id, organization_id);
        assert_eq!(response.name, "Acme Corp");
        assert_eq!(tenant_repo.memory.data.lock().unwrap().len(), 1);

        let keys = signing_keys.memory.data.lock().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(
            keys.values().next().unwrap().tenant_id(),
            response.tenant_id
        );
    }

    #[tokio::test]
    async fn generates_a_long_random_pepper_per_tenant() {
        // The pepper used to be whatever the caller sent (one character was
        // accepted). It is now 32 random bytes, generated server-side.
        let (tenant_repo, _, organization_id, use_case) = setup();
        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command("Tenant A", organization_id),
            )
            .await
            .unwrap();
        use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command("Tenant B", organization_id),
            )
            .await
            .unwrap();

        let tenants = tenant_repo.memory.data.lock().unwrap();
        let peppers: Vec<String> = tenants
            .values()
            .map(|t| t.password_hash_secret().to_string())
            .collect();
        assert_eq!(peppers.len(), 2);
        assert!(peppers.iter().all(|p| p.len() >= 43), "{peppers:?}");
        assert_ne!(peppers[0], peppers[1]);

        // ...and never shows up in a Debug dump of the entity.
        let tenant = tenants.values().next().unwrap();
        assert!(!format!("{tenant:?}").contains(tenant.password_hash_secret()));
    }

    #[tokio::test]
    async fn fails_when_organization_is_missing() {
        let (_, _, _, use_case) = setup();

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command("Orphan Corp", Uuid::now_v7()),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn fails_when_tenant_name_already_exists() {
        let (tenant_repo, _, organization_id, use_case) = setup();
        let existing = crate::domain::entities::tenant::Tenant::new(
            "Duplicate Name".to_string(),
            None,
            "secret".to_string(),
            true,
            false,
            false,
            organization_id,
        );
        tenant_repo
            .memory
            .data
            .lock()
            .unwrap()
            .insert(existing.id(), existing);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                make_command("Duplicate Name", organization_id),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, DomainError::Conflict(_)));
    }
}
