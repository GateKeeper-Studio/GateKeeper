use fake::Fake;
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::{FirstName, LastName};
use uuid::Uuid;

use crate::{
    domain::entities::{user::User, user_credentials::UserCredentials, user_profile::UserProfile},
    utils::password_hasher::hash_password,
};

// Exposed so tests can verify passwords against generated users.
pub const TEST_PASSWORD: &str = "Test@Factory1234!";
pub const TEST_PASSWORD_SECRET: &str = "test_factory_secret";

pub fn generate_users(tenant_id: Uuid, quantity: usize) -> Vec<User> {
    // Hash once and reuse — argon2 with 64 MiB memory cost is intentionally
    // slow. Hashing once per batch keeps test suites from timing out when
    // generating large sets.
    let password_hash =
        hash_password(TEST_PASSWORD, TEST_PASSWORD_SECRET).expect("test password hashing failed");

    (0..quantity)
        .map(|_| {
            let first_name: String = FirstName().fake();
            let last_name: String = LastName().fake();
            let email: String = SafeEmail().fake();

            let mut user = User::new(tenant_id, email, None, None);

            let profile = UserProfile::new(
                user.id(),
                Some(format!("{} {}", first_name, last_name)),
                first_name,
                last_name,
            );

            let credentials = UserCredentials::new(
                user.id(),
                password_hash.clone(),
                "argon2id".to_string(),
                false,
            );

            user.set_profile(profile);
            user.set_credentials(credentials);
            user
        })
        .collect()
}

pub fn generate_user(tenant_id: Uuid) -> User {
    generate_users(tenant_id, 1).into_iter().next().unwrap()
}
