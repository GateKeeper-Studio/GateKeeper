#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uuid::Uuid;

    use crate::{
        application::use_cases::webhook_subscription::update_webhook_subscription::{
            command::Command, use_case::UseCase,
        },
        domain::{entities::webhook_subscription::WebhookSubscription, errors::DomainError},
        testing::{
            in_memory_repositories::{
                in_memory_tenant_repository::InMemoryTenantRepository,
                in_memory_user_repository::InMemoryUserRepository,
                in_memory_webhook_subscription_repository::InMemoryWebhookSubscriptionRepository,
            },
            in_memory_unit_of_work::InMemoryUnitOfWorkFactory,
            stub_secret_cipher::StubSecretCipher,
        },
    };

    fn existing(tenant_id: Uuid) -> WebhookSubscription {
        WebhookSubscription::new(
            tenant_id,
            "Old name".to_string(),
            "https://old.example.com/hook".to_string(),
            "enc:original-secret".to_string(),
            vec!["user.created".to_string()],
            true,
        )
    }

    fn setup(
        subscription: Option<&WebhookSubscription>,
    ) -> (UseCase, Arc<InMemoryWebhookSubscriptionRepository>) {
        let subscriptions = Arc::new(InMemoryWebhookSubscriptionRepository::new());
        if let Some(s) = subscription {
            subscriptions
                .memory
                .data
                .lock()
                .unwrap()
                .insert(s.id(), s.clone());
        }

        let factory = InMemoryUnitOfWorkFactory::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryTenantRepository::new()),
        )
        .with_webhook_subscription_repository(subscriptions.clone());

        (
            UseCase::new(Arc::new(factory), Arc::new(StubSecretCipher), false),
            subscriptions,
        )
    }

    fn command(tenant_id: Uuid, id: Uuid) -> Command {
        Command {
            tenant_id,
            id,
            name: "New name".to_string(),
            url: "https://new.example.com/hook".to_string(),
            event_types: vec!["user.updated".to_string()],
            is_active: false,
            rotate_secret: false,
        }
    }

    #[tokio::test]
    async fn updates_editable_details() {
        let tenant_id = Uuid::now_v7();
        let sub = existing(tenant_id);
        let (use_case, subscriptions) = setup(Some(&sub));

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                command(tenant_id, sub.id()),
            )
            .await
            .unwrap();

        assert_eq!(response.subscription.name, "New name");
        assert_eq!(response.subscription.url, "https://new.example.com/hook");
        assert_eq!(response.subscription.event_types, vec!["user.updated"]);
        assert!(!response.subscription.is_active);

        let stored = subscriptions.memory.data.lock().unwrap();
        assert_eq!(stored.get(&sub.id()).unwrap().name(), "New name");
    }

    // An ordinary edit must NOT invalidate every consumer's signature verification.
    #[tokio::test]
    async fn an_ordinary_edit_keeps_the_existing_secret_and_returns_none() {
        let tenant_id = Uuid::now_v7();
        let sub = existing(tenant_id);
        let (use_case, subscriptions) = setup(Some(&sub));

        let response = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                command(tenant_id, sub.id()),
            )
            .await
            .unwrap();

        assert!(response.secret.is_none());
        let stored = subscriptions.memory.data.lock().unwrap();
        assert_eq!(
            stored.get(&sub.id()).unwrap().secret_encrypted(),
            "enc:original-secret"
        );
    }

    #[tokio::test]
    async fn rotate_secret_replaces_it_and_returns_the_new_one_once() {
        let tenant_id = Uuid::now_v7();
        let sub = existing(tenant_id);
        let (use_case, subscriptions) = setup(Some(&sub));

        let mut cmd = command(tenant_id, sub.id());
        cmd.rotate_secret = true;

        let response = use_case
            .execute(&crate::application::shared::actor::Actor::System, cmd)
            .await
            .unwrap();
        let secret = response.secret.expect("a rotated secret must be returned");

        let stored = subscriptions.memory.data.lock().unwrap();
        let saved = stored.get(&sub.id()).unwrap();
        assert_ne!(saved.secret_encrypted(), "enc:original-secret");
        assert_eq!(saved.secret_encrypted(), format!("enc:{secret}"));
    }

    // The SSRF check must be re-applied on edit — otherwise an admin could create a
    // benign subscription and then repoint it at the metadata endpoint.
    #[tokio::test]
    async fn re_validates_the_url_on_edit() {
        let tenant_id = Uuid::now_v7();
        let sub = existing(tenant_id);
        let (use_case, subscriptions) = setup(Some(&sub));

        let mut cmd = command(tenant_id, sub.id());
        cmd.url = "https://169.254.169.254/latest/meta-data/".to_string();

        assert!(matches!(
            use_case
                .execute(&crate::application::shared::actor::Actor::System, cmd)
                .await,
            Err(DomainError::InvalidRequest(_))
        ));
        // Unchanged on disk.
        let stored = subscriptions.memory.data.lock().unwrap();
        assert_eq!(
            stored.get(&sub.id()).unwrap().url(),
            "https://old.example.com/hook"
        );
    }

    #[tokio::test]
    async fn refuses_an_unknown_event_type_on_edit() {
        let tenant_id = Uuid::now_v7();
        let sub = existing(tenant_id);
        let (use_case, _) = setup(Some(&sub));

        let mut cmd = command(tenant_id, sub.id());
        cmd.event_types = vec!["user.renamed".to_string()];

        assert!(matches!(
            use_case
                .execute(&crate::application::shared::actor::Actor::System, cmd)
                .await,
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[tokio::test]
    async fn not_found_for_an_unknown_subscription() {
        let (use_case, _) = setup(None);

        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                command(Uuid::now_v7(), Uuid::now_v7()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }

    // Tenant isolation: the lookup is scoped, so another tenant's id is a 404 rather
    // than an edit.
    #[tokio::test]
    async fn cannot_edit_another_tenants_subscription() {
        let owner = Uuid::now_v7();
        let sub = existing(owner);
        let (use_case, _) = setup(Some(&sub));

        let attacker = Uuid::now_v7();
        let err = use_case
            .execute(
                &crate::application::shared::actor::Actor::System,
                command(attacker, sub.id()),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::NotFound(_)));
    }
}
