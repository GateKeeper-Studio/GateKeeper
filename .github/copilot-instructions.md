# GateKeeper — AI Coding Instructions

## Architecture Overview

GateKeeper is an **OIDC-compliant Identity Provider** with three components:

| Component                            | Stack                                 | Port  | Path           |
| ------------------------------------ | ------------------------------------- | ----- | -------------- |
| **Server** (Go API)                  | Chi router, pgx/v5, SQLC, JWT (HS256) | :8080 | `server/`      |
| **Dashboard** (IDP frontend)         | Next.js 16, ShadCN/Radix, SWR, Axios  | :3000 | `next-app/`    |
| **Client Test** (sample OIDC client) | Next.js 15, native fetch              | :3001 | `client-test/` |

The server is the single source of truth. The dashboard hosts **both** the admin panel (`/dashboard`) and the **auth pages** (`/auth/[applicationId]/sign-in`, `mfa-*`, etc.) that client apps redirect to. Client-test is a consumer — it must **never** contain auth/MFA logic; that belongs to the IDP.

---

## OIDC Authorization Code Flow (with PKCE)

```
Client → IDP Login Page → [MFA?] → SessionCode → Authorize (PKCE) → AuthorizationCode → Token Exchange → JWT + ID Token
```

### Flow Steps

1. **Client initiates login** — generates `state`, `nonce`, PKCE `code_verifier`/`code_challenge`, stores them in httpOnly cookies, redirects to `DASHBOARD_URL/auth/{appId}/sign-in?redirect_uri=...&code_challenge=...&state=...&nonce=...`

2. **Login** (`POST /v1/auth/login`) — verifies credentials, branches into MFA or returns a `SessionCode` (56 random chars, 15-min TTL, one-time use)

3. **MFA verification** (if required) — `POST /v1/auth/verify-mfa/{email|app|webauthn}` → produces same `SessionCode` on success

4. **Authorize** (`POST /v1/auth/authorize`) — validates `response_type=code`, PKCE method, consumes + deletes `SessionCode`, creates `ApplicationAuthorizationCode` (128 random chars, 5-min TTL, stores `code_challenge`, `nonce`, `redirect_uri`)

5. **Token exchange** (`POST /v1/auth/sign-in`) — verifies `code_verifier` against stored `code_challenge` (PKCE), validates `client_secret`, `redirect_uri` exact match, `client_id` match. Deletes auth code (one-time use). Returns access token (JWT, 15 min), ID token (if `openid` scope, includes `nonce`), refresh token

### External OAuth (Google/GitHub)

Uses **double-PKCE**: GateKeeper generates its own `code_verifier` for the external provider while preserving the client's original PKCE params in `ExternalOAuthState`. On callback, if adaptive MFA requires verification, the server redirects directly to the IDP's MFA pages (`DASHBOARD_URL/auth/{appId}/mfa-*`), never back to the client.

### JWT Token Claims

- **Access token**: `sub`=userID, `app_id`, `aud`=issuer URL, `iss`=`ISSUER_URL`, `exp`=15min
- **ID token**: `sub`=userID, `aud`=clientID (different from access token!), `nonce`=from authorize, `auth_time`

### Security Controls

| Control           | Implementation                                                   |
| ----------------- | ---------------------------------------------------------------- |
| PKCE              | Required (`S256` or `plain`), verified at token exchange         |
| State             | Client-provided, echoed unmodified for CSRF protection           |
| Nonce             | Stored on auth code → embedded in ID Token for replay protection |
| SessionCode       | 56-char random, 15-min TTL, one-time use                         |
| AuthorizationCode | 128-char random, 5-min TTL, one-time use                         |
| Redirect URI      | Exact match at token exchange                                    |
| Passwords         | Argon2id hashing via `golang.org/x/crypto`                       |

---

## Adaptive MFA Policy Engine

Located at `internal/domain/services/mfa_policy.go`. Decides whether local MFA is required after external provider authentication based on AMR (Authentication Methods References, RFC 8176) claims.

### Decision Rules (evaluated in order)

| #   | Condition                                                    | Requires MFA? | Rationale                                   |
| --- | ------------------------------------------------------------ | ------------- | ------------------------------------------- |
| 1   | `Application.RequiresHighSecurity` + user has MFA configured | **Yes**       | High-security apps always enforce local MFA |
| 1b  | `RequiresHighSecurity` but user has NO MFA                   | No            | Can't enforce what's not configured         |
| 2   | User is nil or has no `Preferred2FAMethod`                   | No            | No MFA method to challenge                  |
| 3   | Auth provider is `email` (local login)                       | **Yes**       | Login handler already handles local MFA     |
| 4   | External provider + AMR contains a strong value              | No            | Provider already performed strong auth      |
| 5   | External provider + no strong AMR                            | **Yes**       | Weak external auth → require local MFA      |

**Strong AMR values**: `mfa`, `otp`, `hwk`, `swk`, `sms`, `pin`, `fpt`, `face`, `iris`, `retina`, `vbm`, `pop`, `sc`

### MFA Challenge Creation

`application_utils.CreateMfaChallenge()` dispatches by `user.Preferred2FAMethod`:

- **email**: creates `MfaEmailCode`, sends email asynchronously
- **totp**: creates `MfaTotpCode` record
- **webauthn**: calls `webauthn.BeginLogin()`, stores `MfaPasskeySession`, returns challenge options JSON

---

## Step-Up Authentication

Sensitive self-service operations require **re-authentication** before proceeding.

### Flow

1. Frontend calls `requestStepUp()` from `StepUpProvider` → opens `ReauthDialog`
2. User enters password (+ TOTP if configured) → `POST /v1/account/reauthenticate`
3. Server verifies credentials, creates `StepUpToken` (5-min TTL, time-based not single-use)
4. Frontend sends `X-Step-Up-Token` header on protected requests
5. `StepUpAuthHandler` middleware validates token existence + expiry → 403 if invalid

### Routes requiring step-up

`/v1/account/change-password`, `/v1/account/mfa/methods/{method}` (DELETE), `/v1/account/sessions`, `/v1/account/backup-codes`, `/v1/account/email/change`

---

## Server (Go)

### Module & Package Naming

- Module: `github.com/gate-keeper`
- Feature packages: lowercase concatenated folder name → `package getapplicationbyid`
- Shared utils: `package application_utils` at `internal/features/utils/`
- SQLC generated: `package pgstore` at `internal/infra/database/sqlc/`
- Domain services: `package services` at `internal/domain/services/` (imported as `mfa_policy`)

### Vertical Slice Handler Pattern

Each feature is a self-contained package at `internal/features/handlers/{domain}/{verb-noun}/` with these files:

| File                      | Purpose                                                              |
| ------------------------- | -------------------------------------------------------------------- |
| `command.go` / `query.go` | Input DTO with `json` + `validate` tags                              |
| `handler.go`              | Business logic implementing `ServiceHandler` or `ServiceHandlerRs`   |
| `endpoint.go`             | HTTP glue: parse request → wire `WithTransaction(Rs)` → send JSON    |
| `repository.go`           | Feature-scoped `IRepository` interface + composition of shared repos |
| `response.go`             | Output DTO with `json:"camelCase"` tags                              |

Handler factory: `func New(q *pgstore.Queries) repositories.ServiceHandlerRs[Query, *Response]`

### Error Handling

Handlers `panic(err)` with `*errors.CustomError` — the `ErrorHandler` middleware recovers and serializes to JSON with `correlation_id`. Errors are package-level vars in `internal/domain/errors/`.

### Transaction Pattern

All handler execution is wrapped in a DB transaction via generic `WithTransaction` / `WithTransactionRs`. The endpoint never touches the DB directly.

### Repository Composition

Feature repos define a minimal `IRepository` interface, then compose shared repos via embedding:

```go
type Repository struct {
    repositories.ApplicationRepository
    repositories.UserRepository
}
```

Convention: `nil, nil` return means "not found" (no error) — handler decides behavior.

### Domain Entities

Entities live in `internal/domain/entities/` — **no JSON tags** (serialization is in DTOs). Conventions:

- `NewXxx(...)` = reconstruct from DB, `AddXxx(...)` / `CreateXxx(...)` = new instance (generates UUID + timestamps)
- Nullable fields use pointers (`*string`, `*time.Time`)

### Database & Migrations

- SQLC config: `internal/infra/database/sqlc.yml`; queries in `commands/*.sql`; generated code in `sqlc/`
- Migrations: **Tern**, sequential `NNN_description.sql` in `internal/infra/database/migrations/`
- Separator: `---- create above / drop below ----`

### Key Commands

| Action         | Command / Task                                                                                 |
| -------------- | ---------------------------------------------------------------------------------------------- |
| Run server     | `go run ./cmd/server` or task "🚀 Run Server"                                                  |
| Watch mode     | `air` or task "🚀 Run Server (watch mode)"                                                     |
| Run tests      | `go test ./...` or task "Run Server Tests"                                                     |
| Generate SQLC  | `sqlc generate` from `internal/infra/database/` or task "Generate SQLC commands"               |
| Run migrations | `tern migrate` from `internal/infra/database/migrations/` or task "Generate Server Migrations" |
| Build check    | `go build ./...`                                                                               |

### Environment Variables

| Variable                           | Where       | Purpose                                                   |
| ---------------------------------- | ----------- | --------------------------------------------------------- |
| `JWT_SECRET`                       | server      | HS256 signing key for access/ID tokens                    |
| `ISSUER_URL`                       | server      | `iss` claim in tokens, OIDC discovery                     |
| `BASE_URL`                         | server      | Endpoint URLs in OIDC discovery document                  |
| `CLIENT_APPLICATION_URL`           | server      | Client-test origin (`:3001`), used as OAuth redirect base |
| `DASHBOARD_URL`                    | server      | IDP frontend origin (`:3000`), used for MFA redirects     |
| `WEBAUTHN_RPID`                    | server      | WebAuthn Relying Party ID (`localhost`)                   |
| `WEBAUTHN_RPORIGIN`                | server      | Comma-separated allowed origins for WebAuthn              |
| `MAIL_HOST/PORT/USERNAME/PASSWORD` | server      | SMTP configuration                                        |
| `NEXT_PUBLIC_BASE_API_URL`         | next-app    | Server API base URL for axios                             |
| `GATEKEEPER_CLIENT_ID`             | client-test | Application ID for OIDC flow                              |
| `GATEKEEPER_CLIENT_SECRET`         | client-test | Application secret for token exchange                     |
| `GATEKEEPER_SERVICE_URL`           | client-test | Server URL for token exchange                             |
| `GATEKEEPER_IDP_URL`               | client-test | IDP frontend URL (default `http://localhost:3000`)        |
| `SESSION_SECRET`                   | client-test | Symmetric key for encrypting session cookies              |

---

## Dashboard — next-app (IDP Frontend)

### Service Layer Pattern

- Base: `api` axios instance from `@/services/base/gatekeeper-api` (uses `NEXT_PUBLIC_BASE_API_URL`)
- Functions named `<verb><Entity>Api()` returning `Promise<Result<T, APIError>>` (tuple `[data, error]`)
- SWR hooks named `use<Entity>SWR()` in `services/dashboard/` with `revalidateOnFocus: false`, `dedupingInterval: 600000`
- Auth via `IServiceOptions.accessToken` → `Authorization: Bearer`
- Step-up via `X-Step-Up-Token` header on sensitive operations

### UI & Form Patterns

- ShadCN/Radix components in `@/components/ui/`
- Forms: `react-hook-form` + `zod` schema in co-located `auth-schema.ts`
- Route components co-located in `(components)/` folders within route segments
- Toasts: `sonner` with `richColors`

### Dashboard Structure

- **Layout**: `OrganizationsContextProvider` → `SidebarProvider` → `DashboardSidebar`
- **Sidebar**: Organization dropdown (persisted in cookie), applications list, settings dialog trigger
- **Header**: Responsive breadcrumbs (collapsing with ellipsis), command palette (`⌘K`)
- **Profile**: 4-tab self-service portal (Personal, Account, Security, Notifications) protected by `SessionProvider` → `StepUpProvider`

### Auth Pages

Auth UI at `/auth/[applicationId]/{sign-in,sign-up,mfa-mail,mfa-app,mfa-webauthn,...}`. These pages receive OIDC params (`redirect_uri`, `code_challenge`, `state`, `email`, `mfa_id`, `nonce`) via URL search params. After MFA verification, the page calls the authorize API and redirects to `redirect_uri` with the authorization code.

---

## Client Test (Sample OIDC Consumer)

Minimal app demonstrating the OIDC flow. Uses native `fetch`, PKCE (`S256`), and httpOnly cookies for session. The sign-in route generates `state` + `nonce` + `code_verifier` and redirects to the IDP. **All auth/MFA UI must stay in the IDP (next-app), never here.** Session management uses `SessionProvider` with auto-refresh scheduled 2 minutes before JWT expiry.

---

## Testing

- Server: `testify/mock` against `IRepository` interfaces. Mock struct asserts `var _ IRepository = (*mockRepo)(nil)`.
- Tests live alongside handlers as `handler_test.go`.
- Run: `go test ./...` or target specific packages: `go test ./internal/domain/services/... -v`
