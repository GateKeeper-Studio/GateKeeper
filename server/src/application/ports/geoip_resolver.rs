// Application-owned seam for turning an IP address into an approximate location +
// network (ASN/ISP). Implemented in infrastructure by a MaxMind GeoLite2 adapter;
// a no-op adapter is wired when no database is configured so the login flow keeps
// working and geo-based rules simply never fire (spec §20.2 graceful degradation).

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GeoLocation {
    pub country_code: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub asn: Option<i32>,
    pub isp: Option<String>,
}

// The credit a geolocation database's licence asks for wherever its data is shown
// (DB-IP Lite is CC BY 4.0; GeoLite2's licence asks for one too).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeoIpAttribution {
    pub name: &'static str,
    pub url: &'static str,
}

pub trait GeoIpResolver: Send + Sync {
    // Synchronous local-file lookup (fast, no network — never call a slow external
    // API during login). Returns None for private/loopback IPs, unparseable input,
    // or when no GeoIP database is loaded.
    fn lookup(&self, ip: &str) -> Option<GeoLocation>;

    // The credit to show next to located data; None when the loaded database needs
    // none (or none is loaded).
    fn attribution(&self) -> Option<GeoIpAttribution> {
        None
    }
}
