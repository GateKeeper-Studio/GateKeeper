# Proposed issues — GateKeeper

---

## [Feature] SAML 2.0 Service Provider for enterprise clients

**Context:** today GateKeeper only federates identity via OIDC (`external_identity_provider_client`). Many B2B (enterprise) clients require SAML 2.0 as an SP.

**Proposal:** implement a SAML SP flow in parallel to the existing OIDC one, reusing the `external_identity_provider_client` layer as an abstraction (the `ExternalIdentityProviderClient` port already exists in `application/ports/external_identity_provider_client.rs` — evaluate whether a new implementation fits there or whether a separate `SamlAssertionConsumer` port is warranted).

**Best practices / architecture:**
- Keep assertion signing/validation (XML Signature) isolated in `infraestructure/`, never in the application layer (following the dependency rule already enforced in A5).
- Handle XML with a parser that protects against XXE and "XML Signature Wrapping" (a classic SAML attack) — use a mature library, not a hand-rolled parser.
- Map the result into the same `external_login/callback` use case already used by OIDC, to avoid duplicating account provisioning/linking logic.

**Points of attention:**
- Reuse the existing audit trail and `user.created`/webhook emission, so the contract documented in `FLOWS-WEBHOOKS.md` doesn't break.
- The partner IdP's signing certificates/keys should follow the same versioned key-ring pattern already adopted for `SECRETS_ENCRYPTION_KEY` (S7).

---

## [Feature] SCIM 2.0 for automatic user/group provisioning

**Context:** today users and groups are managed manually via the dashboard or the admin API. Enterprise clients expect automatic provisioning/deprovisioning from their own IdP (Okta, Azure AD, Google Workspace).

**Proposal:** a SCIM 2.0 endpoint (`/scim/v2/Users`, `/scim/v2/Groups`) per tenant, authenticated via a dedicated service token (not the 15-minute admin token used by the dashboard).

**Best practices / architecture:**
- Treat this as a new route family under `presentation/routes/`, following the pattern already established in A3 (one router per protection level: `admin.rs`, `me.rs`, etc.) — create `scim.rs` with its own `AppState` slice via `FromRef`.
- Reuse the existing `group`/`role` use cases (`create_group`, `add_group_members`, etc.) instead of duplicating business rules — SCIM should be a protocol translation layer only.
- Use the `Actor` (A4) to represent the "SCIM system" as an actor, so per-use-case authorization checks keep applying.

**Points of attention:**
- SCIM `PATCH` with partial operations is the part most prone to bugs — map it carefully onto the existing commands rather than creating a second, parallel write path.
- Rate limiting and idempotency: bulk provisioning from an IdP can spike traffic; consider queuing instead of heavy synchronous writes.

---

## [Feature] Organization-level identity federation ("bring your own IdP")

**Context:** today external federation is configured per OAuth *client* (`oauth_client/external_provider`). There's no way for an entire organization to delegate authentication to its own Entra ID/Okta, valid across all of the org's clients.

**Proposal:** a new `organization_identity_provider` entity, resolved before the login screen when the e-mail domain matches an org with federation enabled (home realm discovery).

**Best practices / architecture:**
- Don't couple this to the SAML/SCIM issues above — design a common "external identity provider" abstraction reusable by all three (OIDC, SAML, and this one).
- Follow the layering already in place: domain resolution → business rule in `application/use_cases/`, never in the route handler.

**Points of attention:**
- This adds one more decision branch to the main login flow — needs clear tests (similar to the pattern in `authentication/login/tests.rs`) covering "org has an IdP, user has no account yet" and "org has an IdP, but the client doesn't allow federation".

---

## [Feature] Statistical/ML risk scoring beyond deterministic rules

**Context:** `fraud-specification.md` itself already lists this as "out of scope for the MVP, part of the advanced roadmap." The current risk engine (`default_risk_engine.rs`) is rule-based and explainable.

**Proposal:** add a second `RiskEngine` (the port already exists in `application/ports/risk_engine.rs`) that combines the current rules with a statistical score (e.g. simple logistic regression over features already collected: login velocity, geo, IP reputation).

**Best practices / architecture:**
- Preserve explainability: every decision must keep exposing the signals that produced it (an explicit requirement in the original spec) — a "black box" model with no explanation breaks that requirement.
- Implement it as an alternative adapter behind the existing port, selectable by config/tenant, not as a replacement for the rules engine.

**Points of attention:**
- Any model needs a training dataset and an update process — define this before starting, or the feature goes stale quickly.
- Watch out for bias/false positives from geolocation (e.g. users on a corporate VPN) — the spec already calls for being "safe against excessive false positives."

**Status:** implemented — see [`RISK-SCORING.md`](RISK-SCORING.md). `HybridRiskEngine` behind the existing port (`RISK_SCORING=hybrid`; per tenant `rules`/`shadow`/`hybrid`), logistic regression over identifier-free features, exact per-feature explanations stored and shown, can only add step-up MFA. Dataset = admin labels + weak step-up MFA outcomes; `train_risk_model` CLI trains candidates with quality gates, manual promote/rollback, drift report, 90-day staleness fallback.

---

## [Feature] SIEM integration (security event export)

**Context:** cited as out of scope for the MVP in `fraud-specification.md`. Today security events stay only in the internal database/audit log.

**Proposal:** a new `SecurityEventExporter` port (new, under `application/ports/`), with Syslog/HTTP adapters (Splunk HEC, Datadog) implemented in `infraestructure/`.

**Best practices / architecture:**
- Reuse the transactional-outbox pattern already used for webhooks (`FLOWS-WEBHOOKS.md`) — security events should be as consistent with the transaction that produced them, for the same reason that motivated the webhook outbox.
- Don't mix this with the user webhook pipeline: different contracts and different consumers (security team vs. partner applications).

**Points of attention:**
- Volume: security events can be far more numerous than user events — consider batching/buffering before dispatch.

---

## [Feature] Passwordless login with a passkey as the primary factor

**Status:** implemented — see [`FLOWS-AUTHENTICATION.md` → Passwordless login](FLOWS-AUTHENTICATION.md#passwordless-login-a-passkey-as-the-primary-factor). Email-first through `login`; eligible = accounts with no password and a confirmed passkey; recovery = forgot-password reset + a last-passkey deletion guard. Open follow-up: step-up for passwordless accounts.

**Context:** WebAuthn already exists, but only as a second MFA factor (`start_webauthn_login`, `verify_webauthn`). Passkey-first (no password) is now standard in several modern IdPs.

**Proposal:** allow a user with no password set to authenticate with a passkey alone, reusing the existing WebAuthn use cases and skipping the password step where applicable.

**Best practices / architecture:**
- Be careful not to turn this into a third, inconsistent, parallel "login mode" — ideally the `login` use case should treat this as "no password factor configured, require WebAuthn," not as a fully separate flow.
- Reuse the existing `AuthorizeSession`/`session_policy.rs` for post-login session policy.

**Points of attention:**
- Account recovery becomes trickier without a password (losing the device holding the passkey) — a well-defined second recovery factor is needed before shipping this to production.

---

## [Feature] Magic link login via e-mail

**Status:** implemented — see [`FLOWS-AUTHENTICATION.md` → Magic link login](FLOWS-AUTHENTICATION.md#magic-link-login-a-one-time-link-by-email). Per-client opt-in; any active, verified account; the link only works in the browser that asked for it (bound to that browser's session code, token not burned on a mismatch); the account-dependent work runs after the `202` through `TaskSpawner`.

**Context:** today login is password + optional MFA. A magic link is an alternative passwordless method, simpler to adopt than a passkey.

**Proposal:** an endpoint that issues a short-lived, single-use token, sends it by e-mail (reusing the existing `email_sender`/`email_renderer`/`email_template`), and exchanges it for a session when clicked.

**Best practices / architecture:**
- Treat the token like any other opaque token already hashed in the database (same pattern applied to refresh tokens) — never store it in plain text.
- Apply the same "constant time" care already fixed in S6 (login must not reveal whether an e-mail exists via response-time differences).

**Points of attention:**
- Magic links are vulnerable to "racing" if the e-mail is intercepted (link forwarding, corporate proxies that pre-visit links) — consider an additional check (e.g. requiring the exchange to happen in the same browser/device that requested it, via a context cookie).

---

## [Feature] Device Authorization Grant (RFC 8628)

**Context:** there's currently no flow for CLIs, TV apps, or IoT devices without a full browser.

**Proposal:** implement the `device_code` grant type as a new use case under `application/use_cases/oauth/`, analogous to the existing `token_exchange`, reusing `TokenIssuer`/`token_policy.rs` (already centralized after C2).

**Best practices / architecture:**
- Follow the same folder pattern used by the other grants (`command.rs`/`response.rs`/`use_case.rs`/`tests.rs`) to stay consistent with the rest of `use_cases/oauth/`.

**Points of attention:**
- Needs dedicated rate limiting on `device_code` polling (a misbehaving client could poll aggressively) — `SmartIpKeyExtractor` already exists for rate limiting, but here the relevant key is the `device_code`, not the IP.

---

## [Feature] Active session management per user (list + revocation)

**Status:** implemented — see [`FLOWS-OAUTH2-OIDC.md` → Active sessions](FLOWS-OAUTH2-OIDC.md#active-sessions-list--sign-out). A session is an active refresh-token family (a view, no new table); `/me/sessions` and the admin API list, end one, or end all; the dashboard shows them on the user page's Security tab. No device details are stored or shown.

**Context:** refresh-token rotation and reuse detection already exist, but there's no UI/API for a user or admin to see "logged-in sessions/devices" and revoke them individually.

**Proposal:** expose, in the `/me` API and the dashboard, a listing of the active refresh-token families (already tracked internally for reuse detection), with an individual revocation action.

**Best practices / architecture:**
- This is essentially a *view* over data that already exists (refresh-token families) — it shouldn't require new domain modeling, just a new read/write use case over the table already used by token rotation.

**Points of attention:**
- Be careful not to over-expose sensitive metadata (exact IP, full user agent) unnecessarily — follow the same minimization care already applied to IP handling (S2).

---

## [Feature] OIDC Back-Channel Logout

**Context:** today logout is per-client. There's no propagation of logout across all of a user's OAuth clients at once (e.g. a user logs out of the dashboard but stays "logged in" on another partner app).

**Proposal:** implement back-channel logout per the OIDC spec, using the existing outbox/webhook sender (`webhook_sender.rs`) as the delivery mechanism — it's conceptually the same reliable-delivery problem the user webhooks already solve.

**Best practices / architecture:**
- Reuse the existing delivery infrastructure (HMAC signing, retries, transactional outbox) documented in `FLOWS-WEBHOOKS.md`; don't build a second HTTP delivery mechanism from scratch.

**Points of attention:**
- The `logout_token` must be its own signed JWT (the spec requires it) — don't directly reuse the user-webhook event format, only the delivery infrastructure.

**Status:** implemented — see [`FLOWS-OAUTH2-OIDC.md` → Logout](FLOWS-OAUTH2-OIDC.md#8-logout-rp-initiated-and-back-channel). Every path that ends sessions enqueues a back-channel logout in its own transaction; delivery reuses the outbox, the delivery table, retries and the HTTP client, but sends a `logout+jwt` minted per attempt (no HMAC: the JWT signature is the spec's authentication, and there is no recoverable per-client secret). New `/oauth/logout` (RP-Initiated Logout) signs the user out of every app when called from a live session; ID tokens carry `sid`.

---

## [Feature] Row-Level Security in PostgreSQL by tenant_id

**Context:** `ARCHITECTURE-REVIEW.md` itself (item S8) already flagged this as pending: "Row-Level Security is not added" as a second barrier of isolation, beyond the application-level `WHERE tenant_id = ...` clauses.

**Proposal:** enable RLS on multi-tenant tables, with a policy using `current_setting('app.tenant_id')` set on the connection per request.

**Best practices / architecture:**
- This is defense in depth, not a replacement for the application filters — keep both.
- Must be tested with `#[sqlx::test]`, following the pattern already established in A6 (`tests/db_repositories.rs`) — add specific cases for "a query with no tenant set must return empty, not error."

**Points of attention:**
- Requires the connection pool to run `SET app.tenant_id` per request/transaction — watch out for connection pooling (pgbouncer in transaction mode can break this if not done correctly).

---

## [Feature] Bulk audit log export (CSV/JSON)

**Context:** today `list_audit_events` exists but only paginated for UI viewing — there's no bulk export for compliance/external audit purposes.

**Proposal:** an endpoint that runs the same `list_audit_events` query without UI pagination, but streamed (avoiding loading everything into memory), serialized to CSV or JSON.

**Best practices / architecture:**
- Reuse the existing use case as the base rather than duplicating the query — only change the serialization/response layer.
- Run as an async job for large tenants (generate a file and notify/make it available via a temporary link), instead of keeping the HTTP request open for a long time.

**Points of attention:**
- `audit_events` already has a trigger preventing UPDATE/DELETE (S8) — export is read-only, so there's no conflict, but volume can be large; use a database cursor/stream, not an unbounded `SELECT *`.

---

## [Feature] Native mobile SDKs (iOS/Android) and React Native

**Context:** current SDK coverage (`packages/`) is web-only: SPA (React/Vue/Angular/AngularJS) and BFF (Next.js/Remix/SvelteKit). There's nothing for native mobile.

**Proposal:** new `@gatekeeper/ios`, `@gatekeeper/android`, `@gatekeeper/react-native` packages, using Authorization Code + PKCE with `ASWebAuthenticationSession`/Chrome Custom Tabs (never an embedded WebView, for security — this is the practice recommended by RFC 8252 itself for native apps).

**Best practices / architecture:**
- Follow the same principle already applied in `spa-oauth`/`bff-core`: a reusable OAuth-logic "core," with thin per-platform bindings.
- Tokens must go into Keychain (iOS) / Keystore (Android), never in plain-text `SharedPreferences`/`UserDefaults`.

**Points of attention:**
- A native app's redirect URI needs a custom scheme or a verified App Link/Universal Link — check whether the current `oauth_client` model already supports this without changes to `redirect_uris`.

---
