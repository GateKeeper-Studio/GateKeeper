use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::{SaltString, errors::Error as PasswordHashError},
};
use rand_core::OsRng;

pub fn hash_password(
    password: &str,
    password_hash_secret: &str,
) -> Result<String, PasswordHashError> {
    let params = Params::new(
        64 * 1024, // memory cost (64 MiB)
        3,         // time cost (3 iterations)
        4,         // parallelism (4 threads)
        Some(32),  // output length (32 bytes)
    )?;

    let argon2 = Argon2::new_with_secret(
        password_hash_secret.as_bytes(),
        Algorithm::Argon2id,
        Version::V0x13,
        params,
    )
    .map_err(|_| PasswordHashError::Algorithm)?;

    let salt = SaltString::generate(&mut OsRng);

    let hash = argon2.hash_password(password.as_bytes(), &salt)?;

    Ok(hash.to_string())
}

pub fn compare_password(encoded_hash: &str, password: &str, password_hash_secret: &str) -> bool {
    let parsed_hash = match PasswordHash::new(encoded_hash) {
        Ok(hash) => hash,
        Err(_) => return false,
    };

    // The m/t/p cost parameters are read from the encoded PHC hash during
    // verification, so the `Params` passed here are irrelevant — only the secret
    // (pepper) and algorithm need to match what was used to hash.
    let argon2 = match Argon2::new_with_secret(
        password_hash_secret.as_bytes(),
        Algorithm::Argon2id,
        Version::V0x13,
        Params::default(),
    ) {
        Ok(a) => a,
        Err(_) => return false,
    };

    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}
