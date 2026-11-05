use std::sync::Arc;

use crate::{
    application::ports::password_hasher::PasswordHasher,
    infraestructure::crypto::argon2_password_hasher::Argon2PasswordHasher,
};

// The real Argon2id adapter, so use-case tests verify hashes made by
// `user_factory` (and vice versa) exactly as production does.
pub fn test_password_hasher() -> Arc<dyn PasswordHasher> {
    Arc::new(Argon2PasswordHasher::new(4))
}
