#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;

    use crate::{
        application::use_cases::oauth_client::get_client_auth_data::{
            query::Query, use_case::UseCase,
        },
        domain::{
            entities::{
                external_identity_provider::{ExternalIdentityProvider, ExternalProviderKind},
                oauth_client::{ClientType, MfaPolicy, OAuthClient},
                tenant::Tenant,
            },
            errors::DomainError,
        },
        testing::in_memory_repositories::{
            in_memory_custom_field_definition_repository::InMemoryCustomFieldDefinitionRepository,
            in_memory_external_identity_provider_repository::InMemoryExternalIdentityProviderRepository,
            in_memory_oauth_client_repository::InMemoryOAuthClientRepository,
            in_memory_tenant_repository::InMemoryTenantRepository,
        },
    };

    fn make_tenant(id: Uuid, is_active: bool, can_register: bool, can_forgot: bool) -> Tenant {
        Tenant::reconstruct(
            id,
            "T".to_string(),
            None,
            "s".to_string(),
            is_active,
            can_register,
            can_forgot,
            uuid::Uuid::now_v7(),
            chrono::Utc::now(),
            None,
        )
    }

    fn client_id() -> Uuid {
        Uuid::parse_str("01890000-0000-7000-8000-000000000001").unwrap()
    }

    fn make_client(tenant_id: Uuid, client_id: Uuid, is_active: bool) -> OAuthClient {
        OAuthClient::reconstruct(
            Uuid::now_v7(),
            tenant_id,
            client_id,
            None,
            ClientType::Public,
            "App".to_string(),
            vec!["https://app.example.com/cb".to_string()],
            vec![],
            vec!["openid".to_string()],
            vec!["authorization_code".to_string()],
            true,
            true,
            None,
            Some("https://app.example.com/terms".to_string()),
            Some("https://app.example.com/privacy".to_string()),
            None,
            vec!["en".to_string()],
            "en".to_string(),
            MfaPolicy::default(),
            is_active,
            chrono::Utc::now(),
            None,
        )
    }

    fn setup(tenant: Option<Tenant>, client: Option<&OAuthClient>) -> UseCase {
        let tenants = Arc::new(InMemoryTenantRepository::new());
        if let Some(t) = tenant {
            tenants.memory.data.lock().unwrap().insert(t.id(), t);
        }

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        if let Some(c) = client {
            clients
                .memory
                .data
                .lock()
                .unwrap()
                .insert(c.id(), c.clone());
        }
        UseCase::new(
            clients,
            tenants,
            Arc::new(InMemoryExternalIdentityProviderRepository::new()),
            Arc::new(InMemoryCustomFieldDefinitionRepository::new()),
        )
    }

    #[tokio::test]
    async fn returns_public_auth_data_with_tenant_flags() {
        let tenant_id = Uuid::now_v7();
        let tenant = make_tenant(tenant_id, true, true, false);
        let client = make_client(tenant_id, client_id(), true);
        let use_case = setup(Some(tenant), Some(&client));

        let view = use_case
            .execute(Query {
                client_id: client_id().to_string(),
                requested_locale: None,
            })
            .await
            .unwrap();

        assert_eq!(view.id, client_id().to_string());
        assert_eq!(view.name, "App");
        assert!(view.can_self_sign_up);
        assert!(!view.can_self_forgot_pass);
        assert_eq!(
            view.tos_uri.as_deref(),
            Some("https://app.example.com/terms")
        );
        assert_eq!(
            view.policy_uri.as_deref(),
            Some("https://app.example.com/privacy")
        );
        assert!(view.oauth_providers.is_empty());
        assert!(!view.external_login_only);
    }

    // The flag surfaces so the hosted page can drop the password form, AND it forces
    // both self-service entry points off — there is no password to create or reset,
    // whatever the tenant allows.
    #[tokio::test]
    async fn external_login_only_forces_both_self_service_flags_off() {
        let tenant_id = Uuid::now_v7();
        // Tenant permits BOTH self-registration and self-service reset.
        let tenant = make_tenant(tenant_id, true, true, true);
        let mut client = make_client(tenant_id, client_id(), true);
        client.set_external_login_only(true);
        let use_case = setup(Some(tenant), Some(&client));

        let view = use_case
            .execute(Query {
                client_id: client_id().to_string(),
                requested_locale: None,
            })
            .await
            .unwrap();

        assert!(view.external_login_only);
        assert!(!view.can_self_sign_up);
        assert!(!view.can_self_forgot_pass);
    }

    #[tokio::test]
    async fn surfaces_only_active_external_providers() {
        let tenant_id = Uuid::now_v7();
        let tenant = make_tenant(tenant_id, true, true, true);
        let client = make_client(tenant_id, client_id(), true);

        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let providers = Arc::new(InMemoryExternalIdentityProviderRepository::new());
        let active = ExternalIdentityProvider::new(
            client.id(),
            tenant_id,
            ExternalProviderKind::Google,
            "g-id".to_string(),
            "enc:secret".to_string(),
            "https://idp.example.com/cb/google".to_string(),
            true,
        );
        let inactive = ExternalIdentityProvider::new(
            client.id(),
            tenant_id,
            ExternalProviderKind::Github,
            "gh-id".to_string(),
            "enc:secret".to_string(),
            "https://idp.example.com/cb/github".to_string(),
            false,
        );
        {
            let mut data = providers.memory.data.lock().unwrap();
            data.insert(active.id(), active);
            data.insert(inactive.id(), inactive);
        }

        let use_case = UseCase::new(
            clients,
            tenants,
            providers,
            Arc::new(InMemoryCustomFieldDefinitionRepository::new()),
        );
        let view = use_case
            .execute(Query {
                client_id: client_id().to_string(),
                requested_locale: None,
            })
            .await
            .unwrap();

        // Only the active Google provider surfaces as a button.
        assert_eq!(view.oauth_providers.len(), 1);
        assert_eq!(view.oauth_providers[0].name, "google");
    }

    #[tokio::test]
    async fn surfaces_active_custom_field_definitions_in_display_order() {
        use std::collections::HashMap;

        use crate::domain::entities::custom_field_definition::{
            CustomFieldDefinition, CustomFieldType,
        };

        fn locale_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }

        let tenant_id = Uuid::now_v7();
        let tenant = make_tenant(tenant_id, true, true, true);
        let client = make_client(tenant_id, client_id(), true);

        let tenants = Arc::new(InMemoryTenantRepository::new());
        tenants
            .memory
            .data
            .lock()
            .unwrap()
            .insert(tenant.id(), tenant);

        let clients = Arc::new(InMemoryOAuthClientRepository::new());
        clients
            .memory
            .data
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());

        let fields = Arc::new(InMemoryCustomFieldDefinitionRepository::new());
        let active = CustomFieldDefinition::new(
            tenant_id,
            "referral_source".to_string(),
            locale_map(&[("en", "How did you hear about us?")]),
            None,
            CustomFieldType::Text,
            None,
            true,
            0,
        );
        let mut inactive = CustomFieldDefinition::new(
            tenant_id,
            "legacy".to_string(),
            locale_map(&[("en", "Legacy")]),
            None,
            CustomFieldType::Text,
            None,
            false,
            1,
        );
        inactive.update(locale_map(&[("en", "Legacy")]), None, None, false, 1, false);
        {
            let mut data = fields.memory.data.lock().unwrap();
            data.insert(active.id(), active);
            data.insert(inactive.id(), inactive);
        }

        let use_case = UseCase::new(
            clients,
            tenants,
            Arc::new(InMemoryExternalIdentityProviderRepository::new()),
            fields,
        );
        let view = use_case
            .execute(Query {
                client_id: client_id().to_string(),
                requested_locale: None,
            })
            .await
            .unwrap();

        assert_eq!(view.custom_field_definitions.len(), 1);
        assert_eq!(
            view.custom_field_definitions[0].field_key,
            "referral_source"
        );
        assert_eq!(
            view.custom_field_definitions[0].label,
            "How did you hear about us?"
        );
    }

    #[tokio::test]
    async fn not_found_for_unknown_client() {
        let tenant_id = Uuid::now_v7();
        let use_case = setup(Some(make_tenant(tenant_id, true, true, true)), None);

        let err = use_case
            .execute(Query {
                client_id: "nope".to_string(),
                requested_locale: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn not_found_when_client_inactive() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, client_id(), false);
        let use_case = setup(
            Some(make_tenant(tenant_id, true, true, true)),
            Some(&client),
        );

        let err = use_case
            .execute(Query {
                client_id: client_id().to_string(),
                requested_locale: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    #[tokio::test]
    async fn not_found_when_tenant_inactive() {
        let tenant_id = Uuid::now_v7();
        let client = make_client(tenant_id, client_id(), true);
        let use_case = setup(
            Some(make_tenant(tenant_id, false, true, true)),
            Some(&client),
        );

        let err = use_case
            .execute(Query {
                client_id: client_id().to_string(),
                requested_locale: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
