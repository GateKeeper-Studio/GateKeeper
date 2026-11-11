use chrono::Duration;

use crate::{
    application::ports::geoip_resolver::GeoIpResolver,
    domain::{
        entities::{oauth_client::OAuthClient, qr_login_request::QrLoginRequest},
        errors::DomainError,
        repositories::oauth_client_repository::OAuthClientRepository,
    },
    utils::oauth_crypto::{constant_time_eq, sha256_hex},
};

// How long a shown code can be scanned. Short on purpose: a code that lives long is
// a code that can be photographed and relayed. The browser simply asks for a new one.
pub fn code_lifetime() -> Duration {
    Duration::seconds(120)
}

// After a scan, how long the scanning device has to decide. Long enough to sign in on
// that device (a password and an emailed code), never past the browser's session.
pub fn decision_window() -> Duration {
    Duration::minutes(10)
}

// How long the browser's waiting session lives: a scan, its decision, and a minute
// for the poll that collects the approval. The decision deadline therefore always
// falls inside it.
pub fn session_lifetime() -> Duration {
    code_lifetime() + decision_window() + Duration::minutes(1)
}

// How often the browser should ask. Sent to it, so it is tuned in one place.
pub const POLL_INTERVAL_SECONDS: i64 = 2;

// One message for every refusal of a scan or a decision: a wrong token, an expired,
// already scanned or already decided code, a foreign tenant. The caller learns nothing
// about which check failed.
pub const INVALID_CODE: &str = "this QR code is invalid, expired or was already used";

pub fn invalid_code() -> DomainError {
    DomainError::AuthenticationError(INVALID_CODE.to_string())
}

// The request the QR token opens, compared in constant time.
pub fn matching(request: Option<QrLoginRequest>, token: &str) -> Option<QrLoginRequest> {
    request.filter(|r| constant_time_eq(&sha256_hex(token), r.token_hash()))
}

// The client the request signs in to, if it still offers QR sign-in. An admin who
// switches the feature off stops codes already shown.
pub async fn offering_client(
    clients: &dyn OAuthClientRepository,
    request: &QrLoginRequest,
) -> Result<Option<OAuthClient>, DomainError> {
    Ok(clients
        .find_by_id(request.tenant_id(), request.client_id())
        .await?
        .filter(|c| c.is_active() && c.allows_qr_login()))
}

// Whether `client_id` (the public id, as the hosted page sends it) names `client`.
pub fn is_client(client: &OAuthClient, client_id: &str) -> bool {
    uuid::Uuid::parse_str(client_id)
        .map(|id| id == client.client_id())
        .unwrap_or(false)
}

// "São Paulo, SP, BR" — what the approving device is told about where the browser
// is. None without a GeoIP database or for a private address.
pub fn approximate_location(geoip: &dyn GeoIpResolver, ip: Option<&str>) -> Option<String> {
    let geo = geoip.lookup(ip?)?;
    let parts: Vec<String> = [geo.city, geo.region, geo.country_code]
        .into_iter()
        .flatten()
        .filter(|p| !p.trim().is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}
