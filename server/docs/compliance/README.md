# Compliance documentation (LGPD · GDPR · ISO 27001 · SOC 2)

This directory holds the **governance / evidence** half of the compliance program
described in [`../compliance-implementation-plan.md`](../compliance-implementation-plan.md).
The other half — the controls themselves — lives in code (`src/`) and the dashboard
(`dashboard/`).

> **Status legend:** ✅ implemented in code · 🟡 partial / in progress · 📄 documented only.

## What is implemented in code today

| Control | Where | Status |
|---|---|---|
| Append-only audit log (WI-02) | `src/domain/entities/audit_event.rs`, `src/infraestructure/database/{repositories/audit_event_repository.rs,audit_logger.rs}`, `migrations/*_create_audit_events` | ✅ |
| Admin audit-log query API | `GET /tenants/{id}/audit-events` → `presentation/handlers/audit_handler.rs` | ✅ |
| Bearer-auth + RBAC middleware (WI-08) | `src/presentation/middleware/auth.rs` (`require_admin`, `require_auth`, tenant isolation) | ✅ |
| Data-subject **export** (WI-05) | `GET /tenants/{id}/users/{uid}/data-export`, `GET /me/data-export` → `compliance_handler.rs`, `use_cases/compliance/export_user_data` | ✅ |
| Health / readiness (WI-11) | `GET /health`, `GET /ready` | ✅ |
| Data-subject **erasure** (WI-06) | `use_cases/compliance/erase_user` (hybrid anonymize + legal-hold guard), `POST /tenants/{id}/users/{uid}/erase`, `DELETE /me` | ✅ |
| Policy/ToS acceptance (WI-04) | `policy_acceptances` table + `record/list/required` use cases; admin + `/me` endpoints | ✅ |
| Retention purges (WI-03) | `purge_expired_data` use case (refresh tokens/sessions/auth codes), `POST /admin/maintenance/purge`, self-service `PUT /me/profile` | ✅ |
| Broad audit emission (WI-02) | login success/failure, access-denied, token issued/refreshed/reuse, all admin CUD (tenant/user/role/group/oauth-client + assignments + consent), all DSR events | ✅ |
| Dashboard compliance UI | Audit Log page, Compliance (purge) page, user **Privacy tab** (export/erase/policy), `audit.ts`/`compliance.ts` services, bearer-token attach in `api.ts` | ✅ |
| Dashboard admin **sign-in** | `admin_users` table + `POST /admin/login` (HS256 token) → `src/presentation/handlers/admin_auth_handler.rs`, `use_cases/admin_auth/login`, HS256 branch in `middleware/auth.rs`; dashboard `/login` page + `setAccessToken` storage + `dashboard/+layout.ts` guard + logout; bootstrap via `cargo run --bin create_admin`. `REQUIRE_ADMIN_AUTH=true` enforced. | ✅ |
| Password/email-flow audit emission | register, confirm/reset/change password | 🟡 not yet (login + admin CUD done) |

## Documents in this directory

| File | Work item | Frameworks |
|---|---|---|
| [data-processing-register.md](data-processing-register.md) | WI-01 | GDPR Art. 30 · LGPD Art. 37 |
| [data-classification.md](data-classification.md) | WI-01 | ISO A.5.12 |
| [retention-schedule.md](retention-schedule.md) | WI-03 | GDPR Art. 5(1)(e) · LGPD Art. 16 |
| [access-control-policy.md](access-control-policy.md) | WI-08 | ISO A.5.15/A.8.2/8.3 · SOC 2 CC6 |
| [crypto-key-management.md](crypto-key-management.md) | WI-09 | ISO A.8.24 · SOC 2 CC6.1/6.7 |
| [incident-response-plan.md](incident-response-plan.md) | WI-10 | GDPR Art. 33/34 · LGPD Art. 48 |
| [monitoring-availability.md](monitoring-availability.md) | WI-11 | ISO A.8.15/8.16 · SOC 2 CC7/A1 |
| [secure-sdlc.md](secure-sdlc.md) | WI-12 | ISO A.8.25–8.31 · SOC 2 CC8 |
| [dpia.md](dpia.md) | WI-12 | GDPR Art. 35 |
| [statement-of-applicability.md](statement-of-applicability.md) | — | ISO 27001 6.1.3 |

## Ownership & review cadence

Each document names an **Owner** and a **Review cadence** in its header. Default:
Owner = Security/Engineering lead; cadence = annually or on material change
(whichever first). The DPO/Encarregado (see incident-response-plan.md) owns the
data-subject-rights and breach-notification material.
