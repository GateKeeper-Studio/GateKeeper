use hmac::{Hmac, Mac};
use sha1::Sha1;

// RFC 6238 TOTP with the parameters every authenticator app defaults to:
// HMAC-SHA1, 30-second timestep, 6 digits. Pure functions over (secret, time)
// so verification is deterministic and directly unit-testable.

pub const STEP_SECONDS: i64 = 30;
pub const DIGITS: u32 = 6;

// The otpauth:// URI encoded into the enrollment QR code. `label` is what the
// authenticator app displays (issuer:account).
pub fn otpauth_url(issuer: &str, account: &str, secret_base32: &str) -> String {
    // Percent-encode the few characters that commonly appear in names/emails and
    // would break the URI; issuer/account are display-only values.
    let enc = |s: &str| {
        s.replace('%', "%25")
            .replace(' ', "%20")
            .replace(':', "%3A")
            .replace('/', "%2F")
    };
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        enc(issuer),
        enc(account),
        secret_base32,
        enc(issuer),
        DIGITS,
        STEP_SECONDS,
    )
}

// The timestep counter for a unix timestamp.
pub fn step_for(unix_seconds: i64) -> i64 {
    unix_seconds / STEP_SECONDS
}

// RFC 4226 HOTP truncation over one timestep.
pub fn generate(secret: &[u8], step: i64) -> String {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&(step as u64).to_be_bytes());
    let digest = mac.finalize().into_bytes();

    let offset = (digest[19] & 0x0f) as usize;
    let code = ((u32::from(digest[offset]) & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);

    format!(
        "{:0width$}",
        code % 10u32.pow(DIGITS),
        width = DIGITS as usize
    )
}

// Accept the current step ± `skew` steps (clock drift between the phone and
// the server). Returns the matched step so callers can persist it and reject
// a replay of the same code within its window.
pub fn verify(secret: &[u8], code: &str, unix_seconds: i64, skew: i64) -> Option<i64> {
    if code.len() != DIGITS as usize || !code.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let current = step_for(unix_seconds);
    for step in (current - skew)..=(current + skew) {
        if step >= 0 && constant_time_eq(&generate(secret, step), code) {
            return Some(step);
        }
    }
    None
}

// 6-digit codes are tiny, but keep the comparison constant-time anyway: it is
// free and removes the timing side-channel class entirely.
fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 Appendix B test vectors (SHA-1, 8 digits truncated to last 6 of
    // the published values would be wrong — instead we recompute with 6 digits
    // against the reference secret and known times via the HOTP layer).
    const RFC_SECRET: &[u8] = b"12345678901234567890";

    #[test]
    fn generates_rfc6238_reference_codes() {
        // RFC 6238 8-digit reference values, modulo 10^6 to our 6 digits:
        // T=59 → 94287082 → 287082 ; T=1111111109 → 07081804 → 081804
        assert_eq!(generate(RFC_SECRET, step_for(59)), "287082");
        assert_eq!(generate(RFC_SECRET, step_for(1_111_111_109)), "081804");
        assert_eq!(generate(RFC_SECRET, step_for(1_234_567_890)), "005924");
    }

    #[test]
    fn verifies_current_step_and_returns_it() {
        let now = 1_111_111_109;
        let code = generate(RFC_SECRET, step_for(now));
        assert_eq!(verify(RFC_SECRET, &code, now, 1), Some(step_for(now)));
    }

    #[test]
    fn accepts_adjacent_steps_within_skew() {
        let now = 1_111_111_109;
        let previous = generate(RFC_SECRET, step_for(now) - 1);
        let next = generate(RFC_SECRET, step_for(now) + 1);
        assert_eq!(
            verify(RFC_SECRET, &previous, now, 1),
            Some(step_for(now) - 1)
        );
        assert_eq!(verify(RFC_SECRET, &next, now, 1), Some(step_for(now) + 1));
    }

    #[test]
    fn rejects_code_outside_skew() {
        let now = 1_111_111_109;
        let stale = generate(RFC_SECRET, step_for(now) - 2);
        assert_eq!(verify(RFC_SECRET, &stale, now, 1), None);
    }

    #[test]
    fn rejects_malformed_codes() {
        assert_eq!(verify(RFC_SECRET, "12345", 59, 1), None); // too short
        assert_eq!(verify(RFC_SECRET, "1234567", 59, 1), None); // too long
        assert_eq!(verify(RFC_SECRET, "12a456", 59, 1), None); // non-digit
    }

    #[test]
    fn otpauth_url_encodes_label_parts() {
        let url = otpauth_url("My App", "user@example.com", "JBSWY3DPEHPK3PXP");
        assert!(
            url.starts_with("otpauth://totp/My%20App:user@example.com?secret=JBSWY3DPEHPK3PXP")
        );
        assert!(url.contains("issuer=My%20App"));
        assert!(url.contains("digits=6"));
        assert!(url.contains("period=30"));
    }
}
