use std::net::IpAddr;

use maxminddb::{Reader, geoip2};

use crate::application::ports::geoip_resolver::{GeoIpAttribution, GeoIpResolver, GeoLocation};

// MMDB adapter. Loads the City database (country/region/city/lat/lon) and, optionally,
// the ASN database (ASN/ISP). Both are memory-loaded local files — lookups are fast
// and never touch the network (spec §12.2). Reads MaxMind GeoLite2/GeoIP2 and DB-IP
// (Lite or paid), which share the GeoIP2 layout; `scripts/geoip-update.sh` fetches
// them.
//
// If the City DB fails to load, `open` returns None and the composition root falls
// back to `NoopGeoIpResolver`, so the login flow keeps working with geo rules off.
pub struct MaxmindGeoIpResolver {
    city: Reader<Vec<u8>>,
    asn: Option<Reader<Vec<u8>>>,
}

impl MaxmindGeoIpResolver {
    // The City database's type, from its metadata: `GeoLite2-City`, `DBIP-City-Lite`, …
    pub fn database_type(&self) -> &str {
        &self.city.metadata().database_type
    }

    pub fn open(city_db_path: &str, asn_db_path: Option<&str>) -> Option<Self> {
        let city = match Reader::open_readfile(city_db_path) {
            Ok(reader) => reader,
            Err(e) => {
                tracing::warn!("GeoIP City database '{city_db_path}' failed to load: {e}");
                return None;
            }
        };

        let asn = asn_db_path.and_then(|path| match Reader::open_readfile(path) {
            Ok(reader) => Some(reader),
            Err(e) => {
                tracing::warn!("GeoIP ASN database '{path}' failed to load: {e}");
                None
            }
        });

        Some(Self { city, asn })
    }
}

// RFC1918 / loopback / link-local / unique-local addresses have no meaningful public
// geolocation — skip enrichment for them (spec §12.3).
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                // fc00::/7 unique-local
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // fe80::/10 link-local
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

impl GeoIpResolver for MaxmindGeoIpResolver {
    fn lookup(&self, ip: &str) -> Option<GeoLocation> {
        let addr: IpAddr = ip.parse().ok()?;
        if is_private_ip(&addr) {
            return None;
        }

        let mut location = GeoLocation::default();

        if let Ok(result) = self.city.lookup(addr)
            && let Ok(Some(city)) = result.decode::<geoip2::City>()
        {
            location.country_code = city.country.iso_code.map(str::to_string);
            // MaxMind gives the ISO 3166-2 code ("SP"); DB-IP only the name.
            location.region = city
                .subdivisions
                .first()
                .and_then(|s| s.iso_code.or(s.names.english))
                .map(str::to_string);
            location.city = city.city.names.english.map(str::to_string);
            location.latitude = city.location.latitude;
            location.longitude = city.location.longitude;
        }

        if let Some(asn_reader) = &self.asn
            && let Ok(result) = asn_reader.lookup(addr)
            && let Ok(Some(asn)) = result.decode::<geoip2::Asn>()
        {
            location.asn = asn.autonomous_system_number.map(|n| n as i32);
            location.isp = asn.autonomous_system_organization.map(str::to_string);
        }

        // Nothing resolved ⇒ behave like "no data" so callers don't record empty geo.
        if location == GeoLocation::default() {
            None
        } else {
            Some(location)
        }
    }

    fn attribution(&self) -> Option<GeoIpAttribution> {
        attribution_for(self.database_type())
    }
}

// The free databases carry a licence credit; the paid GeoIP2 / DB-IP ones do not.
fn attribution_for(database_type: &str) -> Option<GeoIpAttribution> {
    // `DBIP-City-Lite`, `DBIP-ASN-Lite (compat=GeoLite2-ASN)`.
    if database_type.starts_with("DBIP-") && database_type.contains("-Lite") {
        Some(GeoIpAttribution {
            name: "IP geolocation by DB-IP",
            url: "https://db-ip.com",
        })
    } else if database_type.starts_with("GeoLite2") {
        Some(GeoIpAttribution {
            name: "GeoLite2 data created by MaxMind",
            url: "https://www.maxmind.com",
        })
    } else {
        None
    }
}

// Used when no GeoIP database is configured: every lookup returns None, so geo-based
// rules never fire but the login flow is otherwise unaffected (spec §20.2).
pub struct NoopGeoIpResolver;

impl GeoIpResolver for NoopGeoIpResolver {
    fn lookup(&self, _ip: &str) -> Option<GeoLocation> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_free_databases_are_credited_and_the_paid_ones_are_not() {
        assert_eq!(
            attribution_for("DBIP-City-Lite").map(|a| a.url),
            Some("https://db-ip.com")
        );
        assert_eq!(
            attribution_for("GeoLite2-City").map(|a| a.url),
            Some("https://www.maxmind.com")
        );
        assert_eq!(
            attribution_for("DBIP-ASN-Lite (compat=GeoLite2-ASN)").map(|a| a.url),
            Some("https://db-ip.com")
        );
        assert_eq!(attribution_for("GeoIP2-City"), None);
        assert_eq!(
            attribution_for("DBIP-Location-ISP (compat=Enterprise)"),
            None
        );
    }

    #[test]
    fn private_and_reserved_addresses_are_never_looked_up() {
        for ip in [
            "10.0.0.1",
            "192.168.1.5",
            "127.0.0.1",
            "169.254.0.1",
            "::1",
            "fd00::1",
        ] {
            assert!(is_private_ip(&ip.parse().unwrap()), "{ip}");
        }
        assert!(!is_private_ip(&"8.8.8.8".parse().unwrap()));
    }

    // Runs against the configured database (GEOIP_DIR or GEOIP_CITY_DB_PATH, .env
    // included): cargo test --lib geoip -- --ignored
    #[test]
    #[ignore = "needs a downloaded GeoIP database (GEOIP_DIR or GEOIP_CITY_DB_PATH)"]
    fn a_real_database_places_a_public_address() {
        let config = crate::infraestructure::config::GeoIpConfig::from_env()
            .expect("GEOIP_DIR or GEOIP_CITY_DB_PATH");
        let asn = config.asn_db_path.clone();
        let resolver =
            MaxmindGeoIpResolver::open(&config.city_db_path, asn.as_deref()).expect("opens");

        let google = resolver.lookup("8.8.8.8").expect("8.8.8.8 is located");
        assert_eq!(google.country_code.as_deref(), Some("US"));
        assert!(google.latitude.is_some() && google.longitude.is_some());
        assert!(google.region.is_some(), "a region code or name");
        if asn.is_some() {
            assert_eq!(google.asn, Some(15169));
        }
        assert_eq!(resolver.lookup("192.168.0.10"), None);
    }
}
