use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::refresh_token::RefreshToken,
        errors::DomainError,
        repositories::refresh_token_repository::{RefreshTokenFamily, RefreshTokenRepository},
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryRefreshTokenRepository {
    pub memory: Arc<InMemoryRepository<Uuid, RefreshToken>>,
}

impl InMemoryRefreshTokenRepository {
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
impl RefreshTokenRepository for InMemoryRefreshTokenRepository {
    async fn find_by_token_hash(
        &self,
        tenant_id: Uuid,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        Ok(data
            .values()
            .find(|t| t.tenant_id() == tenant_id && t.token_hash() == token_hash)
            .cloned())
    }

    async fn add(&self, token: &RefreshToken) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(token.id(), token.clone());
        Ok(())
    }

    async fn rotate(&self, old: &RefreshToken, new: &RefreshToken) -> Result<(), DomainError> {
        let mut draft = self.memory.draft.lock().unwrap();
        draft.insert(old.id(), old.clone());
        draft.insert(new.id(), new.clone());
        Ok(())
    }

    async fn revoke_family(&self, tenant_id: Uuid, family_id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now();
        let data = self.memory.data.lock().unwrap();
        let mut draft = self.memory.draft.lock().unwrap();

        // Revoke every still-live token in the family (committed and drafted alike).
        let members: Vec<RefreshToken> = data
            .values()
            .chain(draft.values())
            .filter(|t| {
                t.tenant_id() == tenant_id && t.family_id() == family_id && t.revoked_at().is_none()
            })
            .cloned()
            .collect();

        for mut token in members {
            token.revoke(now);
            draft.insert(token.id(), token);
        }

        Ok(())
    }

    async fn revoke_by_user_and_client(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        client_id: Uuid,
    ) -> Result<(), DomainError> {
        let now = Utc::now();
        let data = self.memory.data.lock().unwrap();
        let mut draft = self.memory.draft.lock().unwrap();

        let members: Vec<RefreshToken> = data
            .values()
            .chain(draft.values())
            .filter(|t| {
                t.tenant_id() == tenant_id
                    && t.user_id() == user_id
                    && t.client_id() == client_id
                    && t.revoked_at().is_none()
            })
            .cloned()
            .collect();

        for mut token in members {
            token.revoke(now);
            draft.insert(token.id(), token);
        }

        Ok(())
    }

    async fn revoke_by_user(&self, tenant_id: Uuid, user_id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now();
        let data = self.memory.data.lock().unwrap();
        let mut draft = self.memory.draft.lock().unwrap();

        let members: Vec<RefreshToken> = data
            .values()
            .chain(draft.values())
            .filter(|t| {
                t.tenant_id() == tenant_id && t.user_id() == user_id && t.revoked_at().is_none()
            })
            .cloned()
            .collect();

        for mut token in members {
            token.revoke(now);
            draft.insert(token.id(), token);
        }

        Ok(())
    }

    async fn find_active_families_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<RefreshTokenFamily>, DomainError> {
        // Mirrors the SQL: group the user's committed tokens by family, keep a family
        // when its newest token is live.
        let data = self.memory.data.lock().unwrap();
        let mut families: HashMap<Uuid, Vec<&RefreshToken>> = HashMap::new();
        for token in data
            .values()
            .filter(|t| t.tenant_id() == tenant_id && t.user_id() == user_id)
        {
            families.entry(token.family_id()).or_default().push(token);
        }

        let mut out: Vec<RefreshTokenFamily> = families
            .into_iter()
            .filter_map(|(family_id, tokens)| {
                let latest = tokens
                    .iter()
                    .max_by_key(|t| (t.created_at(), t.id()))
                    .copied()?;
                if !latest.is_active(now) {
                    return None;
                }
                let signed_in_at = tokens.iter().map(|t| t.created_at()).min()?;
                Some(RefreshTokenFamily {
                    family_id,
                    client_id: latest.client_id(),
                    scopes: latest.scopes().to_vec(),
                    signed_in_at,
                    last_used_at: latest.created_at(),
                    expires_at: latest.expires_at(),
                })
            })
            .collect();
        out.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        Ok(out)
    }

    async fn revoke_family_of_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        family_id: Uuid,
    ) -> Result<bool, DomainError> {
        let now = Utc::now();
        let data = self.memory.data.lock().unwrap();
        let mut draft = self.memory.draft.lock().unwrap();

        let owned: Vec<RefreshToken> = data
            .values()
            .chain(draft.values())
            .filter(|t| {
                t.tenant_id() == tenant_id && t.user_id() == user_id && t.family_id() == family_id
            })
            .cloned()
            .collect();
        let found = !owned.is_empty();

        for mut token in owned.into_iter().filter(|t| t.revoked_at().is_none()) {
            token.revoke(now);
            draft.insert(token.id(), token);
        }

        Ok(found)
    }

    async fn delete_expired(&self, before: chrono::DateTime<Utc>) -> Result<u64, DomainError> {
        let ids: Vec<Uuid> = self
            .memory
            .data
            .lock()
            .unwrap()
            .values()
            .filter(|t| t.expires_at() < before)
            .map(|t| t.id())
            .collect();
        let mut deleted = self.memory.deleted.lock().unwrap();
        let count = ids.len() as u64;
        for id in ids {
            deleted.insert(id);
        }
        Ok(count)
    }
}
