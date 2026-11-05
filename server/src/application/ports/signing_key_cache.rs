use uuid::Uuid;

// Invalidation hook for the in-process cache of published signing keys. The auth
// middleware calls it when a token fails to verify against the cached set (the
// usual cause is a key published since the entry was stored), then refetches.
pub trait SigningKeyCache: Send + Sync {
    fn invalidate(&self, tenant_id: Uuid);
}
