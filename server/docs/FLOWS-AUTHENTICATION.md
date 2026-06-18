# Authentication Flow

How end users and platform admins authenticate. This covers the **first-party login
SPA flow**, **MFA**, **registration + email confirmation**, **password reset /
forced change**, and **platform-admin authentication**. For the raw OAuth/OIDC
protocol endpoints see [`FLOWS-OAUTH2-OIDC.md`](FLOWS-OAUTH2-OIDC.md); for the
layered design see [`ARCHITECTURE.md`](ARCHITECTURE.md).

There are **two distinct identities** in this system:

- **End users** (`users`) — sign in through a tenant's hosted login SPA and receive
  tenant-issued **ES256** OAuth/OIDC tokens.
- **Platform admins** (`admin_users`) — operate the dashboard and receive server-wide
  **HS256** admin tokens. They belong to an `organization`, not a tenant.

---

## 1. First-party login flow (`/v1/auth/*`)

The `frontend/` login SPA is a first-party app, so instead of the generic
`/oauth/authorize` + consent screen it uses a **session-based** Authorization Code +
PKCE flow: authenticate first (`login`), then spend the session for a code
(`authorize_session`). Protocol parameters are **frozen in the session** between the
two calls so they can't be tampered with.

```
  Login SPA                                  Provider
     │ 1. POST /v1/auth/login                    │  authentication::login
     │   { client_id, email, password,           │  · resolve client → tenant
     │     redirect_uri, scope, state,           │  · validate PKCE/redirect/scope
     │     code_challenge(S256), nonce }         │  · verify password (Argon2 + pepper)
     │   → { session_code, mfa_type?, ... }      │  · require email verified
     │◀──────────────────────────────────────────│  · open authenticated AuthSession
     │                                            │
     │ 2. (if MFA required) POST /v1/auth/verify-mfa/<method>   │  mfa::verify_*
     │   → second factor proven, session unblocked│
     │                                            │
     │ 3. POST /v1/auth/authorize                 │  authentication::authorize_session
     │   { client_id, email, session_code }       │  · load frozen request from session
     │   → { authorization_code, state,           │  · implicit consent, mint code,
     │       redirect_uri, code_challenge, ... }  │    burn the session
     │◀──────────────────────────────────────────│
     │ 4. POST /oauth/token (code + code_verifier) → access/id/refresh tokens
```

### Step 1 — `POST /v1/auth/login` (`login`)
1. Resolve the client by its globally-unique public `client_id` (`..._unscoped`) and
   derive the tenant from it — the SPA only knows the `client_id`.
2. Validate the authorization request **before** spending a password check (so a
   malformed request can't be used as an auth oracle): exact `redirect_uri`,
   `response_type == code`, PKCE `S256` present, scopes non-empty after intersection.
3. **Authenticate** from the user's own credentials (Argon2id + tenant pepper, on
   `spawn_blocking`). The user id is derived here; failures are generic.
4. Reject if the email isn't verified (`EmailNotConfirmed`) — the SPA routes to the
   confirm-email screen.
5. Open an **`AuthSession`** that *freezes* the request (redirect_uri, scopes, state,
   code_challenge, nonce) and stores only the **hash** of an opaque session token.
   The raw token is returned as `session_code`.
6. Evaluate the client's **MFA policy** (see §2). If a second factor is required, the
   session is marked MFA-pending and the response carries what the SPA needs to
   prompt (`mfa_type`, `mfa_id`, WebAuthn options, `available_mfa_methods`, or
   `mfa_enrollment_required` + `enrollable_methods`).
7. If the credentials are flagged `should_change_password` (e.g. an admin-issued
   temporary password), mint a one-time `change_password_code` the SPA must redeem at
   `/v1/auth/change-password` first.

### Step 2 — MFA verification (only if required)
The SPA calls the matching `/v1/auth/verify-mfa/*` endpoint (§2). On success the
second factor is recorded against the session, clearing its MFA-pending state.

### Step 3 — `POST /v1/auth/authorize` (`authorize_session`)
Spends the authenticated session for a single-use authorization code, reading the
protocol parameters **back from the frozen session** (not the request body):
1. Resolve client → tenant; reject inactive clients.
2. Find the session by the **hash** of the presented `session_code`; reject if
   expired, not authenticated, **still MFA-pending**, or opened for a different
   client (all failures generic).
3. Load the bound user; the body's `email` is only a consistency cross-check against
   the session's user id. A pending forced password change blocks here.
4. Record consent implicitly (submitting credentials on our own login page **is** the
   approval) — idempotent union of scopes.
5. Mint the single-use **authorization code** (TTL 60s, stored hashed) from the frozen
   request.
6. **Burn the session** so the `session_code` can't be replayed.

Returns the `authorization_code` + the frozen `redirect_uri`/`state`/`code_challenge`
so the SPA can complete `POST /oauth/token` (see
[`FLOWS-OAUTH2-OIDC.md`](FLOWS-OAUTH2-OIDC.md) §3, step 5).

### Supporting endpoint
`GET /v1/auth/client/{client_id}/auth-data` returns the public auth-flow config the
login SPA needs to render its screens (client name, allowed MFA methods, etc.).

---

## 2. Multi-factor authentication (MFA)

Methods (`domain/entities/mfa_method.rs`): **`email`** (email OTP), **`totp`**
(authenticator apps), **`webauthn`** (passkeys / security keys). MFA is governed
**per client** by `MfaPolicy { required, email_enabled, totp_enabled,
webauthn_enabled }`.

### Method selection at login
When a client requires MFA, `login` computes the methods actually available to the
user (email is always available if enabled; TOTP/WebAuthn require a **confirmed**
enrollment), then selects one by priority: **user's preferred method → client's
hinted default → client's hinted order → fallback (WebAuthn → TOTP → Email)**. If the
client requires MFA but the user has no usable enrolled factor,
`mfa_enrollment_required` is returned with the `enrollable_methods`.

### Verification endpoints (login-time, public)
| Method | Endpoint(s) |
|---|---|
| Email OTP | `POST /v1/auth/verify-mfa/email/send`, `POST /v1/auth/verify-mfa/email` |
| TOTP | `POST /v1/auth/verify-mfa/app` |
| WebAuthn | `POST /v1/auth/verify-mfa/webauthn/start`, `POST /v1/auth/verify-mfa/webauthn` |

Each opens/consumes an **`MfaChallenge`** bound to the user + session. Email OTP
codes and WebAuthn challenge state are stored hashed/opaque with short TTLs (email
OTP **10 min**, WebAuthn challenge **5 min**). TOTP is verified against the enrolled
secret (decrypted via `SecretCipher` at verify time). On success the session's
second factor is satisfied and `authorize_session` may proceed.

### Self-service enrollment & management (`/me/mfa/*`, behind `require_auth`)
| Endpoint | Action |
|---|---|
| `GET /me/mfa/methods` | List the user's enrolled methods |
| `PUT /me/mfa/preferred-method` | Set preferred default factor |
| `POST /me/mfa/totp` → `POST /me/mfa/totp/confirm` | Start then confirm TOTP enrollment |
| `POST /me/mfa/webauthn/start` → `POST /me/mfa/webauthn/finish` | Register a passkey |
| `PATCH`/`DELETE /me/mfa/methods/{id}` | Rename / remove an enrollment |

TOTP secrets are encrypted at rest (AES-256-GCM); WebAuthn uses `webauthn-rs` with
the relying-party config resolved from `WEBAUTHN_RP_ID` / `WEBAUTHN_RP_ORIGIN` /
`WEBAUTHN_ADDITIONAL_ORIGINS` (see [`ARCHITECTURE.md`](ARCHITECTURE.md) §10).

---

## 3. Registration & email confirmation

| Endpoint | Use case | Purpose |
|---|---|---|
| `POST /authentication/register` | `register` | Create a user + send a verification code |
| `POST /authentication/confirm-email` | `confirm_email` | Verify the code, mark the email confirmed |
| `POST /v1/auth/confirm-email/resend` | `resend_confirm_email` | Re-send the verification code |

Verification codes are stored **hashed** with an attempt counter (`email_verification_tokens`).
Email is delivered through the client/tenant's configured SMTP provider
(`SmtpEmailSender`), falling back to a logging sender when none is configured.
Sign-in (`login`) refuses unverified accounts.

---

## 4. Password reset & forced change

| Endpoint | Use case | Purpose |
|---|---|---|
| `POST /v1/auth/forgot-password` | `forgot_password` | Email a reset link to `AUTH_UI_BASE_URL/auth/{client_id}/change-password` |
| `POST /v1/auth/reset-password` | `reset_password` | Set a new password from the emailed token |
| `POST /v1/auth/change-password` | `change_password` | Complete a forced change (admin-issued temp password) using the `change_password_code` from login |

Reset and change tokens are single-use, stored hashed, and short-lived; issuing a new
one invalidates any earlier live token for that user. A pending forced change blocks
`authorize_session` until completed, so a temporary password can't be used to obtain
tokens.

---

## 5. Platform-admin authentication

Admins are global (organization-scoped, not tenant-scoped) and use **HS256** tokens
signed with the server-wide `JWT_SECRET` (`AdminTokenService`). These endpoints live
**outside** the admin guard (they're how an admin *gets* a token):

| Endpoint | Use case | Purpose |
|---|---|---|
| `POST /admin/login` | `admin_login` | Verify admin password (Argon2 + `JWT_SECRET` pepper) → issue access token + refresh token |
| `POST /admin/refresh` | `admin_refresh` | Rotate the refresh token → new access token |
| `POST /admin/logout` | `admin_logout` | Revoke the refresh-token family (server-side sign-out) |

Bootstrap the first admin from the CLI:

```bash
cargo run --bin create_admin -- <email> <password> [organization_id]
```

It hashes with the same `JWT_SECRET` pepper the running API verifies against, and
attaches the admin to the given organization (or a find-or-created "Default
Organization").

---

## 6. How the API authenticates a request

`presentation/middleware/auth.rs` runs on protected routes:

- **`require_auth`** — any valid token (used by `/me/*`).
- **`require_admin`** — valid token **and** an admin role (`admin` / `tenant_admin`);
  records `admin.access_denied` audit events on failure.

Token verification picks the right path automatically:

1. **HS256** header ⇒ a **platform-admin** token, verified with `JWT_SECRET`. Yields
   a principal with `is_platform_admin = true`, no tenant, authorized across all
   tenants.
2. Otherwise it's a **tenant ES256** token: the unverified `tid` claim selects which
   tenant's **published** keys to verify against, then the signature is verified for
   real. The principal is pinned to that tenant.

The resulting **`AuthPrincipal`** (`user_id`, `tenant_id`, `roles`, `groups`,
`scopes`, `is_platform_admin`) is placed in request extensions. Handlers pull it via
the `AuthPrincipal` extractor and enforce tenant isolation with
`principal.can_access_tenant(tenant_id)` — a platform admin passes for any tenant; a
tenant-scoped caller only for its own. A `RequestContext` (client IP, User-Agent) is
also captured for audit logging.

> `REQUIRE_ADMIN_AUTH=false` turns `require_admin` into best-effort mode (parses a
> token for audit attribution but does not block) — a dashboard-rollout aid only.
> Keep it `true` in any shared environment.

---

## 7. Token & secret lifetimes (summary)

| Credential | TTL | Storage |
|---|---|---|
| Auth session (`session_code`) | 10 min | hash only |
| Email OTP (MFA) | 10 min | hash only |
| WebAuthn challenge | 5 min | opaque state |
| Authorization code | 60 s | hash only, single-use |
| Access token (ES256) | 15 min | stateless JWT |
| Refresh token | 30 days | hash only, rotating + reuse detection |
| Email verification / reset / change tokens | short-lived | hash only, single-use |

All passwords use Argon2id with a per-tenant pepper (admin passwords: `JWT_SECRET`
pepper); all opaque credentials are returned once and persisted only as SHA-256
hashes.
