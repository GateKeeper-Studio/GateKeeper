# Service Decoupling Plan

## Purpose

This document describes a performance-oriented and reliable path to decouple the current API server into multiple services. The goal is not to rewrite the system at once. The goal is to separate business capabilities at the right boundaries, reduce shared logic, make ownership explicit, and keep the identity provider reliable while the architecture evolves.

The current server is a Rust Axum API using Clean Architecture and DDD-style layers. It already has useful internal boundaries: domain entities, repository traits, use cases, ports, infrastructure adapters, and HTTP handlers. The migration should preserve those strengths and turn selected bounded contexts into independently deployable services only when their contracts and data ownership are stable.

## Target Service Boundaries

### API Gateway or BFF Layer

Keep a thin public entry point in front of the services. It should preserve the external API shape while internal routing changes.

Responsibilities:

- Route requests to internal services.
- Enforce request IDs, tracing headers, rate limits, CORS, compression, and security headers.
- Validate public tokens where possible, but delegate business authorization to owning services.
- Provide backward-compatible paths during migration.

This layer should not own business data.

### Identity Provider Service

Own the latency-sensitive OAuth 2.1 and OpenID Connect protocol surface.

Responsibilities:

- Authorization requests.
- Token exchange and refresh token rotation.
- Userinfo.
- OIDC discovery and JWKS.
- Authorization codes.
- Refresh tokens.
- Auth sessions.
- Signing keys.
- Consent grants.

This service is the most sensitive service in the split. It should be extracted late, after contracts, observability, and rollback paths are mature. Login and token flows should avoid unnecessary network hops.

### Tenant and User Service

Own tenant-scoped identity records and user lifecycle.

Responsibilities:

- Tenants.
- Users.
- User credentials.
- User profiles.
- Tenant email configuration references.
- User creation, update, deletion, anonymization state, and legal hold state.
- Email verification, password reset, and password change tokens if these remain tightly coupled to user lifecycle.

Other services should reference users and tenants by IDs. They should not query tenant or user tables directly once the boundary is enforced.

### Access Control Service

Own authorization data that is not token issuance itself.

Responsibilities:

- Groups.
- Group memberships.
- Roles.
- Role assignments.
- User group and user role lookup.

The Identity Provider Service may need fast access to effective roles/claims. Use one of these approaches:

- Synchronous lookup for admin or low-volume paths.
- Local projection in the Identity Provider Service for high-volume token issuance.
- Token claim generation based on cached role snapshots with short TTLs.

### MFA Service

Own second-factor enrollment and verification.

Responsibilities:

- MFA enrollments.
- MFA challenges.
- TOTP verification.
- Email OTP verification.
- WebAuthn/passkey enrollment and assertion verification.
- Preferred MFA method selection.

Authentication flows can call this service synchronously because MFA is part of the interactive login decision. Calls must have short timeouts and clear failure behavior.

### Admin and Organization Service

Own platform administration and organization-level state.

Responsibilities:

- Admin users.
- Admin refresh tokens.
- Organizations.
- Admin profile management.
- Organization profile management.

This service can be separated earlier than the core identity protocol because its traffic is lower volume and primarily dashboard-driven.

### Compliance and Audit Service

Own audit, retention, policy acceptance, export, and erasure orchestration.

Responsibilities:

- Audit events.
- Policy acceptances.
- Required legal documents for users.
- User data export orchestration.
- Right-to-erasure orchestration.
- Retention purge orchestration.

Audit writes should use asynchronous events where possible. Critical security and admin actions can use an outbox to guarantee audit event delivery without blocking the main transaction on another service.

### Security and Risk Service

Own login intelligence, risk policy, and security blocks.

Responsibilities:

- Login events.
- Login risk signals.
- Tenant risk policies.
- Security blocks.
- Suspicious IP and attacked-user reporting.
- Risk scoring.

This service can consume login events asynchronously and publish risk decisions or maintain a low-latency read API for login-time checks.

### Notification Service

Own outbound messaging configuration and delivery.

Responsibilities:

- Email providers.
- Email templates.
- Template rendering.
- Test email sending.
- Verification, reset, and MFA email delivery.

This service should expose a command API such as `send_email(template_key, tenant_id, recipient, variables, idempotency_key)`. Business services should request notifications; they should not render templates or talk directly to SMTP.

## Database Strategy

### Option 1: One Shared Database for All Services

This is the safest short-term migration option.

Benefits:

- Lowest operational complexity.
- Existing SQLx migrations can continue to run.
- Existing foreign keys preserve referential integrity.
- Easier debugging during the first extraction steps.
- Easier rollback to the monolith.

Costs:

- Services remain coupled through tables.
- Cross-service joins and shared transactions can keep business logic tangled.
- Database migrations become coordination-heavy.
- Performance tuning one service can affect all services.
- Service ownership is weaker because any service may be tempted to read another service's tables.

Use this only as a transition phase, not as the final architecture.

### Option 2: One Database per Service

This is the recommended final architecture for stable service boundaries.

Benefits:

- Strong ownership: each service owns its schema, migrations, credentials, indexes, backups, and performance tuning.
- Fewer accidental couplings.
- Independent scaling and maintenance.
- Better blast-radius control.
- Clearer compliance and data-retention ownership.

Costs:

- More operational complexity.
- No cross-service foreign keys.
- No distributed transactions.
- Eventual consistency must be designed explicitly.
- Reporting requires projections, data warehouse pipelines, or read models.
- Local development and migrations become more involved.

In this model, services store external references as IDs, not foreign keys. Integrity is enforced through service APIs, events, idempotent workflows, and reconciliation jobs.

### Recommended Database Path

Use a staged database migration:

1. **Current state: one shared Postgres database**
   - Keep the current database while extracting service interfaces.
   - Do not split physical databases before the service contracts are stable.

2. **Schema-per-service ownership inside the same Postgres instance**
   - Group tables by future owner.
   - Start moving migrations into service-owned folders or modules.
   - Give each service its own database role.
   - Restrict each role to its owned schema.
   - Remove new cross-service joins from application code.

3. **Outbox and event projections**
   - Add a transactional outbox per service-owned schema.
   - Publish domain events after local commits.
   - Build local read models for cross-service reads.

4. **Database-per-service where justified**
   - Move low-risk services first: Notification, Compliance/Audit reads, Security/Risk reporting.
   - Move Identity Provider data later because login and token issuance are latency-sensitive and strongly connected to users, tenants, OAuth clients, sessions, MFA, and signing keys.

5. **Remove cross-service foreign keys**
   - Replace foreign keys across service boundaries with ID references.
   - Keep foreign keys inside each service database.
   - Add reconciliation jobs to detect broken external references.

The final target should be database-per-service for core bounded contexts. The transition should use schema-per-service to reduce risk.

## Migration Roadmap

### Phase 1: Prepare the Monolith

Keep one deployable server, but make boundaries explicit.

Actions:

- Group use cases, repositories, entities, and ports by target service boundary.
- Document table ownership for every table.
- Prevent new cross-context repository usage in use cases.
- Add internal service traits in the application layer for cross-boundary calls.
- Keep handlers thin and route requests through use cases only.
- Add request IDs and trace context to all logs.

Exit criteria:

- Every table has one planned owner.
- Every use case has one planned service owner.
- Cross-context dependencies are visible as ports, not direct repository calls.

### Phase 2: Extract Low-Risk Services Behind Local Ports

Still deploy one binary, but replace direct in-process calls with service-shaped ports.

Good first candidates:

- Notification Service.
- Compliance/Audit read APIs.
- Security/Risk reporting.

Actions:

- Define command/query contracts for each candidate service.
- Add idempotency keys to commands that create external side effects.
- Add outbox events for important writes.
- Keep in-process adapters first.
- Add HTTP or gRPC adapters only after the interface is stable.

Exit criteria:

- Use cases call service ports instead of directly sharing repositories across boundaries.
- Contracts are covered by unit and integration tests.
- Side-effect commands are idempotent.

### Phase 3: Introduce Independent Deployments

Extract services one at a time.

Actions:

- Create one service binary per extracted boundary.
- Give each service its own health checks, metrics, tracing, config, and deployment pipeline.
- Put services behind the gateway or internal service discovery.
- Keep the old monolith path available during rollout.
- Use feature flags or route-level traffic shifting for gradual migration.

Exit criteria:

- Service can be deployed, rolled back, and observed independently.
- Public API behavior is unchanged.
- Dashboard and login clients do not need to know whether a request is served by the monolith or an extracted service.

### Phase 4: Split Data Ownership

Move from shared tables to service-owned schemas and then service-owned databases.

Actions:

- Move owned tables to service schemas.
- Add service-specific migration runners.
- Restrict database permissions by service role.
- Replace cross-service foreign keys with external IDs.
- Build projections for query paths that previously used joins.
- Add reconciliation jobs for cross-service references.

Exit criteria:

- Each service can run migrations for its own data.
- No service needs direct table access to another service's data.
- Cross-service reads use APIs or projections.

### Phase 5: Harden Identity and Authentication Splits

Extract the highest-risk flows only after the supporting services are stable.

Actions:

- Keep token issuance in the Identity Provider Service.
- Minimize synchronous calls in `/oauth/token`, `/oauth/authorize`, JWKS, and userinfo paths.
- Cache stable tenant, OAuth client, signing key, and role projection data with short TTLs.
- Use local projections for claims needed during token issuance.
- Keep MFA synchronous during interactive login, with short timeouts and explicit retry behavior.

Exit criteria:

- Login and token latency remain within SLO.
- Failed downstream services degrade predictably.
- Refresh token rotation remains atomic inside the Identity Provider Service database.

## Communication Patterns

### Synchronous APIs

Use synchronous HTTP or gRPC calls when the caller needs an immediate decision.

Good use cases:

- MFA verification during login.
- Tenant/user lookup required for interactive authentication.
- Admin dashboard commands where the user expects a direct result.

Rules:

- Use strict timeouts.
- Propagate trace context.
- Use typed error responses.
- Add circuit breakers for non-critical dependencies.
- Never hold a database transaction open while waiting for another service.

### Asynchronous Events

Use events when the caller does not need an immediate response.

Good use cases:

- Audit event recording.
- Email delivery.
- Risk signal enrichment.
- Reporting projections.
- Compliance export preparation.

Rules:

- Publish through a transactional outbox.
- Make consumers idempotent.
- Store event IDs and processed offsets.
- Version event schemas.
- Do not use events as hidden remote procedure calls.

### Transactional Outbox

Each service that changes state and emits events should write events to an outbox table in the same local transaction.

Flow:

1. Use case writes service-owned data.
2. Use case writes outbox event in the same transaction.
3. Local transaction commits.
4. Outbox worker publishes events to the message broker.
5. Consumers process events idempotently.

This avoids distributed transactions while preserving reliable event delivery.

## Reliability Requirements

### No Distributed Transactions

Do not use two-phase commit across services. Each service owns one local transaction. Cross-service workflows should use sagas, retries, compensating actions, and reconciliation.

### Idempotency

All commands that create state or external side effects must accept an idempotency key.

Examples:

- Send email.
- Record audit event.
- Start MFA challenge.
- Rotate client secret.

### Timeouts and Retries

Every service client should define:

- Connect timeout.
- Request timeout.
- Retry policy for transient failures.
- Maximum retry budget.
- Backoff with jitter.
- Non-retryable error classes.

Do not retry non-idempotent commands unless an idempotency key is used.

### Observability

Before splitting traffic, every service must emit:

- Structured logs.
- Request IDs.
- Distributed traces.
- Metrics for latency, throughput, errors, and saturation.
- Dependency call metrics.
- Database pool metrics.
- Outbox lag.
- Consumer lag.

Minimum useful SLOs:

- OAuth token endpoint latency and error rate.
- Login flow latency and error rate.
- MFA verification latency and error rate.
- Email delivery acceptance rate.
- Audit event publish lag.

### Rollback

Every extraction must have a rollback path:

- Keep the old in-process path until the new service is stable.
- Use feature flags or route-level traffic shifting.
- Keep database migrations backward-compatible during rollout.
- Avoid destructive schema changes until all old code paths are retired.

## Performance Guidance

### Keep Hot Paths Short

The following paths are hot or reliability-sensitive:

- `/oauth/token`
- `/oauth/authorize`
- JWKS
- OIDC discovery
- Userinfo
- Login
- Refresh session
- MFA verification

For these paths:

- Avoid unnecessary synchronous service calls.
- Cache stable configuration.
- Keep token rotation and session writes local to the owning service.
- Precompute or project claims needed for token issuance.
- Use bounded connection pools.
- Avoid long-running queries and unindexed lookups.

### Use Projections for Cross-Service Reads

Do not rebuild monolithic joins through service-to-service call chains. If a service frequently needs data owned by another service, create a local projection.

Examples:

- Identity Provider Service keeps a projection of OAuth client settings and tenant signing metadata.
- Security and Risk Service keeps a projection of tenant risk policy and login events.
- Compliance Service builds export projections asynchronously.

### Design for Failure

Each service dependency should have a defined failure mode:

- **Fail closed** for token issuance, MFA verification, authorization decisions, and security blocks.
- **Fail open with audit** only for non-critical reporting surfaces where availability is preferred and risk is acceptable.
- **Defer asynchronously** for email delivery, audit publishing, reporting, and analytics.

## Suggested Extraction Order

1. **Notification Service**
   - Lower business risk.
   - Clear external side effects.
   - Natural async boundary.

2. **Compliance and Audit Service**
   - Audit writes should move to events/outbox.
   - Reporting and export can tolerate asynchronous processing.

3. **Security and Risk Service**
   - Can consume login events asynchronously.
   - Risk decisions can later become a synchronous login dependency if needed.

4. **Admin and Organization Service**
   - Lower traffic and mostly dashboard-driven.
   - Useful for separating platform administration from tenant identity.

5. **Access Control Service**
   - Requires careful claim and projection design.
   - Should move after role/group read patterns are understood.

6. **MFA Service**
   - Login-critical and interactive.
   - Extract after service timeouts and failure handling are mature.

7. **Tenant and User Service**
   - Core identity data.
   - Strongly connected to authentication and compliance.

9. **Identity Provider Service**
   - Highest reliability and latency sensitivity.
   - Extract last, or keep it as the core service while others move out around it.

## Table Ownership Draft

| Table or Data Area | Target Owner |
|---|---|
| `tenants` | Tenant and User Service |
| `users`, `user_credentials`, `user_profiles` | Tenant and User Service |
| `email_verification_tokens`, `password_reset_tokens`, `password_change_tokens` | Tenant and User Service or Identity Provider Service, depending on final auth flow ownership |
| `oauth_clients` | Identity Provider Service |
| `authorization_codes`, `refresh_tokens`, `auth_sessions` | Identity Provider Service |
| `signing_keys` | Identity Provider Service |
| `consent_grants` | Identity Provider Service |
| `groups`, `group_members` | Access Control Service |
| `roles`, `role_assignments` | Access Control Service |
| `mfa_enrollments`, `mfa_challenges` | MFA Service |
| `admin_users`, `admin_refresh_tokens` | Admin and Organization Service |
| `organizations` | Admin and Organization Service |
| `audit_events`, `policy_acceptances`, `legal_documents` | Compliance and Audit Service |
| `login_events`, `login_risk_signals`, `security_blocks`, `tenant_risk_policies` | Security and Risk Service |
| `tenant_emails`, `email_templates` | Notification Service |
| `external_identity_providers`, `social_identities`, `external_login_flows` | Identity Provider Service, with possible adapter support from Tenant and User Service |

## Contract Guidelines

Each service contract should define:

- Command and query names.
- Request and response schemas.
- Error codes.
- Idempotency behavior.
- Authorization expectations.
- Timeout budget.
- Retry policy.
- Event schemas emitted by successful writes.
- Compatibility policy for version changes.

Use stable IDs between services. Do not expose internal database row structure as public service contracts.

## Testing Strategy

### Unit Tests

Keep fast use-case tests with in-memory fakes. Each service should preserve the current testing style: domain and application logic should not need Postgres.

### Contract Tests

Add contract tests for service clients and service servers. These tests should verify request/response schemas, error mapping, authentication headers, idempotency behavior, and backward compatibility.

### Integration Tests

Run service integration tests with real Postgres for service-owned migrations and repository behavior. Use a test database per service.

### End-to-End Tests

Protect user-facing flows:

- Register user.
- Confirm email.
- Login.
- MFA verification.
- Authorization code flow with PKCE.
- Token refresh rotation.
- Userinfo.
- Admin login.
- Tenant/user CRUD.
- Audit event publication.
- User export and erasure.

### Performance Tests

Before moving identity traffic, measure:

- Token endpoint p50, p95, p99 latency.
- Login flow p50, p95, p99 latency.
- MFA verification p95 latency.
- Database pool wait time.
- Cache hit rate for tenant/client/signing metadata.
- Outbox publish lag.

## Acceptance Criteria

A service is ready to run independently when:

- It owns a clear bounded context.
- It has a documented API contract.
- It has service-owned migrations or a schema ownership plan.
- It does not query another service's tables.
- It emits traces, metrics, and structured logs.
- It has health checks.
- It supports rollback.
- Its commands are idempotent where needed.
- Its consumers are idempotent.
- It has unit, contract, integration, and relevant end-to-end coverage.
- Its production SLOs and dashboards exist before traffic migration.

## Final Recommendation

Do not split everything immediately. Start by turning the existing monolith into a modular monolith with explicit service-shaped ports. Extract low-risk services first, especially Notification, Compliance/Audit reads, and Security/Risk reporting. Keep core OAuth/OIDC token issuance and authentication paths close to their data until the rest of the platform has proven service communication, outbox events, observability, and rollback discipline.

For data, use a shared Postgres database only as a migration step. Move next to schema-per-service ownership with restricted roles. The long-term target should be database-per-service for stable bounded contexts, with cross-service references represented by IDs, APIs, events, and local projections instead of shared joins and foreign keys.
