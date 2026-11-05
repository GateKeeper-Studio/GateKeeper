-- The per-tenant password pepper is now sealed at rest with the server's secrets
-- key (AES-256-GCM, see infraestructure/crypto/aes_gcm_secret_cipher.rs). A pepper
-- stored in plain text next to the password hashes gives no protection against a
-- database dump, which is the one thing a pepper is for.
--
-- Existing rows keep their plaintext value with `password_hash_secret_sealed =
-- false`; the server re-seals them on startup (infraestructure/crypto/secret_reseal.rs).
-- The pepper itself must NOT change: every password hash in the tenant uses it.
ALTER TABLE tenants
    ADD COLUMN password_hash_secret_sealed BOOLEAN NOT NULL DEFAULT FALSE;

-- The sealed value is longer than the old 255-character limit allows for long
-- peppers (v1:<kid>:base64(nonce || ciphertext || tag)).
ALTER TABLE tenants
    ALTER COLUMN password_hash_secret TYPE TEXT;
