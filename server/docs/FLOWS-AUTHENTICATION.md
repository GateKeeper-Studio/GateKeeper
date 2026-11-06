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

The `frontends/sveltekit-bff/` login SPA is a first-party app, so instead of the generic
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
   `password` is optional: when it is absent or empty the request is a
   **passwordless login** and continues as described in
   [Passwordless login](#passwordless-login-a-passkey-as-the-primary-factor) below.
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
`passkeyLoginEnabled` tells the hosted page whether to offer "Sign in with a passkey"
(the client takes email sign-in and allows WebAuthn).

### Passwordless login (a passkey as the primary factor)

An account with **no password** (no `user_credentials` row — today, accounts
provisioned by social sign-in) that has a **confirmed passkey** can sign in with the
passkey alone. This is not a separate flow: `login` treats it as "no password factor
configured, require WebAuthn", and every later step is the existing one.

```
  Login SPA                                     Provider
     │ 1. POST /v1/auth/login { email, (no password), …frozen request }
     │                                            │  · same client/request/pre-auth checks
     │                                            │  · eligible? → AuthSession with
     │                                            │    primary_factor = passkey, locked
     │   → { session_code, mfa_type: "webauthn",  │    to WebAuthn + Login challenge
     │       mfa_id, web_authn_options }          │  · not eligible → generic 401
     │◀──────────────────────────────────────────│
     │ 2. navigator.credentials.get(...)          │
     │ 3. POST /v1/auth/verify-mfa/webauthn       │  mfa::verify_webauthn
     │   { session_code, mfa_id, assertion }      │  · account still eligible? pre-auth gate
     │                                            │  · verify assertion (UV required)
     │                                            │  · risk-score + audit the LOGIN here
     │◀──────────────────────────────────────────│  · session authenticated + unlocked
     │ 4. POST /v1/auth/authorize (unchanged)     │  authentication::authorize_session
```

- **Eligibility** — the account exists, is active, has a verified email, has **no
  password** and at least one confirmed WebAuthn enrollment, and the client allows
  WebAuthn (and password login: `external_login_only` clients refuse `login`
  outright). An account **with** a password always signs in with it.
- **Every ineligible case answers like a wrong password**: the same generic
  `AuthenticationError`, and the attempt is recorded by the risk engine
  (`failure_reason = passkey_unavailable`) so it feeds the velocity / spraying rules.
  Never `EmailNotConfirmed` — no proof was offered, so that would be an oracle. No
  Argon2 runs on this path for anyone.
- **The lock** — the session is opened with `AuthSession::identify_for_passkey`:
  the user is bound but `authenticated_at` stays empty, `primary_factor = passkey`
  (column `auth_sessions.primary_factor`), and only WebAuthn may clear it.
  `accepts_mfa_method` makes the email-OTP and TOTP verifiers refuse it (a mailbox or a
  TOTP code alone must never become a primary factor), and `is_mfa_pending` enforces
  the same rule again as defense in depth. `authorize_session` needs no change: it
  refuses any session that is still pending.
- **No MFA block, no forced password change** — webauthn-rs requires user verification
  on every assertion (`UserVerificationPolicy::Required`), so the passkey is already
  multi-factor; it satisfies `client.mfa_required` and a risk `ALLOW_WITH_MFA`.
- **Where the login happens** — `verify_webauthn` re-checks that the account is still
  active and still has no password (a reset mid-flow does not kill sessions), runs the
  pre-auth IP gate, verifies the assertion, and only then records the `login_event`
  (`credentials_valid` = the assertion verified) and the `user.login.success` /
  `user.login.failure` audit event (`metadata.method = "passkey"`). A risk
  `BLOCK_TEMPORARILY` burns the challenge and the session. `POST /v1/auth/login` does
  **not** audit a success for a passkey challenge — nothing was proven yet — and omits
  `userId` from its response.
- **Ceremonies are per session** — opening a passkey session needs no proof, so neither
  `login` nor `verify-mfa/webauthn/start` invalidates the user's other live WebAuthn
  challenges (that would let anyone who knows an email cancel the real user's
  sign-in); `start` replaces only its own session's challenge.
- **Failed assertions count** — every WebAuthn verify failure (second factor or
  primary) increments the challenge's attempt counter; it dies after
  `MfaChallenge::MAX_ATTEMPTS`, and the hosted page then fetches a fresh one.
- **Recovery** — a user who loses their passkey device uses **forgot password**: the
  reset link proves the mailbox and sets a password (the account then signs in with
  it, back on the password path). This needs `tenant.can_self_forgot_password`. To keep
  that path meaningful, `DELETE /me/mfa/methods/{id}` refuses (409) to remove the
  **last** passkey of an account with no password unless a social identity is linked.
- **Known trade-off: enumeration.** Email-first passkey login cannot hide which emails
  are eligible: they get WebAuthn options (whose `allowCredentials` lists the account's
  credential ids, WebAuthn §14.6.2), every other email gets 401. Mitigated by the
  strict credential rate limit and by recording ineligible attempts in the risk engine
  (IP velocity / spraying ⇒ temporary IP block). Note `register` already reveals
  whether an email exists.
- **Rolling deploys** — upgrade every server replica before exposing the hosted
  page's passkey button: an older replica's email/TOTP verifier does not know about
  `primary_factor`.
- **Open follow-up** — step-up for passwordless accounts: `confirm_current_password`
  passes for an account with no password, and enrolling a passkey needs only the
  `account`-scoped token. A stolen token could therefore plant a passkey; notify on
  enrollment / require recent authentication before relying on this broadly.

### External-provider-only clients (`external_login_only`)

A client can be configured to authenticate **only** through its external identity
providers (Google/GitHub, configured per client and started at
`/v1/auth/oauth-provider/{provider}/…`). Its
users never pick a password: the social callback already provisions passwordless
accounts (`User::new(..., None, None)` with the email pre-verified by the provider).

When `oauth_clients.external_login_only` is true, **every password-credential use case
refuses the client with `AccessDenied` (403)** — `register`, `login`, `forgot_password`,
`reset_password`, `change_password` — each checking `client.allows_password_login()`
immediately after resolving the client, **before** any tenant lookup or Argon2
verification. A correct password is never even verified.

The public auth-data response reflects this: `external_login_only: true`, and
`can_self_sign_up` / `can_self_forgot_pass` are forced `false` whatever the tenant
allows. The hosted pages render provider buttons only, and `/sign-up` and
`/forgot-password` 302 to `/sign-in` (preserving the OAuth query string).

**Lockout invariant.** Two guards make it impossible to leave a client with no way in:

- `update_oauth_client` refuses to turn the flag on unless the client already has at
  least one **active** external provider (`create_oauth_client` allows it — a new client
  has none yet, and providers are configured afterwards);
- `delete_external_provider` / `update_external_provider` refuse to remove or deactivate
  the **last active** provider while the flag is on.

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
codes are stored hashed; WebAuthn ceremony state is stored server-side and never sent
to the browser. Both have short TTLs (email OTP **10 min**, WebAuthn challenge
**5 min**) and a cap of `MAX_ATTEMPTS` wrong tries. TOTP is verified against the
enrolled secret (decrypted via `SecretCipher` at verify time). On success the session's
second factor is satisfied and `authorize_session` may proceed.

Every verifier gates on `AuthSession::accepts_mfa_method`: a **passkey session**
(passwordless login, §1) accepts **WebAuthn only** — email OTP and TOTP refuse it.

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

Beyond MFA, the same `require_auth` surface exposes **`GET /me/security/login-events`**
— a user's *own* recent sign-in activity (location, device, risk level, decision) drawn
from the login-event history, so they can spot unfamiliar logins. Identity is pinned to
the token's `AuthPrincipal`; a user can never read another user's events.

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
Sign-in (`login`) refuses unverified accounts. Registration is refused outright for an
external-login-only client (§1).

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
tokens. All three endpoints are refused for an external-login-only client (§1) — there
is no password to reset, and a token minted before the flag was turned on is refused
rather than redeemed.

---

## 5. Platform-admin authentication

Admins are global (organization-scoped, not tenant-scoped) and use **HS256** tokens
(15-minute TTL, `typ: "admin"`, `kid` header) signed with `ADMIN_TOKEN_SIGNING_KEY`
(`AdminTokenService`). The key rotates without logging admins out: the previous key in
`ADMIN_TOKEN_PREVIOUS_SIGNING_KEY` keeps verifying for one token lifetime. These endpoints live
**outside** the admin guard (they're how an admin *gets* a token):

| Endpoint | Use case | Purpose |
|---|---|---|
| `POST /admin/login` | `admin_login` | Verify admin password (Argon2 + `ADMIN_PASSWORD_PEPPER`; an unknown admin pays a dummy verification so timing does not reveal it) → issue access token + refresh token |
| `POST /admin/refresh` | `admin_refresh` | Rotate the refresh token → new access token |
| `POST /admin/logout` | `admin_logout` | Revoke the refresh-token family (server-side sign-out) |

Bootstrap the first admin from the CLI:

```bash
cargo run --bin create_admin -- <email> <password> [organization_id]
```

It hashes with the same `ADMIN_PASSWORD_PEPPER` (legacy: `JWT_SECRET`) the running API verifies against, and
attaches the admin to the given organization (or a find-or-created "Default
Organization").

---

## 6. How the API authenticates a request

`presentation/middleware/auth.rs` runs on protected routes:

- **`require_auth`** — a valid tenant **access** token with the **`account`** scope
  (used by `/me/*`). Platform-admin tokens are refused (they use `/admin/me`).
- **`require_admin`** — a platform-admin token, **or** a tenant access token with an
  admin role (`admin` / `tenant_admin`) **and** the **`admin`** scope; records
  `admin.access_denied` audit events on failure. Without the scope checks, any OAuth
  client of a tenant received tokens that opened these APIs.
- Destructive self-service calls (`DELETE /me`, `DELETE /me/mfa/methods/{id}`) also
  need `{"current_password": "..."}` in the body (step-up, `step_up_required` 403).

Token verification picks the right path automatically:

1. **HS256** header ⇒ a **platform-admin** token, verified by `AdminTokenService`
   (key by `kid`, `typ` must be `admin`). Yields a principal with
   `is_platform_admin = true`, no tenant, authorized for the tenants **of its
   organization**.
2. Otherwise it's a **tenant ES256 access token** (`typ: at+jwt`; ID tokens are
   refused): the unverified `tid` claim selects which tenant's **published** keys to
   verify against, then the signature is verified for real. The principal is pinned
   to that tenant.

The resulting **`AuthPrincipal`** (`user_id`, `tenant_id`, `roles`, `groups`,
`scopes`, `is_platform_admin`) is placed in request extensions. Handlers pull it via
the `AuthPrincipal` extractor and enforce tenant isolation with the `TenantScope`
extractor — a platform admin passes for the tenants of its own organization; a
tenant-scoped caller only for its own tenant. A **`RequestContext`** (client IP,
User-Agent) is captured app-wide by the `capture_client_context` middleware. The IP is
the **TCP peer address** unless that peer is listed in `TRUSTED_PROXIES`; only then is
`X-Forwarded-For` believed, read **from the right** (the first address that is not a
trusted proxy), with `X-Real-IP` as a fallback (`middleware/client_ip.rs`). The same
value keys the rate limiter and feeds audit logging and the adaptive risk engine —
geolocation and the new-country / impossible-travel / new-device signals — so a client
cannot pick its own IP.

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

All passwords use Argon2id with a per-tenant pepper (generated by the server, sealed
at rest; admin passwords: `ADMIN_PASSWORD_PEPPER`), run on the blocking pool behind a
concurrency limit (`PasswordHasher` port). OAuth client secrets (256 random bits) are
stored as HMAC-SHA-256 keyed with the tenant pepper. All other opaque credentials are
returned once and persisted only as SHA-256 hashes.
