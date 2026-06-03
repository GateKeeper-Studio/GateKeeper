# Implementation steps — OAuth2/OIDC handlers & use cases

This is the build sheet for wiring the **authentication flow** entities
(`OAuthClient`, `AuthorizationCode`, `AuthSession`, `RefreshToken`, `ConsentGrant`,
`SigningKey`) into real HTTP endpoints and use cases. It complements
[`oauth2-oidc-provider-implementation.md`](./oauth2-oidc-provider-implementation.md),
which explains *why*. This file is the *how* / *in what order*.

Conventions are taken from the existing codebase (2-space indent, `run!` macro,
`UnitOfWork`/`UnitOfWorkFactory` for writes, repo-arcs for reads, one directory
per use case with `use_case.rs` + `command.rs|query.rs` + `response.rs` + `tests.rs`,
handlers using `AppJson` and `State`-first extractor order).

---

## The four endpoints (the "best four")

These four cover the full provider surface end-to-end. Each maps to one new
handler file and one (or two) use cases.

| # | Route | Handler | Use case(s) | Write? | Entities touched |
|---|-------|---------|-------------|--------|------------------|
| 1 | `GET /oauth/authorize` + `POST /oauth/authorize/consent` | `oauth_authorize_handler` | `authorize_request`, `authorize_complete` | yes | `OAuthClient`, `AuthSession`, `ConsentGrant`, `AuthorizationCode` |
| 2 | `POST /oauth/token` | `oauth_token_handler` | `token_exchange` (code), `token_refresh` (rotation) | yes | `OAuthClient`, `AuthorizationCode`, `RefreshToken`, `SigningKey` |
| 3 | `GET /oauth/userinfo` | `oauth_userinfo_handler` | `userinfo` | no | `SigningKey` (verify), `User` |
| 4 | `GET /.well-known/jwks.json` + `GET /.well-known/openid-configuration` | `oauth_discovery_handler` | `jwks`, `discovery` | no | `SigningKey` |

> Why these four: `/authorize` proves the user and records consent, `/token` is
> the only place secrets/codes are exchanged for JWTs, `/userinfo` is the OIDC
> resource endpoint resource servers call, and the `.well-known` pair is what
> makes the provider self-describing and verifiable. Everything else (client
> registration admin, key rotation cron) is operational, not part of the flow.

---

## Prerequisite — finish the domain layer first

The entities exist as files but several are **behaviour-only** (no `new()` /
`reconstruct()` / getters yet) and four have **no repository trait**. Do this
before any use case, or the use cases won't compile.

### Step 0.1 — Complete the entity constructors & getters

For each of `OAuthClient`, `AuthorizationCode`, `RefreshToken`, `AuthSession`,
`ConsentGrant`, add `new(...)`, `reconstruct(...)`, and field getters following the
exact pattern in `src/domain/entities/signing_key.rs` (already complete) and
`src/domain/entities/user.rs`.

- `AuthorizationCode::new(...)` takes the **raw** code only to hash it; it stores
  `code_hash = sha256(raw)` and never the raw value. Expose `mark_consumed(&mut self)`
  that sets `consumed_at = Some(now)`.
- `RefreshToken::new(...)` mints `family_id = Uuid::now_v7()` for a fresh login;
  add `rotate(&mut self)` (sets `rotated_at`) and `revoke(&mut self)`.
- `OAuthClient` already has the policy methods — just add `new`/`reconstruct`/getters
  plus `verify_secret(&self, presented: &str, pepper) -> bool` (Argon2, constant-time).

### Step 0.2 — Add the four missing repository traits

Mirror `src/domain/repositories/signing_key_repository.rs`. Create under
`src/domain/repositories/`:

```rust
// oauth_client_repository.rs
#[async_trait]
pub trait OAuthClientRepository: Send + Sync {
  async fn find_by_client_id(&self, tenant_id: Uuid, client_id: &str) -> Result<Option<OAuthClient>, DomainError>;
  async fn add(&self, client: &OAuthClient) -> Result<(), DomainError>;
}

// authorization_code_repository.rs
#[async_trait]
pub trait AuthorizationCodeRepository: Send + Sync {
  async fn find_by_code_hash(&self, tenant_id: Uuid, code_hash: &str) -> Result<Option<AuthorizationCode>, DomainError>;
  async fn add(&self, code: &AuthorizationCode) -> Result<(), DomainError>;
  async fn mark_consumed(&self, code: &AuthorizationCode) -> Result<(), DomainError>;
}

// refresh_token_repository.rs
#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
  async fn find_by_token_hash(&self, tenant_id: Uuid, token_hash: &str) -> Result<Option<RefreshToken>, DomainError>;
  async fn add(&self, token: &RefreshToken) -> Result<(), DomainError>;
  async fn rotate(&self, old: &RefreshToken, new: &RefreshToken) -> Result<(), DomainError>;
  async fn revoke_family(&self, tenant_id: Uuid, family_id: Uuid) -> Result<(), DomainError>; // reuse-detection kill switch
}

// auth_session_repository.rs
#[async_trait]
pub trait AuthSessionRepository: Send + Sync {
  async fn find_by_id(&self, tenant_id: Uuid, session_id: Uuid) -> Result<Option<AuthSession>, DomainError>;
  async fn add(&self, session: &AuthSession) -> Result<(), DomainError>;
  async fn update(&self, session: &AuthSession) -> Result<(), DomainError>;
}
```

Register each in `src/domain/repositories.rs` (the module file).

### Step 0.3 — Extend `DomainError`

Already present: `InvalidClient`, `InvalidGrant`, `InvalidScope`, `UnauthorizedClient`,
`InvalidRequest`, `AccessDenied`. These map 1:1 to the OAuth `error=` codes — good.
Add only if missing:

```rust
#[error("Login required: {0}")]
LoginRequired(String),
#[error("Consent required: {0}")]
ConsentRequired(String),
```

### Step 0.4 — Map the new errors in `presentation/errors.rs`

In `From<DomainError> for ApiError`, route the OAuth variants to the right status +
the right OAuth JSON body (`{ "error": "...", "error_description": "..." }`):

- `InvalidClient` / `UnauthorizedClient` → `401`
- `InvalidGrant` / `InvalidRequest` / `InvalidScope` → `400`
- `AccessDenied` → `403`
- `LoginRequired` / `ConsentRequired` → `302` redirect back to client (authorize only)

---

## Prerequisite — infrastructure plumbing

### Step 0.5 — DB repository implementations

Under `src/infraestructure/database/repositories/`, add one file per trait
(`oauth_client_repository.rs`, etc.) following `user_repository.rs` exactly:
`DbXRepository { executor: DbExecutor }`, `new(pool)` + `with_tx(SharedTx)`, every
query through the `run!` macro, errors through `map_sqlx_error`. Register in the
module file. (Migrations already exist per §4 of the implementation doc.)

### Step 0.6 — Extend `UnitOfWork` with the write-side accessors

In `src/application/ports/unit_of_work.rs`, add to the trait:

```rust
fn oauth_client_repository(&self) -> Arc<dyn OAuthClientRepository>;
fn authorization_code_repository(&self) -> Arc<dyn AuthorizationCodeRepository>;
fn refresh_token_repository(&self) -> Arc<dyn RefreshTokenRepository>;
fn consent_grant_repository(&self) -> Arc<dyn ConsentGrantRepository>;
fn auth_session_repository(&self) -> Arc<dyn AuthSessionRepository>;
```

Then implement them in `DbUnitOfWork` (`infraestructure/.../unit_of_work.rs`) by
handing each `DbXRepository::with_tx(self.shared_tx())`, and in
`InMemoryUnitOfWork` (`testing/`) for the tests. `SigningKey` is read-mostly; expose
`signing_key_repository()` too so `token_exchange` can sign inside the same tx.

### Step 0.7 — Crypto / signing services (application ports + infra impls)

`/token` needs to *sign* JWTs and `/userinfo` needs to *verify* them. Keep the JWT
library (`jsonwebtoken`) out of the domain. Define ports in `application/ports/`:

```rust
// token_signer.rs
pub trait TokenSigner: Send + Sync {
  // Signs claims with the tenant's ACTIVE SigningKey, stamping `kid` in the header.
  async fn sign(&self, key: &SigningKey, claims: &serde_json::Value) -> Result<String, DomainError>;
}
// token_verifier.rs — verifies against published keys by `kid`.
```

Implement in `infraestructure/crypto/`. Also add `src/utils/oauth_crypto.rs` with
`generate_opaque_token() -> String` and `sha256_hex(&str) -> String` (used for codes
and refresh tokens). Reuse the existing `password_hasher` for client secrets.

---

## Endpoint 1 — `/oauth/authorize`

Split into **two** use cases because there's a human step in the middle.

### Step 1.1 — `authorize_request` (read-mostly validation)

Directory `application/use_cases/oauth/authorize_request/`.

- `query.rs` — the parsed query string: `tenant_id, client_id, redirect_uri,
  response_type, scope, state, code_challenge, code_challenge_method, nonce`.
- `use_case.rs` — takes `Arc<dyn OAuthClientRepository>`. Validates, in order:
  1. client exists & active (`find_by_client_id`) → else `InvalidClient`.
  2. `is_redirect_uri_allowed(redirect_uri)` (exact match) → else `InvalidRequest`
     **and do not redirect** (open-redirect guard).
  3. `response_type == "code"` → else `InvalidRequest`.
  4. `require_pkce` ⇒ `code_challenge` present & `code_challenge_method == "S256"`.
  5. `filter_scopes(requested)` ⇒ unknown scopes dropped or `InvalidScope`.
- `response.rs` — returns either "needs login", "needs consent" (with the delta
  scopes), or "auto-approve" decision data. **No DB writes here.**

### Step 1.2 — `authorize_complete` (the write, one transaction)

Directory `application/use_cases/oauth/authorize_complete/`. Takes
`Arc<dyn UnitOfWorkFactory>`. This runs *after* the user authenticated (reuse the
existing `login` use case to verify the password) and approved consent.

```rust
let uow = self.factory.begin().await?;

// 1. Re-validate client + redirect_uri (never trust the posted form alone).
let client = uow.oauth_client_repository()
  .find_by_client_id(cmd.tenant_id, &cmd.client_id).await?
  .ok_or_else(|| DomainError::InvalidClient("unknown client".into()))?;

// 2. Consent: skip the screen if an existing grant already covers the scopes.
let existing = uow.consent_grant_repository()
  .find_by_user_and_client(cmd.tenant_id, cmd.user_id, client.id()).await?;
let missing = match &existing {
  Some(g) => g.missing_scopes(&cmd.scopes),
  None => cmd.scopes.clone(),
};
if !missing.is_empty() && !cmd.user_approved {
  return Err(DomainError::ConsentRequired(missing.join(" ")));
}
let grant = match existing {
  Some(mut g) => { g.grant_scopes(&cmd.scopes); g }
  None => ConsentGrant::new(cmd.tenant_id, cmd.user_id, client.id(), cmd.scopes.clone()),
};
uow.consent_grant_repository().upsert(&grant).await?;

// 3. Mint the single-use authorization code (raw returned once, hash persisted).
let raw_code = generate_opaque_token();
let code = AuthorizationCode::new(
  cmd.tenant_id, &raw_code, cmd.client_id, cmd.user_id, cmd.redirect_uri,
  cmd.scopes, cmd.code_challenge, cmd.code_challenge_method, cmd.nonce,
  /* ttl */ Duration::seconds(60),
);
uow.authorization_code_repository().add(&code).await?;

uow.commit().await?;
// 4. Redirect: 302 to `${redirect_uri}?code=${raw_code}&state=${state}`.
Ok(Response { redirect_uri, code: raw_code, state: cmd.state })
```

### Step 1.3 — handler `presentation/handlers/oauth_authorize_handler.rs`

- `GET /oauth/authorize` → deserialize `Query<AuthorizeParams>`, call
  `authorize_request`. On "auto-approve" call `authorize_complete` and return the
  `302`. On "needs login/consent" render/redirect to your login+consent UI carrying
  the original params (signed, so they can't be tampered).
- `POST /oauth/authorize/consent` → `AppJson<ConsentForm>` (the user's approve/deny +
  the carried params), call `authorize_complete`. `State` extractor first, always.

---

## Endpoint 2 — `POST /oauth/token`

The form field `grant_type` selects the branch. One handler, two use cases.

### Step 2.1 — `token_exchange` (grant_type=authorization_code)

Directory `application/use_cases/oauth/token_exchange/`. `Arc<dyn UnitOfWorkFactory>`
(+ `TokenSigner`). Transaction body:

```rust
let uow = self.factory.begin().await?;

// 1. Authenticate the client (confidential ⇒ verify secret; public ⇒ PKCE only).
let client = uow.oauth_client_repository()
  .find_by_client_id(cmd.tenant_id, &cmd.client_id).await?
  .ok_or_else(|| DomainError::InvalidClient("unknown client".into()))?;
if client.is_confidential() && !client.verify_secret(&cmd.client_secret, pepper) {
  return Err(DomainError::InvalidClient("bad secret".into()));
}

// 2. Look the code up BY HASH and enforce single use atomically.
let code = uow.authorization_code_repository()
  .find_by_code_hash(cmd.tenant_id, &sha256_hex(&cmd.code)).await?
  .ok_or_else(|| DomainError::InvalidGrant("unknown code".into()))?;
if !code.is_redeemable(Utc::now()) {
  // Replay of an already-consumed code ⇒ treat as theft: revoke its tokens.
  return Err(DomainError::InvalidGrant("code not redeemable".into()));
}

// 3. Bind checks: same client, same redirect_uri, PKCE verifier matches challenge.
if code.client_id() != cmd.client_id { return Err(DomainError::InvalidGrant("client mismatch".into())); }
if code.redirect_uri() != cmd.redirect_uri { return Err(DomainError::InvalidGrant("redirect mismatch".into())); }
verify_pkce(code.code_challenge(), code.code_challenge_method(), &cmd.code_verifier)?; // S256 only

// 4. Consume the code (now; a second redemption must fail).
uow.authorization_code_repository().mark_consumed(&code).await?;

// 5. Sign tokens with the tenant's ACTIVE signing key (kid in header).
let key = uow.signing_key_repository().find_active(cmd.tenant_id).await?
  .ok_or_else(|| DomainError::InfrastructureError("no active signing key".into()))?;
let access_jwt = self.signer.sign(&key, &access_claims(&code, &client)).await?;
let id_jwt     = self.signer.sign(&key, &id_claims(&code, code.nonce())).await?; // OIDC if openid scope

// 6. Issue + store a rotating refresh token (store hash only).
let raw_refresh = generate_opaque_token();
let refresh = RefreshToken::new(cmd.tenant_id, sha256_hex(&raw_refresh), client.id(), code.user_id(), code.scopes());
uow.refresh_token_repository().add(&refresh).await?;

uow.commit().await?;
Ok(Response { access_token: access_jwt, id_token: id_jwt, refresh_token: raw_refresh, token_type: "Bearer", expires_in: 900 })
```

### Step 2.2 — `token_refresh` (grant_type=refresh_token)

Same directory family `application/use_cases/oauth/token_refresh/`. Core rule is
**rotation with reuse detection**:

```rust
let presented = uow.refresh_token_repository()
  .find_by_token_hash(cmd.tenant_id, &sha256_hex(&cmd.refresh_token)).await?
  .ok_or_else(|| DomainError::InvalidGrant("unknown refresh token".into()))?;

if !presented.is_active(Utc::now()) {
  // Already rotated/revoked ⇒ token theft. Nuke the whole family.
  uow.refresh_token_repository().revoke_family(cmd.tenant_id, presented.family_id()).await?;
  uow.commit().await?;
  return Err(DomainError::InvalidGrant("refresh token reuse detected".into()));
}

let raw_new = generate_opaque_token();
let mut rotated = presented.clone();
let new_token = rotated.rotate_into(cmd.tenant_id, sha256_hex(&raw_new)); // keeps family_id
uow.refresh_token_repository().rotate(&presented, &new_token).await?;
// re-sign a fresh access (+id) token exactly like step 2.1 §5.
```

### Step 2.3 — handler `presentation/handlers/oauth_token_handler.rs`

`POST /oauth/token`, body is `application/x-www-form-urlencoded` (per spec, **not**
JSON) → use `axum::extract::Form<TokenForm>`. Branch on `grant_type` to the right
use case. Return JSON with `Cache-Control: no-store`. Client secret may arrive via
HTTP Basic auth header instead of the form — accept both.

---

## Endpoint 3 — `GET /oauth/userinfo`

### Step 3.1 — `userinfo` use case (read-only)

Directory `application/use_cases/oauth/userinfo/`. Takes `Arc<dyn UserRepository>`
(+ `Arc<dyn SigningKeyRepository>` and the verifier). Steps:

1. Verify the bearer access token signature against the tenant's published keys
   (by `kid`), check `exp`, `iss`, `aud`.
2. Load the user (`find_by_id_with_profile`) for `sub`.
3. Return only the claims the token's **scopes** allow (`profile` ⇒ name, `email` ⇒
   email+verified). Never leak fields outside granted scopes.

### Step 3.2 — handler `presentation/handlers/oauth_userinfo_handler.rs`

`GET /oauth/userinfo` (also allow `POST` per spec). Pull the token from the
`Authorization: Bearer` header via a `TypedHeader`/extractor, call the use case,
return JSON. `401` with `WWW-Authenticate: Bearer error="invalid_token"` on failure.

---

## Endpoint 4 — `.well-known`

### Step 4.1 — `jwks` use case (read-only)

Directory `application/use_cases/oauth/jwks/`. Takes `Arc<dyn SigningKeyRepository>`.
Calls `find_published(tenant_id)`, returns `{ "keys": [ <public_jwk>... ] }` built
from each key's `public_jwk()` (already JWK JSON) + its `kid`. **Never** touch
`private_key_encrypted`.

### Step 4.2 — `discovery` use case (static-ish)

Directory `application/use_cases/oauth/discovery/`. Returns the OIDC metadata doc:
`issuer`, `authorization_endpoint`, `token_endpoint`, `userinfo_endpoint`,
`jwks_uri`, `response_types_supported=["code"]`,
`code_challenge_methods_supported=["S256"]`, `grant_types_supported`,
`scopes_supported`, `id_token_signing_alg_values_supported=["RS256","ES256"]`.
Build from config + the tenant's published key algs.

### Step 4.3 — handler `presentation/handlers/oauth_discovery_handler.rs`

`GET /.well-known/jwks.json` → `jwks`; `GET /.well-known/openid-configuration` →
`discovery`. Both `200 JSON`, cacheable (`Cache-Control: max-age=...`). These are
the only OAuth endpoints that *should* be cached.

---

## Step 5 — Wire everything into `routes.rs`

1. Add the new use cases to `AppState` (one `Arc<...::use_case::UseCase>` field each).
2. In `create_router`, build the new repo arcs (`DbOAuthClientRepository`, etc.) and
   the signer/verifier, then instantiate each use case — write use cases get
   `Arc::clone(&uow_factory)`, read ones get the repo arcs, matching the existing
   tenant/user wiring.
3. Register routes (note the namespacing under `/oauth` and `/.well-known`):

```rust
.route("/oauth/authorize", get(oauth_authorize_handler::authorize))
.route("/oauth/authorize/consent", post(oauth_authorize_handler::consent))
.route("/oauth/token", post(oauth_token_handler::token))
.route("/oauth/userinfo", get(oauth_userinfo_handler::userinfo).post(oauth_userinfo_handler::userinfo))
.route("/.well-known/jwks.json", get(oauth_discovery_handler::jwks))
.route("/.well-known/openid-configuration", get(oauth_discovery_handler::discovery))
```

4. Register the handler modules in `src/presentation/handlers.rs`.

---

## Step 6 — Tests (per use case, in-memory)

Each new use-case directory gets a `tests.rs` using `InMemoryUnitOfWorkFactory` /
the in-memory repos (extend `testing/` with the four new in-memory repos first).
Minimum cases to assert the security properties actually hold:

- **authorize_request**: rejects unknown client; rejects non-exact `redirect_uri`
  without redirecting; rejects missing/`plain` PKCE; drops out-of-scope scopes.
- **authorize_complete**: skips consent when a covering `ConsentGrant` exists;
  prompts only the delta; persists code as hash; sets 60s expiry.
- **token_exchange**: happy path issues 3 tokens; rejects bad PKCE verifier; rejects
  redirect/client mismatch; **second redemption of the same code fails**.
- **token_refresh**: rotates and revokes the old token; **replay of a rotated token
  revokes the whole family**.
- **userinfo**: returns only scope-allowed claims; rejects expired/forged token.
- **jwks**: never serializes a private key; returns only published keys.

---

## Build order checklist

```
[ ] 0.1 entity new/reconstruct/getters (+ mark_consumed/rotate/revoke)
[ ] 0.2 four repository traits + module registration
[ ] 0.3 DomainError: LoginRequired, ConsentRequired
[ ] 0.4 ApiError mapping for OAuth errors (status + OAuth JSON body)
[ ] 0.5 Db*Repository impls (run! macro, map_sqlx_error)
[ ] 0.6 UnitOfWork accessors (Db + InMemory)
[ ] 0.7 oauth_crypto util + TokenSigner/TokenVerifier ports & impls
[ ] 1   authorize_request + authorize_complete + handler
[ ] 2   token_exchange + token_refresh + handler
[ ] 3   userinfo + handler
[ ] 4   jwks + discovery + handler
[ ] 5   routes.rs wiring (AppState, use cases, routes, handler modules)
[ ] 6   tests.rs per use case (+ in-memory repos)
[ ] -   cargo test (all in-memory, no DB) green
```

Do the prerequisites (0.x) in order first — nothing in §1–§4 compiles until the
domain traits and UoW accessors exist.
