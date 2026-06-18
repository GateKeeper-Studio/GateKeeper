# Data Protection Impact Assessment (DPIA)

> **Work item:** WI-12 · **Frameworks:** GDPR Art. 35 (DPIA) · LGPD Art. 38 (RIPD) · ISO Clause 6.1
> **Owner:** DPO · **Review:** on material change to processing.

## 1. Description of processing

A multi-tenant OAuth2/OIDC Identity Provider. It authenticates end users, issues
tokens, and manages identity/authorization data (profiles, credentials, consent,
roles/groups) on behalf of tenant controllers. See
[data-processing-register.md](data-processing-register.md).

## 2. Necessity & proportionality

Processing is necessary to deliver authentication (contract) and, for consent grants,
relies on explicit consent. Data minimization is applied (PII-minimal audit, no secrets
in exports). Retention is bounded (see [retention-schedule.md](retention-schedule.md)).

## 3. Risks & mitigations

| Risk | Likelihood | Impact | Mitigation | Residual |
|---|---|---|---|---|
| Credential database compromise | Low | High | Argon2id + per-tenant pepper; encryption at rest; least privilege | Low |
| Signing-key theft → token forgery | Low | High | KMS target; rotation; `iss` pinning; short token TTL | Low–Med |
| Token theft / replay | Med | Med | Rotation + **reuse detection** (auto family revoke); PKCE; short TTL | Low |
| Unauthorized admin access | Med | High | Bearer-auth + RBAC + tenant isolation; audit; planned MFA | Low–Med |
| Cross-tenant data leak | Low | High | Tenant boundary enforced on every query/route | Low |
| Excessive retention | Med | Med | Retention schedule + purge jobs | Low |
| Inability to honor DSR | Med | Med | Export (✅) + erasure (planned) + rectification | Low |
| Audit tampering | Low | High | Append-only table + DB `REVOKE UPDATE/DELETE` | Low |

## 4. Data-subject rights

Access/portability (export ✅), erasure/anonymization (planned), rectification,
consent withdrawal (consent revoke ✅). Surfaced via admin endpoints and `/me/*`
self-service.

## 5. Conclusion

Residual risk is acceptable subject to completing the planned items (MFA, erasure,
retention purges, KMS migration) and verifying at-rest encryption in the target
deployment. Re-assess on any material change.
