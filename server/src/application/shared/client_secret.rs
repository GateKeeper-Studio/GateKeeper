// OAuth client secrets at rest.
//
// A client secret is 256 random bits (`generate_opaque_token`), so a slow password
// hash adds no protection — there is nothing to brute-force — while it costs an
// Argon2id run (64 MiB) on every `/oauth/token` call. New secrets are stored as a
// keyed HMAC-SHA-256 instead, keyed with the tenant pepper so a leaked database row
// alone cannot be checked against a guess.
//
// Secrets hashed before this change are Argon2id PHC strings. They keep verifying
// through the `PasswordHasher` port until the client rotates its secret.
use crate::{
    application::ports::password_hasher::PasswordHasher,
    utils::oauth_crypto::{constant_time_eq, hmac_sha256_hex},
};

const HMAC_PREFIX: &str = "hmac-sha256$";

pub fn hash_client_secret(secret: &str, tenant_pepper: &str) -> String {
    format!("{HMAC_PREFIX}{}", hmac_sha256_hex(tenant_pepper, secret))
}

pub async fn verify_client_secret(
    stored_hash: &str,
    presented: &str,
    tenant_pepper: &str,
    hasher: &dyn PasswordHasher,
) -> bool {
    match stored_hash.strip_prefix(HMAC_PREFIX) {
        Some(expected) => constant_time_eq(expected, &hmac_sha256_hex(tenant_pepper, presented)),
        // Legacy Argon2id hash.
        None => hasher.verify(stored_hash, presented, tenant_pepper).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infraestructure::crypto::argon2_password_hasher::Argon2PasswordHasher;

    #[tokio::test]
    async fn hmac_hashes_verify_only_the_right_secret_and_pepper() {
        let hasher = Argon2PasswordHasher::new(1);
        let stored = hash_client_secret("s3cret", "pepper");
        assert!(stored.starts_with(HMAC_PREFIX));
        assert!(verify_client_secret(&stored, "s3cret", "pepper", &hasher).await);
        assert!(!verify_client_secret(&stored, "other", "pepper", &hasher).await);
        assert!(!verify_client_secret(&stored, "s3cret", "other-pepper", &hasher).await);
    }

    #[tokio::test]
    async fn legacy_argon2_hashes_still_verify() {
        let hasher = Argon2PasswordHasher::new(1);
        let legacy = hasher.hash("s3cret", "pepper").await.unwrap();
        assert!(verify_client_secret(&legacy, "s3cret", "pepper", &hasher).await);
        assert!(!verify_client_secret(&legacy, "nope", "pepper", &hasher).await);
    }
}
