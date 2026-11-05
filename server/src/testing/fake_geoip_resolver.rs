use std::collections::HashMap;

use crate::application::ports::geoip_resolver::{GeoIpResolver, GeoLocation};

// GeoIP fake for Risk Engine tests: returns a per-IP location seeded up front.
// Unseeded IPs resolve to None (like a private IP or missing DB).
#[derive(Default)]
pub struct FakeGeoIpResolver {
    locations: HashMap<String, GeoLocation>,
}

impl FakeGeoIpResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, ip: &str, location: GeoLocation) -> Self {
        self.locations.insert(ip.to_string(), location);
        self
    }

    // Convenience: seed an IP with a country + coordinates.
    pub fn with_point(mut self, ip: &str, country: &str, lat: f64, lon: f64) -> Self {
        self.locations.insert(
            ip.to_string(),
            GeoLocation {
                country_code: Some(country.to_string()),
                region: None,
                city: None,
                latitude: Some(lat),
                longitude: Some(lon),
                asn: None,
                isp: None,
            },
        );
        self
    }
}

impl GeoIpResolver for FakeGeoIpResolver {
    fn lookup(&self, ip: &str) -> Option<GeoLocation> {
        self.locations.get(ip).cloned()
    }
}
