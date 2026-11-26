use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::domain::entities::admin_user::normalize_email;

// An invitation for someone to become an admin of an organization. The inviter gets
// a one-time link (shown once) and hands it over out of band; only the SHA-256 of
// its token is stored. Accepting it creates the admin account with the password the
// invitee chooses, so no password is ever shared.
#[derive(Debug, Clone)]
pub struct AdminInvitation {
    id: Uuid,
    organization_id: Uuid,
    email: String,
    token_hash: String,
    invited_by: Option<Uuid>,
    expires_at: DateTime<Utc>,
    accepted_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl AdminInvitation {
    pub fn new(
        organization_id: Uuid,
        email: &str,
        token_hash: String,
        invited_by: Uuid,
        now: DateTime<Utc>,
        lifetime: Duration,
    ) -> Self {
        AdminInvitation {
            id: Uuid::now_v7(),
            organization_id,
            email: normalize_email(email),
            token_hash,
            invited_by: Some(invited_by),
            expires_at: now + lifetime,
            accepted_at: None,
            revoked_at: None,
            created_at: now,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        organization_id: Uuid,
        email: String,
        token_hash: String,
        invited_by: Option<Uuid>,
        expires_at: DateTime<Utc>,
        accepted_at: Option<DateTime<Utc>>,
        revoked_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
    ) -> Self {
        AdminInvitation {
            id,
            organization_id,
            email,
            token_hash,
            invited_by,
            expires_at,
            accepted_at,
            revoked_at,
            created_at,
        }
    }

    // Can still be accepted: not accepted, not revoked, not expired.
    pub fn is_open(&self, now: DateTime<Utc>) -> bool {
        self.accepted_at.is_none() && self.revoked_at.is_none() && now < self.expires_at
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn organization_id(&self) -> Uuid {
        self.organization_id
    }
    pub fn email(&self) -> &str {
        &self.email
    }
    pub fn token_hash(&self) -> &str {
        &self.token_hash
    }
    pub fn invited_by(&self) -> Option<Uuid> {
        self.invited_by
    }
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
    pub fn accepted_at(&self) -> Option<DateTime<Utc>> {
        self.accepted_at
    }
    pub fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invitation_is_open_until_it_expires() {
        let now = Utc::now();
        let invitation = AdminInvitation::new(
            Uuid::now_v7(),
            " New@Example.com ",
            "hash".into(),
            Uuid::now_v7(),
            now,
            Duration::hours(72),
        );
        assert_eq!(invitation.email(), "new@example.com");
        assert!(invitation.is_open(now + Duration::hours(71)));
        assert!(!invitation.is_open(now + Duration::hours(72)));
    }
}
