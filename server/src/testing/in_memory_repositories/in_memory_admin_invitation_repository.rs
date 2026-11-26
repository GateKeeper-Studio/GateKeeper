use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    domain::{
        entities::{admin_invitation::AdminInvitation, admin_user::AdminUser},
        errors::DomainError,
        repositories::admin_invitation_repository::AdminInvitationRepository,
    },
    testing::in_memory_repositories::in_memory_admin_user_repository::InMemoryAdminUserRepository,
};

// In-memory invitations for use-case tests. Accepting creates the admin in the
// admin store it was built with, mirroring the database transaction.
pub struct InMemoryAdminInvitationRepository {
    pub invitations: Mutex<Vec<AdminInvitation>>,
    admins: Arc<InMemoryAdminUserRepository>,
}

impl InMemoryAdminInvitationRepository {
    pub fn new(admins: Arc<InMemoryAdminUserRepository>) -> Self {
        Self {
            invitations: Mutex::new(Vec::new()),
            admins,
        }
    }
}

fn with_state(
    invitation: &AdminInvitation,
    accepted_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
) -> AdminInvitation {
    AdminInvitation::reconstruct(
        invitation.id(),
        invitation.organization_id(),
        invitation.email().to_string(),
        invitation.token_hash().to_string(),
        invitation.invited_by(),
        invitation.expires_at(),
        accepted_at,
        revoked_at,
        invitation.created_at(),
    )
}

#[async_trait]
impl AdminInvitationRepository for InMemoryAdminInvitationRepository {
    async fn create_replacing_open(&self, invitation: &AdminInvitation) -> Result<(), DomainError> {
        let mut invitations = self.invitations.lock().unwrap();
        for slot in invitations.iter_mut() {
            if slot.organization_id() == invitation.organization_id()
                && slot.email() == invitation.email()
                && slot.accepted_at().is_none()
                && slot.revoked_at().is_none()
            {
                *slot = with_state(slot, None, Some(invitation.created_at()));
            }
        }
        invitations.push(invitation.clone());
        Ok(())
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<AdminInvitation>, DomainError> {
        Ok(self
            .invitations
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.token_hash() == token_hash)
            .cloned())
    }

    async fn list_open(
        &self,
        organization_id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<Vec<AdminInvitation>, DomainError> {
        let mut open: Vec<_> = self
            .invitations
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.organization_id() == organization_id && i.is_open(now))
            .cloned()
            .collect();
        open.sort_by_key(|i| std::cmp::Reverse(i.created_at()));
        Ok(open)
    }

    async fn revoke(
        &self,
        id: Uuid,
        organization_id: Uuid,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        let mut invitations = self.invitations.lock().unwrap();
        match invitations
            .iter_mut()
            .find(|i| i.id() == id && i.organization_id() == organization_id && i.is_open(at))
        {
            Some(slot) => {
                *slot = with_state(slot, None, Some(at));
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn accept(
        &self,
        invitation_id: Uuid,
        admin: &AdminUser,
        at: DateTime<Utc>,
    ) -> Result<bool, DomainError> {
        // Synchronous throughout: no lock is held across an await.
        let mut invitations = self.invitations.lock().unwrap();
        let Some(slot) = invitations.iter_mut().find(|i| {
            i.id() == invitation_id
                && i.organization_id() == admin.organization_id()
                && i.email() == admin.email()
                && i.is_open(at)
        }) else {
            return Ok(false);
        };
        let mut admins = self.admins.admins.lock().unwrap();
        if admins.iter().any(|a| a.email() == admin.email()) {
            return Err(DomainError::Conflict(
                "an admin with that email already exists".to_string(),
            ));
        }
        admins.push(admin.clone());
        *slot = with_state(slot, Some(at), None);
        Ok(true)
    }

    async fn delete_expired(&self, now: DateTime<Utc>) -> Result<u64, DomainError> {
        let mut invitations = self.invitations.lock().unwrap();
        let before = invitations.len();
        invitations.retain(|i| i.expires_at() > now);
        Ok((before - invitations.len()) as u64)
    }
}
