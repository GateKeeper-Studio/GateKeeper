use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::signing_key::SigningKey, errors::DomainError,
    repositories::signing_key_repository::SigningKeyRepository,
};

/// Caching decorator over a `SigningKeyRepository`.
///
/// WHY: `require_auth`/`require_admin` call `find_published` to pick the keys to
/// verify a bearer token against — so **every authenticated request** was doing a
/// database round trip before the handler ran any of its own queries. Against a
/// small connection pool that made authentication overhead scale 1:1 with request
/// volume regardless of what the endpoint actually did. Signing keys change on
/// tenant creation and key rotation, which is to say almost never.
///
/// Only `find_published` is cached. `find_active` picks the key that *signs* new
/// tokens: it runs on token issuance rather than on every request, and serving a
/// stale answer there would mint tokens with a retired key.
///
/// STALENESS: bounded two ways. A short TTL caps how long a rotation can go
/// unnoticed, and `invalidate` lets the caller drop an entry the moment a
/// verification fails against it — which covers the case that actually matters, a
/// freshly published key that the cache has not seen yet (see
/// `presentation::middleware::auth::authenticate`). The reverse case, a retired key
/// still being accepted, is bounded by the TTL alone; keep it short.
///
/// The cache is per-process. Replicas warm independently, which is fine: the
/// entries are derived from the database, never authoritative.
pub struct CachingSigningKeyRepository {
    inner: Arc<dyn SigningKeyRepository>,
    ttl: Duration,
    // RwLock over a plain map rather than a cache crate: reads dominate overwhelmingly,
    // the key space is bounded by tenant count, and entries are tiny. Writers only
    // appear on a miss or an invalidation.
    entries: RwLock<HashMap<Uuid, CacheEntry>>,
}

struct CacheEntry {
    keys: Vec<SigningKey>,
    stored_at: Instant,
}

impl CachingSigningKeyRepository {
    pub fn new(inner: Arc<dyn SigningKeyRepository>, ttl: Duration) -> Self {
        Self {
            inner,
            ttl,
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Drops a tenant's cached keys so the next read goes to the database.
    ///
    /// Called when a token fails to verify against the cached set: the usual cause
    /// is a key published since the entry was stored.
    pub fn invalidate(&self, tenant_id: Uuid) {
        // A poisoned lock means another thread panicked while holding it. The map is
        // just a cache, so recovering the guard and carrying on is strictly better
        // than propagating the panic into every future request.
        let mut entries = match self.entries.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        entries.remove(&tenant_id);
    }

    fn cached(&self, tenant_id: Uuid) -> Option<Vec<SigningKey>> {
        let entries = match self.entries.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let entry = entries.get(&tenant_id)?;
        (entry.stored_at.elapsed() < self.ttl).then(|| entry.keys.clone())
    }

    fn store(&self, tenant_id: Uuid, keys: &[SigningKey]) {
        let mut entries = match self.entries.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        entries.insert(
            tenant_id,
            CacheEntry {
                keys: keys.to_vec(),
                stored_at: Instant::now(),
            },
        );
    }
}

impl crate::application::ports::signing_key_cache::SigningKeyCache for CachingSigningKeyRepository {
    fn invalidate(&self, tenant_id: Uuid) {
        CachingSigningKeyRepository::invalidate(self, tenant_id);
    }
}

#[async_trait]
impl SigningKeyRepository for CachingSigningKeyRepository {
    // NOT cached — see the type-level note. This selects the signing key for newly
    // issued tokens, and it runs per token issuance, not per request.
    async fn find_active(&self, tenant_id: Uuid) -> Result<Option<SigningKey>, DomainError> {
        self.inner.find_active(tenant_id).await
    }

    async fn find_published(&self, tenant_id: Uuid) -> Result<Vec<SigningKey>, DomainError> {
        if let Some(keys) = self.cached(tenant_id) {
            return Ok(keys);
        }

        let keys = self.inner.find_published(tenant_id).await?;
        // An empty result is cached too. A tenant with no published keys cannot
        // authenticate anyone, and not caching it would leave the miss path open as a
        // way to force a database read on every request with a bogus `tid`.
        self.store(tenant_id, &keys);
        Ok(keys)
    }

    async fn add(&self, key: &SigningKey) -> Result<(), DomainError> {
        self.inner.add(key).await?;
        self.invalidate(key.tenant_id());
        Ok(())
    }

    async fn update(&self, key: &SigningKey) -> Result<(), DomainError> {
        self.inner.update(key).await?;
        self.invalidate(key.tenant_id());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::domain::entities::signing_key::{SigningAlgorithm, SigningKey};

    // Counts calls so the tests can assert what actually reached the database.
    struct CountingRepository {
        published_calls: AtomicUsize,
        active_calls: AtomicUsize,
        keys: RwLock<Vec<SigningKey>>,
    }

    impl CountingRepository {
        fn new(keys: Vec<SigningKey>) -> Self {
            Self {
                published_calls: AtomicUsize::new(0),
                active_calls: AtomicUsize::new(0),
                keys: RwLock::new(keys),
            }
        }
    }

    #[async_trait]
    impl SigningKeyRepository for CountingRepository {
        async fn find_active(&self, _tenant_id: Uuid) -> Result<Option<SigningKey>, DomainError> {
            self.active_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.keys.read().unwrap().first().cloned())
        }

        async fn find_published(&self, _tenant_id: Uuid) -> Result<Vec<SigningKey>, DomainError> {
            self.published_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.keys.read().unwrap().clone())
        }

        async fn add(&self, key: &SigningKey) -> Result<(), DomainError> {
            self.keys.write().unwrap().push(key.clone());
            Ok(())
        }

        async fn update(&self, _key: &SigningKey) -> Result<(), DomainError> {
            Ok(())
        }
    }

    fn key(tenant_id: Uuid) -> SigningKey {
        SigningKey::new(
            tenant_id,
            "kid-1".to_string(),
            SigningAlgorithm::ES256,
            "{}".to_string(),
            "enc".to_string(),
        )
    }

    fn setup(ttl: Duration) -> (Arc<CountingRepository>, CachingSigningKeyRepository, Uuid) {
        let tenant_id = Uuid::now_v7();
        let inner = Arc::new(CountingRepository::new(vec![key(tenant_id)]));
        let cache = CachingSigningKeyRepository::new(inner.clone(), ttl);
        (inner, cache, tenant_id)
    }

    #[tokio::test]
    async fn repeated_reads_hit_the_database_once() {
        // The whole point: authentication must stop costing a query per request.
        let (inner, cache, tenant_id) = setup(Duration::from_secs(60));

        for _ in 0..25 {
            assert_eq!(cache.find_published(tenant_id).await.unwrap().len(), 1);
        }

        assert_eq!(inner.published_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn each_tenant_is_cached_separately() {
        let (inner, cache, tenant_id) = setup(Duration::from_secs(60));
        let other = Uuid::now_v7();

        cache.find_published(tenant_id).await.unwrap();
        cache.find_published(other).await.unwrap();
        cache.find_published(tenant_id).await.unwrap();

        assert_eq!(inner.published_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn an_expired_entry_is_refetched() {
        let (inner, cache, tenant_id) = setup(Duration::from_millis(20));

        cache.find_published(tenant_id).await.unwrap();
        tokio::time::sleep(Duration::from_millis(40)).await;
        cache.find_published(tenant_id).await.unwrap();

        assert_eq!(inner.published_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn invalidate_forces_the_next_read_to_the_database() {
        // This is what lets a freshly rotated key be picked up immediately rather
        // than after the TTL: the auth middleware invalidates on a failed verify.
        let (inner, cache, tenant_id) = setup(Duration::from_secs(60));

        cache.find_published(tenant_id).await.unwrap();
        cache.invalidate(tenant_id);
        cache.find_published(tenant_id).await.unwrap();

        assert_eq!(inner.published_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn writing_a_key_invalidates_that_tenant() {
        let (inner, cache, tenant_id) = setup(Duration::from_secs(60));

        cache.find_published(tenant_id).await.unwrap();
        cache.add(&key(tenant_id)).await.unwrap();

        // The newly added key must be visible, not the stale single-key set.
        assert_eq!(cache.find_published(tenant_id).await.unwrap().len(), 2);
        assert_eq!(inner.published_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn an_empty_result_is_cached_so_a_bogus_tenant_cannot_force_reads() {
        let inner = Arc::new(CountingRepository::new(vec![]));
        let cache = CachingSigningKeyRepository::new(inner.clone(), Duration::from_secs(60));
        let unknown = Uuid::now_v7();

        for _ in 0..10 {
            assert!(cache.find_published(unknown).await.unwrap().is_empty());
        }

        assert_eq!(inner.published_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn find_active_is_never_cached() {
        // Serving a stale active key would sign new tokens with a retired one.
        let (inner, cache, tenant_id) = setup(Duration::from_secs(60));

        for _ in 0..5 {
            cache.find_active(tenant_id).await.unwrap();
        }

        assert_eq!(inner.active_calls.load(Ordering::SeqCst), 5);
    }
}
