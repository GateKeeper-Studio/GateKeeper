use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::domain::{entities::domain_event_type::DomainEventType, errors::DomainError};

// SECURITY: this is the trust boundary for webhook targets, and it is a real
// control rather than an input-formatting nicety.
//
// A tenant admin supplies a URL that OUR server then makes an outbound request to,
// from inside our network. That is textbook SSRF. The highest-value target is the
// cloud metadata endpoint (169.254.169.254), which is reachable from inside most
// VPCs and will happily hand out IAM credentials; close behind are internal admin
// panels on RFC1918 addresses that trust anything originating from the VPC.
//
// Rules:
//   * https only — plain http is allowed for local hosts ONLY (loopback, a
//     private-range IP literal, or an internal-looking hostname such as
//     `host.docker.internal`), and only when the operator has explicitly set
//     WEBHOOK_ALLOW_PRIVATE_TARGETS (dev/self-hosted). A containerised server
//     cannot reach a consumer on the docker host through loopback, so loopback
//     alone would make local development impossible;
//   * no fragment (a browser would drop it; here it just hides intent);
//   * no userinfo — `https://internal-admin@evil.test/` is a classic parser-confusion
//     trick, and credentials do not belong in a stored URL either way;
//   * IP-literal hosts in private / loopback / link-local / unique-local ranges are
//     refused outright.
//
// DOCUMENTED LIMITATION: a hostname that passes this check can still RESOLVE to a
// private address later — DNS rebinding, or simply an internal DNS name that looks
// public. Closing that fully requires resolving the name and pinning the socket to
// the vetted IP at request time, which reqwest does not expose cleanly. The
// redirect-disabled client (see ReqwestWebhookSender) closes the easier variant
// where a public URL 302s to a private one. Treat this as defence in depth against
// a careless or semi-trusted tenant admin, not as an airtight boundary against a
// determined attacker who already controls DNS.
pub fn validate_webhook_url(url: &str, allow_private_targets: bool) -> Result<(), DomainError> {
    validate_outbound_url("webhook url", url, allow_private_targets)
}

// The same SSRF rules for any admin-supplied URL our server will POST to. Besides
// webhooks this guards an OAuth client's `backchannel_logout_uri`, which the
// dispatcher calls in exactly the same way. `label` only names the field in errors.
pub fn validate_outbound_url(
    label: &str,
    url: &str,
    allow_private_targets: bool,
) -> Result<(), DomainError> {
    let invalid =
        |reason: &str| DomainError::InvalidRequest(format!("{label} is not allowed: {reason}"));

    let url = url.trim();
    if url.is_empty() {
        return Err(invalid("must not be empty"));
    }
    if url.contains('#') {
        return Err(invalid("must not contain a fragment"));
    }

    let lower = url.to_ascii_lowercase();
    let Some((scheme, rest)) = lower.split_once("://") else {
        return Err(invalid("must be an absolute http(s) URL"));
    };

    // Everything up to the first '/', '?' is the authority.
    let authority = rest.split(['/', '?']).next().unwrap_or("");
    if authority.is_empty() {
        return Err(invalid("must have a host"));
    }
    // Reject userinfo rather than trying to parse around it.
    if authority.contains('@') {
        return Err(invalid("must not contain credentials"));
    }

    let host = strip_port(authority);
    if host.is_empty() {
        return Err(invalid("must have a host"));
    }

    match scheme {
        "https" => {}
        "http" => {
            // Plain http leaks the payload (which carries PII) and lets anyone on
            // the path forge deliveries, so it is only ever a local-development
            // affordance — and only when the operator opted in.
            if !allow_private_targets {
                return Err(invalid(
                    "must use https (set WEBHOOK_ALLOW_PRIVATE_TARGETS=true to allow http on local hosts)",
                ));
            }
            if !is_local_host(host) {
                return Err(invalid(
                    "plain http is only allowed for loopback, private addresses or internal hostnames",
                ));
            }
        }
        _ => return Err(invalid("must be an absolute http(s) URL")),
    }

    if let Some(ip) = parse_ip_host(host)
        && is_blocked_ip(&ip)
        && !allow_private_targets
    {
        return Err(invalid(
            "must not target a private, loopback or link-local address",
        ));
    }

    Ok(())
}

// Splits a trailing ":port", handling the bracketed IPv6 form.
fn strip_port(authority: &str) -> &str {
    if let Some(end) = authority.strip_prefix('[').and_then(|r| r.find(']')) {
        // "[::1]:8080" -> "::1"
        return &authority[1..=end];
    }
    match authority.rsplit_once(':') {
        // Only treat it as a port if what follows is actually numeric — an
        // unbracketed IPv6 literal has many colons and no port.
        Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => head,
        _ => authority,
    }
}

fn parse_ip_host(host: &str) -> Option<IpAddr> {
    host.parse::<IpAddr>().ok()
}

// Hosts that plain http may target once private targets are allowed: anything
// that cannot be on the public internet, so the payload never crosses it in
// cleartext. Hostnames qualify only when they are single-label (a compose
// service name) or use a suffix that is never publicly delegated.
fn is_local_host(host: &str) -> bool {
    if let Some(ip) = parse_ip_host(host) {
        return is_blocked_ip(&ip);
    }
    let host = host.trim_end_matches('.');
    !host.contains('.')
        || [".localhost", ".internal", ".local"]
            .iter()
            .any(|suffix| host.ends_with(suffix))
}

// The ranges an outbound request must never reach.
fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_v4(v4),
        IpAddr::V6(v6) => is_blocked_v6(v6),
    }
}

fn is_blocked_v4(ip: &Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_loopback()            // 127/8
        || ip.is_private()      // 10/8, 172.16/12, 192.168/16
        || ip.is_link_local()   // 169.254/16 — the cloud metadata endpoint
        || ip.is_broadcast()
        || ip.is_unspecified()  // 0.0.0.0
        || a == 100 && (64..128).contains(&b) // 100.64/10 carrier-grade NAT
        || a >= 224 // multicast + reserved
}

fn is_blocked_v6(ip: &Ipv6Addr) -> bool {
    let segments = ip.segments();
    ip.is_loopback()
        || ip.is_unspecified()
        || (segments[0] & 0xfe00) == 0xfc00   // fc00::/7 unique local
        || (segments[0] & 0xffc0) == 0xfe80   // fe80::/10 link local
        || (segments[0] & 0xff00) == 0xff00   // ff00::/8 multicast
        // IPv4-mapped (::ffff:169.254.169.254) would otherwise sail past every
        // check above and reach the metadata endpoint anyway.
        || ip.to_ipv4_mapped().is_some_and(|v4| is_blocked_v4(&v4))
}

// Every entry must be a known event type. An admin who typos "user.create" should
// get a 400, not a subscription that silently never fires.
pub fn validate_event_types(event_types: Vec<String>) -> Result<Vec<String>, DomainError> {
    let mut normalized: Vec<String> = Vec::new();
    for raw in event_types {
        let value = raw.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if DomainEventType::parse_subscribable(&value).is_none() {
            return Err(DomainError::InvalidRequest(format!(
                "unknown event type: '{value}'"
            )));
        }
        if !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(url: &str) {
        assert!(
            validate_webhook_url(url, false).is_ok(),
            "expected {url} to be allowed"
        );
    }

    fn blocked(url: &str) {
        assert!(
            matches!(
                validate_webhook_url(url, false),
                Err(DomainError::InvalidRequest(_))
            ),
            "expected {url} to be BLOCKED"
        );
    }

    #[test]
    fn accepts_ordinary_https_endpoints() {
        allowed("https://consumer.example.com/hooks/users");
        allowed("https://consumer.example.com:8443/hooks");
        allowed("https://consumer.example.com/hooks?tenant=1");
    }

    #[test]
    fn rejects_non_http_schemes() {
        blocked("ftp://consumer.example.com/hook");
        blocked("file:///etc/passwd");
        blocked("javascript:alert(1)");
        blocked("consumer.example.com/hook"); // not absolute
    }

    #[test]
    fn rejects_plain_http_by_default() {
        blocked("http://consumer.example.com/hook");
        blocked("http://127.0.0.1/hook");
    }

    #[test]
    fn rejects_fragments_and_credentials() {
        blocked("https://consumer.example.com/hook#frag");
        blocked("https://user:pass@consumer.example.com/hook");
        blocked("https://internal-admin@evil.test/hook");
    }

    // The headline case: the cloud metadata endpoint.
    #[test]
    fn rejects_the_cloud_metadata_endpoint() {
        blocked("https://169.254.169.254/latest/meta-data/");
        blocked("http://169.254.169.254/latest/meta-data/");
        blocked("https://[fe80::1]/hook");
        // IPv4-mapped IPv6 must not be a bypass.
        blocked("https://[::ffff:169.254.169.254]/hook");
    }

    #[test]
    fn rejects_every_private_range() {
        for url in [
            "https://10.0.0.5/hook",
            "https://10.255.255.255/hook",
            "https://172.16.0.1/hook",
            "https://172.31.255.254/hook",
            "https://192.168.1.1/hook",
            "https://127.0.0.1/hook",
            "https://0.0.0.0/hook",
            "https://100.64.0.1/hook",
            "https://[::1]/hook",
            "https://[fc00::1]/hook",
            "https://[::ffff:10.0.0.1]/hook",
        ] {
            blocked(url);
        }
    }

    #[test]
    fn a_port_does_not_smuggle_a_blocked_host_past_the_check() {
        blocked("https://10.0.0.5:8080/hook");
        blocked("https://[::1]:9000/hook");
        blocked("https://169.254.169.254:80/hook");
    }

    // The escape hatch relaxes the private-range rule, and http only on local hosts.
    #[test]
    fn allow_private_targets_permits_loopback_for_local_development() {
        assert!(validate_webhook_url("http://127.0.0.1:9099/hook", true).is_ok());
        assert!(validate_webhook_url("http://localhost:9099/hook", true).is_ok());
        assert!(validate_webhook_url("https://10.0.0.5/hook", true).is_ok());
    }

    // A containerised server reaches a consumer on the docker host through the
    // bridge gateway or host.docker.internal, never through its own loopback.
    #[test]
    fn allow_private_targets_permits_plain_http_to_the_docker_host() {
        for url in [
            "http://host.docker.internal:8081/webhooks/gatekeeper",
            "http://172.18.0.1:8081/webhooks/gatekeeper",
            "http://192.168.1.10:8081/hook",
            "http://[fc00::1]:8081/hook",
            "http://consumer-api:8081/hook",
            "http://consumer.localhost:8081/hook",
            "http://printer.local/hook",
        ] {
            assert!(
                validate_webhook_url(url, true).is_ok(),
                "expected {url} to be allowed"
            );
        }
    }

    #[test]
    fn plain_http_to_the_docker_host_is_refused_without_the_escape_hatch() {
        blocked("http://host.docker.internal:8081/webhooks/gatekeeper");
        blocked("http://172.18.0.1:8081/webhooks/gatekeeper");
        blocked("http://consumer-api:8081/hook");
    }

    // Even with the escape hatch on, plain http to a public host stays refused:
    // that is payload PII in cleartext across a real network.
    #[test]
    fn allow_private_targets_still_refuses_plain_http_to_public_hosts() {
        for url in [
            "http://consumer.example.com/hook",
            "http://8.8.8.8/hook",
            "http://internal.example.com/hook",
            "http://evil.test/hook",
        ] {
            assert!(
                matches!(
                    validate_webhook_url(url, true),
                    Err(DomainError::InvalidRequest(_))
                ),
                "expected {url} to be BLOCKED"
            );
        }
    }

    // Back-channel logout notices ride the outbox but are not webhook events.
    #[test]
    fn the_internal_logout_event_cannot_be_subscribed_to() {
        assert!(matches!(
            validate_event_types(vec!["oidc.backchannel_logout".to_string()]),
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[test]
    fn event_types_must_all_be_known() {
        assert!(matches!(
            validate_event_types(vec!["user.create".to_string()]),
            Err(DomainError::InvalidRequest(_))
        ));
        assert!(matches!(
            validate_event_types(vec!["user.login.success".to_string()]),
            Err(DomainError::InvalidRequest(_))
        ));
    }

    #[test]
    fn event_types_are_trimmed_deduped_and_blanks_dropped() {
        let result = validate_event_types(vec![
            " user.created ".to_string(),
            "user.created".to_string(),
            "".to_string(),
            "user.deleted".to_string(),
        ])
        .unwrap();
        assert_eq!(result, vec!["user.created", "user.deleted"]);
    }

    #[test]
    fn an_empty_event_type_list_is_valid_and_means_all() {
        assert_eq!(validate_event_types(vec![]).unwrap(), Vec::<String>::new());
    }
}
