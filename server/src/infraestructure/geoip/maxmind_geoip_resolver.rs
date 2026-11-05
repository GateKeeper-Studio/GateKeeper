use std::net::IpAddr;

use maxminddb::{Reader, geoip2};

use crate::application::ports::geoip_resolver::{GeoIpResolver, GeoLocation};

// MaxMind GeoLite2/GeoIP2 adapter. Loads the City database (country/region/city/
// lat/lon) and, optionally, the ASN database (ASN/ISP). Both are memory-loaded local
// files — lookups are fast and never touch the network (spec §12.2).
//
// If the City DB fails to load, `open` returns None and the composition root falls
// back to `NoopGeoIpResolver`, so the login flow keeps working with geo rules off.
pub struct MaxmindGeoIpResolver {
    city: Reader<Vec<u8>>,
    asn: Option<Reader<Vec<u8>>>,
}

impl MaxmindGeoIpResolver {
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

        if let Ok(result) = self.city.lookup(addr) {
            if let Ok(Some(city)) = result.decode::<geoip2::City>() {
                location.country_code = city.country.iso_code.map(str::to_string);
                location.region = city
                    .subdivisions
                    .first()
                    .and_then(|s| s.iso_code)
                    .map(str::to_string);
                location.city = city.city.names.english.map(str::to_string);
                location.latitude = city.location.latitude;
                location.longitude = city.location.longitude;
            }
        }

        if let Some(asn_reader) = &self.asn {
            if let Ok(result) = asn_reader.lookup(addr) {
                if let Ok(Some(asn)) = result.decode::<geoip2::Asn>() {
                    location.asn = asn.autonomous_system_number.map(|n| n as i32);
                    location.isp = asn.autonomous_system_organization.map(str::to_string);
                }
            }
        }

        // Nothing resolved ⇒ behave like "no data" so callers don't record empty geo.
        if location == GeoLocation::default() {
            None
        } else {
            Some(location)
        }
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
