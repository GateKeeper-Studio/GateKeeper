-- Sealed peppers cannot be restored to plain text by SQL; run the server's
-- re-seal job in reverse (decrypt) before rolling back, or this loses them.
ALTER TABLE tenants DROP COLUMN password_hash_secret_sealed;
ALTER TABLE tenants ALTER COLUMN password_hash_secret TYPE VARCHAR(255);
