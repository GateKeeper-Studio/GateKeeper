# Compliance Implementation Plan — LGPD, GDPR, ISO 27001, SOC 2

> **Purpose of this document.** This is a *plan and reference*, not a finished
> implementation. It maps the requirements of **LGPD** (Lei Geral de Proteção de
> Dados, Brazil), **GDPR** (EU/EEA), **ISO/IEC 27001:2022**, and **SOC 2** (AICPA
> Trust Services Criteria) onto concrete, codebase-specific engineering tasks for
> this Rust multi-tenant OAuth2/OIDC Identity Provider. It is intended to be fed
> back to Claude as context so individual work items can be implemented in
> isolation, in any order, while keeping a coherent overall design.
>
> **How to use it.** Pick a Work Item (WI-xx). Each one states: what it is, which
> frameworks demand it, the data it touches, and the concrete files/layers to
> change following this project's Clean Architecture (`domain → application →
> infraestructure → presentation`, with `testing/` fakes). Treat the SQL and Rust
> sketches as direction, not literal final code.

---

## 0. System context (what we are protecting)

This service is an **Identity Provider**. Almost everything it stores is, by
definition, either personal data or a security-sensitive credential. That makes
the compliance surface unusually concentrated — getting a handful of cross-cutting
controls right (audit, consent, retention, deletion, encryption, access control)
satisfies the large majority of all four frameworks.

### 0.1 Personal-data inventory (the "Record of Processing", GDPR Art. 30 / LGPD Art. 37)

This table is itself a compliance deliverable. Keep it current; **WI-01** turns it
into a living document.

| Data element | Entity / table | Classification | Legal basis (typical) | Notes |
|---|---|---|---|---|
| Email address | `users` | PII (identifier) | Contract / Legitimate interest | Unique per tenant; also a login credential |
| First / last / display name | `user_profiles` | PII | Contract | |
| Password hash | `user_credentials` | Sensitive credential | Contract | Argon2/bcrypt hash + tenant pepper; never plaintext |
| Email-verification tokens | `email_verification_tokens` | Security token | Contract | Short-lived, hashed |
| Password reset / change tokens | `password_reset_tokens`, `password_change_tokens` | Security token | Contract | Short-lived, hashed |
| Consent grants (per OAuth client) | `consent_grants` | Consent record | **Consent** | Already models grant/revoke + scopes |
| Auth sessions | `auth_sessions` | Session / behavioural | Contract | Holds login state, SSO, IP-adjacent |
| Refresh tokens | `refresh_tokens` | Security token | Contract | Hashed, rotating, family-based reuse detection |
| Authorization codes | `authorization_codes` | Security token | Contract | Short-lived, single-use, hashed |
| Roles / Groups & assignments | `roles`, `groups`, `*_assignments`/`members` | Authorization metadata | Legitimate interest | Now also embedded in issued JWTs |
| OAuth clients | `oauth_clients` | Tenant config | Contract | Client secrets hashed |
| Tenant config + pepper | `tenants` | Secret material | — | `password_hash_secret` is a pepper; treat as a secret |
| Signing keys | `signing_keys` | Secret material (private keys) | — | JWT signing; rotation already modelled |

### 0.2 What is conspicuously **missing** today (the gap list)

These absences are the backbone of the roadmap:

1. **No audit/event log.** There is no immutable record of who did what, when
   (logins, admin actions, data access, consent changes, deletions). This is the
   single biggest gap — it is *required, explicitly or effectively, by all four
   frameworks*.
2. **No data-subject rights (DSR) machinery.** No export ("right to access /
   portability"), no erasure ("right to be forgotten"), no rectification workflow.
   GDPR Arts. 15–20, LGPD Art. 18.
3. **No retention / TTL enforcement.** Expired tokens, dead sessions, and stale
   accounts are never purged on a schedule. GDPR Art. 5(1)(e), LGPD Art. 16.
4. **No consent versioning / ToS-Privacy acceptance.** `consent_grants` covers
   OAuth scopes but not acceptance of *Terms / Privacy Policy* with versioning and
   timestamp.
5. **Encryption-at-rest and key management are undocumented** (likely DB-level
   only). ISO A.8.24, SOC 2 CC6.
6. **No formalized RBAC enforcement at the API layer** for admin actions (roles
   are issued in JWTs but enforcement middleware is not described). ISO A.8.3,
   SOC 2 CC6.1.
7. **No breach-notification runbook / incident process** (LGPD Art. 48 — ANPD +
   data subject; GDPR Art. 33/34 — 72h to supervisory authority).
8. **Policy & governance documents** (the human-process half of ISO/SOC 2) do not
   exist in-repo.

---

## 1. Framework cheat-sheet (what each one actually asks for)

> You do not implement "GDPR" as a feature. You implement *controls*; each control
> satisfies clauses across multiple frameworks. The cross-reference matrix in
> §3 is the real map. This section is the orientation.

### 1.1 LGPD (Brazil) — Law 13.709/2018
- **Legal bases** for processing (Art. 7) and for sensitive data (Art. 11).
- **Data-subject rights** (Art. 18): confirmation, access, correction,
  anonymization/blocking/deletion, portability, info on sharing, revoke consent.
- **Consent** must be specific, informed, and revocable (Arts. 8–9).
- **Security & governance** (Arts. 46–50): technical/administrative measures,
  good-practice program, incident response.
- **Breach notification** to ANPD and data subjects "in a reasonable time" (Art.
  48).
- **DPO ("Encarregado")** designation (Art. 41).
- Penalties: up to 2% of Brazil revenue, capped R$50M per infraction.

### 1.2 GDPR (EU/EEA) — Regulation 2016/679
- **Principles** (Art. 5): lawfulness, purpose limitation, data minimization,
  accuracy, **storage limitation**, integrity/confidentiality, **accountability**.
- **Lawful basis** (Art. 6) + conditions for consent (Art. 7).
- **Data-subject rights** (Arts. 12–22): access, rectification, **erasure**,
  restriction, **portability** (machine-readable export), object, automated-decision
  safeguards.
- **Data Protection by Design & by Default** (Art. 25).
- **Records of processing** (Art. 30).
- **Security of processing** (Art. 32) — encryption, resilience, testing.
- **Breach notification** (Arts. 33/34): **72 hours** to supervisory authority.
- **DPIA** (Art. 35) for high-risk processing.
- **DPO** (Arts. 37–39); **international transfer** rules (Arts. 44–49).

### 1.3 ISO/IEC 27001:2022
- An **ISMS** (Information Security Management System): scope, risk assessment,
  Statement of Applicability, management review, continual improvement (Clauses
  4–10).
- **Annex A** — 93 controls in 4 themes. The ones with direct code impact:
  - **A.5 Organizational** — policies, supplier security, IR (A.5.24–.28), threat
    intel, classification (A.5.12).
  - **A.8 Technological** — access control (A.8.2/8.3/8.5 MFA), crypto (A.8.24),
    logging (**A.8.15**), monitoring (A.8.16), secure dev (A.8.25–8.28),
    backup (A.8.13), data masking (A.8.11), DLP (A.8.12), deletion (A.8.10).
- Certification = external audit; the artifact is *evidence* the controls operate.

### 1.4 SOC 2 (AICPA TSC 2017, rev. 2022)
- Five **Trust Services Criteria**: **Security (Common Criteria, required)**,
  Availability, Processing Integrity, Confidentiality, Privacy.
- **Common Criteria** map to COSO: CC1 governance, CC2 communication, CC3 risk,
  CC4 monitoring, **CC5 control activities**, **CC6 logical/physical access**,
  **CC7 system operations (incident, change detection)**, **CC8 change management**,
  CC9 risk mitigation.
- **Type I** = controls designed at a point in time; **Type II** = controls
  *operated effectively over a period* (3–12 months) — requires sustained evidence
  (logs, tickets, reviews). This is why **audit logging + retention + review
  cadence** are non-negotiable.

---

## 2. Cross-cutting foundations (build these first — they unlock everything)

These are the shared primitives. Almost every framework clause traces back to one
of them. Implement in this order.

### WI-01 — Data Processing Register + Data Classification (documentation)
**Frameworks:** GDPR Art. 30 · LGPD Art. 37 · ISO A.5.12 · SOC 2 CC3/Privacy
**Type:** docs + a small code annotation convention.
- Promote §0.1 to a maintained doc: `docs/compliance/data-processing-register.md`.
- Establish a **data-classification convention**: `Public | Internal | Confidential
  | Restricted (PII/secret)`. Annotate each domain entity with a one-line doc
  comment stating its class and legal basis (cheap, high audit value).
- Deliverable: register + a `CLASSIFICATION.md` legend.

### WI-02 — Audit / Security Event Log (the keystone)
**Frameworks:** GDPR Art. 5(2)/30 · LGPD Art. 37 · **ISO A.8.15/8.16** · **SOC 2
CC7.2/CC7.3, CC6.1** · SOC 2 Type II evidence.
**Type:** full vertical slice (new entity → repo → use case → emission points).

Design — an **append-only** audit trail. Never `UPDATE`/`DELETE` rows.

Migration `*_create_audit_events`:
```sql
CREATE TABLE IF NOT EXISTS "audit_events" (
  id            UUID PRIMARY KEY,                 -- uuid_v7 (time-sortable)
  tenant_id     UUID NULL,                        -- NULL for platform-level events
  actor_type    VARCHAR(32)  NOT NULL,            -- 'user' | 'client' | 'system' | 'admin'
  actor_id      UUID NULL,                        -- who performed it
  action        VARCHAR(128) NOT NULL,            -- 'user.login', 'consent.revoked', 'user.deleted', ...
  resource_type VARCHAR(64)  NULL,                -- 'user' | 'oauth_client' | 'role' ...
  resource_id   UUID NULL,
  outcome       VARCHAR(16)  NOT NULL,            -- 'success' | 'failure'
  ip_address    INET NULL,
  user_agent    TEXT NULL,
  metadata      JSONB NOT NULL DEFAULT '{}',      -- action-specific, MUST be PII-minimal
  created_at    TIMESTAMPTZ  NOT NULL DEFAULT now()
);
CREATE INDEX idx_audit_tenant_created ON audit_events (tenant_id, created_at DESC);
CREATE INDEX idx_audit_actor          ON audit_events (actor_id, created_at DESC);
CREATE INDEX idx_audit_action         ON audit_events (action, created_at DESC);
-- Enforce append-only at the DB: REVOKE UPDATE, DELETE from the app role.
```

Layers:
- `domain/entities/audit_event.rs` — `AuditEvent` value object + an `Action` enum
  (or `&'static str` constants) to avoid stringly-typed drift.
- `domain/repositories/audit_event_repository.rs` — `record(&AuditEvent)`,
  `find_by_tenant(tenant_id, filters, page, page_size)`, `find_by_actor(...)`.
- `application/ports` — an **`AuditLogger` port** (`async fn record(event)`), so use
  cases depend on the port, not the repo. Critically: **audit writes must not roll
  back with the business transaction** for *failure* events (a failed login must be
  logged even though the tx aborts) — give the logger its own connection/uow, or
  write it outside the business `uow`. For *success* events that must be atomic with
  the change (e.g., "user deleted"), write within the same `uow`.
- `infraestructure/database/repositories/audit_event_repository.rs` — `DbAuditEventRepository`.
- **Emission points** (instrument these existing use cases): authentication
  (login success/failure, logout), password reset/change, email verification,
  consent grant/revoke, token issuance/refresh/reuse-detection, every admin CUD on
  tenants/users/clients/roles/groups, and **every DSR action** (WI-05/06).
- `presentation/handlers/audit_handler.rs` — `GET /tenants/{t}/audit-events`
  (admin-only, paginated, filterable). Read-only.
- Testing: `InMemoryAuditEventRepository` + a `FakeAuditLogger` recording emitted
  events so use-case tests assert "this action emitted that audit event".

> **Why first:** SOC 2 Type II and ISO A.8.15 cannot be evidenced without this, and
> every later WI (deletion, consent, DSR) must emit audit events. Build the rail
> before the trains.

### WI-03 — Retention & TTL enforcement (storage limitation)
**Frameworks:** GDPR Art. 5(1)(e) · LGPD Art. 16 · ISO A.8.10 · SOC 2 Privacy/CC.
**Type:** scheduled job + per-table policy + config.
- Define a **retention policy table** (doc + optional `retention_policies` config)
  stating TTL per data class: expired auth codes (minutes), used/expired refresh
  tokens (e.g., 30–90d after expiry), dead `auth_sessions`, verification/reset
  tokens (hours), audit events (e.g., **1–7 years** — *longer*, deliberately; logs
  are evidence), inactive accounts (policy-driven, e.g., notify then anonymize).
- Implement a **purge use case** per data class
  (`use_cases/compliance/purge_expired_*`) that deletes rows past TTL and **emits an
  audit event** summarizing counts.
- Driver: a scheduled runner (cron/`tokio` interval task / external scheduler
  invoking an admin endpoint). Make each purge idempotent and batched.
- **Caution:** never purge audit logs below the *legal* minimum; retention here is a
  floor, not a ceiling.

### WI-04 — Consent & Policy-version acceptance
**Frameworks:** GDPR Arts. 6/7 · LGPD Arts. 7–9 · SOC 2 Privacy.
**Type:** extend existing consent model + new policy-acceptance slice.
- `consent_grants` already handles **OAuth-scope** consent well (grant/revoke/widen).
  Extend audit emission on every change (WI-02).
- Add **legal-document acceptance**: migration `*_create_policy_acceptances`:
  ```sql
  CREATE TABLE policy_acceptances (
    id UUID PRIMARY KEY, tenant_id UUID NOT NULL, user_id UUID NOT NULL,
    document_type VARCHAR(32) NOT NULL,   -- 'terms_of_service' | 'privacy_policy'
    document_version VARCHAR(32) NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL, ip_address INET NULL,
    UNIQUE (user_id, document_type, document_version)
  );
  ```
- New use cases: `record_policy_acceptance`, `get_user_acceptances`,
  `list_required_documents` (so the UI can force re-acceptance when a version bumps).
- Store **legal basis per processing purpose** as config so the privacy notice and
  the register (WI-01) stay in sync.

### WI-05 — Data-Subject Access & Portability (export)
**Frameworks:** GDPR Arts. 15/20 · LGPD Art. 18 (II, V) · SOC 2 Privacy.
**Type:** read-heavy use case producing a machine-readable bundle.
- `use_cases/compliance/export_user_data` — aggregates everything tied to a
  `user_id` across tenant boundary checks: profile, email, verification status,
  consent grants, policy acceptances, roles/groups, active sessions (metadata, **not**
  token secrets), and a **redacted** audit slice. Output: **JSON** (portability =
  structured, commonly used, machine-readable). Never include password hashes,
  token hashes, or signing keys.
- Endpoints: self-service `GET /me/data-export` (authenticated user) **and**
  admin `GET /tenants/{t}/users/{id}/data-export`.
- **Emit an audit event** for every export (it is itself access to PII).
- Consider async generation + signed download link for large exports; for this
  dataset size, synchronous JSON is acceptable initially.

### WI-06 — Right to Erasure / Anonymization ("right to be forgotten")
**Frameworks:** GDPR Art. 17 · LGPD Art. 18 (VI) · ISO A.8.10 · SOC 2 Privacy.
**Type:** write use case, transactional, with a deliberate strategy choice.

The hard design decision: **hard-delete vs. anonymize**. Recommended hybrid:
- **Anonymize** the `users`/`user_profiles` row (scrub email → a tombstone like
  `deleted+<uuid>@anonymized.invalid`, null names, set `is_active=false`,
  `anonymized_at`) so referential history (audit, who-did-what) survives without PII.
- **Hard-delete** cascade-safe satellites: credentials, all tokens, sessions,
  consent grants, role/group assignments.
- **Preserve** audit events but ensure their `metadata` was PII-minimal from the
  start (WI-02 design) — pseudonymize any residual PII in metadata.
- Add columns: `users.anonymized_at TIMESTAMPTZ NULL`; treat anonymized users as
  non-authenticatable everywhere (login, token, reset must reject).
- Use case `use_cases/compliance/erase_user` runs in **one `UnitOfWork`**, calls each
  repo's deletion, then commits; **emits an audit event** (`user.erased`) with the
  actor and legal basis. Self-service (`DELETE /me`) and admin variants.
- **Legal-hold guard:** refuse erasure if an active legal hold / open obligation
  flag is set (a `legal_holds` concept; can be a simple flag initially).

### WI-07 — Rectification (already mostly present)
**Frameworks:** GDPR Art. 16 · LGPD Art. 18(III).
- Existing profile-update use cases satisfy correction. Just ensure they **emit
  audit events** and are reachable by the data subject (self-service), not only
  admins.

### WI-08 — Access Control hardening + MFA readiness
**Frameworks:** ISO A.5.15/A.8.2/A.8.3/A.8.5 · SOC 2 CC6.1/CC6.2/CC6.3 · GDPR
Art. 32 · LGPD Art. 46.
**Type:** presentation-layer middleware + policy.
- **RBAC enforcement middleware**: now that roles/groups are in the JWT, add an
  Axum extractor/middleware that authorizes admin endpoints against required roles
  (e.g., `tenant_admin`). Today's gap: claims are *issued* but enforcement at the
  API edge is not described. Centralize it; deny-by-default.
- **MFA/2FA**: design hook now even if implemented later (TOTP). Note it as the
  primary lever for ISO A.8.5 and SOC 2 CC6.1. Add `user_credentials.mfa_*`
  placeholders in the design.
- **Least privilege on the DB role**: the app's Postgres role should not have
  superuser; `REVOKE UPDATE/DELETE` on `audit_events` (WI-02).
- **Session controls**: idle + absolute timeout already exist in `auth_sessions`;
  document them and add forced re-auth for sensitive actions.

### WI-09 — Cryptography & Key Management policy
**Frameworks:** ISO A.8.24 · SOC 2 CC6.1/CC6.7 · GDPR Art. 32 · LGPD Art. 46.
**Type:** docs + config + verification.
- Document: **in transit** TLS 1.2+ everywhere; **at rest** DB encryption (verify
  the deployment actually enables it — RDS/cloud volume encryption or `pgcrypto`
  for column-level on the most sensitive fields).
- **Secrets**: tenant pepper (`tenants.password_hash_secret`) and **signing private
  keys** must come from a secrets manager / KMS, not plaintext env or DB columns in
  the clear. Document current state and the target. Signing-key **rotation** is
  already modelled — document the cadence (e.g., 90 days) and the JWKS overlap
  window.
- Password hashing: confirm Argon2id (or bcrypt cost ≥ 12) + per-tenant pepper.
  Record parameters.

### WI-10 — Incident Response & Breach Notification
**Frameworks:** GDPR Arts. 33/34 (**72h**) · LGPD Art. 48 · ISO A.5.24–A.5.28 ·
SOC 2 CC7.3/CC7.4/CC7.5.
**Type:** runbook (docs) + supporting detection hooks.
- `docs/compliance/incident-response-plan.md`: roles, severity levels, detection →
  triage → containment → eradication → recovery → **notification** → post-mortem.
- Notification clocks: GDPR **72h** to supervisory authority; LGPD "reasonable
  time" to ANPD + affected subjects. Pre-draft notification templates.
- Detection hooks the code should provide: alert on audit-log signals already
  available — repeated login failures, refresh-token **reuse detection** (already
  implemented!), mass-export, mass-delete, privilege escalation. Wire WI-02 events
  into alerting (WI-11).

### WI-11 — Monitoring, Alerting & Availability
**Frameworks:** ISO A.8.15/A.8.16 · SOC 2 CC7.2 + Availability criteria.
**Type:** ops + light code (health/metrics endpoints).
- Structured logging (already idiomatic in Rust via `tracing`?) → ship to a
  central, **tamper-evident** store with retention ≥ audit policy.
- `/health` + `/ready` endpoints; uptime SLO; backup + **restore test** evidence
  (ISO A.8.13). SOC 2 Availability wants demonstrated backup *and* recovery.

### WI-12 — Secure SDLC & Change Management
**Frameworks:** ISO A.8.25–A.8.31 · SOC 2 CC8.1/CC3.
**Type:** CI/process.
- CI gates: `cargo test`, `cargo clippy -D warnings`, `cargo audit`
  (dependency CVEs — ISO A.8.8 vuln mgmt), `cargo deny` (license/supply chain),
  secret scanning, SAST. PR review required (segregation of duties — SOC 2 CC8).
- Migration review discipline (down-migrations exist — good). Tag releases;
  changelog = change-management evidence.
- **DPIA** (GDPR Art. 35) doc for the IdP processing — template in
  `docs/compliance/`.

---

## 3. Requirement → Work-Item traceability matrix

The proof that the WIs above actually *cover* the frameworks. (Abbrev: each cell =
the WI that primarily discharges the requirement.)

| Concern | LGPD | GDPR | ISO 27001:2022 | SOC 2 TSC | Work item |
|---|---|---|---|---|---|
| Record of processing / inventory | Art. 37 | Art. 30 | A.5.9/5.12 | CC3, Privacy | WI-01 |
| Audit / event logging | Art. 37 | Art. 5(2) | A.8.15/8.16 | CC7.2/7.3, CC6.1 | **WI-02** |
| Retention / storage limitation | Art. 16 | Art. 5(1)(e) | A.8.10 | Privacy, CC | WI-03 |
| Consent (specific, revocable) | Arts. 8–9 | Arts. 6–7 | A.5.34 | Privacy | WI-04 |
| Policy/ToS version acceptance | Art. 8 | Art. 7 | A.5.34 | Privacy | WI-04 |
| Right of access | Art. 18 II | Art. 15 | A.5.34 | Privacy P5 | WI-05 |
| Data portability | Art. 18 V | Art. 20 | — | Privacy | WI-05 |
| Right to erasure / anonymize | Art. 18 VI | Art. 17 | A.8.10 | Privacy P4 | WI-06 |
| Rectification | Art. 18 III | Art. 16 | — | Privacy | WI-07 |
| Access control / least privilege | Art. 46 | Art. 32 | A.5.15/8.2/8.3 | CC6.1–6.3 | WI-08 |
| MFA | Art. 46 | Art. 32 | A.8.5 | CC6.1 | WI-08 |
| Encryption in transit/at rest | Art. 46 | Art. 32 | A.8.24 | CC6.1/6.7 | WI-09 |
| Key & secret management | Art. 46 | Art. 32 | A.8.24 | CC6.1 | WI-09 |
| Incident response | Art. 48 | Arts. 33/34 | A.5.24–5.28 | CC7.3–7.5 | WI-10 |
| Breach notification clock | Art. 48 | Art. 33 (72h) | A.5.26 | CC7.4 | WI-10 |
| Monitoring / detection | — | Art. 32 | A.8.16 | CC7.2 | WI-11 |
| Availability / backup+restore | — | Art. 32(1)(c) | A.8.13 | Availability A1 | WI-11 |
| Vulnerability mgmt / supply chain | Art. 46 | Art. 32 | A.8.8/8.28 | CC7.1, CC8 | WI-12 |
| Secure SDLC / change mgmt | Art. 50 | Art. 25 | A.8.25–8.31 | CC8.1 | WI-12 |
| Data protection by design/default | Art. 46 | Art. 25 | A.8.27 | CC3 | all (principle) |
| DPO / Encarregado | Art. 41 | Arts. 37–39 | A.5.2 (roles) | CC1 | governance (docs) |
| DPIA / risk assessment | — | Art. 35 | Clause 6.1 / A.5.7 | CC3 | WI-12 + governance |
| Vendor/sub-processor mgmt | Art. 39 | Art. 28 | A.5.19–5.23 | CC9.2 | governance (docs) |

**Reading the matrix:** WI-02 (audit) and WI-08/09 (access + crypto) light up the
most cells. If forced to triage, that is the order. The "governance (docs)" rows
are **non-code** deliverables but are still mandatory for ISO certification and a
SOC 2 report — they live in `docs/compliance/` as policies.

---

## 4. Codebase integration conventions

So that every WI lands consistently in this Clean-Architecture project:

- **New bounded concept** (audit, policy-acceptance, retention) → its own
  `domain/entities/*.rs` + `domain/repositories/*_repository.rs` trait, registered
  in `entities.rs` / `repositories.rs`.
- **New cross-cutting behavior** (audit emission) → an **application port**
  (`application/ports/*.rs`) so use cases stay infrastructure-agnostic; concrete impl
  in `infraestructure/`.
- **Write operations** → take `Arc<dyn UnitOfWorkFactory>`, do work inside one
  `uow`, `commit()`. **Reads** → take repository `Arc`s directly. (Per `CLAUDE.md`.)
- **Audit caveat:** failure-path audit events must be written **outside** the
  business transaction (which rolls back), so the `AuditLogger` port needs its own
  short transaction/connection. Success-path "state changed" events go **inside** the
  business `uow` for atomicity. Decide per emission point.
- **New compliance use cases** → group under `use_cases/compliance/` (export, erase,
  purge_*) with the standard `use_case.rs` / `command.rs|query.rs` / `response.rs` /
  `tests.rs` layout.
- **Testing** → every new repo gets an `InMemory*` fake with draft/commit semantics
  in `testing/in_memory_repositories/`, wired into `InMemoryUnitOfWork(+Factory)`;
  every new port gets a recording `Fake*` in `testing/`. All tests stay DB-free.
- **Migrations** → `sqlx migrate add -r <name>`, fill `.up.sql`/`.down.sql`, run
  `sqlx migrate run` **before** `cargo build` (sqlx macros are compile-time checked).
- **Multi-tenancy is a security boundary**: every compliance query/command must take
  and enforce `tenant_id`. Cross-tenant access is itself a reportable incident.

---

## 5. Phased roadmap (suggested sequence)

> Each phase is shippable and independently valuable. Ordering maximizes how many
> matrix cells turn green per unit of work.

**Phase 1 — Evidence foundation (unblocks SOC 2 Type II & ISO logging)**
WI-01 (register) · **WI-02 (audit log)** · WI-09 (crypto/secrets *documentation* +
verify at-rest). Outcome: every action is recorded; data map exists.

**Phase 2 — Data-subject rights (the GDPR/LGPD core)**
WI-04 (consent/policy versioning) · WI-05 (export) · WI-06 (erasure) · WI-07
(rectification self-service) · WI-03 (retention purges). Outcome: all DSRs
serviceable end-to-end, every one audited.

**Phase 3 — Security controls & ops**
WI-08 (RBAC enforcement + MFA design) · WI-11 (monitoring/backup-restore) · WI-10
(incident-response runbook + detection wiring).

**Phase 4 — Governance & assurance**
WI-12 (secure SDLC/CI gates, `cargo audit`/`deny`, DPIA) + the docs-only governance
deliverables (policies, DPO designation, vendor register, Statement of
Applicability for ISO, SOC 2 control narratives).

---

## 6. Verification per WI (definition of done)

For any code-bearing WI:
1. `sqlx migrate run` then `cargo build` — schema + wiring compile.
2. `cargo test` — in-memory unit tests pass (new fakes used; no DB).
3. New behavior asserts an **audit event was emitted** where applicable.
4. `cargo clippy -D warnings` clean.
5. Tenant isolation covered by a test (cross-tenant access denied).
6. Dashboard slice (if any) — `npm run check` + `npm run build` clean.

For docs/governance WIs: the artifact exists in `docs/compliance/`, is referenced
from the register, and names an owner + review cadence.

---

## 7. Non-code (governance) deliverables checklist

Required for ISO certification and a clean SOC 2 report; cannot be satisfied by code
alone. Stub these in `docs/compliance/` so they are tracked:

- [ ] Information Security Policy (ISO Clause 5.2 / A.5.1)
- [ ] Risk Assessment + Treatment Plan + **Statement of Applicability** (ISO 6.1/A)
- [ ] Data Protection / Privacy Policy (public-facing) + internal data-handling SOP
- [ ] Data Processing Register (WI-01) + ROPA
- [ ] DPIA for the IdP processing (GDPR Art. 35)
- [ ] Incident Response Plan + breach-notification templates (WI-10)
- [ ] Business Continuity / Backup & Restore policy (ISO A.5.29/A.8.13)
- [ ] Access Control Policy + user access review cadence (ISO A.5.15–5.18)
- [ ] Cryptography & Key Management Policy (WI-09)
- [ ] Secure Development Policy + change management (WI-12)
- [ ] Supplier / sub-processor security & DPA register (GDPR Art. 28)
- [ ] DPO / Encarregado designation + contact published
- [ ] Retention Schedule (WI-03) with legal minimums per data class
- [ ] Acceptable Use / asset management / onboarding-offboarding (ISO A.5.9–5.11)

---

### Appendix A — Quick "where does it live" map for new compliance code

```
src/
  domain/
    entities/         audit_event.rs, policy_acceptance.rs, (legal_hold.rs)
    repositories/     audit_event_repository.rs, policy_acceptance_repository.rs
  application/
    ports/            audit_logger.rs            # the AuditLogger port
    use_cases/
      compliance/     export_user_data/, erase_user/, purge_expired_tokens/,
                      purge_expired_sessions/, record_policy_acceptance/,
                      list_user_acceptances/
  infraestructure/
    database/
      repositories/   audit_event_repository.rs, policy_acceptance_repository.rs
      audit_logger.rs # DbAuditLogger (own tx for failure-path events)
  presentation/
    handlers/         audit_handler.rs, compliance_handler.rs (export/erase/me)
    middleware/       rbac.rs (require_role), audit context (ip/ua extraction)
  testing/
    in_memory_repositories/  in_memory_audit_event_repository.rs, ...
    fakes:                    FakeAuditLogger (records emitted events)
migrations/
  *_create_audit_events, *_create_policy_acceptances,
  *_add_anonymized_at_to_users, *_create_legal_holds (optional)
docs/compliance/
  data-processing-register.md, incident-response-plan.md, dpia.md,
  retention-schedule.md, policies/...
```

### Appendix B — Action name catalog (seed for WI-02 `action` field)

```
user.created  user.login.success  user.login.failure  user.logout
user.email.verification_sent  user.email.verified
user.password.reset_requested  user.password.reset  user.password.changed
user.profile.updated  user.erased  user.anonymized  user.data_exported
consent.granted  consent.revoked  consent.scopes_widened
policy.accepted
token.issued  token.refreshed  token.reuse_detected  token.revoked
oauth_client.created  oauth_client.updated  oauth_client.deleted
role.created role.updated role.deleted role.assigned role.unassigned
group.created group.updated group.deleted group.member_added group.member_removed
tenant.created tenant.updated tenant.deleted
admin.access_denied  data.purged
```

---

*End of plan. Implement WIs incrementally; keep this document and the data register
in sync as the schema evolves.*
