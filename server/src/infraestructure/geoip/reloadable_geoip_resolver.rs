use std::sync::{Arc, PoisonError, RwLock};

use crate::application::ports::geoip_resolver::{GeoIpAttribution, GeoIpResolver, GeoLocation};

// The resolver every use case holds. It forwards to the database currently loaded and
// lets the GeoIP updater swap in a newer one while the server runs, so a refresh never
// needs a restart.
//
// A lookup clones the inner `Arc` and releases the lock before searching, so a swap
// waits for no lookup, and the old database is freed when the last lookup using it ends.
pub struct ReloadableGeoIpResolver {
    current: RwLock<Arc<dyn GeoIpResolver>>,
}

impl ReloadableGeoIpResolver {
    pub fn new(initial: Arc<dyn GeoIpResolver>) -> Self {
        Self {
            current: RwLock::new(initial),
        }
    }

    pub fn replace(&self, next: Arc<dyn GeoIpResolver>) {
        *self.current.write().unwrap_or_else(PoisonError::into_inner) = next;
    }

    fn snapshot(&self) -> Arc<dyn GeoIpResolver> {
        Arc::clone(&self.current.read().unwrap_or_else(PoisonError::into_inner))
    }
}

impl GeoIpResolver for ReloadableGeoIpResolver {
    fn lookup(&self, ip: &str) -> Option<GeoLocation> {
        self.snapshot().lookup(ip)
    }

    fn attribution(&self) -> Option<GeoIpAttribution> {
        self.snapshot().attribution()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infraestructure::geoip::maxmind_geoip_resolver::NoopGeoIpResolver;

    struct Fixed(&'static str);

    impl GeoIpResolver for Fixed {
        fn lookup(&self, _ip: &str) -> Option<GeoLocation> {
            Some(GeoLocation {
                city: Some(self.0.to_string()),
                ..GeoLocation::default()
            })
        }

        fn attribution(&self) -> Option<GeoIpAttribution> {
            Some(GeoIpAttribution {
                name: self.0,
                url: "https://example.test",
            })
        }
    }

    #[test]
    fn a_swap_takes_effect_on_the_next_lookup() {
        let resolver = ReloadableGeoIpResolver::new(Arc::new(NoopGeoIpResolver));
        assert_eq!(resolver.lookup("8.8.8.8"), None);
        assert_eq!(resolver.attribution(), None);

        resolver.replace(Arc::new(Fixed("Lisbon")));
        assert_eq!(
            resolver.lookup("8.8.8.8").and_then(|l| l.city).as_deref(),
            Some("Lisbon")
        );
        assert_eq!(resolver.attribution().map(|a| a.name), Some("Lisbon"));

        resolver.replace(Arc::new(Fixed("Porto")));
        assert_eq!(
            resolver.lookup("8.8.8.8").and_then(|l| l.city).as_deref(),
            Some("Porto")
        );
    }
}
