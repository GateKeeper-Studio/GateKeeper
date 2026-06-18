# Data Retention Schedule

> **Work item:** WI-03 · **Frameworks:** GDPR Art. 5(1)(e) · LGPD Art. 16 · ISO A.8.10
> **Owner:** DPO / Security lead · **Review:** annually.

Storage limitation: personal data is kept no longer than necessary. The values below
are **defaults**; a tenant DPA may tighten them. Audit-log retention is a **floor**
(minimum), not a ceiling — purges never delete audit rows below the legal minimum.

| Data | Table | Retention (default) | Mechanism | Status |
|---|---|---|---|---|
| Authorization codes | `authorization_codes` | Minutes (single use); purge ≤ 24 h after expiry | `purge_expired_auth_codes` (WI-03) | 🟡 planned |
| Email/password tokens | `email_verification_tokens`, `password_*_tokens` | Hours; purge ≤ 24 h after expiry | `purge_expired_tokens` | 🟡 planned |
| Auth sessions | `auth_sessions` | Until idle/absolute timeout; purge after | `purge_expired_sessions` | 🟡 planned |
| Refresh tokens | `refresh_tokens` | 30 d rolling; purge ≤ 90 d after expiry | `purge_expired_tokens` | 🟡 planned |
| Consent grants | `consent_grants` | Until revoked + tenant policy | DSR erasure (WI-06) | 🟡 planned |
| User account + profile | `users`, `user_profiles` | Life of account; on erasure → anonymized | `erase_user` (WI-06) | 🟡 planned |
| **Audit events** | `audit_events` | **1–7 years (floor)** — evidence | Never auto-purged | ✅ append-only enforced |

## Enforcement

Purge use cases live under `src/application/use_cases/compliance/purge_expired_*`,
are batched + idempotent, and **emit a `data.purged` audit event** with counts. They
are driven by a scheduled runner (cron / `tokio` interval) and/or the admin endpoint
`POST /admin/maintenance/purge`. TTLs come from a `RetentionPolicy` config struct.

## Inactive accounts

Policy: after a configurable inactivity period, notify the user, then anonymize if no
response. Implemented via the erasure path (anonymize, not hard-delete) to preserve
audit history. _Threshold to be set per deployment._
