# Access Control Policy

> **Work item:** WI-08 · **Frameworks:** ISO A.5.15/A.5.18/A.8.2/A.8.3/A.8.5 · SOC 2 CC6.1–6.3 · GDPR Art. 32 · LGPD Art. 46
> **Owner:** Security lead · **Review:** annually + on role-model change.

## Principles

- **Deny by default.** The admin API surface (`/tenants/**`, audit, compliance) sits
  behind the `require_admin` middleware; a request without a valid token AND an admin
  role is rejected.
- **Least privilege.** Authorization is role-based (RBAC). Roles/groups are embedded
  in the access token and enforced at the API edge.
- **Tenant isolation is a security boundary.** Every admin request is checked so a
  token minted for tenant A cannot act on tenant B (`principal.tenant_id == path
  tenant_id`). Cross-tenant access is a reportable incident.

## Implementation (code)

| Control | Location |
|---|---|
| Bearer-token authentication | `src/presentation/middleware/auth.rs` — verifies JWT via `TokenVerifier` against the signing tenant's published keys |
| Principal + roles | `AuthPrincipal { user_id, tenant_id, roles, groups, scopes }` |
| Admin gate | `require_admin` (route_layer on admin router); `ADMIN_ROLES = ["admin","tenant_admin"]` |
| Self-service gate | `require_auth` (route_layer on `/me/**`) — valid token, no admin role |
| Tenant isolation | `principal.tenant_id` compared to path `tenant_id` in handlers + key-set selection by `tid` |
| Audit of denials | `admin.access_denied` action (to be emitted from the middleware) |
| Request context | `RequestContext` (IP, User-Agent) captured for audit |

Enforcement is unconditional (the former `REQUIRE_ADMIN_AUTH` toggle was removed).
Tenant tokens reach the admin API only with an admin role **and** the `admin` scope;
`/me/*` requires the `account` scope; platform admins reach only their own
organization's tenants.

## Roles

- `admin` / `tenant_admin` — full tenant administration via the dashboard.
- Additional roles are tenant-defined and matched by name in `roles` claim.
- **MFA (A.8.5):** design hook reserved on `user_credentials`; TOTP enrolment/verification
  is a planned enhancement and is the primary lever for CC6.1 step-up. _Not yet implemented._

## Reviews & lifecycle

- **User access reviews:** tenant admins review role/group assignments at least
  quarterly (evidence: dashboard role/group pages + audit log).
- **Joiner/mover/leaver:** account deactivation (`is_active=false`) immediately blocks
  login; erasure (WI-06) anonymizes on request.
- **DB role least privilege:** the application Postgres role is non-superuser and must
  have `UPDATE`/`DELETE` **revoked** on `audit_events` (append-only) — see
  [crypto-key-management.md](crypto-key-management.md).
