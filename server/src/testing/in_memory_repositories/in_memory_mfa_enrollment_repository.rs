use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    domain::{
        entities::{mfa_enrollment::MfaEnrollment, mfa_method::MfaMethod},
        errors::DomainError,
        repositories::mfa_enrollment_repository::MfaEnrollmentRepository,
    },
    testing::in_memory_unit_of_work::InMemoryRepository,
};

pub struct InMemoryMfaEnrollmentRepository {
    pub memory: Arc<InMemoryRepository<Uuid, MfaEnrollment>>,
}

impl InMemoryMfaEnrollmentRepository {
    pub fn new() -> Self {
        Self {
            memory: Arc::new(InMemoryRepository {
                data: Mutex::new(HashMap::new()),
                draft: Mutex::new(HashMap::new()),
                deleted: Mutex::new(HashSet::new()),
            }),
        }
    }

    // Reads see committed data minus pending deletes, plus pending writes —
    // mirroring what a transaction would observe.
    fn visible(&self) -> Vec<MfaEnrollment> {
        let data = self.memory.data.lock().unwrap();
        let draft = self.memory.draft.lock().unwrap();
        let deleted = self.memory.deleted.lock().unwrap();
        let mut merged: HashMap<Uuid, MfaEnrollment> = data.clone();
        for (k, v) in draft.iter() {
            merged.insert(*k, v.clone());
        }
        merged.retain(|k, _| !deleted.contains(k));
        merged.into_values().collect()
    }
}

#[async_trait]
impl MfaEnrollmentRepository for InMemoryMfaEnrollmentRepository {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<MfaEnrollment>, DomainError> {
        Ok(self
            .visible()
            .into_iter()
            .find(|e| e.tenant_id() == tenant_id && e.id() == id))
    }

    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<MfaEnrollment>, DomainError> {
        let mut found: Vec<_> = self
            .visible()
            .into_iter()
            .filter(|e| e.user_id() == user_id)
            .collect();
        found.sort_by_key(|e| e.created_at());
        Ok(found)
    }

    async fn find_confirmed_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<MfaEnrollment>, DomainError> {
        let mut found: Vec<_> = self
            .visible()
            .into_iter()
            .filter(|e| e.user_id() == user_id && e.is_confirmed())
            .collect();
        found.sort_by_key(|e| e.created_at());
        Ok(found)
    }

    async fn find_totp_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<MfaEnrollment>, DomainError> {
        Ok(self
            .visible()
            .into_iter()
            .find(|e| e.user_id() == user_id && *e.method() == MfaMethod::Totp))
    }

    async fn find_by_credential_id(
        &self,
        credential_id: &str,
    ) -> Result<Option<MfaEnrollment>, DomainError> {
        Ok(self
            .visible()
            .into_iter()
            .find(|e| e.credential_id() == Some(credential_id)))
    }

    async fn add(&self, enrollment: &MfaEnrollment) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment.clone());
        Ok(())
    }

    async fn update(&self, enrollment: &MfaEnrollment) -> Result<(), DomainError> {
        self.memory
            .draft
            .lock()
            .unwrap()
            .insert(enrollment.id(), enrollment.clone());
        Ok(())
    }

    async fn delete_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<(), DomainError> {
        let exists = self
            .visible()
            .iter()
            .any(|e| e.tenant_id() == tenant_id && e.id() == id);
        if !exists {
            return Err(DomainError::MfaError(
                "MFA enrollment not found".to_string(),
            ));
        }
        self.memory.deleted.lock().unwrap().insert(id);
        Ok(())
    }
}
