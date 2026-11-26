use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserCredentials {
    user_id: Uuid,
    password_hash: String,
    password_hash_algorithm: String,
    should_change_password: bool,
    // Set on a temporary password an admin reset emailed to the user: it signs in
    // once, before this moment, and only to choose a new password. Signing in moves
    // it to that moment (`burn_temporary_password`). None for any other password.
    temporary_password_expires_at: Option<DateTime<Utc>>,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl UserCredentials {
    pub fn new(
        user_id: Uuid,
        password_hash: String,
        password_hash_algorithm: String,
        should_change_password: bool,
    ) -> Self {
        UserCredentials {
            user_id,
            password_hash,
            password_hash_algorithm,
            should_change_password,
            temporary_password_expires_at: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    // The single-use password of an admin reset: it forces a change and expires.
    pub fn temporary(
        user_id: Uuid,
        password_hash: String,
        password_hash_algorithm: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        UserCredentials {
            temporary_password_expires_at: Some(expires_at),
            updated_at: Some(Utc::now()),
            ..Self::new(user_id, password_hash, password_hash_algorithm, true)
        }
    }

    // Read back from storage.
    pub fn with_temporary_password_expires_at(mut self, expires_at: Option<DateTime<Utc>>) -> Self {
        self.temporary_password_expires_at = expires_at;
        self
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }
    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }
    pub fn password_hash_algorithm(&self) -> &str {
        &self.password_hash_algorithm
    }
    pub fn should_change_password(&self) -> bool {
        self.should_change_password
    }
    pub fn temporary_password_expires_at(&self) -> Option<DateTime<Utc>> {
        self.temporary_password_expires_at
    }

    // Whether a matching password may sign in at `now`. False for a temporary
    // password that expired or was already used: it must then fail exactly like a
    // wrong password, so nothing tells an attacker it was ever right.
    pub fn accepts_password_at(&self, now: DateTime<Utc>) -> bool {
        self.temporary_password_expires_at
            .is_none_or(|expires_at| now < expires_at)
    }

    // Whether this password may stand in for the user at a step-up (`current_password`
    // on `/me`, a QR approval, a trusted-device registration). Never while a change is
    // pending: that password is one the user did not choose, and an admin reset's may
    // already have been used.
    pub fn proves_account_owner(&self) -> bool {
        !self.should_change_password
    }

    // Spend a temporary password: it stops signing in from `now`. The database does
    // the same in one conditional statement (`UserRepository::burn_temporary_password`).
    pub fn burn_temporary_password(&mut self, now: DateTime<Utc>) {
        if let Some(expires_at) = self.temporary_password_expires_at {
            self.temporary_password_expires_at = Some(expires_at.min(now));
        }
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn a_temporary_password_signs_in_once_and_never_after_it_expires() {
        let now = Utc::now();
        let mut temporary = UserCredentials::temporary(
            Uuid::now_v7(),
            "hash".to_string(),
            "argon2".to_string(),
            now + Duration::hours(24),
        );
        assert!(temporary.should_change_password());
        assert!(temporary.accepts_password_at(now));
        assert!(!temporary.accepts_password_at(now + Duration::hours(24)));
        assert!(!temporary.proves_account_owner());

        temporary.burn_temporary_password(now);
        assert!(!temporary.accepts_password_at(now));
        // Burning again never moves the moment later.
        temporary.burn_temporary_password(now + Duration::hours(1));
        assert_eq!(temporary.temporary_password_expires_at(), Some(now));
    }

    #[test]
    fn other_passwords_never_expire() {
        let mut chosen =
            UserCredentials::new(Uuid::now_v7(), "hash".into(), "argon2".into(), false);
        chosen.burn_temporary_password(Utc::now());
        assert!(chosen.accepts_password_at(Utc::now() + Duration::days(3650)));
        assert!(chosen.proves_account_owner());
        assert_eq!(chosen.temporary_password_expires_at(), None);
    }
}
