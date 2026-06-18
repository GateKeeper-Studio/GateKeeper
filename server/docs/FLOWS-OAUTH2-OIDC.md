# OAuth 2.1 / OpenID Connect Flow

How this provider implements the OAuth 2.1 / OIDC protocol. For the first-party
login SPA flow (and MFA, registration, password reset, admin auth), see
[`FLOWS-AUTHENTICATION.md`](FLOWS-AUTHENTICATION.md). For the layered design, see
[`ARCHITECTURE.md`](ARCHITECTURE.md).

The implementation follows **OAuth 2.1** and the OAuth Security BCP (RFC 9700):
Authorization Code grant with **mandatory PKCE (S256)**, **no implicit flow**,
exact redirect-URI matching, and rotating refresh tokens with reuse detection.

---

## 1. Roles & key concepts

- **Authorization Server / OpenID Provider** — this service. Multi-tenant: **each
  tenant is its own issuer** with its own ES256 signing keys, so a token minted for
  one tenant is invalid for another.
- **Client (Relying Party)** — a registered application (`oauth_clients`). Two types:
  - **Public** (SPA / mobile / native): cannot keep a secret; **PKCE is the only
    proof** at the token endpoint.
  - **Confidential** (server-side web app): has a `client_secret` (stored Argon2-
    hashed with the tenant pepper) and authenticates at the token endpoint.
- **Resource Owner** — the end user, authenticated from their own credentials.
- **`tenant_id`** is carried explicitly in protocol requests because the provider is
  multi-tenant.

### Client model (`domain/entities/oauth_client.rs`)
`redirect_uris` (exact match, no wildcards), `allowed_scopes` (the superset a client
may ever request — requested scopes are intersected with it, never widened),
`allowed_grant_types`, `require_pkce` (kept true), `require_consent`, `mfa_policy`,
`email_provider_id`, `is_active`.

---

## 2. Endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/.well-known/openid-configuration` | OIDC discovery metadata |
| `GET` | `/.well-known/jwks.json` | Tenant public signing keys (JWKS), by `kid` |
| `GET` | `/oauth/authorize` | Validate an authorization request |
| `POST` | `/oauth/authorize/consent` | Authenticate user + record consent + issue code |
| `POST` | `/oauth/token` | Token endpoint (`authorization_code`, `refresh_token`) |
| `GET`/`POST` | `/oauth/userinfo` | OIDC UserInfo (bearer access token) |

> Discovery advertises: `response_types_supported: ["code"]`,
> `grant_types_supported: ["authorization_code", "refresh_token"]`,
> `code_challenge_methods_supported: ["S256"]`,
> `scopes_supported: ["openid", "profile", "email", "offline_access"]`,
> `token_endpoint_auth_methods_supported: ["client_secret_basic",
> "client_secret_post", "none"]`. Tokens are signed with ES256.

---

## 3. Authorization Code + PKCE flow

This provider is API-first: rather than rendering HTML, `/oauth/authorize` returns
the validated request as JSON, and the front-end submits the user's decision to
`/oauth/authorize/consent`. (The first-party login SPA uses a session-based variant
— see [`FLOWS-AUTHENTICATION.md`](FLOWS-AUTHENTICATION.md) §1.)

```
   Client app                Browser / SPA                 Provider (this server)
       │                          │                                │
       │ 1. build PKCE verifier + S256 challenge, state, nonce     │
       │─────────────────────────▶│                                │
       │                          │ 2. GET /oauth/authorize?...    │
       │                          │───────────────────────────────▶│  authorize_request
       │                          │   validated request (JSON)     │  (validate; DO NOT
       │                          │◀───────────────────────────────│   trust redirect yet)
       │                          │ 3. user enters credentials +   │
       │                          │    approves scopes             │
       │                          │ 4. POST /oauth/authorize/consent│
       │                          │───────────────────────────────▶│  authorize_complete
       │                          │   302 redirect_uri?code=&state=│  (authenticate user,
       │                          │   (or JSON for fetch clients)   │   upsert consent, mint code)
       │                          │◀───────────────────────────────│
       │ 5. POST /oauth/token (code + code_verifier + redirect_uri) │
       │──────────────────────────────────────────────────────────▶│  token_exchange
       │   access_token, id_token, refresh_token                    │  (verify PKCE, consume
       │◀──────────────────────────────────────────────────────────│   code, sign tokens)
       │ 6. GET /oauth/userinfo (Bearer access_token)  ─────────────▶  userinfo
```

### Step 2 — `GET /oauth/authorize` (`authorize_request`)
Validation happens in a deliberate order, because **what you may do with an error
depends on whether the `redirect_uri` is trusted yet** (open-redirect guard):

1. Client must exist, belong to `tenant_id`, and be active → else an **error page**
   (never a redirect; the redirect_uri isn't trusted yet).
2. `redirect_uri` must **exactly** match a registered URI → else an error page.
3. *(redirect_uri now trusted — later errors may redirect with `?error=&state=`)*
4. `response_type` must be exactly `code`.
5. PKCE present and `code_challenge_method == "S256"` (when the client requires PKCE).
6. `state` is required (CSRF protection).
7. Requested scopes are intersected with the client's allow-list; empty → `invalid_scope`.

Returns the validated, tamper-checked request (`client_name`, allowed `scopes`,
`requires_consent`, …) for the UI to render.

### Step 4 — `POST /oauth/authorize/consent` (`authorize_complete`)
Runs in **one transaction**. It re-validates everything (never trusts the posted
form), then:
1. Re-resolve + re-check the client and `redirect_uri`; re-intersect scopes.
2. **Authenticate the end user** from the `email` + `password` in the body — the
   user id is *derived here*, never taken from the request (otherwise anyone could
   mint a code for any account). Failures are generic to prevent account enumeration.
3. Consent: if the client requires consent and an existing `ConsentGrant` doesn't
   already cover the requested scopes and the user hasn't approved, return
   `ConsentRequired` with the missing-scope delta.
4. Upsert the consent grant (idempotent union of scopes).
5. Mint a single-use **authorization code** (TTL **60s**): random raw code returned
   once, **SHA-256 hash** persisted, bound to client, redirect_uri, user, scopes,
   `code_challenge`/method, and `nonce`.

The handler content-negotiates the response: a browser navigation gets the spec
**302** to `redirect_uri?code=…&state=…`; a `fetch`/XHR caller that sends `Accept:
application/json` gets the code in JSON (a SPA can't follow a cross-origin 302).

### Step 5 — `POST /oauth/token`, `grant_type=authorization_code` (`token_exchange`)
One transaction; the code is consumed and the refresh token stored together so a
crash can't leave a half-redeemed code:

1. **Authenticate the client** — confidential ⇒ verify `client_secret` against the
   stored Argon2 hash (tenant pepper); public ⇒ PKCE is the only proof.
2. Look the code up **by hash**; reject if not redeemable (expired/used) — replay of
   a consumed code is a theft signal.
3. **Bind checks:** same client, same `redirect_uri`, and **PKCE verify** the
   presented `code_verifier` against the stored `code_challenge`.
4. **Consume the code** (single use).
5. Load the tenant's **active signing key**; resolve the user's current `roles` +
   `groups`.
6. Sign the **access token** (claims: `iss`, `sub`, `aud`=client_id, `tid`, `scope`,
   `roles`, `groups`, `iat`, `exp`, `jti`; TTL **900s / 15 min**).
7. If `openid` scope was granted, sign an **id_token** (echoes `nonce` only when the
   client supplied one).
8. Issue + store a rotating **refresh token** (raw returned once, hash persisted; TTL
   **30 days**).
9. Emit a `token.issued` audit event after commit.

Response: `access_token`, optional `id_token`, `refresh_token`, `token_type:
"Bearer"`, `expires_in`, `scope`.

---

## 4. Refresh-token rotation & reuse detection

`POST /oauth/token`, `grant_type=refresh_token` (`token_refresh`). The core security
property is **rotation with reuse detection**:

1. Authenticate the client (confidential ⇒ verify secret).
2. Look the presented refresh token up **by hash**; it must belong to this client.
3. **Reuse detection:** if the presented token is inactive *and* was already rotated
   or revoked, that signals theft → **revoke the entire token family**, emit
   `token.reuse_detected`, and reject.
4. Otherwise **rotate**: mark the presented token rotated and mint a successor in the
   same family.
5. Re-mint the access (+ id) token with the active signing key, **re-resolving roles
   and groups** so the refreshed token reflects the user's *current* authorization
   context.
6. Emit `token.refreshed`.

Same response shape as the code exchange, with a fresh rotated `refresh_token`.

---

## 5. UserInfo

`GET`/`POST /oauth/userinfo` (`userinfo`) takes the bearer **access token**, verifies
it against the tenant's published keys (selected by `kid`), and returns the user's
claims filtered by the granted scopes.

---

## 6. Signing keys & JWKS

- Each tenant owns an ECDSA P-256 (**ES256**) keypair, generated at tenant creation
  by `EcdsaKeyGenerator` and stored in `signing_keys`.
- `TokenSigner::sign` signs with the tenant's **active** key and stamps its `kid`.
- `TokenVerifier::verify` selects the verifying key by `kid` among the tenant's
  **published** keys — the same set served at `/.well-known/jwks.json`.
- This supports key rotation: a new key can be published (verifiable) before becoming
  active (signing), and old keys stay published until all tokens they signed expire.

---

## 7. Security properties (summary)

| Property | Where enforced |
|---|---|
| PKCE S256 mandatory; no implicit flow | `authorize_request`, `token_exchange` (`verify_pkce`) |
| Exact redirect-URI match (no wildcards) | `OAuthClient::is_redirect_uri_allowed` |
| Scopes intersected with client allow-list (never widened) | `OAuthClient::filter_scopes` |
| User authenticated from own credentials, id never client-supplied | `authorize_complete` / login flow |
| Authorization codes single-use, short-lived (60s), stored hashed | `authorize_complete`, `token_exchange` |
| Confidential client secret verified (Argon2 + tenant pepper) | `token_exchange`, `token_refresh` |
| Refresh tokens rotate; reuse revokes the family | `token_refresh` |
| Per-tenant ES256 keys; tokens pinned to issuer + `tid` | signing key services, auth middleware |
| `roles`/`groups` re-resolved on issue **and** refresh | `resolve_user_claims` |
| Security headers (CSP, XFO=DENY, HSTS, no-referrer) | `security_headers` middleware |
| Token issue/refresh/reuse audited | `AuditLogger` |
