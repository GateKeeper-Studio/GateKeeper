#![forbid(unsafe_code)]

// CLI: look IP addresses up in the GeoIP databases the server uses, exactly as a
// sign-in would (same resolver, same private-address rules).
//
//   cargo run --bin geoip_lookup -- 8.8.8.8 2804:14c::1
//   cargo run --bin geoip_lookup -- --city .geoip/city.mmdb --asn .geoip/asn.mmdb 1.1.1.1
//
// Without --city / --asn it reads GEOIP_CITY_DB_PATH / GEOIP_ASN_DB_PATH (and .env).
// Prints one JSON object per address. `scripts/geoip-update.sh` downloads the files.

use web_api::{
    application::ports::geoip_resolver::GeoIpResolver,
    infraestructure::{config::GeoIpConfig, geoip::maxmind_geoip_resolver::MaxmindGeoIpResolver},
};

const USAGE: &str = "usage: geoip_lookup [--city <city.mmdb>] [--asn <asn.mmdb>] <ip>...";

fn main() {
    let mut city = None;
    let mut asn = None;
    let mut ips = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--city" => city = args.next(),
            "--asn" => asn = args.next(),
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            _ => ips.push(arg),
        }
    }
    if ips.is_empty() {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }

    let configured = GeoIpConfig::from_env();
    let Some(city) = city.or_else(|| configured.as_ref().map(|c| c.city_db_path.clone())) else {
        eprintln!(
            "no city database: pass --city or set GEOIP_CITY_DB_PATH (see scripts/geoip-update.sh)"
        );
        std::process::exit(2);
    };
    let asn = asn.or_else(|| configured.and_then(|c| c.asn_db_path));

    let Some(resolver) = MaxmindGeoIpResolver::open(&city, asn.as_deref()) else {
        eprintln!("could not open {city}");
        std::process::exit(1);
    };
    eprintln!(
        "database: {} ({city}){}",
        resolver.database_type(),
        asn.as_deref()
            .map(|a| format!(", ASN: {a}"))
            .unwrap_or_default()
    );

    for ip in ips {
        let line = match resolver.lookup(&ip) {
            Some(location) => serde_json::json!({
                "ip": ip,
                "country_code": location.country_code,
                "region": location.region,
                "city": location.city,
                "latitude": location.latitude,
                "longitude": location.longitude,
                "asn": location.asn,
                "isp": location.isp,
            }),
            None => serde_json::json!({
                "ip": ip,
                "located": false,
                "why": "not a public address, not parseable, or not in the database",
            }),
        };
        println!("{line}");
    }
}
