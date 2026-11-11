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
`magicLinkLoginEnabled` tells it whether to offer "Email me a sign-in link" (see
[Magic link login](#magic-link-login-a-one-time-link-by-email)).

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

### Magic link login (a one-time link by email)

On clients where an admin turns it on (`oauth_clients.magic_link_enabled`, off by
default), any active account with a verified email can ask for a one-time sign-in link
instead of typing a password. It is the same flow as a password login from the moment
the link is redeemed: same session, same MFA gate, same `authorize`.

```
  Asking tab (sign-in page)                Provider                       Mailbox
     │ 1. POST /v1/auth/magic-link { email, …frozen request }
     │                                       │  request_magic_link (SAME work for every email)
     │                                       │  · client offers links? request valid? IP gate
     │                                       │  · AuthSession primary_factor = magic_link,
     │   ← 202 { sessionCode, expiresAt }    │    NO user bound
     │◀──────────────────────────────────────│  · spawn the dispatch, return
     │ keep sessionCode in localStorage      │
     │ ("check your email" page)             │  MagicLinkDispatcher (background)
     │                                       │  · eligible account? cooldown?
     │                                       │  · magic_link_token (hash) → session
     │                                       │──────── link …/magic-link#id=…&token=… ──▶
     │                                                                              │
  Link tab (same browser)                                                           │
     │ 2. reads #id, #token + sessionCode from localStorage ◀──────────────────────┘
     │ 3. POST /v1/auth/magic-link/redeem { id, token, sessionCode }
     │                                       │  redeem_magic_link
     │                                       │  · token hash (ct-eq), unexpired, unused
     │                                       │  · sessionCode opens THE session of this link
     │                                       │  · account still active + verified; IP gate
     │                                       │  · consume, risk-score, audit the LOGIN
     │   ← login-shaped response, new code   │  · authenticate, rotate code, MFA gate
     │◀──────────────────────────────────────│
     │ 4. BroadcastChannel → asking tab: MFA page or authorize → redirect to the RP
```

- **Constant time (S6).** The request takes only an email, so it must look the same
  whether that email has an account. `request_magic_link` only touches public inputs
  (the client, the authorization request, the caller's IP), opens a session that binds
  nobody, and answers `202`. Everything that depends on the account — the lookup, the
  token insert, the audit row, the SMTP round trip — runs in `MagicLinkDispatcher`
  after the response, through the `TaskSpawner` port (`TokioTaskSpawner` in production).
  Unknown, inactive and unverified accounts simply get no email.
- **Same browser only (the "racing" defence).** Redeeming needs both the link's token
  (the mailbox) and the code of the session that asked for it, which the hosted login
  keeps in this browser's `localStorage`. A forwarded link, a corporate scanner that
  opens links before the user, or a mail app's built-in browser has the token but not
  the code: it is refused (`magic_link_wrong_browser` in the risk log), and **the token
  is not consumed**, so it still works where it belongs. The page the link opens makes
  no API call at all when it finds no binding.
- **The secret is in the fragment** (`#id=…&token=…`): fragments never leave the
  browser, so the token stays out of server, proxy and `Referer` logs; the page also
  removes it from the address bar and sets `referrer: no-referrer`.
- **Single use, short-lived, hashed.** `magic_link_tokens` stores SHA-256 of a 256-bit
  token, looked up by row id and compared in constant time; 15 minutes
  (`MAGIC_LINK_TTL_MINUTES`), the same as the waiting session. Deleting the session
  (the retention purge) cascades to its links.
- **The asking tab finishes.** The relying party keeps its PKCE verifier and `state` in
  the asking tab's `sessionStorage`, so the link tab hands the redeem response back over
  a `BroadcastChannel`; the asking tab acknowledges and continues exactly as after a
  password (`continueAfterLogin`). With no answer within 2 s (asking tab closed), the
  link tab finishes the sign-in itself.
- **MFA still applies, but never by email.** The mailbox is the primary factor, so an
  email code would prove it twice: `AuthSession::accepts_mfa_method` refuses `email` for
  a `magic_link` session (`send-mfa`/`verify-mfa/email` refuse it; `is_mfa_pending`
  enforces it again). TOTP and passkeys qualify, for `client.mfa_required` and for a
  risk `ALLOW_WITH_MFA`. A client that requires MFA but allows only email cannot
  complete a magic-link sign-in, so it does not offer one
  (`OAuthClient::allows_magic_link_login`).
- **Forced password change still applies** — a change code is issued exactly as on
  login.
- **No per-user invalidation.** Anyone who knows an email can request a link, so a new
  request never cancels a link already sent; each link only works with its own session.
  Mailbox flooding is bounded by a per-account cooldown (60 s,
  `MAGIC_LINK_RESEND_COOLDOWN_SECONDS`) on top of the per-IP credential rate limit.
  Only the newest request's binding is kept per browser, so only the newest link works
  there.
- **Audit and risk.** `user.login.magic_link_sent` when a link is mailed;
  `user.login.success` / `user.login.failure` with `metadata.method = "magic_link"` at
  redemption, in the same transaction. Every refused redemption is recorded by the risk
  engine (`invalid_magic_link`, `magic_link_wrong_browser`, `account_unavailable`,
  `ip_blocked`); a `BLOCK_TEMPORARILY` burns the link and the session.
- **Known trade-offs.**
  - Mail apps that open links in their own built-in browser fail the same-browser
    check; the user must open the link in the browser where they asked for it.
  - A background send is not durable: a restart between the `202` and the send loses
    that email (the user presses "Send a new link").
  - `forgot-password` and `confirm-email/resend` still send synchronously, so their
    response time still differs between existing and unknown emails. `TaskSpawner` is
    the tool to fix them.
  - Requests for unknown emails leave sessions that bind nobody; they are cheap, purged
    with the others and rate-limited per IP.
- **Rolling deploys** — upgrade every replica before enabling the flag: an older binary
  reads `primary_factor = 'magic_link'` as `passkey` (it fails closed), so such a
  sign-in cannot complete there.

### QR code sign-in (approve from another device)

On clients where an admin turns it on (`oauth_clients.qr_login_enabled`, off by
default; never on an external-login-only client), the hosted sign-in page offers
"Sign in with a QR code", like a game launcher's "scan with the mobile app". The browser
that wants to sign in shows a code; another device scans it, shows the user which
browser is asking, and approves or denies. From the approval on it is the same flow as
a password login: same session, same risk scoring, same MFA gate, same `authorize`.

Two kinds of device can approve:

- **The hosted approval page.** A phone camera opens the code's URL,
  `{public origin}/auth/{client_id}/qr-approve#{requestId}.{token}`. The user signs in
  there with this client's own methods (password, passkey; magic links and providers
  are hidden because they leave the page), and that completed sign-in is the approval.
- **An app the user is already signed in to** — the "scan with the app" case. With an
  access token carrying the `account` scope, it calls `POST /me/qr-login/scan` with the
  two values from the URL fragment, shows what is asking, then `POST
  /me/qr-login/approve` with the claim code the scan returned **and the account's
  current password** (`/deny` needs only the claim code). An account without a password
  approves through the hosted page instead.

```
  Browser (sign-in page)                Provider                    Phone / app
     │ 1. POST /v1/auth/qr-login { …frozen request }
     │                                    │ start: client offers it? request valid? IP gate
     │                                    │ · AuthSession primary_factor = qr_code, NO user
     │  ← 201 { url, sessionCode, … }     │ · qr_login_requests row (token hash), 2 min
     │◀───────────────────────────────────│
     │ draws the QR code; polls every 2 s │
     │ 2. POST /v1/auth/qr-login/status { requestId, sessionCode } → pending
     │                                    │                            │ 3. camera opens the URL
     │                                    │  POST …/qr-login/scan      │    (or the app scans)
     │                                    │◀── { requestId, token } ───│
     │                                    │ · first scan claims it: pending → scanned,
     │                                    │   claim code (hash), 10 min to decide
     │                                    │── { claimCode, app, device, location } ──▶│
     │ … → scanned                        │                            │ 4. user checks it,
     │                                    │                            │    signs in (hosted page)
     │                                    │  POST …/qr-login/approve   │
     │                                    │◀── { claimCode, sessionCode | token } ───│
     │                                    │ · scanned → approved (user, via, mfa passed?)
     │                                    │ · the phone's session is burned (proof only)
     │ 5. status → approved               │
     │                                    │ · approved → completed (exactly once)
     │                                    │ · bind user, risk-score THIS browser, rotate the
     │  ← { login-shaped response, email }│   code, MFA gate, forced change, audit the LOGIN
     │◀───────────────────────────────────│
     │ 6. "Continue as <email>?" — the user confirms it is their account
     │ 7. MFA page or authorize → redirect to the RP (continueAfterLogin)
```

- **It can only ever sign in the browser that asked.** The browser's half is its login
  session: only that page holds the session code it polls with, and the completed
  sign-in spends the authorization request frozen in it, with the RP's own PKCE
  verifier. The approving device never gets a code; on the hosted page its own sign-in
  (with a random, discarded PKCE challenge) is burned when it approves.
- **Two secrets, both hashed.** The QR token (256 bits, in the code) only lets the first
  device that presents it claim the request: `pending → scanned` is one conditional
  `UPDATE`, so a second scan of the same code — someone photographing the screen — is
  refused. The claim code (256 bits) goes to that device only, and deciding needs it. A
  claim made by an app also names the app's user: only that user can decide. Every
  status change is a single conditional statement (`QrLoginRequestRepository`), so two
  devices, or a deny racing an approve, cannot both win, and the browser collects an
  approval once.
- **Short-lived.** A code can be scanned for 2 minutes; the browser shows a new one by
  itself while the page is in view (up to five in a row). After a scan the user has 10
  minutes to decide (time to sign in on the phone), never past the browser's session,
  which lives for the scan, the decision and one more minute.
- **The secret is in the fragment**, so it stays out of server, proxy and `Referer`
  logs; the approval page removes it from the address bar and sets
  `referrer: no-referrer`.
- **What the approver is shown.** The app's name, the asking browser (parsed from its
  User-Agent, e.g. "Chrome · Windows"), its approximate location (GeoIP, when
  configured) and when it asked. Not its IP address: the scanner may not be the person
  who asked. Audit rows keep both sides' request context.
- **A stolen token is not enough.** An app approves with its access token AND the
  account's current password: a bearer token says nothing about who holds it, and
  without the password a stolen 15-minute token could be turned into a full sign-in
  (with its own refresh token) on the thief's browser. A wrong password denies the
  request, so every guess costs a new code, which the strict rate limit bounds; a
  password is only checked against a code this user really claimed.
- **Second factor.** QR approval is the primary factor (`primary_factor = qr_code`; any
  second factor can follow it). If the approving sign-in on the hosted page passed a
  second factor, the client's MFA policy is satisfied by it and the browser is not asked
  for another one (`approver_mfa_verified`). A risk-forced step-up is different: the
  risk engine judged the browser itself (a new device, another country — what a relayed
  code looks like), so the browser is asked for a second factor whatever the phone
  passed. An app's approval never counts as a second factor (a token and a password are
  not one): the client's MFA policy applies to the browser as after a password.
- **The browser confirms the account.** Whoever approves decides which account the
  browser signs in to, so someone who photographs the code could approve it with their
  OWN account and sign the user's browser in as them (to collect what the user then
  types). The browser therefore shows the approved email and continues only when the
  user confirms; "Not my account" drops that sign-in and shows a new code.
- **Risk is scored for the browser.** The completing poll evaluates the risk engine with
  the browser's own IP and User-Agent; `BLOCK_TEMPORARILY` refuses the sign-in and
  burns its session. Asking for a code goes through the same pre-authentication IP
  gate as a password login.
- **Audit.** `user.login.qr_approved` / `user.login.qr_denied` when the device decides
  (`metadata.via` = `hosted_page` | `signed_in_app`, plus the requester's location and
  IP), then `user.login.success` / `user.login.failure` with `metadata.method =
  "qr_code"` when the browser completes, in the same transaction.
- **Rate limits.** Asking for a code, scanning and deciding are in the strict credential
  bucket. The browser's polling is on the global one: it authenticates with its session
  code, and a poll every two seconds would exhaust the strict bucket.
- **Limits — the phishing model.** Like every "scan to sign in" scheme, it can be relayed:
  a phishing site can show a victim the QR code of the attacker's own browser, and a
  victim who approves it signs the attacker in. What stands in the way is what the
  approver is shown (which app, which browser, where — the location is what gives a
  relay away, when GeoIP is configured), the explicit warning on the approval screen,
  the 2-minute code, and risk scoring of the completing browser, whose step-up asks the
  attacker for a second factor the phone's approval does not answer for. It is not
  phishing-resistant the way a passkey is: browsers already offer a QR code for
  passkeys stored on a phone ("use a phone or tablet", the WebAuthn hybrid transport),
  and that flow checks over Bluetooth that the phone is physically near the computer.
  For high-assurance tenants prefer passkeys, or keep this off.
- **Rolling deploys** — upgrade every replica before enabling the flag: an older binary
  reads `primary_factor = 'qr_code'` as `passkey` (it fails closed), so such a sign-in
  cannot complete there.

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

Platform admins belong to an organization (not a tenant) and administer every tenant
of it. They sign in two ways, both through one `AdminAuthenticator`
(`application/services/admin_authenticator.rs`), so the rules below hold for both:

| Endpoint | Use case | Purpose |
|---|---|---|
| `POST /session` (+ `GET`, `DELETE`) | `admin_web_session::{start,resolve,end}` | **The console.** Opens an httpOnly cookie session; `/api/*` turns it into a per-request admin token. See [`SERVING-THE-DASHBOARD-FROM-THE-SERVER.md`](SERVING-THE-DASHBOARD-FROM-THE-SERVER.md) |
| `POST /admin/login` | `admin_login` | **API clients.** Access token + refresh token |
| `POST /admin/refresh` | `admin_refresh` | Rotate the refresh token (compare-and-set: of two concurrent refreshes with one token, one wins) → new access token |
| `POST /admin/logout` | `admin_logout` | Revoke the refresh-token family |

Access tokens are **HS256** (15-minute TTL, `typ: "admin"`, `kid` header) signed with
`ADMIN_TOKEN_SIGNING_KEY` (`AdminTokenService`); the previous key in
`ADMIN_TOKEN_PREVIOUS_SIGNING_KEY` keeps verifying for one token lifetime, so the key
rotates without signing anyone out.

### Sign-in rules

- **Generic failures.** Unknown email, wrong password, deactivated account, suspended
  organization and locked account all answer `401 {"error":"invalid credentials"}`, and
  each costs one Argon2 verification (a dummy one when there is no usable account), so
  neither the answer nor its timing reveals which admin emails exist or their state.
- **Emails are case-insensitive.** Stored trimmed and lowercased (a database CHECK holds
  it); lookups normalize the same way.
- **Per-account lock.** 5 consecutive failures (wrong password or wrong code, at sign-in
  or at a step-up) lock the account for 15 minutes, then the count restarts; a successful
  sign-in clears it. This is what stops a password spray spread over many IPs, which the
  per-IP rate limit (the strict credential bucket) cannot see. Audited as
  `admin.account.locked`.
- **Suspended organization.** `organizations.is_active = false` stops its admins at
  sign-in, refresh and on the next console request. Only the system can change it; the
  console's own admins cannot (`PUT /admin/me/organization` refuses it).
- **Second factor.** An admin with two-step verification on who sends the right password
  without a code gets `401 {"error":"mfa_required"}` (audited `admin.login.mfa_required`)
  and repeats the request with `mfaCode` (console) / `mfa_code` (API): a 6-digit TOTP code
  or a recovery code. A wrong code is `401 {"error":"invalid_mfa_code"}` and counts toward
  the lock. `mfa_required` is only ever returned for a right password. Nothing is stored
  between the two requests.

### Two-step verification (`/admin/me/mfa*`)

| Endpoint | Needs | Effect |
|---|---|---|
| `GET /admin/me/mfa` | — | `{totpEnabled, enabledAt, recoveryCodesRemaining}` |
| `POST /admin/me/mfa/totp` | `current_password` | New seed (sealed with `SecretPurpose::AdminTotpSeed`, bound to the admin id), pending until confirmed; returns the seed and `otpauth://` URL once. Refused while a factor is on |
| `POST /admin/me/mfa/totp/confirm` | `code` | Turns it on; returns 10 recovery codes once (`XXXXX-XXXXX`, 50 bits each, stored as SHA-256) |
| `POST /admin/me/mfa/recovery-codes` | `current_password` | Replaces every recovery code |
| `DELETE /admin/me/mfa` | `current_password` + `code` | Turns it off (a never-confirmed enrollment needs only the password) |

TOTP is RFC 6238 (SHA-1, 30 s, 6 digits, ±1 step). The last accepted step is stored and
the write is conditional, so a code cannot be replayed, not even by two concurrent
requests; a recovery code is removed from the array in the same conditional statement.
Responses carrying a seed or codes are `Cache-Control: no-store`. An unreadable seed
(lost encryption key) still lets the admin in with a recovery code;
`repair_encrypted_secrets --apply` deletes the factor so they can enroll again.

### The admin's own account (`/admin/me*`)

- **Password change** needs the current password and a new one that meets the admin
  policy (12–128 characters, not the email, not the current one;
  `application/shared/admin_password_policy.rs`). It then **ends every other session**:
  every refresh-token family and every console session except the one making the request.
- **Email change** needs `current_password` (it is the sign-in identifier); a name-only
  change does not.
- **Deactivation** (`DELETE /admin/me`) needs `current_password`, ends every session, and
  is refused for the last active admin of the organization. The check runs under a lock on
  the organization row, so two admins deactivating themselves at the same moment cannot
  leave it with none. Another admin of the organization can reactivate it (below).

### Requiring two-step verification (`PUT /admin/organization/mfa-policy`)

`organizations.require_admin_mfa` makes the second factor mandatory for every admin of
the organization. Turning it on needs `current_password` and two-step verification on the
acting admin's own account; both directions are audited
(`organization.admin_mfa_policy.updated`).

An admin without a second factor in such an organization still signs in, but every token
minted for them (`/admin/login`, `/admin/refresh`, each console request) carries
`mfa_setup: true`, and `require_admin` then answers everything except
`GET /admin/me/profile`, `GET /admin/me/organization`, `GET /admin/me/mfa`,
`POST /admin/me/mfa/totp` and `POST /admin/me/mfa/totp/confirm` with
`403 {"error":"mfa_setup_required"}`. The flag is recomputed on every console request,
so turning the policy on restricts open sessions at once and enrolling lifts the
restriction at once (a bearer token keeps its flag until it expires, ≤ 15 min).
`GET /session` reports it as `mfaSetupRequired`; the console then shows only
`/login/two-step`. The check costs a query only in organizations that require it.

### Managing the organization's admins (`/admin/organization/*`)

All admins of an organization are equal (there are no admin roles). Each action is
authorized as the caller's own organization (`Actor::authorize_organization`), needs the
acting admin's `current_password` where it changes anything about an account, and is
audited with the colleague as the resource.

| Endpoint | Effect |
|---|---|
| `GET /admin/organization/admins` | Every admin: status, two-step on/off, locked, last sign-in |
| `POST /admin/organization/admins/{id}/deactivate` | Deactivates a colleague and ends all their sessions; refused for yourself (use `DELETE /admin/me`) and for the last active admin |
| `POST /admin/organization/admins/{id}/reactivate` | Reactivates a colleague (also clears any sign-in lock) |
| `DELETE /admin/organization/admins/{id}/mfa` | Recovery for a colleague who lost their authenticator and recovery codes: removes the factor and ends their sessions |
| `GET /admin/organization/invitations` | Open invitations |
| `POST /admin/organization/invitations` | `{email, current_password}` → a one-time link `<origin>/login/invite#<token>`, shown once (256-bit token, only its SHA-256 stored, 72 h). Re-inviting an email replaces its open link; an email that is already an admin is a 409 |
| `DELETE /admin/organization/invitations/{id}` | Withdraws an invitation |

The link is handed over out of band (there is no platform-level email sender). Its token
is in the URL fragment, which browsers never send to a server, so it stays out of access
logs; the invitation page removes it from the address bar on load. The invitee then uses
two public endpoints in the strict credential rate-limit bucket:
`POST /admin/invitations/lookup {token}` (which email and organization; any unusable link
is the same 404) and `POST /admin/invitations/accept {token, name, password}`. Accepting
claims the invitation and creates the admin in one transaction, so a link creates at most
one admin; the password policy applies, and the new admin signs in normally.

### Bootstrapping an admin

```bash
cargo run --bin create_admin -- <email> [organization_id]          # prompts for the password twice
printf '%s\n' "$PASSWORD" | cargo run --bin create_admin -- <email>  # or reads it from stdin
```

The password is never taken from the command line (shell history, `ps`); the old
`<email> <password> [organization_id]` form still works with a warning. It must meet the
admin password policy. The hash uses the same `ADMIN_PASSWORD_PEPPER` (legacy:
`JWT_SECRET`) the running API verifies against, and the admin is attached to the given
organization (or a find-or-created "Default Organization").

Expired admin refresh tokens, console sessions and invitations are removed by the
retention purge.

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
| Auth session (`session_code`) | 10 min (15 min while waiting for a magic link, 13 for a QR approval) | hash only |
| Magic sign-in link | 15 min | hash only, single-use, same browser only |
| QR sign-in code / claim code | 2 min to scan / 10 min to decide | hash only, first scan claims it |
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
