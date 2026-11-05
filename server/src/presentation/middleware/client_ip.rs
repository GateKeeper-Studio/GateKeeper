// Client-IP resolution that cannot be steered by the client.
//
// `X-Forwarded-For` is a list the CLIENT starts and every proxy appends to, so its
// left-most entry is whatever the caller chose to send. Using it as "the client IP"
// let any caller pick a fresh rate-limit bucket per request, dodge IP blocks and geo
// rules, or frame a victim's IP so the risk engine blocks them.
//
// The only trustworthy hops are the ones written by proxies we operate. So:
//   1. If the TCP peer is not a trusted proxy, it IS the client — headers ignored.
//   2. Otherwise walk `X-Forwarded-For` from the RIGHT (nearest hop first), skipping
//      trusted proxies; the first untrusted address is the client.
//   3. With no `X-Forwarded-For`, a trusted peer may supply `X-Real-IP`.
//
// With no `TRUSTED_PROXIES` configured, nothing is trusted and the peer address is
// always used — the safe default for a directly exposed instance.

use std::net::IpAddr;
use std::sync::Arc;

use axum::http::{HeaderMap, Request};
use tower_governor::{GovernorError, key_extractor::KeyExtractor};

use super::auth::RequestContext;

// One CIDR block (`10.0.0.0/8`, `fd00::/8`) or a single address (`192.0.2.1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpNetwork {
    network: IpAddr,
    prefix: u8,
}

impl IpNetwork {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        let (addr, prefix) = match raw.split_once('/') {
            Some((addr, prefix)) => (addr, Some(prefix)),
            None => (raw, None),
        };
        let network: IpAddr = addr
            .parse()
            .map_err(|_| format!("`{raw}` is not an IP address or CIDR block"))?;
        let max = if network.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            Some(p) => p
                .parse::<u8>()
                .ok()
                .filter(|p| *p <= max)
                .ok_or_else(|| format!("`{raw}` has an invalid prefix length"))?,
            None => max,
        };
        Ok(Self { network, prefix })
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, normalize(ip)) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(net) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(net) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }
}

// An IPv4 peer reached over a dual-stack socket arrives as `::ffff:a.b.c.d`; compare
// it as the IPv4 address it is.
fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        v4 => v4,
    }
}

// The set of reverse proxies whose forwarding headers are believed.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    networks: Vec<IpNetwork>,
}

impl TrustedProxies {
    pub fn new(networks: Vec<IpNetwork>) -> Self {
        Self { networks }
    }

    // Parses a list of CIDR blocks / addresses (the `TRUSTED_PROXIES` entries).
    pub fn parse<S: AsRef<str>>(entries: &[S]) -> Result<Self, String> {
        entries
            .iter()
            .map(|e| IpNetwork::parse(e.as_ref()))
            .collect::<Result<Vec<_>, _>>()
            .map(Self::new)
    }

    pub fn is_empty(&self) -> bool {
        self.networks.is_empty()
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        self.networks.iter().any(|n| n.contains(ip))
    }

    // The client address for a request that arrived from `peer`.
    pub fn client_ip(&self, headers: &HeaderMap, peer: Option<IpAddr>) -> Option<IpAddr> {
        let peer = peer.map(normalize);
        match peer {
            // Not from a trusted proxy (or nothing is trusted): the peer is the client.
            Some(ip) if !self.contains(ip) => return Some(ip),
            // No socket address at all (only in tests / unusual transports): there is
            // nothing to anchor trust on, so headers are not believed either.
            None => return None,
            Some(_) => {}
        }

        let forwarded: Vec<IpAddr> = headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .filter_map(|hop| hop.trim().parse::<IpAddr>().ok())
            .map(normalize)
            .collect();

        if !forwarded.is_empty() {
            // Nearest hop first. The first address no trusted proxy accounts for is the
            // client; if every hop is a trusted proxy, the furthest one is the best we
            // have.
            return forwarded
                .iter()
                .rev()
                .find(|ip| !self.contains(**ip))
                .or_else(|| forwarded.first())
                .copied();
        }

        headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
            .map(normalize)
            .or(peer)
    }
}

// Rate-limit key: the client IP resolved by `capture_client_context` (which runs
// outside every governor layer), so the limiter, the audit log and the risk engine
// all see the same, non-spoofable address.
#[derive(Debug, Clone, Default)]
pub struct ClientIpKeyExtractor;

impl KeyExtractor for ClientIpKeyExtractor {
    type Key = IpAddr;

    fn name(&self) -> &'static str {
        "client IP"
    }

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        req.extensions()
            .get::<RequestContext>()
            .and_then(|ctx| ctx.ip_address.as_deref())
            .and_then(|ip| ip.parse().ok())
            .ok_or(GovernorError::UnableToExtractKey)
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(key.to_string())
    }
}

// Shared handle so the middleware state is cheap to clone.
pub type SharedTrustedProxies = Arc<TrustedProxies>;

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn headers(xff: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", xff.parse().unwrap());
        h
    }

    fn proxies(list: &[&str]) -> TrustedProxies {
        TrustedProxies::parse(list).unwrap()
    }

    #[test]
    fn parses_cidrs_and_single_addresses() {
        let net = IpNetwork::parse("10.0.0.0/8").unwrap();
        assert!(net.contains(ip("10.200.1.1")));
        assert!(!net.contains(ip("11.0.0.1")));

        let single = IpNetwork::parse("192.0.2.1").unwrap();
        assert!(single.contains(ip("192.0.2.1")));
        assert!(!single.contains(ip("192.0.2.2")));

        let v6 = IpNetwork::parse("fd00::/8").unwrap();
        assert!(v6.contains(ip("fd12::1")));
        assert!(!v6.contains(ip("fe80::1")));

        assert!(
            IpNetwork::parse("0.0.0.0/0")
                .unwrap()
                .contains(ip("8.8.8.8"))
        );
    }

    #[test]
    fn rejects_malformed_entries() {
        assert!(IpNetwork::parse("not-an-ip").is_err());
        assert!(IpNetwork::parse("10.0.0.0/33").is_err());
        assert!(IpNetwork::parse("10.0.0.0/x").is_err());
    }

    #[test]
    fn untrusted_peer_ignores_forwarding_headers() {
        // Nothing trusted: a client sending X-Forwarded-For cannot pick its own IP.
        let none = TrustedProxies::default();
        let got = none.client_ip(&headers("1.2.3.4"), Some(ip("203.0.113.7")));
        assert_eq!(got, Some(ip("203.0.113.7")));

        // Trusted proxies configured, but this peer is not one of them.
        let p = proxies(&["10.0.0.0/8"]);
        let got = p.client_ip(&headers("1.2.3.4"), Some(ip("203.0.113.7")));
        assert_eq!(got, Some(ip("203.0.113.7")));
    }

    #[test]
    fn trusted_peer_uses_the_right_most_untrusted_hop() {
        let p = proxies(&["10.0.0.0/8"]);
        // The client spoofed "6.6.6.6"; the edge proxy appended the real peer 198.51.100.9,
        // an internal hop appended 10.0.0.2. The answer is 198.51.100.9, not 6.6.6.6.
        let got = p.client_ip(
            &headers("6.6.6.6, 198.51.100.9, 10.0.0.2"),
            Some(ip("10.0.0.3")),
        );
        assert_eq!(got, Some(ip("198.51.100.9")));
    }

    #[test]
    fn all_trusted_hops_fall_back_to_the_furthest() {
        let p = proxies(&["10.0.0.0/8"]);
        let got = p.client_ip(&headers("10.0.0.9, 10.0.0.2"), Some(ip("10.0.0.3")));
        assert_eq!(got, Some(ip("10.0.0.9")));
    }

    #[test]
    fn trusted_peer_may_supply_x_real_ip() {
        let p = proxies(&["10.0.0.0/8"]);
        let mut h = HeaderMap::new();
        h.insert("x-real-ip", "192.0.2.5".parse().unwrap());
        assert_eq!(p.client_ip(&h, Some(ip("10.0.0.3"))), Some(ip("192.0.2.5")));
        // ...and without any header the proxy itself is all we know.
        assert_eq!(
            p.client_ip(&HeaderMap::new(), Some(ip("10.0.0.3"))),
            Some(ip("10.0.0.3"))
        );
    }

    #[test]
    fn ipv4_mapped_peers_are_matched_as_ipv4() {
        let p = proxies(&["10.0.0.0/8"]);
        let got = p.client_ip(&headers("198.51.100.9"), Some(ip("::ffff:10.0.0.3")));
        assert_eq!(got, Some(ip("198.51.100.9")));
    }

    #[test]
    fn garbage_hops_are_skipped() {
        let p = proxies(&["10.0.0.0/8"]);
        let got = p.client_ip(&headers("198.51.100.9, garbage"), Some(ip("10.0.0.3")));
        assert_eq!(got, Some(ip("198.51.100.9")));
    }

    #[test]
    fn no_peer_means_no_client_ip() {
        let p = proxies(&["10.0.0.0/8"]);
        assert_eq!(p.client_ip(&headers("198.51.100.9"), None), None);
    }
}
