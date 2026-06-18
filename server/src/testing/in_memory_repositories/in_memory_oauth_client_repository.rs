use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::oauth_client::OAuthClient, errors::DomainError,
        repositories::oauth_client_repository::OAuthClientRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryOAuthClientRepository {
    pub memory: Arc<InMemoryRepository<Uuid, OAuthClient>>,
}

impl InMemoryOAuthClientRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }
}

#[async_trait]
impl OAuthClientRepository for InMemoryOAuthClientRepository {
    async fn find_by_client_id(
        &self,
        tenant_id: Uuid,
        client_id: &str,
    ) -> Result<Option<OAuthClient>, DomainError> {
        let Ok(client_id) = Uuid::parse_str(client_id) else {
            return Ok(None);
        };
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|c| c.tenant_id() == tenant_id && c.client_id() == client_id)
            .cloned())
    }

    async fn find_by_client_id_unscoped(
        &self,
        client_id: &str,
    ) -> Result<Option<OAuthClient>, DomainError> {
        let Ok(client_id) = Uuid::parse_str(client_id) else {
            return Ok(None);
        };
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().find(|c| c.client_id() == client_id).cloned())
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<OAuthClient>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .get(&id)
            .filter(|c| c.tenant_id() == tenant_id)
            .cloned())
    }

    async fn find_by_tenant_id(
        &self,
        tenant_id: Uuid,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<OAuthClient>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let mut clients: Vec<OAuthClient> = data
            .values()
            .filter(|c| c.tenant_id() == tenant_id)
            .cloned()
            .collect();
        // Deterministic order (newest first), mirroring the DB query.
        clients.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        let start = ((page.max(1) - 1) * page_size) as usize;
        Ok(clients
            .into_iter()
            .skip(start)
            .take(page_size as usize)
            .collect())
    }

    async fn count_by_tenant_id(&self, tenant_id: Uuid) -> Result<u32, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data.values().filter(|c| c.tenant_id() == tenant_id).count() as u32)
    }

    async fn add(&self, client: &OAuthClient) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());
        Ok(())
    }

    async fn update(&self, client: &OAuthClient) -> Result<(), DomainError> {
        let exists = self.memory.data.lock().unwrap().contains_key(&client.id())
            || self.memory.draft.lock().unwrap().contains_key(&client.id());
        if !exists {
            return Err(DomainError::OAuthClientNotFound(
                "OAuth client not found".to_string(),
            ));
        }
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(client.id(), client.clone());
        Ok(())
    }

    async fn delete_by_id(&self, _tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        self.memory.deleted.lock().unwrap().insert(id);
        Ok(())
    }
}
