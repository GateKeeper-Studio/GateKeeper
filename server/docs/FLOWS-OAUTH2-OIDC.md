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
`email_provider_id`, `is_active`, `post_logout_redirect_uris` (exact match, same rules as
`redirect_uris`), and `backchannel_logout_uri` / `backchannel_logout_session_required`
(OIDC Back-Channel Logout, §8).

---

## 2. Endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/.well-known/openid-configuration` | OIDC discovery metadata |
| `GET` | `/.well-known/jwks.json` | Tenant public signing keys (JWKS), by `kid` |
| `GET` | `/oauth/authorize` | Validate an authorization request (read-only; no auth, no code issued) |
| `POST` | `/oauth/token` | Token endpoint (`authorization_code`, `refresh_token`) |
| `GET`/`POST` | `/oauth/userinfo` | OIDC UserInfo (bearer access token) |
| `GET`/`POST` | `/oauth/logout` | RP-Initiated Logout (`id_token_hint`); signs the user out of every app (§8) |

> Discovery advertises: `response_types_supported: ["code"]`,
> `grant_types_supported: ["authorization_code", "refresh_token"]`,
> `code_challenge_methods_supported: ["S256"]`,
> `scopes_supported: ["openid", "profile", "email", "offline_access"]`,
> `token_endpoint_auth_methods_supported: ["client_secret_basic",
> "client_secret_post", "none"]`, `end_session_endpoint`,
> `backchannel_logout_supported: true`, `backchannel_logout_session_supported: true`.
> Tokens are signed with ES256.

---

## 3. Authorization Code + PKCE flow

This provider is API-first: rather than rendering HTML, `/oauth/authorize` returns
the validated request as JSON for the login SPA to render. Authentication and code
issuance happen **only** through the first-party `/v1/auth/*` flow, which enforces
password, MFA, adaptive risk, and email-verification controls — see
[`FLOWS-AUTHENTICATION.md`](FLOWS-AUTHENTICATION.md) §1. There is no direct
password-authenticating OAuth endpoint: a second, unguarded credential path would
bypass those controls (notably MFA), so it is intentionally absent.

```
   Client app                Browser / SPA                 Provider (this server)
       │                          │                                │
       │ 1. build PKCE verifier + S256 challenge, state, nonce     │
       │─────────────────────────▶│                                │
       │                          │ 2. GET /oauth/authorize?...    │
       │                          │───────────────────────────────▶│  authorize_request
       │                          │   validated request (JSON)     │  (validate; DO NOT
       │                          │◀───────────────────────────────│   trust redirect yet)
       │                          │ 3. login + MFA via /v1/auth/*  │  login → verify-mfa →
       │                          │───────────────────────────────▶│  authorize_session
       │                          │   302 redirect_uri?code=&state=│  (authenticate user,
       │                          │◀───────────────────────────────│   upsert consent, mint code)
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

### Steps 3–4 — authenticate + issue the code (`/v1/auth/*`)
Authentication and code issuance are owned by the first-party login flow, not by a
dedicated OAuth endpoint. The SPA carries the validated request into
`POST /v1/auth/login`; the server authenticates the password, runs the adaptive
risk gate, and — when required — challenges for a second factor
(`POST /v1/auth/verify-mfa/*`). Only once the auth session is fully verified does
`POST /v1/auth/authorize` (`authorize_session`) spend it for a single-use
**authorization code** (TTL **60s**): random raw code returned once, **SHA-256 hash**
persisted, bound to client, redirect_uri, user, scopes, `code_challenge`/method, and
`nonce`. See [`FLOWS-AUTHENTICATION.md`](FLOWS-AUTHENTICATION.md) §1 for the full
sequence, including MFA enrollment and forced-password-change gating.

Routing every interactive login through this single path is deliberate: it ensures
MFA, lockout, and risk evaluation cannot be sidestepped by an alternate credential
endpoint.

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
   client supplied one, and carries `sid`: the session id, which is the new refresh
   token's family id — see §8).
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
3. **Reuse detection (with a rotation-leeway grace window):** if the presented token is
   inactive:
   - It was rotated **within the leeway window** and not revoked → treat as a legitimate
     concurrent-refresh / retry race, **not** theft. Reissue (step 4b); the family is
     kept.
   - It was already rotated (outside the window) or revoked → theft signal → **revoke the
     entire token family**, emit `token.reuse_detected`, and reject. A revoked token never
     gets grace.
   - It is merely expired → reject (no family revoke).
4. Mint the successor:
   - **(a) active token →** *rotate*: mark the presented token rotated and mint a
     successor in the same family.
   - **(b) within-leeway token →** *reissue*: leave the presented token untouched and mint
     a fresh **sibling** in the same family. Because the presented token isn't re-stamped,
     its leeway window can't slide forward on repeated presentation.
5. Re-mint the access (+ id) token with the active signing key, **re-resolving roles
   and groups** so the refreshed token reflects the user's *current* authorization
   context.
6. Emit `token.refreshed` (with `grace: true` in its metadata on the leeway path).

Same response shape as the code exchange, with a fresh rotated `refresh_token`.

> **Adaptive risk on refresh.** Before minting the successor, the presented refresh is run
> through the same risk engine as an interactive login: it is geo-enriched and recorded as a
> login event (so refreshes show up in the security views and per-user sign-in activity), and
> a **critical** decision — e.g. impossible travel or an IP under an active block — refuses the
> refresh with `invalid_grant`, forcing a full re-authentication. Being a non-interactive path,
> it never triggers MFA step-up. The client IP comes from `capture_client_context` (proxy
> header, or the TCP peer address as a fallback). The social-login callback
> (`external_login`) is risk-scored the same way.

### Rotation leeway

The grace window in step 3 defaults to **10 seconds** and is configurable via
`REFRESH_REUSE_LEEWAY_SECONDS` (`0` disables it). It exists because a client — notably a
Backend-for-Frontend refreshing per request — can legitimately present the *same* refresh
token on several concurrent requests, or retry after a network blip. Without leeway, strict
rotation + reuse detection would see the second presentation as theft, revoke the whole
family, and log the user out. The window lets those races resolve while a genuinely stolen
token replayed *later* is still caught. The tradeoff: a token replayed **inside** the window
is honored, so keep the window small. (BFF clients should still coalesce concurrent
refreshes so only one token is presented; leeway covers the residual races.)

### Active sessions (list + sign out)

A **session** is an active refresh-token family: one sign-in to one app, followed by its
refreshes. It is a view over the rotation rows above (`application/use_cases/session/`),
with no table of its own. A family is listed when its newest token is still live (not
revoked, not rotated, not expired). For each session the list shows:

| Field | Meaning |
|---|---|
| `id` | The family id. Pass it to the revoke endpoint. |
| `client_id`, `client_name`, `scopes` | The app (public client id) and what it may do. |
| `signed_in_at` | The family's first token: when the authorization code was exchanged. |
| `last_active_at` | The newest token: the last refresh. Only as precise as the access-token lifetime. |
| `expires_at` | When the live refresh token expires if not used again. |

| Surface | Endpoints | Who |
|---|---|---|
| Self-service | `GET /me/sessions`, `DELETE /me/sessions/{id}`, `DELETE /me/sessions` (all) | the user (`account` scope) |
| Admin | `GET /tenants/{t}/users/{u}/sessions`, `DELETE …/sessions/{id}`, `DELETE …/sessions` (all) | tenant admins; dashboard: user page → Security |

- **Minimal by construction.** The refresh-token table stores no IP or user agent, and the
  view adds none: no token ids or hashes leave the server, only the fields above.
- **One use case for both surfaces.** `revoke_user_session` / `revoke_all_user_sessions`
  authorize with `Actor::authorize_account`: the user themself or an admin of the tenant.
  The revoke is scoped to the user (`revoke_family_of_user`), so a session id of another
  user is simply not found. No step-up: ending a session only removes access.
- **What revoking does.** Every token of the family is revoked, so the app's next refresh
  fails with `invalid_grant` (and is reported as reuse). An **access token already issued
  stays valid until it expires** — access tokens are stateless JWTs, at most
  `ACCESS_TOKEN_TTL_SECONDS` (15 min).
- **Audit.** `user.session.revoked`, in the same transaction as the revoke: actor type
  `user` for your own session, `admin` otherwise; `resource = session/<family id>`, or
  `metadata.all = true` for "sign out everywhere".
- **The app is told.** Ending a session sends the app an OIDC back-channel logout
  (§8), if it registered a `backchannel_logout_uri`.
- **Not provided:** device details, which would need new data at sign-in; a "this session"
  marker on `/me` (ID tokens carry `sid`, access tokens do not); and the platform-admin
  dashboard's own sessions (`admin_refresh_tokens`).

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
| User authenticated from own credentials, id never client-supplied | `/v1/auth/*` login flow (`login`, `authorize_session`) |
| MFA / risk / email-verification cannot be bypassed by an alt endpoint | single login path; no direct-password OAuth endpoint |
| Authorization codes single-use, short-lived (60s), stored hashed | `authorize_session`, `token_exchange` |
| Confidential client secret verified (Argon2 + tenant pepper) | `token_exchange`, `token_refresh` |
| Refresh tokens rotate; reuse (outside the `REFRESH_REUSE_LEEWAY_SECONDS` window) revokes the family | `token_refresh` |
| Per-tenant ES256 keys; tokens pinned to issuer + `tid` | signing key services, auth middleware |
| `roles`/`groups` re-resolved on issue **and** refresh | `resolve_user_claims` |
| Security headers (CSP, XFO=DENY, HSTS, no-referrer) | `security_headers` middleware |
| Token issue/refresh/reuse audited | `AuditLogger` |
| Logout tokens `typ: logout+jwt`, minted per delivery attempt, ≤ 2 min | `dispatch_pending_events`, `backchannel_logout.rs` |
| A logout from an ended session cannot sign the user out elsewhere | `end_session` (live `sid` required) |

---

## 8. Logout: RP-initiated and back-channel

Logout used to be per app: an app cleared its own cookie and the user stayed signed in
everywhere else. Two OIDC specs now close that gap:

- **RP-Initiated Logout 1.0** — `/oauth/logout` is where an app sends the user to sign
  out (`end_session`).
- **Back-Channel Logout 1.0** — when a session ends, for any reason, each app that holds
  one of the user's sessions and registered a `backchannel_logout_uri` gets a signed
  `logout_token`, server to server, so it can end its own session.

### Sessions and `sid`

This OP has no session cookie of its own: every sign-in to an app is a separate
**refresh-token family** (§4). The family id is the **session id**:

- ID tokens carry it as `sid`, at the code exchange and on every refresh.
- Logout tokens carry the same `sid`, so an app can match a logout to the session it
  opened.
- The `/me/sessions` ids are the same values.

### `GET`/`POST /oauth/logout`

| Parameter | |
|---|---|
| `id_token_hint` | **Required.** An ID token issued to the app. May be expired. |
| `client_id` | Optional; if present it must be the hint's `aud`. |
| `post_logout_redirect_uri` | Optional; must exactly match one of the client's `post_logout_redirect_uris`. |
| `state` | Optional; appended to the redirect. |

1. The hint's `aud` (read unverified) picks the client and its tenant; the hint is then
   verified for real: `typ: JWT`, signed by one of the tenant's published keys, this
   issuer, and an `aud` equal to that client. Expiry is not checked, as the spec asks.
2. The redirect URI is checked **before anything changes**. An error never redirects: it
   answers `400` with an OAuth error body.
3. If the hint's `sid` names a **live** session of this user with this client, **every**
   session of the user in the tenant ends:
   - each other app is notified by back-channel logout; the asking app is not, because
     it ended its own session already;
   - all the user's refresh tokens are revoked;
   - `user.logout` is audited.
4. The response is a `303` to `post_logout_redirect_uri?state=…`, or `200
   {"signed_out": true}` without a redirect. Both are `Cache-Control: no-store`.

**Why a live `sid` is required.** Without it, an old or leaked ID token could be replayed
long after its session ended, to sign the user out of the sessions they still have. A
hint whose session already ended is not an error: the user is signed out of it either
way, and nothing else changes. The same applies to a hint with no `sid` (an ID token
issued before `sid` existed).

**Logging out of one app logs out of all.** That follows from having no OP session: the
user's sign-in state at the OP is the set of all their families.

### Back-channel logout delivery

**Triggers.** Every use case that ends sessions calls `announce_ended_sessions`
(`application/shared/backchannel_logout.rs`). It runs in the use case's own transaction,
**before** the tokens are revoked: afterwards there would be nothing live left to find.

| Trigger | Sessions announced |
|---|---|
| `/oauth/logout` | all, except the asking one |
| Sign out one session (`/me`, admin) | that session |
| Sign out everywhere (`/me`, admin) | all |
| Password change or reset | all |
| Consent revoked | the sessions with that app |
| User erased or deleted | all (before the delete cascades the tokens away) |
| Refresh-token reuse detected | that family, if it was still live |

Only live sessions are announced, so a session is never announced twice. Each announced
session whose client is active and has a `backchannel_logout_uri` becomes one outbox
event (`oidc.backchannel_logout`, payload: client, `sub`, `sid`).

**Delivery reuses the webhook infrastructure** ([`FLOWS-WEBHOOKS.md`](FLOWS-WEBHOOKS.md)):

- the same transactional outbox;
- the same `webhook_deliveries` table, whose rows now target either a subscription or
  an OAuth client;
- the same `SKIP LOCKED` claims, retry schedule (6 attempts over about 7 hours) and HTTP
  client: timeout, no redirects, and the SSRF rules applied to the URI when it is saved.

Only the wire format differs:

```http
POST <backchannel_logout_uri>
Content-Type: application/x-www-form-urlencoded

logout_token=eyJhbGciOiJFUzI1NiIsInR5cCI6ImxvZ291dCtqd3QiLCJraWQiOiIuLi4ifQ...
```

The logout token is its own JWT, not the webhook envelope:

- **Header:** `alg: ES256`, `typ: logout+jwt`, and the tenant key's `kid`.
- **Claims:** `iss`, `aud` (the public client id), `iat`, `exp` (`iat` + 120 s), a fresh
  `jti`, `sub`, `sid`, and `events: {"http://schemas.openid.net/event/backchannel-logout": {}}`.
  There is never a `nonce`.

The token is **minted per attempt**: a retry hours later carries a new, unexpired token
with a new `jti`. Any 2xx is success; the spec asks for `200` but tells OPs to accept
`204`.

A delivery is **abandoned** (`dead`, no further retry) if its client has since been
deactivated or has removed its URI. Deleting the client deletes its deliveries.

**No HMAC.** The request is authenticated by the logout token's signature, which the RP
verifies against `/.well-known/jwks.json` as the spec prescribes. There is also no shared
secret to sign with: client secrets are stored hashed, and public clients have none.

**What the RP must do** (Back-Channel Logout 1.0 §2.6):

1. Verify the JWT: signature via JWKS, `iss`, `aud` equal to its client id, `typ` equal to
   `logout+jwt`, `iat`/`exp`.
2. Require the `events` member and the absence of `nonce`.
3. Optionally reject a replayed `jti`.
4. End the session that has this `sid`, or the user's sessions for `sub`.
5. Answer `200`.

`backchannel_logout_session_required` is stored and returned, but has no effect: `sid` is
always sent.

**Limits.**

- An access token already issued stays valid until it expires (at most 15 min). Back-channel
  logout is what lets an app end its own session sooner.
- Logout deliveries are not listed in the webhook delivery log, which shows subscriptions
  only. They can be inspected in `webhook_deliveries` (`client_id IS NOT NULL`).
- Deactivating a user does not end their sessions, so it sends no logout either; deleting
  or erasing the user does.
