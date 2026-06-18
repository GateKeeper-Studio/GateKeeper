# Data Classification Standard

> **Work item:** WI-01 · **Frameworks:** ISO 27001 A.5.12 · SOC 2 CC3
> **Owner:** Security lead · **Review:** annually.

## Classes

| Class | Definition | Examples here | Handling |
|---|---|---|---|
| **Public** | Intended for anyone | JWKS public keys, OIDC discovery | No restriction |
| **Internal** | Operational, non-sensitive | Role/group names, tenant config (non-secret) | Authenticated access |
| **Confidential** | Harmful if disclosed | Sessions, tokens, consent records, **audit events** | Need-to-know; encrypted at rest |
| **Restricted** | PII or secret material | Email, names, password hashes, client secrets, signing private keys | Strict need-to-know; encrypted; never logged in clear; never in audit `metadata` |

## Rules

1. **Restricted data never appears in logs or audit `metadata`.** Audit rows store
   ids and enums, not emails/names — this is what lets audit history survive erasure
   (WI-06) without re-exposing PII.
2. **Secrets** (peppers, signing private keys, client secrets) come from a secrets
   manager / KMS, not from source or plaintext columns where avoidable
   (see [crypto-key-management.md](crypto-key-management.md)).
3. **Restricted/Confidential at rest** is encrypted (DB-level minimum).
4. Each `src/domain/entities/*.rs` for a PII/secret entity carries a one-line
   classification comment so the class travels with the code.

## Entity → class map

`users`, `user_profiles`, `user_credentials`, `oauth_clients`(secret), `signing_keys`,
`tenants`(pepper) → **Restricted**. `auth_sessions`, `refresh_tokens`,
`authorization_codes`, `*_tokens`, `consent_grants`, `audit_events` → **Confidential**.
`roles`, `groups`, assignments → **Internal**. JWKS / discovery → **Public**.
