use chrono::{DateTime, Utc};
use uuid::Uuid;

// A platform administrator account. Not tenant-scoped: an admin operates across
// every tenant of its organization through the console. The password is hashed with
// Argon2id (a server-wide pepper); the algorithm string records which scheme so the
// hash stays self-describing if it is ever migrated.
//
// The sign-in email is stored lowercased (`normalize_email`); the database refuses
// anything else.
#[derive(Debug, Clone)]
pub struct AdminUser {
    id: Uuid,
    // The organization this admin belongs to (an organization has many admins).
    organization_id: Uuid,
    email: String,
    // Human display name shown in the dashboard. May be empty for accounts bootstrapped
    // before the admin set one; the UI falls back to deriving a label from the email.
    name: String,
    password_hash: String,
    password_algorithm: String,
    is_active: bool,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,

    // Read-only sign-in state, loaded with the account and written only through the
    // repository's dedicated statements (never by `update`).
    //
    // Whether the admin's organization is active. A suspended organization's admins
    // cannot sign in, refresh, or keep using a console session.
    organization_active: bool,
    // Consecutive failed sign-ins since the last success or lock.
    failed_sign_in_count: i32,
    // While in the future, sign-in is refused whatever the password.
    locked_until: Option<DateTime<Utc>>,
    // The organization requires two-step verification of every admin.
    organization_requires_mfa: bool,
    last_sign_in_at: Option<DateTime<Utc>>,
    // The highest version of the console's guided tour the admin has finished or
    // closed (0: never seen). A console preference, not account data: written only by
    // `AdminUserRepository::record_console_tour`, never by `update`.
    console_tour_version: i32,
}

// One admin as the organization's admin list shows it. A read model: no credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminSummary {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub mfa_enabled: bool,
    pub locked_until: Option<DateTime<Utc>>,
    pub last_sign_in_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// The canonical form of an admin sign-in email: trimmed and lowercased. Applied on
// every write and every lookup, so the address matches whatever case it is typed in.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

impl AdminUser {
    pub fn new(
        organization_id: Uuid,
        email: String,
        name: String,
        password_hash: String,
        password_algorithm: String,
    ) -> Self {
        AdminUser {
            id: Uuid::now_v7(),
            organization_id,
            email: normalize_email(&email),
            name,
            password_hash,
            password_algorithm,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
            organization_active: true,
            failed_sign_in_count: 0,
            locked_until: None,
            organization_requires_mfa: false,
            last_sign_in_at: None,
            console_tour_version: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: Uuid,
        organization_id: Uuid,
        email: String,
        name: String,
        password_hash: String,
        password_algorithm: String,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    ) -> Self {
        AdminUser {
            id,
            organization_id,
            email,
            name,
            password_hash,
            password_algorithm,
            is_active,
            created_at,
            updated_at,
            organization_active: true,
            failed_sign_in_count: 0,
            locked_until: None,
            organization_requires_mfa: false,
            last_sign_in_at: None,
            console_tour_version: 0,
        }
    }

    // Attaches the sign-in state the repository loads alongside the account.
    pub fn with_sign_in_state(
        mut self,
        organization_active: bool,
        failed_sign_in_count: i32,
        locked_until: Option<DateTime<Utc>>,
    ) -> Self {
        self.organization_active = organization_active;
        self.failed_sign_in_count = failed_sign_in_count;
        self.locked_until = locked_until;
        self
    }

    // The organization's two-step policy and the last successful sign-in, loaded
    // alongside the account.
    pub fn with_account_state(
        mut self,
        organization_requires_mfa: bool,
        last_sign_in_at: Option<DateTime<Utc>>,
    ) -> Self {
        self.organization_requires_mfa = organization_requires_mfa;
        self.last_sign_in_at = last_sign_in_at;
        self
    }

    // The console tour version the admin has seen, loaded alongside the account.
    pub fn with_console_tour_version(mut self, version: i32) -> Self {
        self.console_tour_version = version;
        self
    }

    // Whether this account may authenticate at all: active, in an active
    // organization. Checked at sign-in, refresh, and on every console request.
    pub fn can_sign_in(&self) -> bool {
        self.is_active && self.organization_active
    }

    // Whether sign-in is temporarily refused after too many failures.
    pub fn is_locked(&self, now: DateTime<Utc>) -> bool {
        self.locked_until.is_some_and(|until| until > now)
    }

    // ── Self-service mutations (the account page) ────────────────────────────────
    // Each stamps `updated_at` so the change is observable. Email uniqueness is
    // enforced by the repository (the `admin_users` unique index), not here.

    pub fn rename(&mut self, name: String) {
        self.name = name;
        self.touch();
    }

    pub fn change_email(&mut self, email: String) {
        self.email = normalize_email(&email);
        self.touch();
    }

    pub fn set_password(&mut self, password_hash: String, password_algorithm: String) {
        self.password_hash = password_hash;
        self.password_algorithm = password_algorithm;
        self.touch();
    }

    // Soft-deactivate. Sign-in, refresh and console sessions all reject an inactive
    // admin; the row is kept for audit history. Another admin of the organization
    // can reactivate it (`admin_organization::set_admin_active`).
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.touch();
    }

    fn touch(&mut self) {
        self.updated_at = Some(Utc::now());
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
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
    pub fn password_algorithm(&self) -> &str {
        &self.password_algorithm
    }
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
    pub fn organization_active(&self) -> bool {
        self.organization_active
    }
    pub fn failed_sign_in_count(&self) -> i32 {
        self.failed_sign_in_count
    }
    pub fn locked_until(&self) -> Option<DateTime<Utc>> {
        self.locked_until
    }
    pub fn organization_requires_mfa(&self) -> bool {
        self.organization_requires_mfa
    }
    pub fn last_sign_in_at(&self) -> Option<DateTime<Utc>> {
        self.last_sign_in_at
    }
    pub fn console_tour_version(&self) -> i32 {
        self.console_tour_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn admin() -> AdminUser {
        AdminUser::new(
            Uuid::now_v7(),
            "  Ops.Lead@Acme.COM ".to_string(),
            "Ops".to_string(),
            "hash".to_string(),
            "argon2id".to_string(),
        )
    }

    #[test]
    fn emails_are_stored_trimmed_and_lowercased() {
        let mut admin = admin();
        assert_eq!(admin.email(), "ops.lead@acme.com");
        admin.change_email("New@Acme.com".to_string());
        assert_eq!(admin.email(), "new@acme.com");
    }

    #[test]
    fn a_suspended_organization_or_a_lock_stops_sign_in() {
        let now = Utc::now();
        let admin = admin();
        assert!(admin.can_sign_in() && !admin.is_locked(now));

        let suspended = admin.clone().with_sign_in_state(false, 0, None);
        assert!(!suspended.can_sign_in());

        let locked = admin
            .clone()
            .with_sign_in_state(true, 0, Some(now + Duration::minutes(5)));
        assert!(locked.is_locked(now));
        assert!(!locked.is_locked(now + Duration::minutes(5)));
    }
}
