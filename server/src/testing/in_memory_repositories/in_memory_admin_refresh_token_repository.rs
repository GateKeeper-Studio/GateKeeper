use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::admin_refresh_token::AdminRefreshToken, errors::DomainError,
    repositories::admin_refresh_token_repository::AdminRefreshTokenRepository,
};

// In-memory admin refresh-token store for use-case tests. Admin refresh flows call
// the repository directly (no Unit of Work), so this keeps a single flat vector and
// mutates it in place — no draft/commit split.
pub struct InMemoryAdminRefreshTokenRepository {
    pub tokens: Mutex<Vec<AdminRefreshToken>>,
}

impl InMemoryAdminRefreshTokenRepository {
    pub fn new() -> Self {
        Self {
            tokens: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryAdminRefreshTokenRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AdminRefreshTokenRepository for InMemoryAdminRefreshTokenRepository {
    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminRefreshToken>, DomainError> {
        Ok(self
            .tokens
            .lock()
            .unwrap()
            .iter()
            .find(|t| t.token_hash() == token_hash)
            .cloned())
    }

    async fn add(&self, token: &AdminRefreshToken) -> Result<(), DomainError> {
        self.tokens.lock().unwrap().push(token.clone());
        Ok(())
    }

    async fn rotate(
        &self,
        old: &AdminRefreshToken,
        new: &AdminRefreshToken,
    ) -> Result<bool, DomainError> {
        // Same compare-and-set as the database: only a still-live token rotates.
        let mut tokens = self.tokens.lock().unwrap();
        let Some(rotated_at) = old.rotated_at() else {
            return Ok(false);
        };
        match tokens.iter_mut().find(|t| t.id() == old.id()) {
            Some(slot) if slot.is_active(rotated_at) => *slot = old.clone(),
            _ => return Ok(false),
        }
        tokens.push(new.clone());
        Ok(true)
    }

    async fn revoke_family(&self, family_id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now();
        let mut tokens = self.tokens.lock().unwrap();
        for token in tokens.iter_mut() {
            if token.family_id() == family_id && token.revoked_at().is_none() {
                token.revoke(now);
            }
        }
        Ok(())
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64, DomainError> {
        let mut tokens = self.tokens.lock().unwrap();
        let before = tokens.len();
        tokens.retain(|t| t.expires_at() > now);
        Ok((before - tokens.len()) as u64)
    }

    async fn revoke_all_for_admin(&self, admin_id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now();
        let mut tokens = self.tokens.lock().unwrap();
        for token in tokens.iter_mut() {
            if token.admin_id() == admin_id && token.revoked_at().is_none() {
                token.revoke(now);
            }
        }
        Ok(())
    }
}
