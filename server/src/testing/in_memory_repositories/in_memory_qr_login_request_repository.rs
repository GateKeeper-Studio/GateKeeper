use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::qr_login_request::{QrApprover, QrLoginRequest},
        errors::DomainError,
        repositories::qr_login_request_repository::QrLoginRequestRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryQrLoginRequestRepository {
    pub memory: Arc<InMemoryRepository<Uuid, QrLoginRequest>>,
}

impl InMemoryQrLoginRequestRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    // Committed rows, for test assertions.
    pub fn all(&self) -> Vec<QrLoginRequest> {
        self.memory.data.lock().unwrap().values().cloned().collect()
    }

    pub fn get(&self, id: Uuid) -> Option<QrLoginRequest> {
        self.memory.data.lock().unwrap().get(&id).cloned()
    }

    // Read-your-writes view of one row.
    fn current(&self, id: Uuid) -> Option<QrLoginRequest> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        draft.get(&id).or_else(|| data.get(&id)).cloned()
    }

    // Applies `change` when `condition` holds, like the SQL's conditional UPDATE.
    fn update_if(
        &self,
        id: Uuid,
        condition: impl Fn(&QrLoginRequest) -> bool,
        change: impl FnOnce(&mut QrLoginRequest),
    ) -> bool {
        let Some(mut row) = self.current(id).filter(|r| condition(r)) else {
            return false;
        };
        change(&mut row);
        self.memory.draft.lock().unwrap().insert(id, row);
        true
    }
}

#[async_trait]
impl QrLoginRequestRepository for InMemoryQrLoginRequestRepository {
    async fn add(&self, request: &QrLoginRequest) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(request.id(), request.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<QrLoginRequest>, DomainError> {
        Ok(self.current(id))
    }

    async fn find_by_auth_session(
        &self,
        auth_session_id: Uuid,
    ) -> Result<Option<QrLoginRequest>, DomainError> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        Ok(draft
            .values()
            .chain(data.values())
            .find(|r| r.auth_session_id() == auth_session_id)
            .cloned())
    }

    async fn claim(
        &self,
        id: Uuid,
        claim_hash: &str,
        scanned_by_user_id: Option<Uuid>,
        at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        Ok(self.update_if(
            id,
            |r| r.is_claimable(at),
            |r| r.claim(claim_hash.to_string(), scanned_by_user_id, at, expires_at),
        ))
    }

    async fn approve(
        &self,
        id: Uuid,
        claim_hash: &str,
        user_id: Uuid,
        mfa_verified: bool,
        via: QrApprover,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        Ok(self.update_if(
            id,
            |r| r.is_decidable_by(claim_hash, Some(user_id), at),
            |r| r.approve(user_id, mfa_verified, via, at),
        ))
    }

    async fn deny(
        &self,
        id: Uuid,
        claim_hash: &str,
        user_id: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        Ok(self.update_if(
            id,
            |r| r.is_decidable_by(claim_hash, user_id, at),
            |r| r.deny(at),
        ))
    }

    async fn complete(&self, id: Uuid, at: DateTime<Utc>) -> Result<bool, DomainError> {
        Ok(self.update_if(
            id,
            |r| r.status() == crate::domain::entities::qr_login_request::QrLoginStatus::Approved,
            |r| r.complete(at),
        ))
    }
}
