use axum::{
    Json,
    extract::{ConnectInfo, FromRequestParts, RawPathParams, Request, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::Engine;
use serde_json::json;
use std::net::{IpAddr, SocketAddr};
use tracing::Instrument;
use uuid::Uuid;

use std::sync::Arc;

use crate::{
    application::ports::{
        admin_token_issuer::AdminTokenIssuer, signing_key_cache::SigningKeyCache,
        token_verifier::TokenVerifier,
    },
    domain::{
        entities::{
            audit_action::ADMIN_ACCESS_DENIED,
            audit_event::{ActorType, AuditEvent, Outcome},
            scope::{ACCOUNT_SCOPE, ADMIN_SCOPE},
        },
        repositories::signing_key_repository::SigningKeyRepository,
    },
    presentation::routes::AppState,
};

use super::client_ip::{SharedTrustedProxies, TrustedProxies};

// Roles that authorize tenant-admin (dashboard) actions. Deny-by-default: an
// authenticated user without one of these is forbidden from the admin surface.
//
// Defined in the domain (and enforced as reserved at role creation) so tenants
// cannot mint a role with one of these names and grant themselves the admin
// surface. Note this establishes *role*, not tenant — see `TenantScope`.
pub use crate::domain::entities::role::ADMIN_ROLE_NAMES as ADMIN_ROLES;

// The authenticated caller, derived from a verified access token. Placed into the
// request extensions by the auth middleware and pulled out by handlers (either via
// the `AuthPrincipal` extractor or `req.extensions()`).
#[derive(Debug, Clone)]
pub struct AuthPrincipal {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub scopes: Vec<String>,
    // True for a GLOBAL platform-admin (signed in via /admin/login). Such a principal
    // has no tenant (`tenant_id` is nil) and is authorized across every tenant, so the
    // per-handler tenant-isolation pins must let it through. A tenant-scoped admin
    // (token carrying a `tid`) has this false and stays pinned to its own tenant.
    pub is_platform_admin: bool,
    // The single organization a platform-admin token is bound to (`org` claim). Every
    // organization action is scoped to it, so an admin can only ever manage its own
    // organization. `None` for end-user tokens and the synthetic `system()` principal.
    pub organization_id: Option<Uuid>,
}

impl AuthPrincipal {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    pub fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| ADMIN_ROLES.contains(&r.as_str()))
    }

    // The application-layer view of this caller, for commands that authorize
    // inside the use case.
    pub fn actor(&self) -> crate::application::shared::actor::Actor {
        use crate::application::shared::actor::Actor;
        match (self.is_platform_admin, self.organization_id) {
            (true, Some(organization_id)) => Actor::PlatformAdmin {
                admin_id: self.user_id,
                organization_id,
            },
            // A platform-admin token without an organization authorizes nothing.
            (true, None) => Actor::User {
                tenant_id: Uuid::nil(),
                user_id: self.user_id,
            },
            (false, _) if self.is_admin() => Actor::TenantAdmin {
                tenant_id: self.tenant_id,
                user_id: self.user_id,
            },
            (false, _) => Actor::User {
                tenant_id: self.tenant_id,
                user_id: self.user_id,
            },
        }
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    // Whether this principal may act on `tenant_id`, given whether that tenant
    // belongs to the principal's organization. A platform admin may act on every
    // tenant OF ITS OWN ORGANIZATION; everyone else is confined to the tenant their
    // token was issued for.
    pub fn can_access_tenant(&self, tenant_id: Uuid, tenant_in_admin_org: bool) -> bool {
        if self.is_platform_admin {
            self.organization_id.is_some() && tenant_in_admin_org
        } else {
            self.tenant_id == tenant_id
        }
    }

    // The organization this admin is bound to, or a 403 when the token carries none
    // (e.g. an end-user token). Used by the
    // `/admin/me/organization*` handlers so an admin only ever touches its own org.
    pub fn require_org(&self) -> Result<Uuid, crate::presentation::errors::ApiError> {
        self.organization_id.ok_or_else(|| {
            crate::presentation::errors::ApiError::Forbidden(
                "admin organization required".to_string(),
            )
        })
    }

    // The actor id to attribute an audit event to, or `None` when there is no real
    // authenticated identity (a nil subject). Audit `actor_id` must be NULL in that
    // case — never the nil-UUID sentinel.
    pub fn actor_id(&self) -> Option<Uuid> {
        (!self.user_id.is_nil()).then_some(self.user_id)
    }
}

// Per-request network context captured for audit logging (WI-02): client IP and
// User-Agent. Always present in extensions (fields may be None).
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl RequestContext {
    // The client IP is resolved by `TrustedProxies::client_ip`: forwarding headers are
    // believed only when the TCP peer is a configured reverse proxy, and then read from
    // the right (nearest hop first). See `client_ip.rs` for why the left-most
    // `X-Forwarded-For` value must never be used. `peer_ip` comes from Axum's
    // `ConnectInfo`, available because `main` serves with
    // `into_make_service_with_connect_info`.
    pub fn from_request(
        headers: &HeaderMap,
        peer_ip: Option<IpAddr>,
        trusted_proxies: &TrustedProxies,
    ) -> Self {
        let ip_address = trusted_proxies
            .client_ip(headers, peer_ip)
            .map(|ip| ip.to_string());

        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        RequestContext {
            ip_address,
            user_agent,
        }
    }
}

// Middleware (applied to the whole app): capture the per-request network context
// (client IP + User-Agent) ONCE and store it in extensions, so every handler, the
// rate limiter (`ClientIpKeyExtractor`) and the auth middlewares read one value.
pub async fn capture_client_context(
    State(trusted_proxies): State<SharedTrustedProxies>,
    mut request: Request,
    next: Next,
) -> Response {
    let peer_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip());
    let ctx = RequestContext::from_request(request.headers(), peer_ip, &trusted_proxies);
    request.extensions_mut().insert(ctx);
    next.run(request).await
}

// The header carrying a correlation id, in and out.
const REQUEST_ID_HEADER: &str = "x-request-id";

/// Middleware: give every request a correlation id and put it on a tracing span.
///
/// Without this there was no way to tie the several log lines a single request
/// produces — a repository error, an audit-write failure, the access log — back to
/// one another, which is what makes a production log searchable at all.
///
/// An inbound `x-request-id` is honoured so a trace started at the reverse proxy or
/// the dashboard BFF continues here rather than restarting. It is echoed on the
/// response so a caller can quote it in a bug report.
///
/// SECURITY: the inbound value is attacker-controlled, so it is length-capped and
/// filtered to an unambiguous character set before being logged — an unchecked
/// header would let a caller inject newlines and forge log entries.
pub async fn request_id(mut request: Request, next: Next) -> Response {
    let incoming = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .map(str::to_string);

    let id = incoming.unwrap_or_else(|| Uuid::now_v7().to_string());

    request.extensions_mut().insert(RequestId(id.clone()));

    // `Instrument` rather than `span.enter()`: a guard held across an await point
    // stays entered while the task is parked, so the span would bleed onto whatever
    // else the executor runs on that thread. Instrumenting the future attaches the
    // span to the task instead, which is the only correct form in async code.
    let span = tracing::info_span!("request", request_id = %id);
    let mut response = next.run(request).instrument(span).await;
    if let Ok(value) = header::HeaderValue::from_str(&id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, value);
    }
    response
}

/// The correlation id for the current request, readable by handlers.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|t| t.to_string())
}

// Reads the `tid` claim from the JWT payload WITHOUT verifying the signature, only
// to decide which tenant's published keys to verify against. The actual trust
// decision is made by `verifier.verify(...)` below — this is just key selection.
fn unverified_tenant_id(token: &str) -> Option<Uuid> {
    let payload_b64 = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    claims
        .get("tid")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
}

fn string_array(claims: &serde_json::Value, key: &str) -> Vec<String> {
    claims
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

// A platform-admin token is a symmetric HS256 JWT (no tenant signing key). We
// detect it by the `alg` in the header and verify it with the admin issuer's
// server-wide secret, short-circuiting the tenant-key path. On success it yields an
// admin principal (global: `tenant_id` is nil, used only for audit attribution).
fn try_admin_token(token: &str, admin_issuer: &dyn AdminTokenIssuer) -> Option<AuthPrincipal> {
    let header = jsonwebtoken::decode_header(token).ok()?;
    if header.alg != jsonwebtoken::Algorithm::HS256 {
        return None;
    }
    let claims = admin_issuer.verify(token).ok()?;
    Some(AuthPrincipal {
        user_id: claims.admin_id,
        tenant_id: Uuid::nil(),
        roles: vec!["admin".to_string()],
        groups: Vec::new(),
        scopes: Vec::new(),
        is_platform_admin: true,
        organization_id: Some(claims.organization_id),
    })
}

// Verifies against a key set, collapsing "no keys" and "no key matched" into the
// same `None`. Both mean the same thing to the caller and must produce the same
// generic response — distinguishing them would tell an attacker whether a tenant
// id exists.
async fn verify_against(
    verifier: &dyn TokenVerifier,
    token: &str,
    keys: &[crate::domain::entities::signing_key::SigningKey],
) -> Option<crate::application::ports::token_verifier::VerifiedToken> {
    if keys.is_empty() {
        return None;
    }
    verifier.verify(token, keys).await.ok()
}

// Turns a bearer token into a verified `AuthPrincipal`. Depends only on application
// ports, so the presentation layer never sees the concrete cache or JWT library.
pub struct Authenticator {
    verifier: Arc<dyn TokenVerifier>,
    keys: Arc<dyn SigningKeyRepository>,
    key_cache: Option<Arc<dyn SigningKeyCache>>,
    admin_tokens: Arc<dyn AdminTokenIssuer>,
}

impl Authenticator {
    pub fn new(
        verifier: Arc<dyn TokenVerifier>,
        keys: Arc<dyn SigningKeyRepository>,
        key_cache: Option<Arc<dyn SigningKeyCache>>,
        admin_tokens: Arc<dyn AdminTokenIssuer>,
    ) -> Self {
        Self {
            verifier,
            keys,
            key_cache,
            admin_tokens,
        }
    }

    // A platform-admin (HS256) token is verified against the admin issuer's keys;
    // everything else is verified as an ACCESS token against the signing tenant's
    // published keys. Returns the standard OAuth 401 challenge on any failure.
    pub async fn authenticate(&self, headers: &HeaderMap) -> Result<AuthPrincipal, Response> {
        let token = bearer_token(headers).ok_or_else(|| unauthorized("missing bearer token"))?;

        // Platform-admin tokens carry no `tid` and are signed symmetrically — try them
        // first so they don't fall into the tenant-key lookup below.
        if let Some(principal) = try_admin_token(&token, self.admin_tokens.as_ref()) {
            return Ok(principal);
        }

        let tenant_id =
            unverified_tenant_id(&token).ok_or_else(|| unauthorized("token missing tid"))?;

        // The published key set is cached (see `CachingSigningKeyRepository`), so the
        // common path costs no query at all.
        let keys = self
            .keys
            .find_published(tenant_id)
            .await
            .map_err(|_| unauthorized("key lookup failed"))?;

        let verified = match verify_against(self.verifier.as_ref(), &token, &keys).await {
            Some(verified) => verified,
            None => {
                // Either the tenant has no published keys, or none of the cached ones
                // verified this token. The benign explanation is a key published since
                // the entry was cached — a rotation, or a tenant created moments ago —
                // so drop the entry and retry once against the database. Without this,
                // rotation would reject valid tokens for the whole TTL.
                //
                // A genuinely invalid token also lands here and costs one query; the
                // rate limiter bounds how often that can be provoked.
                let refreshed = match &self.key_cache {
                    Some(cache) => {
                        cache.invalidate(tenant_id);
                        self.keys
                            .find_published(tenant_id)
                            .await
                            .map_err(|_| unauthorized("key lookup failed"))?
                    }
                    None => Vec::new(),
                };

                verify_against(self.verifier.as_ref(), &token, &refreshed)
                    .await
                    .ok_or_else(|| unauthorized("invalid_token"))?
            }
        };

        let user_id =
            Uuid::parse_str(&verified.subject).map_err(|_| unauthorized("invalid subject"))?;
        let claim_tid = verified
            .claims
            .get("tid")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .ok_or_else(|| unauthorized("verified token missing tid"))?;

        Ok(AuthPrincipal {
            user_id,
            tenant_id: claim_tid,
            roles: string_array(&verified.claims, "roles"),
            groups: string_array(&verified.claims, "groups"),
            scopes: verified.scopes,
            // A tenant-issued (asymmetric) token is pinned to its own tenant.
            is_platform_admin: false,
            organization_id: None,
        })
    }
}

// 401 with the RFC 6750 Bearer challenge.
fn unauthorized(description: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(
            header::WWW_AUTHENTICATE,
            format!("Bearer error=\"invalid_token\", error_description=\"{description}\""),
        )],
        Json(json!({ "error": "invalid_token", "error_description": description })),
    )
        .into_response()
}

fn forbidden(description: &str) -> Response {
    (StatusCode::FORBIDDEN, Json(json!({ "error": description }))).into_response()
}

// 403 with the RFC 6750 `insufficient_scope` challenge: the token is valid but was
// not issued for this API.
fn insufficient_scope(scope: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        [(
            header::WWW_AUTHENTICATE,
            format!("Bearer error=\"insufficient_scope\", scope=\"{scope}\""),
        )],
        Json(json!({
            "error": "insufficient_scope",
            "error_description": format!("this API requires the `{scope}` scope"),
            "scope": scope,
        })),
    )
        .into_response()
}

// Why a principal may not use the admin API, or `None` when it may.
//
// A tenant token must carry an admin ROLE *and* the `admin` SCOPE. The role says the
// user is an administrator; the scope says this token was issued FOR the admin API.
// Without the scope check, every OAuth client of the tenant — including third-party
// relying parties — received tokens that opened the admin API whenever an
// administrator signed in to them.
fn admin_denial(principal: &AuthPrincipal) -> Option<Response> {
    if principal.is_platform_admin {
        return None;
    }
    if !principal.is_admin() {
        return Some(forbidden("admin role required"));
    }
    if !principal.has_scope(ADMIN_SCOPE) {
        return Some(insufficient_scope(ADMIN_SCOPE));
    }
    None
}

// Why a principal may not use the `/me` self-service API, or `None` when it may.
// Same reasoning as `admin_denial`: the `account` scope proves the token was issued
// for account management, not merely for signing in to some relying party.
fn account_denial(principal: &AuthPrincipal) -> Option<Response> {
    if principal.is_platform_admin {
        // Platform admins manage themselves under `/admin/me`; they have no tenant
        // account for `/me` to act on.
        return Some(forbidden("platform-admin tokens cannot use /me"));
    }
    if !principal.has_scope(ACCOUNT_SCOPE) {
        return Some(insufficient_scope(ACCOUNT_SCOPE));
    }
    None
}

// Middleware: require a valid token, an admin role and (for tenant tokens) the
// `admin` scope. Inserts `AuthPrincipal` into the request extensions for
// downstream handlers + audit. Every denial is audited.
pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // `capture_client_context` (applied app-wide) already put the enriched context
    // (socket-IP fallback included) into extensions; read it for audit attribution.
    let ctx = request
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_default();

    let principal = match state.auth.authenticate(request.headers()).await {
        Ok(p) => p,
        Err(resp) => {
            // Record the denied attempt (no verified identity available).
            let event = AuditEvent::new(
                None,
                ActorType::System,
                None,
                ADMIN_ACCESS_DENIED,
                Outcome::Failure,
            )
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
            let _ = state.audit_logger.record(event).await;
            return resp;
        }
    };

    if let Some(denied) = admin_denial(&principal) {
        let event = AuditEvent::new(
            Some(principal.tenant_id),
            ActorType::User,
            principal.actor_id(),
            ADMIN_ACCESS_DENIED,
            Outcome::Failure,
        )
        .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
        let _ = state.audit_logger.record(event).await;
        return denied;
    }

    request.extensions_mut().insert(principal);
    next.run(request).await
}

// Middleware: require a valid tenant token carrying the `account` scope. Used by
// the `/me/*` self-service routes — no admin role required, but identity must be
// proven and the token must have been issued for account management.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let principal = match state.auth.authenticate(request.headers()).await {
        Ok(principal) => principal,
        Err(resp) => return resp,
    };
    if let Some(denied) = account_denial(&principal) {
        return denied;
    }
    request.extensions_mut().insert(principal);
    next.run(request).await
}

// Extractor: pulls the principal the middleware placed in extensions. A handler
// using this MUST be mounted behind `require_admin`/`require_auth`, otherwise the
// principal is absent and the handler returns 401.
impl<S> FromRequestParts<S> for AuthPrincipal
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthPrincipal>()
            .cloned()
            .ok_or_else(|| unauthorized("authentication required"))
    }
}

// The authenticated caller, already proven to be authorized for the `:tenant_id`
// in the request path.
//
// WHY THIS EXISTS: `require_admin` authorizes on ROLE ONLY — it checks
// `principal.is_admin()` and never looks at which tenant the request targets.
// Tenant isolation therefore has to be enforced per handler, and "remember to
// call `can_access_tenant` in every handler" is a rule that was silently missed
// on most of the admin surface: a token minted for tenant A could read and write
// tenant B's users, groups, roles, OAuth clients (including rotating their
// secrets), SMTP credentials, email templates, legal documents and consents.
//
// Making the check a *type* rather than a convention means a handler cannot read
// `tenant_id` without having been authorized for it — forgetting the guard is no
// longer expressible. Prefer this over `Path<Uuid>` in every `/tenants/:tenant_id`
// handler.
//
// Uses `RawPathParams` rather than `Path<Uuid>` so it composes with handlers that
// take further path segments (e.g. `/tenants/:tenant_id/groups/:group_id`).
#[derive(Debug, Clone)]
pub struct TenantScope {
    pub tenant_id: Uuid,
    pub principal: AuthPrincipal,
}

impl TenantScope {
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    pub fn principal(&self) -> &AuthPrincipal {
        &self.principal
    }
}

// The authorization decision, split out from the extractor so it is unit-testable
// without building an `AppState`.
fn tenant_access_allowed(
    principal: &AuthPrincipal,
    tenant_id: Uuid,
    tenant_in_admin_org: bool,
) -> bool {
    principal.can_access_tenant(tenant_id, tenant_in_admin_org)
}

impl FromRequestParts<AppState> for TenantScope {
    type Rejection = crate::presentation::errors::ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        use crate::presentation::errors::ApiError;

        let principal = parts
            .extensions
            .get::<AuthPrincipal>()
            .cloned()
            .ok_or_else(|| ApiError::Unauthorized("authentication required".to_string()))?;

        let params = RawPathParams::from_request_parts(parts, state)
            .await
            // Only happens if this extractor is used on a route with no path params
            // at all — a wiring mistake, not something a client can provoke.
            .map_err(|_| ApiError::Internal)?;

        let raw = params
            .iter()
            .find(|(name, _)| *name == "tenant_id")
            .map(|(_, value)| value)
            .ok_or(ApiError::Internal)?;

        let tenant_id = Uuid::parse_str(raw)
            .map_err(|_| ApiError::BadRequest("tenant_id must be a UUID".to_string()))?;

        // Platform admins are bound to one organization: the tenant must belong to
        // it. Without this check an admin of organization A could manage the users,
        // clients and secrets of organization B's tenants given a tenant id.
        let tenant_in_admin_org = match (principal.is_platform_admin, principal.organization_id) {
            (true, Some(organization_id)) => state
                .tenants
                .get_tenant_by_id
                .execute(
                    crate::application::use_cases::tenant::get_tenant_by_id::query::Query {
                        tenant_id,
                        organization_id,
                    },
                )
                .await
                .is_ok(),
            _ => false,
        };

        if !tenant_access_allowed(&principal, tenant_id, tenant_in_admin_org) {
            return Err(ApiError::Forbidden("tenant mismatch".to_string()));
        }

        Ok(Self {
            tenant_id,
            principal,
        })
    }
}

// Extractor: the per-request audit context (never rejects — defaults to empty).
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<RequestContext>()
            .cloned()
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use std::net::Ipv4Addr;

    #[test]
    fn request_context_ignores_forwarding_headers_from_untrusted_peers() {
        let peer = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));
        let none = TrustedProxies::default();

        // No proxy header ⇒ the TCP peer address (direct connection).
        let ctx = RequestContext::from_request(&HeaderMap::new(), Some(peer), &none);
        assert_eq!(ctx.ip_address.as_deref(), Some("203.0.113.7"));

        // A spoofed X-Forwarded-For from an untrusted peer is ignored.
        let mut xff = HeaderMap::new();
        xff.insert("x-forwarded-for", "198.51.100.9, 10.0.0.1".parse().unwrap());
        let ctx = RequestContext::from_request(&xff, Some(peer), &none);
        assert_eq!(ctx.ip_address.as_deref(), Some("203.0.113.7"));

        // From a trusted proxy, the right-most untrusted hop is the client.
        let trusted = TrustedProxies::parse(&["10.0.0.0/8"]).unwrap();
        let proxy = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5));
        let ctx = RequestContext::from_request(&xff, Some(proxy), &trusted);
        assert_eq!(ctx.ip_address.as_deref(), Some("198.51.100.9"));

        // No socket ⇒ None (unknown).
        let ctx = RequestContext::from_request(&HeaderMap::new(), None, &none);
        assert_eq!(ctx.ip_address, None);
    }

    fn principal(roles: &[&str]) -> AuthPrincipal {
        AuthPrincipal {
            user_id: Uuid::now_v7(),
            tenant_id: Uuid::now_v7(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            groups: vec![],
            scopes: vec![],
            is_platform_admin: false,
            organization_id: None,
        }
    }

    #[test]
    fn is_admin_recognizes_admin_roles_and_denies_others() {
        assert!(principal(&["admin"]).is_admin());
        assert!(principal(&["tenant_admin"]).is_admin());
        assert!(principal(&["editor", "admin"]).is_admin());
        assert!(!principal(&["editor"]).is_admin());
        assert!(!principal(&[]).is_admin());
    }

    #[test]
    fn has_role_checks_membership() {
        let p = principal(&["editor", "viewer"]);
        assert!(p.has_role("editor"));
        assert!(!p.has_role("admin"));
    }

    // ── TenantScope: the cross-tenant authorization boundary ──────────────────
    //
    // `require_admin` authorizes on role alone, so these cases are the only thing
    // standing between a tenant-scoped admin and every other tenant's data.

    #[test]
    fn a_tenant_admin_may_act_on_its_own_tenant() {
        let p = principal(&["tenant_admin"]);
        assert!(tenant_access_allowed(&p, p.tenant_id, false));
    }

    #[test]
    fn a_tenant_admin_may_not_act_on_another_tenant() {
        // The bug this whole extractor exists for: holding `tenant_admin` in tenant
        // A must not grant anything in tenant B.
        let p = principal(&["tenant_admin"]);
        assert!(!tenant_access_allowed(&p, Uuid::now_v7(), false));
    }

    #[test]
    fn an_admin_role_name_alone_does_not_cross_tenants() {
        // `admin` and `tenant_admin` both satisfy `is_admin()`; neither may widen
        // the tenant boundary.
        for role in ADMIN_ROLES {
            let p = principal(&[role]);
            assert!(
                !tenant_access_allowed(&p, Uuid::now_v7(), false),
                "role {role} must not reach another tenant"
            );
        }
    }

    #[test]
    fn a_platform_admin_reaches_every_tenant() {
        // Platform admins are cross-tenant within their organization — that is how
        // the dashboard administers all of the organization's tenants.
        let mut p = principal(&["admin"]);
        p.is_platform_admin = true;
        p.organization_id = Some(Uuid::now_v7());
        assert!(tenant_access_allowed(&p, Uuid::now_v7(), true));
    }

    #[test]
    fn a_platform_admin_cannot_reach_another_organizations_tenant() {
        let mut p = principal(&["admin"]);
        p.is_platform_admin = true;
        p.organization_id = Some(Uuid::now_v7());
        assert!(!tenant_access_allowed(&p, Uuid::now_v7(), false));

        // ...and a platform-admin principal without an organization reaches nothing.
        p.organization_id = None;
        assert!(!tenant_access_allowed(&p, Uuid::now_v7(), true));
    }

    // Guards the *next* handler someone writes, not the ones fixed today.
    //
    // `TenantScope` makes the check unforgettable only if handlers actually use it;
    // a new handler that goes back to `Path(tenant_id): Path<Uuid>` would silently
    // reopen the cross-tenant hole and still compile. This scans the handler sources
    // for that shape and fails with instructions.
    #[test]
    fn no_handler_reads_tenant_id_straight_from_the_path() {
        // tenant_handler is the one legitimate exception: its endpoints operate on
        // the tenant *record* and scope by organization instead, via
        // `principal.require_org()` (a tenant-scoped admin carries no `org` claim,
        // so it 403s there). It is asserted below rather than merely skipped.
        const ORG_SCOPED: &str = "tenant_handler.rs";

        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/presentation/handlers");
        let mut offenders = Vec::new();

        for entry in std::fs::read_dir(&dir).expect("handlers directory") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let src = std::fs::read_to_string(&path).expect("read handler");

            let binds_tenant_from_path =
                src.contains("Path(tenant_id)") || src.contains("Path((tenant_id");

            if name == ORG_SCOPED {
                assert!(
                    src.contains("require_org()"),
                    "{name} binds tenant_id from the path but no longer scopes by \
                     organization — it must use TenantScope instead"
                );
                continue;
            }

            if binds_tenant_from_path {
                offenders.push(name);
            }
        }

        assert!(
            offenders.is_empty(),
            "these handlers read `tenant_id` from the path without authorizing it: {offenders:?}\n\
             `require_admin` checks the caller's ROLE, not which tenant they may touch, so this \
             lets any tenant's admin act on any other tenant.\n\
             Take `TenantScope {{ tenant_id, .. }}: TenantScope` instead of `Path<Uuid>`; for \
             extra path segments use `Path((_, other_id))`."
        );
    }

    // ── Scope gates: a token must have been issued FOR the API it calls ────────

    fn with_scopes(mut p: AuthPrincipal, scopes: &[&str]) -> AuthPrincipal {
        p.scopes = scopes.iter().map(|s| s.to_string()).collect();
        p
    }

    #[test]
    fn a_tenant_admin_token_without_the_admin_scope_is_refused() {
        // The S1 hole: a relying party that an administrator signed in to received
        // an access token that opened the admin API.
        let p = with_scopes(principal(&["tenant_admin"]), &["openid", "profile"]);
        let denied = admin_denial(&p).expect("must be denied");
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        assert!(
            denied
                .headers()
                .get(header::WWW_AUTHENTICATE)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.contains("insufficient_scope"))
        );
    }

    #[test]
    fn a_tenant_admin_token_with_the_admin_scope_is_allowed() {
        let p = with_scopes(principal(&["tenant_admin"]), &["openid", ADMIN_SCOPE]);
        assert!(admin_denial(&p).is_none());
    }

    #[test]
    fn the_admin_scope_without_an_admin_role_is_refused() {
        let p = with_scopes(principal(&["editor"]), &[ADMIN_SCOPE]);
        assert!(admin_denial(&p).is_some());
    }

    #[test]
    fn a_platform_admin_needs_no_scope_for_the_admin_api() {
        let mut p = principal(&["admin"]);
        p.is_platform_admin = true;
        assert!(admin_denial(&p).is_none());
    }

    #[test]
    fn the_account_api_requires_the_account_scope() {
        let p = with_scopes(principal(&[]), &["openid", "profile", "email"]);
        assert!(account_denial(&p).is_some());

        let p = with_scopes(principal(&[]), &["openid", ACCOUNT_SCOPE]);
        assert!(account_denial(&p).is_none());
    }

    #[test]
    fn platform_admin_tokens_cannot_use_the_account_api() {
        let mut p = with_scopes(principal(&["admin"]), &[ACCOUNT_SCOPE]);
        p.is_platform_admin = true;
        assert!(account_denial(&p).is_some());
    }

    #[test]
    fn unverified_tenant_id_reads_tid_from_payload() {
        let tid = Uuid::now_v7();
        let payload = serde_json::json!({ "sub": "x", "tid": tid.to_string() });
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).unwrap());
        let token = format!("header.{payload_b64}.signature");
        assert_eq!(unverified_tenant_id(&token), Some(tid));
    }

    #[test]
    fn unverified_tenant_id_rejects_garbage() {
        assert_eq!(unverified_tenant_id("not-a-jwt"), None);
        assert_eq!(unverified_tenant_id("a.b.c"), None);
    }

    #[test]
    fn string_array_extracts_role_claims() {
        let claims = serde_json::json!({ "roles": ["admin", "editor"], "x": 1 });
        assert_eq!(string_array(&claims, "roles"), vec!["admin", "editor"]);
        assert_eq!(string_array(&claims, "missing"), Vec::<String>::new());
    }

    #[test]
    fn request_context_captures_user_agent() {
        let mut headers = HeaderMap::new();
        headers.insert(header::USER_AGENT, "curl/8".parse().unwrap());
        let peer = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 5));
        let ctx = RequestContext::from_request(&headers, Some(peer), &TrustedProxies::default());
        assert_eq!(ctx.ip_address.as_deref(), Some("203.0.113.5"));
        assert_eq!(ctx.user_agent.as_deref(), Some("curl/8"));
    }
}
