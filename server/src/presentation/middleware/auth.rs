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

use crate::{
    application::ports::{admin_token_issuer::AdminTokenIssuer, token_verifier::TokenVerifier},
    domain::{
        entities::{
            audit_action::ADMIN_ACCESS_DENIED,
            audit_event::{ActorType, AuditEvent, Outcome},
        },
        repositories::signing_key_repository::SigningKeyRepository,
    },
    infraestructure::cache::caching_signing_key_repository::CachingSigningKeyRepository,
    presentation::routes::AppState,
};

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

    // Whether this principal may act on `tenant_id`. Global platform admins may act on
    // any tenant; everyone else is confined to the tenant their token was issued for.
    pub fn can_access_tenant(&self, tenant_id: Uuid) -> bool {
        self.is_platform_admin || self.tenant_id == tenant_id
    }

    // The organization this admin is bound to, or a 403 when the token carries none
    // (e.g. an end-user token, or `REQUIRE_ADMIN_AUTH=false`). Used by the
    // `/admin/me/organization*` handlers so an admin only ever touches its own org.
    pub fn require_org(&self) -> Result<Uuid, crate::presentation::errors::ApiError> {
        self.organization_id.ok_or_else(|| {
            crate::presentation::errors::ApiError::Forbidden(
                "admin organization required".to_string(),
            )
        })
    }

    // The actor id to attribute an audit event to, or `None` when there is no real
    // authenticated identity (the synthetic `system()` principal). Audit `actor_id`
    // must be NULL in that case — never the nil-UUID sentinel `system()` carries.
    pub fn actor_id(&self) -> Option<Uuid> {
        (!self.user_id.is_nil()).then_some(self.user_id)
    }

    // A placeholder principal used only when admin-auth enforcement is DISABLED
    // (`REQUIRE_ADMIN_AUTH=false`) and the request carried no usable token. It keeps
    // handlers that structurally depend on `AuthPrincipal` working during the
    // dashboard's bearer-token rollout; audit events it produces carry a NULL actor id
    // (its nil `user_id` collapses to `None` via `actor_id()`).
    pub fn system() -> Self {
        AuthPrincipal {
            user_id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            roles: Vec::new(),
            groups: Vec::new(),
            scopes: Vec::new(),
            is_platform_admin: false,
            organization_id: None,
        }
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
    // Prefer a proxy-supplied client IP — the left-most X-Forwarded-For hop (the
    // original client), then X-Real-IP — and fall back to the TCP peer address when
    // no proxy header is present (a direct connection). `peer_ip` comes from Axum's
    // `ConnectInfo`, available because `main` serves with
    // `into_make_service_with_connect_info`.
    //
    // SECURITY: X-Forwarded-For is client-spoofable; it should only be trusted when
    // this service sits behind a known reverse proxy that overwrites it. The socket
    // address is the unspoofable fallback for direct/unproxied deployments.
    pub fn from_request(headers: &HeaderMap, peer_ip: Option<IpAddr>) -> Self {
        let ip_address = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|s| s.trim().to_string())
            .or_else(|| {
                headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .or_else(|| peer_ip.map(|ip| ip.to_string()));

        let user_agent = headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        RequestContext {
            ip_address,
            user_agent,
        }
    }

    // Header-only context (no socket fallback). Retained for the few call sites that
    // don't have the peer address; handlers should prefer the `RequestContext`
    // extractor, which `capture_client_context` populates with the socket fallback.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self::from_request(headers, None)
    }
}

// Middleware (applied to the whole app): capture the per-request network context
// (client IP + User-Agent) ONCE and store it in extensions, so every handler and the
// auth middlewares read a single, enriched value. The peer socket address — present
// because `main` serves with `into_make_service_with_connect_info::<SocketAddr>()` —
// is used as the IP fallback when no X-Forwarded-For / X-Real-IP proxy header is set.
pub async fn capture_client_context(mut request: Request, next: Next) -> Response {
    let peer_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip());
    let ctx = RequestContext::from_request(request.headers(), peer_ip);
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

// Verifies the bearer token and builds the principal. A platform-admin (HS256)
// token is verified against the server-wide admin secret; everything else is
// verified against the signing tenant's published keys. Returns the standard OAuth
// 401 challenge on any failure.
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

async fn authenticate(
    headers: &HeaderMap,
    verifier: &dyn TokenVerifier,
    keys_reader: &dyn SigningKeyRepository,
    key_cache: Option<&CachingSigningKeyRepository>,
    admin_issuer: &dyn AdminTokenIssuer,
) -> Result<AuthPrincipal, Response> {
    let token = bearer_token(headers).ok_or_else(|| unauthorized("missing bearer token"))?;

    // Platform-admin tokens carry no `tid` and are signed symmetrically — try them
    // first so they don't fall into the tenant-key lookup below.
    if let Some(principal) = try_admin_token(&token, admin_issuer) {
        return Ok(principal);
    }

    let tenant_id =
        unverified_tenant_id(&token).ok_or_else(|| unauthorized("token missing tid"))?;

    // The published key set is cached (see `CachingSigningKeyRepository`), so the
    // common path costs no query at all.
    let keys = keys_reader
        .find_published(tenant_id)
        .await
        .map_err(|_| unauthorized("key lookup failed"))?;

    let verified = match verify_against(verifier, &token, &keys).await {
        Some(verified) => verified,
        None => {
            // Either the tenant has no published keys, or none of the cached ones
            // verified this token. The benign explanation is a key published since
            // the entry was cached — a rotation, or a tenant created moments ago —
            // so drop the entry and retry once against the database. Without this,
            // rotation would reject valid tokens for the whole TTL.
            //
            // A genuinely invalid token also lands here and costs one query, which
            // is what every request cost before the cache existed; the credential
            // rate limiter bounds how often that can be provoked.
            let refreshed = match key_cache {
                Some(cache) => {
                    cache.invalidate(tenant_id);
                    keys_reader
                        .find_published(tenant_id)
                        .await
                        .map_err(|_| unauthorized("key lookup failed"))?
                }
                None => Vec::new(),
            };

            verify_against(verifier, &token, &refreshed)
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

// Middleware: require a valid token AND an admin role. Inserts `AuthPrincipal` and
// `RequestContext` into the request extensions for downstream handlers + audit.
//
// When REQUIRE_ADMIN_AUTH is disabled (dashboard auth-plumbing transition), a valid
// token is still parsed best-effort for audit attribution, but a missing/invalid
// token or non-admin role does NOT block the request. Default: enforced.
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
    let auth = authenticate(
        request.headers(),
        state.token_verifier.as_ref(),
        state.signing_key_reader.as_ref(),
        state.signing_key_cache.as_deref(),
        state.admin_token_service.as_ref(),
    )
    .await;

    if state.require_admin_auth {
        let principal = match auth {
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
        if !principal.is_admin() {
            let event = AuditEvent::new(
                Some(principal.tenant_id),
                ActorType::User,
                principal.actor_id(),
                ADMIN_ACCESS_DENIED,
                Outcome::Failure,
            )
            .with_request_context(ctx.ip_address.clone(), ctx.user_agent.clone());
            let _ = state.audit_logger.record(event).await;
            return forbidden("admin role required");
        }
        request.extensions_mut().insert(principal);
    } else {
        // Enforcement disabled: attach the real principal when a token was present,
        // otherwise a synthetic system principal so handlers still function (and the
        // `AuthPrincipal` extractor never 401s while the dashboard is being wired up).
        let principal = auth.unwrap_or_else(|_| AuthPrincipal::system());
        request.extensions_mut().insert(principal);
    }

    // `ctx` is already in extensions (inserted by `capture_client_context`); no
    // re-insert needed.
    next.run(request).await
}

// Middleware: require a valid token (any authenticated user). Used by `/me/*`
// self-service routes — no admin role required, but identity must be proven.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    // The per-request context is already in extensions (`capture_client_context`).
    match authenticate(
        request.headers(),
        state.token_verifier.as_ref(),
        state.signing_key_reader.as_ref(),
        state.signing_key_cache.as_deref(),
        state.admin_token_service.as_ref(),
    )
    .await
    {
        Ok(principal) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(resp) => {
            if state.require_admin_auth {
                resp
            } else {
                next.run(request).await
            }
        }
    }
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

// The authorization decision, split out from the extractor so it is unit-testable:
// `from_request_parts` needs an `AppState`, which carries ~100 use-case handles and
// cannot be built in a test.
//
// `enforced` mirrors `AppState::require_admin_auth`. When admin-auth enforcement is
// off (a migration aid, refused outside development by `validate_environment`), the
// admin layer already waves the request through, so pinning the tenant here would
// reject requests the guard above accepted.
fn tenant_access_allowed(enforced: bool, principal: &AuthPrincipal, tenant_id: Uuid) -> bool {
    !enforced || principal.can_access_tenant(tenant_id)
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

        if !tenant_access_allowed(state.require_admin_auth, &principal, tenant_id) {
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
    fn request_context_prefers_proxy_headers_then_socket_fallback() {
        let peer = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));

        // No proxy header ⇒ fall back to the TCP peer address (direct connection).
        let ctx = RequestContext::from_request(&HeaderMap::new(), Some(peer));
        assert_eq!(ctx.ip_address.as_deref(), Some("203.0.113.7"));

        // X-Forwarded-For (left-most hop) wins over the socket IP.
        let mut xff = HeaderMap::new();
        xff.insert("x-forwarded-for", "198.51.100.9, 10.0.0.1".parse().unwrap());
        let ctx = RequestContext::from_request(&xff, Some(peer));
        assert_eq!(ctx.ip_address.as_deref(), Some("198.51.100.9"));

        // X-Real-IP wins over the socket IP when no X-Forwarded-For is present.
        let mut real = HeaderMap::new();
        real.insert("x-real-ip", "192.0.2.5".parse().unwrap());
        let ctx = RequestContext::from_request(&real, Some(peer));
        assert_eq!(ctx.ip_address.as_deref(), Some("192.0.2.5"));

        // No header and no socket ⇒ None (unknown).
        let ctx = RequestContext::from_request(&HeaderMap::new(), None);
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
        assert!(tenant_access_allowed(true, &p, p.tenant_id));
    }

    #[test]
    fn a_tenant_admin_may_not_act_on_another_tenant() {
        // The bug this whole extractor exists for: holding `tenant_admin` in tenant
        // A must not grant anything in tenant B.
        let p = principal(&["tenant_admin"]);
        assert!(!tenant_access_allowed(true, &p, Uuid::now_v7()));
    }

    #[test]
    fn an_admin_role_name_alone_does_not_cross_tenants() {
        // `admin` and `tenant_admin` both satisfy `is_admin()`; neither may widen
        // the tenant boundary.
        for role in ADMIN_ROLES {
            let p = principal(&[role]);
            assert!(
                !tenant_access_allowed(true, &p, Uuid::now_v7()),
                "role {role} must not reach another tenant"
            );
        }
    }

    #[test]
    fn a_platform_admin_reaches_every_tenant() {
        // Platform admins are deliberately cross-tenant — that is how the dashboard
        // administers all tenants, and `can_access_tenant`'s existing contract.
        let mut p = principal(&["admin"]);
        p.is_platform_admin = true;
        assert!(tenant_access_allowed(true, &p, Uuid::now_v7()));
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

    #[test]
    fn disabling_enforcement_bypasses_the_tenant_pin() {
        // Matches `require_admin`: with REQUIRE_ADMIN_AUTH=false the admin layer
        // already lets the request through, so this must not 403 it instead.
        // `validate_environment` refuses that setting outside development.
        let p = principal(&["tenant_admin"]);
        assert!(tenant_access_allowed(false, &p, Uuid::now_v7()));
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
    fn request_context_prefers_forwarded_for_left_hop() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.5, 10.0.0.1".parse().unwrap());
        headers.insert(header::USER_AGENT, "curl/8".parse().unwrap());
        let ctx = RequestContext::from_headers(&headers);
        assert_eq!(ctx.ip_address.as_deref(), Some("203.0.113.5"));
        assert_eq!(ctx.user_agent.as_deref(), Some("curl/8"));
    }
}
