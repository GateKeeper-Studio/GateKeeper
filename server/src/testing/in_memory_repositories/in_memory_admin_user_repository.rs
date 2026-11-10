use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::admin_user::{AdminSummary, AdminUser},
    errors::DomainError,
    repositories::admin_user_repository::AdminUserRepository,
};

// In-memory admin account store for use-case tests. Admin repositories are called
// directly (no Unit of Work), so there is no draft/commit split.
pub struct InMemoryAdminUserRepository {
    pub admins: Mutex<Vec<AdminUser>>,
}

impl InMemoryAdminUserRepository {
    pub fn new() -> Self {
        Self {
            admins: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryAdminUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AdminUserRepository for InMemoryAdminUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>, DomainError> {
        Ok(self
            .admins
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.email() == email)
            .cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AdminUser>, DomainError> {
        Ok(self
            .admins
            .lock()
            .unwrap()
            .iter()
            .find(|a| a.id() == id)
            .cloned())
    }

    async fn create(&self, admin: &AdminUser) -> Result<(), DomainError> {
        self.admins.lock().unwrap().push(admin.clone());
        Ok(())
    }

    async fn update(&self, admin: &AdminUser) -> Result<(), DomainError> {
        let mut admins = self.admins.lock().unwrap();

        // Reject an email already used by a *different* admin (mirrors the unique index).
        if admins
            .iter()
            .any(|a| a.id() != admin.id() && a.email() == admin.email())
        {
            return Err(DomainError::Conflict(
                "an admin with that email already exists".to_string(),
            ));
        }

        if let Some(slot) = admins.iter_mut().find(|a| a.id() == admin.id()) {
            *slot = admin.clone();
        }
        Ok(())
    }

    async fn deactivate_unless_last_active(
        &self,
        admin: &AdminUser,
        _at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut admins = self.admins.lock().unwrap();
        let active = admins
            .iter()
            .filter(|a| a.organization_id() == admin.organization_id() && a.is_active())
            .count();
        let Some(slot) = admins
            .iter_mut()
            .find(|a| a.id() == admin.id() && a.is_active())
        else {
            return Ok(false);
        };
        if active <= 1 {
            return Ok(false);
        }
        slot.deactivate();
        Ok(true)
    }

    async fn register_failed_sign_in(
        &self,
        id: Uuid,
        max_failures: i32,
        lock_until: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut admins = self.admins.lock().unwrap();
        let Some(slot) = admins.iter_mut().find(|a| a.id() == id) else {
            return Ok(false);
        };
        let failures = slot.failed_sign_in_count() + 1;
        let (count, locked_until, locked_now) = if failures >= max_failures {
            (0, Some(lock_until), true)
        } else {
            (failures, slot.locked_until(), false)
        };
        let organization_active = slot.organization_active();
        *slot = slot
            .clone()
            .with_sign_in_state(organization_active, count, locked_until);
        Ok(locked_now)
    }

    async fn record_successful_sign_in(
        &self,
        id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let mut admins = self.admins.lock().unwrap();
        if let Some(slot) = admins.iter_mut().find(|a| a.id() == id) {
            let organization_active = slot.organization_active();
            let requires_mfa = slot.organization_requires_mfa();
            *slot = slot
                .clone()
                .with_sign_in_state(organization_active, 0, None)
                .with_account_state(requires_mfa, Some(at));
        }
        Ok(())
    }

    // `mfa_enabled` is always false here: the in-memory stores are independent, so
    // tests that need it read the factor repository.
    async fn list_by_organization(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<AdminSummary>, DomainError> {
        let mut admins: Vec<AdminSummary> = self
            .admins
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.organization_id() == organization_id)
            .map(|a| AdminSummary {
                id: a.id(),
                email: a.email().to_string(),
                name: a.name().to_string(),
                is_active: a.is_active(),
                mfa_enabled: false,
                locked_until: a.locked_until(),
                last_sign_in_at: a.last_sign_in_at(),
                created_at: a.created_at(),
            })
            .collect();
        admins.sort_by_key(|a| (a.created_at, a.id));
        Ok(admins)
    }

    async fn reactivate(
        &self,
        id: Uuid,
        organization_id: Uuid,
        _at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut admins = self.admins.lock().unwrap();
        match admins
            .iter_mut()
            .find(|a| a.id() == id && a.organization_id() == organization_id && !a.is_active())
        {
            Some(slot) => {
                *slot = AdminUser::reconstruct(
                    slot.id(),
                    slot.organization_id(),
                    slot.email().to_string(),
                    slot.name().to_string(),
                    slot.password_hash().to_string(),
                    slot.password_algorithm().to_string(),
                    true,
                    slot.created_at(),
                    Some(Utc::now()),
                )
                .with_account_state(slot.organization_requires_mfa(), slot.last_sign_in_at());
                Ok(true)
            }
            None => Ok(false),
        }
    }
}
