use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{
            mfa_challenge::{MfaChallenge, MfaChallengePurpose},
            mfa_method::MfaMethod,
        },
        errors::DomainError,
        repositories::mfa_challenge_repository::MfaChallengeRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryMfaChallengeRepository {
    pub memory: Arc<InMemoryRepository<Uuid, MfaChallenge>>,
}

impl InMemoryMfaChallengeRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    fn visible(&self) -> Vec<MfaChallenge> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let mut merged: HashMap<Uuid, MfaChallenge> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        merged.into_values().collect()
    }
}

#[async_trait]
impl MfaChallengeRepository for InMemoryMfaChallengeRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<MfaChallenge>, DomainError> {
        Ok(self
            .visible()
            .into_iter()
            .find(|c| c.tenant_id() == tenant_id && c.id() == id))
    }

    async fn find_active_by_session(
        &self,
        auth_session_id: Uuid,
        method: &MfaMethod,
    ) -> Result<Option<MfaChallenge>, DomainError> {
        Ok(self
            .visible()
            .into_iter()
            .filter(|c| {
                c.auth_session_id() == Some(auth_session_id)
                    && c.method() == method
                    && !c.is_consumed()
            })
            .max_by_key(|c| c.created_at()))
    }

    async fn add(&self, challenge: &MfaChallenge) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(challenge.id(), challenge.clone());
        Ok(())
    }

    async fn update(&self, challenge: &MfaChallenge) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(challenge.id(), challenge.clone());
        Ok(())
    }

    async fn invalidate_active_by_user(
        &self,
        user_id: Uuid,
        purpose: &MfaChallengePurpose,
        method: &MfaMethod,
    ) -> Result<(), DomainError> {
        let now = Utc::now();
        let stale: Vec<MfaChallenge> = self
            .visible()
            .into_iter()
            .filter(|c| {
                c.user_id() == user_id
                    && c.purpose() == purpose
                    && c.method() == method
                    && !c.is_consumed()
            })
            .collect();
        let mut draft = self.memory.draft.lock().unwrap();
        for mut challenge in stale {
            challenge.consume(now);
            draft.insert(challenge.id(), challenge);
        }
        Ok(())
    }

    async fn delete_expired(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, DomainError> {
        let mut data = self.memory.data.lock().unwrap();
        let before_count = data.len();
        data.retain(|_, row| row.expires_at() >= before);
        Ok((before_count - data.len()) as u64)
    }
}
