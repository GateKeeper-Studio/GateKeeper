# Building an OAuth 2.1 + OpenID Connect Provider on `web-api`

This document teaches you how to turn this API into a secure **OAuth 2.1 Authorization
Server** and **OpenID Connect (OIDC) Provider** (a.k.a. "OpenID Provider", OP).

It is written specifically for *this* codebase: Clean Architecture (domain →
application → infrastructure → presentation), Axum, SQLx, multi-tenant, with the
`UnitOfWork` write pattern and the in-memory test fakes you already use. Every new
piece is mapped to the exact folder it belongs in, with code in the same style as
the existing files (2-space indent, `new` / `reconstruct` entity constructors,
repository traits in `domain/`, etc.).

> **Scope note.** You are building the **provider** (the server that *issues*
> tokens), not a client. The OAuth roles are:
>
> | Role | Who, in your system |
> |------|---------------------|
> | **Resource Owner** | The human end-user (a `User`) |
> | **Client / Relying Party (RP)** | A third-party or first-party app that wants tokens (new entity: `OAuthClient`) |
> | **Authorization Server (AS) / OpenID Provider (OP)** | **This API** |
> | **Resource Server (RS)** | Any API that accepts your access tokens (may also be this API) |

---

## Table of contents

1. [Which flows to support (and which to forbid)](#1-which-flows-to-support-and-which-to-forbid)
2. [The mental model: the Authorization Code + PKCE dance](#2-the-mental-model-the-authorization-code--pkce-dance)
3. [New domain entities you must create](#3-new-domain-entities-you-must-create)
4. [Database migrations](#4-database-migrations)
5. [The endpoints and how they map to use cases](#5-the-endpoints-and-how-they-map-to-use-cases)
6. [Implementation walkthrough, phase by phase](#6-implementation-walkthrough-phase-by-phase)
7. [Security: every known attack and how this design stops it](#7-security-every-known-attack-and-how-this-design-stops-it)
8. [Token design: JWT signing, JWKS, and key rotation](#8-token-design-jwt-signing-jwks-and-key-rotation)
9. [Implementation checklist](#9-implementation-checklist)
10. [Spec references](#10-spec-references)

---

## 1. Which flows to support (and which to forbid)

OAuth 2.0 historically defined several grant types. OAuth **2.1** (the consolidation
of current best practice) removes the dangerous ones. Build only these:

| Grant / flow | Use it? | For what |
|--------------|---------|----------|
| **Authorization Code + PKCE** | ✅ **Always** | Web apps, SPAs, mobile, desktop — every interactive login |
| **Refresh Token** (with rotation) | ✅ | Getting new access tokens without re-prompting |
| **Client Credentials** | ✅ (optional) | Machine-to-machine, no user involved |
| **Device Authorization** | ⚠️ optional | TVs / CLI / input-constrained devices |
| Implicit (`response_type=token`) | ❌ **Forbid** | Removed in 2.1 — leaks tokens in the URL |
| Resource Owner Password (ROPC) | ❌ **Forbid** | Removed in 2.1 — client sees the password |

**Key rule baked into this design:** PKCE is **mandatory for every authorization
code request**, from confidential *and* public clients. Your existing login
`Command` already has `code_challenge` / `code_challenge_method`, so you are on the
right track.

OpenID Connect is a thin identity layer **on top of** the Authorization Code flow.
"Doing OIDC" means: when the client includes `openid` in `scope`, you additionally
issue a signed **ID Token** (a JWT describing *who* the user is) and you expose a
`/userinfo` endpoint plus discovery metadata.

---

## 2. The mental model: the Authorization Code + PKCE dance

This is the whole protocol in one picture. Read it once; everything below is just
implementing these steps securely.

```
  End-User's        Client App              THIS API (AS/OP)
   Browser          (Relying Party)
      │                   │                        │
      │  1. click login   │                        │
      │──────────────────▶│                        │
      │                   │  generates:            │
      │                   │   code_verifier (random secret, kept by client)
      │                   │   code_challenge = SHA256(code_verifier)
      │                   │   state (CSRF token)   │
      │                   │   nonce (OIDC replay)  │
      │  2. 302 redirect to /authorize?            │
      │     client_id, redirect_uri, scope,        │
      │     state, code_challenge, ...             │
      │◀──────────────────│                        │
      │                                            │
      │  3. GET /authorize ───────────────────────▶│  validates client_id,
      │                                            │  redirect_uri (exact match),
      │                                            │  scope, PKCE params
      │  4. login form / consent screen ◀──────────│  ConsentGrant.covers(scope)?
      │                                            │  → skip screen if already granted
      │  5. POST credentials + consent ───────────▶│  authenticate user,
      │                                            │  upsert ConsentGrant
      │                                            │  (record approved scopes)
      │                                            │  CREATE AuthorizationCode
      │                                            │  (short-lived, single-use,
      │                                            │   binds: client, user, PKCE,
      │                                            │   redirect_uri, scope, nonce)
      │  6. 302 redirect to                        │
      │     redirect_uri?code=XXX&state=YYY ◀──────│
      │                   │                        │
      │  7. browser hits  │                        │
      │     redirect_uri  │                        │
      │──────────────────▶│  client checks state   │
      │                   │                        │
      │                   │  8. POST /token        │
      │                   │     grant_type=authorization_code
      │                   │     code=XXX           │
      │                   │     code_verifier=...  │  ◀── proves possession
      │                   │     redirect_uri=...   │
      │                   │     (+ client auth if confidential)
      │                   │───────────────────────▶│  validates code (unused?
      │                   │                        │  expired? right client?
      │                   │                        │  redirect matches?),
      │                   │                        │  SHA256(verifier)==challenge?
      │                   │                        │  MARK CODE USED
      │                   │                        │  sign tokens with active
      │                   │                        │  SigningKey (kid in header)
      │                   │  9. { access_token,    │
      │                   │       refresh_token,   │
      │                   │       id_token,        │
      │                   │       token_type,      │
      │                   │       expires_in } ◀───│
```

The **`code_verifier` / `code_challenge`** pair (PKCE, RFC 7636) is what defeats
authorization-code interception: even if an attacker steals the `code` from step 6,
they cannot redeem it at `/token` without the `code_verifier`, which never left the
legitimate client.

The **"session code" entity** you mentioned is the **`AuthorizationCode`** in step 5
— the single-use, short-lived credential exchanged for tokens. We model it below.

---

## 3. New domain entities you must create

All of these are pure Rust structs in `src/domain/entities/`, following the exact
`new` / `reconstruct` + private-fields + getters pattern of `User` and `Tenant`.
Each gets a repository trait in `src/domain/repositories/`.

### 3.1 `OAuthClient` — the registered client application

A client (relying party) must be registered before it can ask for tokens. This is
the equivalent of "an app in the developer console."

`src/domain/entities/oauth_client.rs`

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Public clients (SPAs, mobile, native) CANNOT keep a secret → no client_secret,
// PKCE is their only protection. Confidential clients (server-side web apps) CAN
// keep a secret and must authenticate at the token endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientType {
  Public,
  Confidential,
}

#[derive(Debug, Clone)]
pub struct OAuthClient {
  id: Uuid,
  tenant_id: Uuid,                       // multi-tenant: a client belongs to a tenant

  client_id: String,                     // public identifier (opaque, random)
  client_secret_hash: Option<String>,    // Argon2 hash; None for public clients
  client_type: ClientType,

  name: String,
  // SECURITY: redirect URIs are matched EXACTLY (scheme+host+port+path+no wildcard).
  // This is the #1 defense against open-redirect / code-theft.
  redirect_uris: Vec<String>,
  post_logout_redirect_uris: Vec<String>,

  allowed_scopes: Vec<String>,           // the superset this client may ever request
  allowed_grant_types: Vec<String>,      // e.g. ["authorization_code", "refresh_token"]

  require_pkce: bool,                    // keep this TRUE for everyone
  // If true, every authorization needs a fresh consent screen; if false, consent
  // can be remembered (see ConsentGrant). First-party apps often set false.
  require_consent: bool,

  is_active: bool,
  created_at: DateTime<Utc>,
  updated_at: Option<DateTime<Utc>>,
}
```

Give it `new(...)`, `reconstruct(...)`, getters, and a couple of domain methods that
*encapsulate the security rules* so they cannot be forgotten at a call site:

```rust
impl OAuthClient {
  // Exact-match only. Never use `starts_with`, never allow wildcards.
  pub fn is_redirect_uri_allowed(&self, uri: &str) -> bool {
    self.redirect_uris.iter().any(|u| u == uri)
  }

  pub fn is_post_logout_redirect_allowed(&self, uri: &str) -> bool {
    self.post_logout_redirect_uris.iter().any(|u| u == uri)
  }

  pub fn allows_grant(&self, grant: &str) -> bool {
    self.allowed_grant_types.iter().any(|g| g == grant)
  }

  // The granted scope is the intersection of "what the client may have" and
  // "what was requested" — never widen beyond allowed_scopes.
  pub fn filter_scopes<'a>(&self, requested: &'a [String]) -> Vec<String> {
    requested.iter()
      .filter(|s| self.allowed_scopes.iter().any(|a| a == *s))
      .cloned()
      .collect()
  }

  pub fn is_confidential(&self) -> bool {
    matches!(self.client_type, ClientType::Confidential)
  }
}
```

### 3.2 `AuthorizationCode` — your "session code" entity

This is the short-lived, single-use credential created at the end of the login/consent
step and redeemed once at `/token`. **Treat it like a password**: store only a hash.

`src/domain/entities/authorization_code.rs`

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthorizationCode {
  id: Uuid,
  tenant_id: Uuid,

  // SECURITY: never store the raw code. Store SHA-256(code). The raw value is
  // returned to the browser exactly once (step 6) and never persisted.
  code_hash: String,

  client_id: Uuid,                  // FK to oauth_clients.id
  user_id: Uuid,                    // who authenticated

  redirect_uri: String,            // MUST equal the one sent again at /token
  scopes: Vec<String>,

  // PKCE: stored at /authorize, verified at /token.
  code_challenge: String,
  code_challenge_method: String,   // only "S256" — reject "plain"

  nonce: Option<String>,           // OIDC: echoed into the id_token to stop replay
  // OIDC auth_time / acr / amr can live here too if you support them.

  // Single-use enforcement: set once redeemed. A second redemption is an attack
  // signal — see "authorization code replay" in the security section.
  consumed_at: Option<DateTime<Utc>>,

  expires_at: DateTime<Utc>,       // ~30–60 seconds from creation
  created_at: DateTime<Utc>,
}

impl AuthorizationCode {
  pub fn is_expired(&self, now: DateTime<Utc>) -> bool { now >= self.expires_at }
  pub fn is_consumed(&self) -> bool { self.consumed_at.is_some() }
  pub fn is_redeemable(&self, now: DateTime<Utc>) -> bool {
    !self.is_consumed() && !self.is_expired(now)
  }
}
```

### 3.3 `AuthSession` — the interactive login/consent session (optional but recommended)

Between `GET /authorize` (where you validate the request) and the user actually
submitting credentials and consent, you need somewhere to park the validated
authorization request so the user can't tamper with it via hidden form fields. This
is the **login session**. It also backs OIDC Single-Sign-On (`prompt=none`) and
`/logout`.

`src/domain/entities/auth_session.rs`

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthSession {
  id: Uuid,
  tenant_id: Uuid,

  // Stored server-side; the browser only holds an opaque, httpOnly session cookie
  // whose value hashes to session_token_hash.
  session_token_hash: String,

  user_id: Option<Uuid>,           // None until the user authenticates
  authenticated_at: Option<DateTime<Utc>>,

  // The validated /authorize request, frozen so the user cannot alter it.
  pending_client_id: Uuid,
  pending_redirect_uri: String,
  pending_scopes: Vec<String>,
  pending_state: String,
  pending_code_challenge: String,
  pending_code_challenge_method: String,
  pending_nonce: Option<String>,

  expires_at: DateTime<Utc>,       // absolute idle timeout
  created_at: DateTime<Utc>,
}
```

### 3.4 `RefreshToken` — long-lived, rotated, reuse-detected

`src/domain/entities/refresh_token.rs`

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RefreshToken {
  id: Uuid,
  tenant_id: Uuid,

  token_hash: String,              // SHA-256 of the opaque token; never store raw
  client_id: Uuid,
  user_id: Uuid,
  scopes: Vec<String>,

  // Rotation chain: each refresh issues a new token and revokes the old one.
  // A whole family shares a lineage so we can revoke ALL of them if reuse of a
  // already-rotated token is detected (token theft).
  family_id: Uuid,
  // Set when this token is rotated away. Presenting a token that has a
  // rotated_at is the reuse-detection trigger.
  rotated_at: Option<DateTime<Utc>>,
  revoked_at: Option<DateTime<Utc>>,

  expires_at: DateTime<Utc>,
  created_at: DateTime<Utc>,
}

impl RefreshToken {
  pub fn is_active(&self, now: DateTime<Utc>) -> bool {
    self.revoked_at.is_none() && self.rotated_at.is_none() && now < self.expires_at
  }
}
```

> **Access tokens** are normally *not* stored — they are self-contained signed JWTs
> validated by signature + expiry (stateless). If you need instant revocation of
> access tokens too, either keep them short (5–10 min, the usual answer) or
> additionally introduce an opaque-token + introspection model. Start stateless.

### 3.5 `ConsentGrant` — remembering "user X allowed client Y these scopes"

When the user approves the consent screen, you record *what* they approved. This lets
you (a) skip the consent screen on repeat logins, (b) power a "manage / revoke app
access" screen, and (c) detect when a client asks for a **new** scope it was never
granted before, so you re-prompt only for the delta.

`src/domain/entities/consent_grant.rs`

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConsentGrant {
  id: Uuid,
  tenant_id: Uuid,

  user_id: Uuid,                   // who consented
  client_id: Uuid,                 // FK to oauth_clients.id

  // The scopes the user has approved for this client so far. The DB enforces
  // UNIQUE (user_id, client_id) so there is exactly one row per pairing; new
  // approvals widen this set.
  scopes: Vec<String>,

  // Revoking (from the "manage apps" screen) sets this; a revoked grant forces a
  // fresh consent prompt and is the cue to revoke live tokens for the pairing.
  revoked_at: Option<DateTime<Utc>>,

  created_at: DateTime<Utc>,
  updated_at: Option<DateTime<Utc>>,
}

impl ConsentGrant {
  pub fn new(tenant_id: Uuid, user_id: Uuid, client_id: Uuid, scopes: Vec<String>) -> Self {
    ConsentGrant {
      id: Uuid::now_v7(),
      tenant_id,
      user_id,
      client_id,
      scopes,
      revoked_at: None,
      created_at: Utc::now(),
      updated_at: None,
    }
  }

  pub fn reconstruct(
    id: Uuid,
    tenant_id: Uuid,
    user_id: Uuid,
    client_id: Uuid,
    scopes: Vec<String>,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
  ) -> Self {
    ConsentGrant { id, tenant_id, user_id, client_id, scopes, revoked_at, created_at, updated_at }
  }

  // Domain rule: have ALL requested scopes already been consented (and not revoked)?
  // If true, the consent screen can be skipped entirely.
  pub fn covers(&self, requested: &[String]) -> bool {
    self.revoked_at.is_none()
      && requested.iter().all(|s| self.scopes.iter().any(|g| g == s))
  }

  // The scopes the user has NOT yet approved — what a delta-consent screen shows.
  pub fn missing_scopes(&self, requested: &[String]) -> Vec<String> {
    requested.iter()
      .filter(|s| !self.scopes.iter().any(|g| g == *s))
      .cloned()
      .collect()
  }

  // Merge newly approved scopes into the grant (idempotent union).
  pub fn grant_scopes(&mut self, newly_approved: &[String]) {
    for s in newly_approved {
      if !self.scopes.iter().any(|g| g == s) {
        self.scopes.push(s.clone());
      }
    }
    self.revoked_at = None;
    self.updated_at = Some(Utc::now());
  }

  pub fn revoke(&mut self) {
    self.revoked_at = Some(Utc::now());
    self.updated_at = Some(Utc::now());
  }

  pub fn id(&self) -> Uuid { self.id }
  pub fn user_id(&self) -> Uuid { self.user_id }
  pub fn client_id(&self) -> Uuid { self.client_id }
  pub fn scopes(&self) -> &[String] { &self.scopes }
  pub fn is_revoked(&self) -> bool { self.revoked_at.is_some() }
}
```

Its repository trait (`domain/repositories/consent_grant_repository.rs`):

```rust
#[async_trait]
pub trait ConsentGrantRepository: Send + Sync {
  async fn find_by_user_and_client(&self, user_id: Uuid, client_id: Uuid)
    -> Result<Option<ConsentGrant>, DomainError>;
  async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<ConsentGrant>, DomainError>; // "my apps" screen
  async fn upsert(&self, grant: &ConsentGrant) -> Result<(), DomainError>;               // ON CONFLICT (user_id, client_id)
  async fn revoke(&self, user_id: Uuid, client_id: Uuid) -> Result<(), DomainError>;
}
```

> **Security note (consent phishing / over-consent).** Consent is a security
> control, not just UX. Always show the user *which client* and *which scopes*;
> never silently grant a scope the user did not see. A client that asks for a
> previously-unconsented scope MUST trigger a fresh prompt (use `missing_scopes`),
> even when `require_consent = false`, and the consent form itself must carry a CSRF
> token and `frame-ancestors 'none'` (attack #15 in §7).

### 3.6 `SigningKey` — the keys that sign your JWTs (for the JWKS endpoint)

These are the asymmetric key pairs that sign your **ID tokens** and **JWT access
tokens**. The public halves are published at `/.well-known/jwks.json` so resource
servers can verify signatures offline; the private halves never leave the server.
Modeling a key as an entity (rather than a single config value) is what makes
**rotation** possible: you keep the previous public key published while it rotates
out, so tokens signed just before rotation still verify.

`src/domain/entities/signing_key.rs`

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

// Only asymmetric algorithms — the verifier (resource server) must NOT be able to
// mint tokens. See attack #10 in §7 (alg=none / HS256 forgery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningAlgorithm {
  RS256,   // RSA, most interoperable
  ES256,   // ECDSA P-256, smaller tokens
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyStatus {
  Active,    // currently signing new tokens (exactly one per tenant/alg)
  Retiring,  // no longer signs, still published in JWKS so old tokens verify
  Retired,   // removed from JWKS; safe to delete after all its tokens expired
}

#[derive(Debug, Clone)]
pub struct SigningKey {
  id: Uuid,
  tenant_id: Uuid,

  // The "kid" placed in every JWT header so verifiers know which JWKS entry to use.
  kid: String,
  algorithm: SigningAlgorithm,

  // Public half as a JWK JSON object — served verbatim at /jwks.json.
  public_jwk: String,
  // Private half. NEVER serialize this outward. Store encrypted at rest (sealed
  // with a master key from config.rs) or keep only a KMS/HSM key reference here.
  private_key_encrypted: String,

  status: KeyStatus,

  // When this key stops SIGNING (becomes Retiring). Distinct from when it leaves
  // the JWKS, which is governed by the longest token lifetime it signed.
  rotates_at: Option<DateTime<Utc>>,
  created_at: DateTime<Utc>,
}

impl SigningKey {
  pub fn new(
    tenant_id: Uuid,
    kid: String,
    algorithm: SigningAlgorithm,
    public_jwk: String,
    private_key_encrypted: String,
  ) -> Self {
    SigningKey {
      id: Uuid::now_v7(),
      tenant_id,
      kid,
      algorithm,
      public_jwk,
      private_key_encrypted,
      status: KeyStatus::Active,
      rotates_at: None,
      created_at: Utc::now(),
    }
  }

  pub fn reconstruct(
    id: Uuid,
    tenant_id: Uuid,
    kid: String,
    algorithm: SigningAlgorithm,
    public_jwk: String,
    private_key_encrypted: String,
    status: KeyStatus,
    rotates_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
  ) -> Self {
    SigningKey { id, tenant_id, kid, algorithm, public_jwk, private_key_encrypted, status, rotates_at, created_at }
  }

  pub fn is_active(&self) -> bool { matches!(self.status, KeyStatus::Active) }
  // Active OR retiring keys belong in the JWKS (anything that might still verify a token).
  pub fn is_published(&self) -> bool {
    matches!(self.status, KeyStatus::Active | KeyStatus::Retiring)
  }

  // Transition this key out of signing duty when a newer key becomes active.
  pub fn begin_retiring(&mut self) {
    self.status = KeyStatus::Retiring;
    self.rotates_at = Some(Utc::now());
  }
  pub fn retire(&mut self) { self.status = KeyStatus::Retired; }

  pub fn kid(&self) -> &str { &self.kid }
  pub fn algorithm(&self) -> &SigningAlgorithm { &self.algorithm }
  pub fn public_jwk(&self) -> &str { &self.public_jwk }
  pub fn private_key_encrypted(&self) -> &str { &self.private_key_encrypted }
}
```

Its repository trait (`domain/repositories/signing_key_repository.rs`):

```rust
#[async_trait]
pub trait SigningKeyRepository: Send + Sync {
  async fn find_active(&self, tenant_id: Uuid) -> Result<Option<SigningKey>, DomainError>; // signs new tokens
  async fn find_published(&self, tenant_id: Uuid) -> Result<Vec<SigningKey>, DomainError>; // builds /jwks.json
  async fn add(&self, key: &SigningKey) -> Result<(), DomainError>;
  async fn update(&self, key: &SigningKey) -> Result<(), DomainError>;                      // status transitions
}
```

> **Why an entity, not a `.env` secret?** A single static signing secret cannot be
> rotated without invalidating every live token at once, and gives you no way to
> publish a verifier-only public key. Modeling keys lets `find_active()` pick the
> signer and `find_published()` feed the JWKS, so rotation is a status transition
> (`Active → Retiring → Retired`) with zero downtime. See
> [§8](#8-token-design-jwt-signing-jwks-and-key-rotation) for the rotation procedure.
>
> **Tenancy choice.** The signature above scopes keys per `tenant_id` (each tenant
> is its own issuer with its own `kid`s — strongest isolation). If instead you run
> one global issuer for all tenants, drop `tenant_id` from this entity and its
> queries; everything else is identical. Pick one and keep `iss` in your tokens
> consistent with it.

### 3.7 New `DomainError` variants

Add to `src/domain/errors.rs` (these must map to the **exact** OAuth error codes in
the presentation layer):

```rust
  #[error("Invalid client: {0}")]
  InvalidClient(String),
  #[error("Invalid grant: {0}")]
  InvalidGrant(String),
  #[error("Invalid scope: {0}")]
  InvalidScope(String),
  #[error("Unauthorized client: {0}")]
  UnauthorizedClient(String),
  #[error("Invalid authorization request: {0}")]
  InvalidRequest(String),
  #[error("Access denied: {0}")]
  AccessDenied(String),
```

---

## 4. Database migrations

Follow your existing convention (`sqlx migrate add <name>`, paired `.up.sql` /
`.down.sql`, UUID PKs, `tenant_id` FK with `ON DELETE CASCADE`, `TIMESTAMPTZ`).

`migrations/XXXX_create_oauth_clients.up.sql`

```sql
CREATE TABLE IF NOT EXISTS oauth_clients (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  client_id VARCHAR(128) NOT NULL,
  client_secret_hash VARCHAR(255) NULL,          -- NULL for public clients
  client_type VARCHAR(16) NOT NULL,              -- 'public' | 'confidential'
  name VARCHAR(255) NOT NULL,
  redirect_uris TEXT[] NOT NULL,                 -- exact-match list
  post_logout_redirect_uris TEXT[] NOT NULL DEFAULT '{}',
  allowed_scopes TEXT[] NOT NULL,
  allowed_grant_types TEXT[] NOT NULL,
  require_pkce BOOLEAN NOT NULL DEFAULT TRUE,
  require_consent BOOLEAN NOT NULL DEFAULT TRUE,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_oauth_clients_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);
-- client_id is the public lookup key and must be globally unique.
CREATE UNIQUE INDEX uq_oauth_clients_client_id ON oauth_clients (client_id);
```

`migrations/XXXX_create_authorization_codes.up.sql`

```sql
CREATE TABLE IF NOT EXISTS authorization_codes (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  code_hash VARCHAR(64) NOT NULL,                -- hex SHA-256
  client_id UUID NOT NULL,
  user_id UUID NOT NULL,
  redirect_uri TEXT NOT NULL,
  scopes TEXT[] NOT NULL,
  code_challenge VARCHAR(128) NOT NULL,
  code_challenge_method VARCHAR(8) NOT NULL,     -- only 'S256'
  nonce VARCHAR(255) NULL,
  consumed_at TIMESTAMPTZ NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_authcode_client FOREIGN KEY (client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_authcode_user   FOREIGN KEY (user_id)   REFERENCES users (id)        ON DELETE CASCADE
);
CREATE UNIQUE INDEX uq_authorization_codes_code_hash ON authorization_codes (code_hash);
-- a background job deletes rows past expires_at
CREATE INDEX idx_authorization_codes_expires_at ON authorization_codes (expires_at);
```

`migrations/XXXX_create_refresh_tokens.up.sql`

```sql
CREATE TABLE IF NOT EXISTS refresh_tokens (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  token_hash VARCHAR(64) NOT NULL,
  client_id UUID NOT NULL,
  user_id UUID NOT NULL,
  scopes TEXT[] NOT NULL,
  family_id UUID NOT NULL,                        -- rotation lineage
  rotated_at TIMESTAMPTZ NULL,
  revoked_at TIMESTAMPTZ NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_refresh_client FOREIGN KEY (client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE,
  CONSTRAINT fk_refresh_user   FOREIGN KEY (user_id)   REFERENCES users (id)        ON DELETE CASCADE
);
CREATE UNIQUE INDEX uq_refresh_tokens_token_hash ON refresh_tokens (token_hash);
CREATE INDEX idx_refresh_tokens_family ON refresh_tokens (family_id);
```

`migrations/XXXX_create_consent_grants.up.sql`

```sql
CREATE TABLE IF NOT EXISTS consent_grants (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,
  user_id UUID NOT NULL,
  client_id UUID NOT NULL,
  scopes TEXT[] NOT NULL,
  revoked_at TIMESTAMPTZ NULL,
  created_at TIMESTAMPTZ NOT NULL,
  updated_at TIMESTAMPTZ NULL,
  CONSTRAINT fk_consent_user   FOREIGN KEY (user_id)   REFERENCES users (id)         ON DELETE CASCADE,
  CONSTRAINT fk_consent_client FOREIGN KEY (client_id) REFERENCES oauth_clients (id) ON DELETE CASCADE
);
-- Exactly one grant row per (user, client); the upsert relies on this.
CREATE UNIQUE INDEX uq_consent_grants_user_client ON consent_grants (user_id, client_id);
```

`migrations/XXXX_create_signing_keys.up.sql`

```sql
CREATE TABLE IF NOT EXISTS signing_keys (
  id UUID PRIMARY KEY,
  tenant_id UUID NOT NULL,                         -- drop this column for a single global issuer
  kid VARCHAR(128) NOT NULL,                       -- key id embedded in JWT headers
  algorithm VARCHAR(8) NOT NULL,                   -- 'RS256' | 'ES256'
  public_jwk JSONB NOT NULL,                       -- served verbatim at /jwks.json
  private_key_encrypted TEXT NOT NULL,             -- sealed with a master key; never served
  status VARCHAR(16) NOT NULL DEFAULT 'active',    -- 'active' | 'retiring' | 'retired'
  rotates_at TIMESTAMPTZ NULL,
  created_at TIMESTAMPTZ NOT NULL,
  CONSTRAINT fk_signing_keys_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX uq_signing_keys_kid ON signing_keys (kid);
-- find_active() / find_published() filter on status (and tenant_id).
CREATE INDEX idx_signing_keys_tenant_status ON signing_keys (tenant_id, status);
```

Plus an analogous table for `auth_sessions`.

---

## 5. The endpoints and how they map to use cases

Every endpoint becomes a handler in `src/presentation/handlers/` that calls one
application use case, exactly like your `user_handler` → `create_tenant_user`
pattern.

| HTTP | Path | Purpose | Use case (`application/use_cases/oauth/...`) |
|------|------|---------|---------------------------------------------|
| GET  | `/.well-known/openid-configuration` | OIDC discovery document | `discovery::get_metadata` (read-only) |
| GET  | `/.well-known/jwks.json` | Public signing keys (JWKS) | `jwks::get_keys` (read-only) |
| GET  | `/oauth/authorize` | Validate request, start login | `authorize::begin` (write — creates `AuthSession`) |
| POST | `/oauth/login` | User submits credentials | `authorize::authenticate` (write) |
| POST | `/oauth/consent` | User approves scopes → issue code | `authorize::complete` (write — creates `AuthorizationCode`) |
| POST | `/oauth/token` | Exchange code / refresh for tokens | `token::exchange` (write) |
| GET  | `/oauth/userinfo` | OIDC claims about the user | `userinfo::get` (read-only, bearer-auth) |
| POST | `/oauth/revoke` | Revoke a refresh token (RFC 7009) | `revoke::revoke_token` (write) |
| POST | `/oauth/introspect` | Token status (RFC 7662, for resource servers) | `introspect::introspect` (read) |
| GET/POST | `/oauth/logout` | OIDC RP-initiated logout | `logout::end_session` (write) |

Register these in `presentation/routes.rs` and add the use cases to `AppState`,
exactly as the tenant/user routes are wired today.

> The `/authorize` step is split into three use cases (`begin` → `authenticate` →
> `complete`) because it spans multiple HTTP round-trips with a human in the loop.
> If you prefer, `authenticate` and `complete` can be a single POST when consent is
> not required.

---

## 6. Implementation walkthrough, phase by phase

Build it in this order; each phase is independently testable with your in-memory
fakes.

### Phase 0 — Crypto helpers (`src/utils/`)

You already have `password_hasher.rs` (Argon2). Add a small module for the OAuth
primitives. **Use a CSPRNG (`OsRng`) and constant-time comparison.**

`src/utils/oauth_crypto.rs`

```rust
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

// 256 bits of entropy, URL-safe, no padding. Used for auth codes, refresh tokens,
// session tokens, client secrets.
pub fn generate_opaque_token() -> String {
  let mut bytes = [0u8; 32];
  rand::rngs::OsRng.fill_bytes(&mut bytes);
  URL_SAFE_NO_PAD.encode(bytes)
}

// We persist only the hash of opaque tokens so a DB leak does not hand out
// live credentials. SHA-256 is correct here (unlike passwords) because the input
// is already high-entropy random — no Argon2 stretching needed.
pub fn hash_token(token: &str) -> String {
  let digest = Sha256::digest(token.as_bytes());
  hex::encode(digest)
}

// PKCE S256 verification: challenge == BASE64URL(SHA256(verifier))
pub fn verify_pkce_s256(code_verifier: &str, code_challenge: &str) -> bool {
  let digest = Sha256::digest(code_verifier.as_bytes());
  let computed = URL_SAFE_NO_PAD.encode(digest);
  // Constant-time compare to avoid timing oracles.
  computed.as_bytes().ct_eq(code_challenge.as_bytes()).into()
}
```

Add to `Cargo.toml`: `sha2`, `base64`, `subtle`, `hex` (and you already have `rand`).

### Phase 1 — Client registration

1. `OAuthClient` entity + `OAuthClientRepository` trait (`domain/`).
2. `DbOAuthClientRepository` (`infraestructure/database/repositories/`) using the
   `run!` macro and `DbExecutor` pattern — copy `DbUserRepository`.
3. Add `oauth_client_repository()` to the `UnitOfWork` trait and both impls
   (`DbUnitOfWork` + `InMemoryUnitOfWork`).
4. Use case `oauth_client::register_client` (write → `UnitOfWorkFactory`):
   - generate `client_id` = `generate_opaque_token()`
   - if confidential, generate a `client_secret`, hash with Argon2
     (`hash_password`), return the **raw secret once** in the response.
   - **validate every `redirect_uri` is absolute, https (or a registered custom
     scheme), and has no fragment.**
5. In-memory repo + tests.

### Phase 2 — The authorization endpoint (`GET /oauth/authorize`)

Use case `oauth::authorize::begin`. This is where most security validation happens.
Process **in this exact order**, because *what you do with an error depends on
whether you can trust `redirect_uri` yet*:

```rust
// pseudo-Rust inside execute(query: AuthorizeQuery)
// 1. Look up the client by client_id. Unknown/inactive → render an error PAGE.
//    DO NOT redirect — you cannot trust an unverified redirect_uri.
let client = repo.find_by_client_id(query.client_id).await?
  .filter(|c| c.is_active())
  .ok_or(DomainError::InvalidClient("unknown client".into()))?;

// 2. Validate redirect_uri by EXACT match against the registered list.
//    Mismatch → render an error PAGE (again: never redirect to an unverified URI).
if !client.is_redirect_uri_allowed(&query.redirect_uri) {
  return Err(DomainError::InvalidRequest("redirect_uri mismatch".into()));
}

// 3. From here, redirect_uri is trusted. All further protocol errors are returned
//    by REDIRECTING to redirect_uri with ?error=...&state=... (OAuth error response).

// 4. response_type MUST be exactly "code". Reject "token"/"id_token token" (implicit).
if query.response_type != "code" {
  return Err(redirect_error("unsupported_response_type", &query));
}

// 5. PKCE is mandatory. code_challenge present, method == "S256" (reject "plain").
if query.code_challenge.is_empty() || query.code_challenge_method != "S256" {
  return Err(redirect_error("invalid_request", &query));
}

// 6. state present (CSRF). Required for browser flows.
if query.state.is_empty() {
  return Err(redirect_error("invalid_request", &query));
}

// 7. Scope: intersect with client.allowed_scopes; if empty / contains unknown
//    scope, reject. OIDC requires "openid" to be present to get an id_token.
let scopes = client.filter_scopes(&query.requested_scopes);
if scopes.is_empty() {
  return Err(redirect_error("invalid_scope", &query));
}

// 8. Freeze the validated request into an AuthSession, set an httpOnly+Secure+
//    SameSite=Lax session cookie, and render the login form.
```

Then `authenticate` (reuse your existing **login** use case — it already verifies
the password with Argon2 + the tenant pepper) and `complete`, which is where
**`ConsentGrant`** decides whether to even show the consent screen, then issues the
code:

```rust
// oauth::authorize::complete — runs after the user is authenticated in the AuthSession.
// 1. Has the user already consented to (at least) these scopes for this client?
let existing = consent_repo
  .find_by_user_and_client(session.user_id(), client.id()).await?;

let must_prompt = client.require_consent()
  && !existing.as_ref().map(|c| c.covers(session.pending_scopes())).unwrap_or(false);

// 2. If consent is required and not yet covered, render the consent screen showing
//    ONLY the missing scopes, and return — the user re-POSTs with approval.
if must_prompt && !command.user_approved {
  let missing = existing.as_ref()
    .map(|c| c.missing_scopes(session.pending_scopes()))
    .unwrap_or_else(|| session.pending_scopes().to_vec());
  return Ok(Response::NeedsConsent { client_name: client.name().into(), scopes: missing });
}

// 3. Persist the approval (idempotent union) so next time the screen is skipped.
let mut grant = existing.unwrap_or_else(||
  ConsentGrant::new(client.tenant_id(), session.user_id(), client.id(), vec![]));
grant.grant_scopes(session.pending_scopes());
consent_repo.upsert(&grant).await?;

// 4. NOW create the single-use AuthorizationCode (raw returned once, only hash stored)
//    and 302 redirect to redirect_uri?code=RAW_CODE&state=ORIGINAL_STATE.
```

Reusing your `login` use case here is exactly why its `Command` already has the PKCE
fields — keep the auth-code creation in the `complete` step and have it return the
raw code (only the hash is persisted). The whole `complete` step is one
`UnitOfWork` transaction (consent upsert + code insert commit together).

### Phase 3 — The token endpoint (`POST /oauth/token`)

Use case `oauth::token::exchange`. Branch on `grant_type`. This endpoint is
`application/x-www-form-urlencoded` (not JSON) per spec, and must send
`Cache-Control: no-store`.

**Client authentication first** (for confidential clients): read
`Authorization: Basic base64(client_id:client_secret)` (or form `client_secret`),
look up the client, `compare_password(secret_hash, presented_secret, pepper)` in
**constant time**. Public clients authenticate only by PKCE.

**`grant_type=authorization_code`:**

```rust
// 1. hash the presented code, look it up
let code = repo.find_by_code_hash(&hash_token(&form.code)).await?
  .ok_or(DomainError::InvalidGrant("code not found".into()))?;

// 2. single-use + expiry. If ALREADY consumed → this is a replay/theft signal:
//    revoke every token previously issued from this code (defense in depth).
if !code.is_redeemable(now) {
  // optionally: revoke tokens descended from code.id
  return Err(DomainError::InvalidGrant("code expired or already used".into()));
}

// 3. the code is bound to the authenticating client — they must match.
if code.client_id() != authenticated_client.id() {
  return Err(DomainError::InvalidGrant("client mismatch".into()));
}

// 4. redirect_uri sent here MUST equal the one bound at /authorize.
if form.redirect_uri != code.redirect_uri() {
  return Err(DomainError::InvalidGrant("redirect_uri mismatch".into()));
}

// 5. PKCE proof: SHA256(code_verifier) must equal the stored challenge.
if !verify_pkce_s256(&form.code_verifier, code.code_challenge()) {
  return Err(DomainError::InvalidGrant("PKCE verification failed".into()));
}

// 6. ATOMICALLY mark the code consumed (same UnitOfWork transaction as token
//    creation). The UNIQUE(code_hash) + consumed_at update prevents double-spend.
//    Then mint tokens, signing the JWTs with the tenant's ACTIVE SigningKey:
let signing_key = signing_key_repo.find_active(code.tenant_id()).await?
  .ok_or(DomainError::InfrastructureError("no active signing key".into()))?;
// access_token  = JWT signed with signing_key (header kid = signing_key.kid())
// id_token      = JWT signed with the SAME key, only if "openid" in code.scopes(),
//                 carrying nonce = code.nonce()
// refresh_token = opaque random; store hash_token(raw) in refresh_tokens
```

> The `signing_key_repo.find_active(...)` lookup is the only place `/token` touches
> `SigningKey`: it picks today's signer and stamps its `kid` into the JWT header so
> verifiers can later find the matching public key in `/jwks.json`. Wrap signing in
> an infrastructure `JwtSigningService` so the use case stays free of `jsonwebtoken`.

**`grant_type=refresh_token`:** look up by `hash_token`, check `is_active`,
**rotate** (mark old `rotated_at`, issue a new refresh token in the same
`family_id`), re-mint the access token. If a token with a non-null `rotated_at` is
presented → **reuse detected**: revoke the entire `family_id` and return
`invalid_grant`.

All of this is a single `UnitOfWorkFactory.begin()` → `...` → `commit()` block, just
like `create_tenant_user`.

### Phase 4 — ID tokens, UserInfo, JWKS, discovery (the OIDC layer)

- **ID Token**: a JWT signed with your active `SigningKey`, with claims
  `iss` (your issuer URL), `sub` (the `user_id`), `aud` (the `client_id`),
  `exp`, `iat`, `nonce` (echoed from the code), plus profile claims (`email`,
  `email_verified`, `name`) gated by the granted scopes (`openid`, `email`,
  `profile`). Use the `jsonwebtoken` crate you already depend on.
- **`/userinfo`**: requires a valid access token (Bearer); returns the same claim
  set as JSON, filtered by scope. (See [§8](#8-token-design-jwt-signing-jwks-and-key-rotation).)
- **`/.well-known/jwks.json`**: the public halves of your active + retiring signing
  keys, so resource servers can verify signatures offline.
- **`/.well-known/openid-configuration`**: a static-ish JSON document advertising
  your endpoints, supported scopes, `response_types_supported: ["code"]`,
  `grant_types_supported`, `code_challenge_methods_supported: ["S256"]`,
  `id_token_signing_alg_values_supported: ["RS256"]`, etc.

### Phase 5 — Revocation, introspection, logout

- **`/oauth/revoke`** (RFC 7009): client revokes a refresh token; mark
  `revoked_at`. Always return `200` even for unknown tokens (don't leak validity).
- **`/oauth/introspect`** (RFC 7662): resource servers ask "is this token live?".
  Authenticate the *caller* (a client/resource server) first, then return
  `{ "active": true/false, ... }`.
- **`/oauth/logout`**: clear the `AuthSession`, validate
  `post_logout_redirect_uri` against the client's registered list (exact match),
  then redirect.

### Wiring (`presentation/routes.rs`)

```rust
// inside create_router, after building the repos/uow_factory:
let oauth_client_repository: Arc<dyn OAuthClientRepository> =
  Arc::new(DbOAuthClientRepository::new(db_pool.clone()));

let state = AppState {
  // ...existing fields...
  authorize_begin:   Arc::new(oauth::authorize::begin::UseCase::new(Arc::clone(&uow_factory))),
  token_exchange:    Arc::new(oauth::token::exchange::UseCase::new(Arc::clone(&uow_factory), signing_service.clone())),
  get_userinfo:      Arc::new(oauth::userinfo::UseCase::new(Arc::clone(&user_repository))),
  // ...
};

Router::new()
  // ...existing routes...
  .route("/.well-known/openid-configuration", get(oauth_handler::discovery))
  .route("/.well-known/jwks.json",            get(oauth_handler::jwks))
  .route("/oauth/authorize",                  get(oauth_handler::authorize))
  .route("/oauth/login",                      post(oauth_handler::login))
  .route("/oauth/consent",                    post(oauth_handler::consent))
  .route("/oauth/token",                      post(oauth_handler::token))
  .route("/oauth/userinfo",                   get(oauth_handler::userinfo))
  .route("/oauth/revoke",                     post(oauth_handler::revoke))
  .route("/oauth/introspect",                 post(oauth_handler::introspect))
  .route("/oauth/logout",                     get(oauth_handler::logout).post(oauth_handler::logout))
  .with_state(state)
```

---

## 7. Security: every known attack and how this design stops it

This is the heart of the document. Each row is a real, named attack from the OAuth
threat model (RFC 6819) and the OAuth 2.0 Security BCP (RFC 9700).

| # | Attack | What it is | Defense in this design |
|---|--------|-----------|------------------------|
| 1 | **Authorization code interception** | Attacker steals the `code` from the redirect (malicious app, referrer leak, shoulder-surf) | **PKCE mandatory.** Code is useless without `code_verifier`, which never leaves the client. Verified in Phase 3 step 5. |
| 2 | **PKCE downgrade / `plain` method** | Attacker forces `code_challenge_method=plain` so the challenge == verifier | Reject anything but `S256` at both `/authorize` and `/token`. |
| 3 | **CSRF on the redirect** | Attacker tricks a victim's browser into completing a flow with the attacker's code | **`state`** is required, opaque, single-use, and checked by the client. The OIDC **`nonce`** adds the same protection to the id_token. |
| 4 | **Open redirect / redirect_uri manipulation** | Attacker registers `redirect_uri=https://evil.com` or appends a path | **Exact-match** `redirect_uri` against the registered list (no wildcards, no `startsWith`). Errors before validation render a *page*, never a redirect. |
| 5 | **Authorization code replay** | Code redeemed twice | Single-use: `consumed_at` set atomically in the token transaction; `UNIQUE(code_hash)`. A second redemption → revoke issued tokens. |
| 6 | **Code/token theft from DB leak** | DB dump exposes live credentials | Store only `SHA-256` hashes of codes, refresh tokens, session tokens; client secrets are Argon2-hashed. Raw values exist only in transit. |
| 7 | **Refresh token theft** | Stolen refresh token used by attacker | **Rotation + reuse detection.** Using a rotated token revokes the whole `family_id` — both attacker and victim are logged out, theft is contained and detectable. |
| 8 | **Token substitution / audience confusion** | Token for service A replayed against service B | Every token carries `aud` (client_id) and `iss`; resource servers MUST validate both. ID token `aud` == client_id. |
| 9 | **Mix-up attack** | Multi-AS client confused about which AS issued a response | Always set the `iss` claim and (optionally) the `iss` authorization-response parameter so the client can pin the issuer. |
| 10 | **ID token forgery / `alg=none`** | Attacker submits an unsigned or HMAC-with-public-key token | Sign with **asymmetric** RS256/ES256 only. Verifiers reject `none` and reject algorithm switching. Never use HS256 for id_tokens shared with clients. |
| 11 | **Token leakage via referrer / logs** | Tokens in URLs end up in logs, history, `Referer` | No tokens in URLs (implicit flow forbidden). `Cache-Control: no-store` on `/token`. Never log tokens, codes, passwords (your login use case already avoids logging the password). |
| 12 | **Brute force on credentials / codes / secrets** | Guessing passwords, codes, secrets | Argon2 for passwords; 256-bit CSPRNG codes/secrets (infeasible to guess); **rate-limit** `/token`, `/login`, `/authorize` per IP + per client; lock accounts on repeated failure. |
| 13 | **Timing attacks** | Comparison time leaks secret bytes | Constant-time compare (`subtle::ct_eq`) for PKCE and token-hash matches; Argon2's verify is already constant-time. |
| 14 | **Scope escalation** | Client requests more than allowed | Granted scope = intersection with `allowed_scopes`; never widen. Re-validate scope at `/token` and on refresh (cannot exceed original grant). |
| 15 | **Consent phishing / clickjacking** | Consent page framed by attacker to trick clicks | `X-Frame-Options: DENY` / `Content-Security-Policy: frame-ancestors 'none'` on login + consent pages; CSRF token on the consent form. |
| 16 | **Session fixation** | Attacker fixes a victim's session id | Regenerate the `AuthSession` token on successful authentication; httpOnly + Secure + SameSite cookies. |
| 17 | **Insufficient redirect validation on logout** | Open redirect via `post_logout_redirect_uri` | Exact-match against registered `post_logout_redirect_uris`. |
| 18 | **Long-lived access tokens** | Stolen token usable for a long time | Access tokens 5–15 min; rely on refresh for longevity; keep introspection/revocation for the refresh layer. |
| 19 | **TLS stripping / MITM** | Plaintext interception | Enforce HTTPS everywhere (HSTS); reject non-TLS redirect URIs except loopback for native apps. |
| 20 | **Cross-tenant leakage** | Client/user/token from tenant A used in tenant B | Every entity carries `tenant_id`; every lookup filters by it (you already do this for users/emails). Codes/tokens are bound to the issuing tenant + client. |
| 21 | **Account enumeration** | Different responses reveal which emails/clients exist | Generic errors (your login already returns "E-mail/Password do not match"); revoke returns 200 for unknown tokens. |
| 22 | **Authorization code injection** | Attacker injects their own code into a victim's session | PKCE binds the code to the verifier held by the legitimate client; `nonce` binds the id_token to the client's session. |

### Operational must-dos

- **Short lifetimes:** auth code 30–60 s; access token 5–15 min; refresh token
  days–weeks with rotation; id_token a few minutes.
- **Rotate signing keys** regularly (see §8); keep the previous public key in JWKS
  until its tokens expire.
- **Garbage-collect** expired `authorization_codes` and `auth_sessions` (a cron /
  background task; index `expires_at` as shown).
- **Audit log** every issuance, refresh, revocation, and reuse-detection event
  (without logging the secret values).
- **Security headers** on all HTML responses: HSTS, `X-Content-Type-Options:
  nosniff`, CSP, `X-Frame-Options: DENY`.
- **Rate limiting + lockout** via a Tower middleware layer (you already use
  `tower-http`).

---

## 8. Token design: JWT signing, JWKS, and key rotation

### Access token & ID token (JWT, asymmetric)

```jsonc
// header
{ "alg": "RS256", "typ": "at+jwt", "kid": "2026-06-key-1" }
// claims
{
  "iss": "https://auth.yourdomain.com",   // your issuer — clients pin this
  "sub": "0190f...-user-uuid",            // the User id
  "aud": "client_abc123",                 // the client_id the token is for
  "tid": "0190f...-tenant-uuid",          // your multi-tenant discriminator
  "scope": "openid email profile",
  "iat": 1769990400,
  "exp": 1769991300,                      // ~15 min for access tokens
  "jti": "0190f...-token-uuid"            // unique id, enables denylist if needed
}
```

Use the `jsonwebtoken` crate (already a dependency). Build an
`infraestructure`-level `JwtSigningService` that:
- loads the **active** `SigningKey` (private key) and signs with its `kid`,
- exposes the **public** JWKS for `/jwks.json`,
- on rotation, marks a new key active but keeps recently-retired public keys
  published until `exp` of the last token they signed has passed.

> **Why asymmetric (RS256/ES256), not HS256?** With HMAC, anyone who can *verify*
> the token can also *forge* it (same shared secret). Resource servers need to
> verify but must not be able to mint tokens. Asymmetric keys publish only the
> public half via JWKS. ES256 (P-256) gives smaller tokens; RS256 is the most
> widely interoperable. Pick one and advertise it in discovery.

### Refresh tokens (opaque, stateful)

Not JWTs — a refresh token is a 256-bit random string; you store `SHA-256(token)`
in `refresh_tokens`. This makes them instantly revocable and reuse-detectable
(impossible with a stateless JWT). This is the deliberate asymmetry: **access tokens
stateless and short; refresh tokens stateful and rotated.**

### Where signing keys live

Minimum viable: store the private key encrypted at rest in `signing_keys` (encrypt
with a master key from your config/secret manager — you already have a `config.rs`).
Better: a KMS/HSM that signs without exposing the private key. Either way, the
`domain` `SigningKey` entity only models metadata + the public JWK; the actual
signing happens in `infraestructure`.

### Key rotation procedure (maps to `SigningKey::KeyStatus`)

Rotation is just status transitions on the entity, run periodically (e.g. a monthly
cron — you can reuse the `loop`/scheduling tooling, or an external scheduler):

1. **Generate** a new key pair → insert a `SigningKey` with `status = Active` and a
   fresh `kid`. Demote the previous active key with `begin_retiring()`
   (`Active → Retiring`).
2. **Now**: new tokens are signed by the new key (`find_active`), while `/jwks.json`
   (`find_published`) still serves *both* keys, so tokens signed seconds before the
   switch keep verifying.
3. **After the longest token lifetime** signed by the old key has fully elapsed
   (e.g. refresh-token max age), call `retire()` (`Retiring → Retired`). It drops out
   of `/jwks.json`.
4. **Later**: a cleanup job may delete `Retired` rows.

This zero-downtime rotation is *only* possible because keys are an entity with
multiple simultaneous states — a single env-var secret cannot do it. **Bootstrap:**
on first run, if `find_active(tenant)` returns `None`, generate the first key. There
is no separate "Phase" for this; fold it into Phase 3/4 when you build the
`JwtSigningService`.

---

## 9. Implementation checklist

Domain (`src/domain/`)
- [ ] Entities: `OAuthClient`, `AuthorizationCode`, `AuthSession`, `RefreshToken`,
      `ConsentGrant`, `SigningKey`
- [ ] Repository traits for each
- [ ] New `DomainError` variants mapping to OAuth error codes

Application (`src/application/`)
- [ ] Add repo accessors to the `UnitOfWork` trait (incl. `consent_grant_repository()`,
      `signing_key_repository()`)
- [ ] Use cases: `oauth_client::register_client`; `oauth::authorize::{begin,
      authenticate, complete}`; `oauth::token::exchange`; `oauth::userinfo`;
      `oauth::revoke`; `oauth::introspect`; `oauth::logout`; `oauth::jwks`;
      `oauth::discovery`
- [ ] **Consent in the flow:** `complete` checks `ConsentGrant.covers(scopes)` →
      prompts only for `missing_scopes` → `upsert` on approval (same tx as the code)
- [ ] `oauth::consent::{list_user_grants, revoke_grant}` for a "manage apps" screen
- [ ] **Signing in the flow:** `token::exchange` + ID-token issuance use
      `signing_key_repo.find_active()`; `jwks` uses `find_published()`; bootstrap the
      first key if none active
- [ ] Each use case has `command|query.rs`, `response.rs`, `use_case.rs`, `tests.rs`

Infrastructure (`src/infraestructure/`)
- [ ] Db repositories (copy the `DbUserRepository` + `run!` pattern)
- [ ] `oauth_client_repository()` etc. on `DbUnitOfWork`
- [ ] `JwtSigningService` + key loading/rotation
- [ ] Migrations for all new tables

Presentation (`src/presentation/`)
- [ ] `oauth_handler.rs` with one fn per endpoint (form extractor for `/token`)
- [ ] Map new `DomainError`s → OAuth error responses in `errors.rs`
- [ ] Routes + `AppState` wiring
- [ ] Security-header + rate-limit Tower layers

Testing (`src/testing/`)
- [ ] In-memory repos for each new entity
- [ ] `oauth_client_repository()` etc. on `InMemoryUnitOfWork`
- [ ] Use-case tests: happy path + each attack (replay, PKCE fail, redirect
      mismatch, refresh reuse, scope escalation, cross-tenant)
- [ ] Consent tests: screen skipped when `covers()`, re-prompted on a new scope,
      revoked grant forces re-consent
- [ ] Signing tests: token verifies against the active key's public JWK; rotation
      keeps old tokens verifying until the key is `Retired`

Crypto (`src/utils/`)
- [ ] `oauth_crypto.rs` (`generate_opaque_token`, `hash_token`, `verify_pkce_s256`)

Operational
- [ ] Background cleanup of expired codes/sessions
- [ ] Audit logging
- [ ] HTTPS/HSTS enforced

---

## 10. Spec references

- **RFC 6749** — OAuth 2.0 Authorization Framework (the base)
- **OAuth 2.1** (draft) — the consolidated, secure-by-default profile you're building
- **RFC 7636** — PKCE
- **RFC 9700** — OAuth 2.0 Security Best Current Practice (read this fully)
- **RFC 6819** — OAuth 2.0 Threat Model and Security Considerations
- **OpenID Connect Core 1.0** — id_token, `/userinfo`, nonce, claims
- **OpenID Connect Discovery 1.0** — `/.well-known/openid-configuration`
- **RFC 7517 / 7518** — JWK / JWA (JWKS, signing algorithms)
- **RFC 7519** — JSON Web Token
- **RFC 7009** — Token Revocation
- **RFC 7662** — Token Introspection
- **RFC 8252** — OAuth for Native Apps (loopback redirect, custom schemes)

---

### Summary

You are building the **provider**. The "session code" you asked about is the
**`AuthorizationCode`** entity — short-lived, single-use, hashed at rest, and bound
to the client, user, redirect URI, scope, and the PKCE challenge. Around it you add
`OAuthClient` (who may ask), `AuthSession` (the interactive login state),
`RefreshToken` (rotated, reuse-detected), `ConsentGrant` (what the user approved —
checked in `authorize::complete` to skip/delta-prompt the consent screen), and
`SigningKey` (the rotatable asymmetric keys used by `token::exchange` to sign JWTs
and by `jwks` to publish verifiers). Each follows your existing Clean Architecture patterns exactly: pure
entity + repo trait in `domain/`, orchestration in `application/` use cases behind
the `UnitOfWork`, SQLx impls in `infraestructure/`, Axum handlers in `presentation/`,
and in-memory fakes for tests. The security comes from doing the boring things
rigorously: mandatory S256 PKCE, exact redirect matching, single-use hashed codes,
rotated/reuse-detected refresh tokens, asymmetric signing, constant-time
comparisons, strict scope intersection, and per-tenant isolation.
