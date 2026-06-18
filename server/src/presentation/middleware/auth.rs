use axum::{
    Json,
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::Engine;
use serde_json::json;
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
    presentation::routes::AppState,
};

// Roles that authorize tenant-admin (dashboard) actions. Deny-by-default: an
// authenticated user without one of these is forbidden from the admin surface.
pub const ADMIN_ROLES: [&str; 2] = ["admin", "tenant_admin"];

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
    // org/subscription action is scoped to it, so an admin can only ever manage its own
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
    pub fn from_headers(headers: &HeaderMap) -> Self {
        // Prefer the left-most X-Forwarded-For hop (the original client) when behind a
        // proxy; fall back to X-Real-IP. No header ⇒ None (direct/unknown).
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
            });

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
async fn authenticate(
    headers: &HeaderMap,
    verifier: &dyn TokenVerifier,
    keys_reader: &dyn SigningKeyRepository,
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

    let keys = keys_reader
        .find_published(tenant_id)
        .await
        .map_err(|_| unauthorized("key lookup failed"))?;
    if keys.is_empty() {
        return Err(unauthorized("no published keys for tenant"));
    }

    let verified = verifier
        .verify(&token, &keys)
        .await
        .map_err(|_| unauthorized("invalid_token"))?;

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
    let ctx = RequestContext::from_headers(request.headers());
    let auth = authenticate(
        request.headers(),
        state.token_verifier.as_ref(),
        state.signing_key_reader.as_ref(),
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

    request.extensions_mut().insert(ctx);
    next.run(request).await
}

// Middleware: require a valid token (any authenticated user). Used by `/me/*`
// self-service routes — no admin role required, but identity must be proven.
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let ctx = RequestContext::from_headers(request.headers());
    match authenticate(
        request.headers(),
        state.token_verifier.as_ref(),
        state.signing_key_reader.as_ref(),
        state.admin_token_service.as_ref(),
    )
    .await
    {
        Ok(principal) => {
            request.extensions_mut().insert(principal);
            request.extensions_mut().insert(ctx);
            next.run(request).await
        }
        Err(resp) => {
            if state.require_admin_auth {
                resp
            } else {
                request.extensions_mut().insert(ctx);
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
