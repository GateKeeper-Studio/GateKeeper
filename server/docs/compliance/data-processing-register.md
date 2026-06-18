# Record of Processing Activities (ROPA) / Data Processing Register

> **Work item:** WI-01 · **Frameworks:** GDPR Art. 30 · LGPD Art. 37 · ISO A.5.9/5.12
> **Owner:** DPO / Security lead · **Review:** annually or on schema change.

This is the authoritative inventory of personal data processed by this Identity
Provider. Keep it in sync with `migrations/` and `src/domain/entities/`. Each entity
carries a one-line classification comment (see [data-classification.md](data-classification.md)).

## 1. Controller / processor

- **Role:** This service typically acts as a **processor** on behalf of each tenant
  (the controller), and as **controller** for platform-operational data (audit logs,
  signing keys). Confirm per deployment in the DPA with each tenant.
- **DPO / Encarregado:** _to be designated_ — publish name + contact (GDPR Art. 37,
  LGPD Art. 41).

## 2. Processing inventory

| # | Data element | Entity / table | Classification | Purpose | Legal basis | Retention | Recipients |
|---|---|---|---|---|---|---|---|
| 1 | Email address | `users` | Restricted (PII) | Identification, login, comms | Contract / Legit. interest | Life of account + see schedule | Tenant; email provider |
| 2 | First/last/display name | `user_profiles` | Restricted (PII) | Personalization, identity | Contract | Life of account | Tenant; relying-party clients (via OIDC `profile` scope) |
| 3 | Password hash + pepper ref | `user_credentials`, `tenants.password_hash_secret` | Restricted (secret) | Authentication | Contract | Life of account | None |
| 4 | Email-verification tokens | `email_verification_tokens` | Confidential (token) | Prove email control | Contract | Hours (short TTL) | None |
| 5 | Password reset/change tokens | `password_reset_tokens`, `password_change_tokens` | Confidential (token) | Account recovery | Contract | Hours (short TTL) | None |
| 6 | Consent grants | `consent_grants` | Confidential (consent) | OAuth scope authorization | **Consent** | Until revoked | Relying-party client |
| 7 | Auth sessions | `auth_sessions` | Confidential | Login/SSO state | Contract | Idle/absolute timeout | None |
| 8 | Refresh tokens | `refresh_tokens` | Confidential (token) | Session continuation | Contract | 30 d rolling | None |
| 9 | Authorization codes | `authorization_codes` | Confidential (token) | OAuth code exchange | Contract | Minutes (single use) | None |
| 10 | Roles / groups + assignments | `roles`, `groups`, `role_assignments`, `group_members` | Internal | Authorization (RBAC) | Legit. interest | Life of account | Relying-party clients (JWT claims) |
| 11 | OAuth clients + secrets | `oauth_clients` | Restricted (secret) | RP registration | Contract | Life of registration | None |
| 12 | Signing keys (private) | `signing_keys` | Restricted (secret) | JWT signing | — (operational) | Rotation window | Public half via JWKS |
| 13 | **Audit events** | `audit_events` | Confidential | Security, accountability, evidence | Legit. interest / legal obligation | **1–7 yrs (floor)** | Auditors, SIEM |

## 3. Data subject categories

End users of each tenant; tenant administrators; (indirectly) operators of
relying-party OAuth clients.

## 4. International transfers

Document hosting region(s) and any sub-processor outside the data subject's region;
attach SCCs / adequacy basis (GDPR Ch. V) and ANPD transfer basis (LGPD Ch. V).
_To be completed per deployment._

## 5. Technical & organizational measures (summary)

Encryption in transit/at rest, hashed credentials + per-tenant pepper, rotating
short-lived tokens with reuse detection, append-only audit log, RBAC + tenant
isolation at the API edge. See [crypto-key-management.md](crypto-key-management.md)
and [access-control-policy.md](access-control-policy.md).
