# Cryptography & Key Management Policy

> **Work item:** WI-09 · **Frameworks:** ISO A.8.24 · SOC 2 CC6.1/CC6.7 · GDPR Art. 32 · LGPD Art. 46
> **Owner:** Security lead · **Review:** annually.

## In transit

- TLS 1.2+ required for all external traffic (terminate at the load balancer / reverse
  proxy). HSTS is set by the app (`Strict-Transport-Security`, see `security_headers`
  in `routes.rs`). No plaintext HTTP in any shared environment.

## At rest

- **Database encryption:** enable storage-level encryption (cloud volume / RDS
  encryption). _Verify it is ON in the target deployment — the code assumes but cannot
  enforce this._ For the most sensitive columns, column-level encryption (`pgcrypto`)
  is an option but is not currently applied.
- **SMTP provider passwords** are encrypted at rest with AES-256-GCM
  (`AesGcmSecretCipher`), key from `EMAIL_ENCRYPTION_KEY`.

## Secrets & keys

| Secret | Current source | Target |
|---|---|---|
| Tenant pepper (`tenants.password_hash_secret`) | DB column | Move to / wrap with KMS; rotate per policy |
| JWT signing private keys (`signing_keys`) | DB (per tenant, ES256) | KMS-backed; private half never leaves the boundary |
| OAuth client secrets | Hashed in `oauth_clients` | OK (hash only) |
| `EMAIL_ENCRYPTION_KEY`, `JWT_SECRET` | Env | Secrets manager |

- **Password hashing:** Argon2id (or bcrypt cost ≥ 12) + per-tenant pepper. Record the
  exact parameters here once finalized.
- **Signing-key rotation:** the `signing_keys` model supports active + published
  (retiring) keys. Rotate on a fixed cadence (recommended **90 days**) with a JWKS
  overlap window so in-flight tokens still verify. Verifier pins `iss`.

## Least privilege at the database (append-only audit)

The `audit_events` table is **write-once**. In production, the application's Postgres
role must not be able to mutate it:

```sql
REVOKE UPDATE, DELETE ON audit_events FROM <app_role>;
```

Grant only `INSERT, SELECT`. This makes the audit trail tamper-evident (ISO A.8.15,
SOC 2 CC7.2). The application code already never issues UPDATE/DELETE on this table.

## Algorithms (allow-list)

JWT: RS256 / ES256 only — `alg=none` and HMAC are rejected by `JwtVerifierService`.
TLS: modern ciphersuites only. Hashing: SHA-256 for token hashes; Argon2id for passwords.
