# Statement of Applicability (ISO/IEC 27001:2022 Annex A)

> **Framework:** ISO 27001 Clause 6.1.3 · **Owner:** Security lead · **Review:** annually.

Subset of Annex A controls with direct relevance to this codebase. Full SoA must cover
all 93 controls; organizational controls (HR, physical, supplier) are tracked in the
broader ISMS, not here. Status: ✅ implemented · 🟡 partial · 📄 documented · ⬜ planned.

| Control | Title | Applicable | Status | Evidence |
|---|---|---|---|---|
| A.5.7 | Threat intelligence | Yes | ⬜ | IR plan detection hooks |
| A.5.12 | Classification of information | Yes | ✅/📄 | data-classification.md |
| A.5.15 | Access control | Yes | ✅ | auth middleware; access-control-policy.md |
| A.5.16 | Identity management | Yes | ✅ | this service IS the IdP |
| A.5.17 | Authentication information | Yes | ✅ | Argon2id + pepper; token hashing |
| A.5.18 | Access rights (review/revoke) | Yes | 🟡 | role/group mgmt; access reviews 📄 |
| A.5.24–5.28 | Incident management | Yes | 📄 | incident-response-plan.md |
| A.5.34 | Privacy & PII protection | Yes | 🟡 | DSR export ✅; erasure ⬜; ROPA 📄 |
| A.8.2 | Privileged access rights | Yes | ✅ | RBAC admin roles |
| A.8.3 | Information access restriction | Yes | ✅ | tenant isolation + deny-by-default |
| A.8.5 | Secure authentication | Yes | 🟡 | bearer auth ✅; MFA ⬜ |
| A.8.8 | Technical vulnerability mgmt | Yes | ⬜ | `cargo audit` in CI (secure-sdlc.md) |
| A.8.10 | Information deletion | Yes | 🟡 | retention purges ⬜; erasure ⬜ |
| A.8.13 | Information backup | Yes | 📄 | monitoring-availability.md |
| A.8.15 | Logging | Yes | ✅ | append-only `audit_events` |
| A.8.16 | Monitoring activities | Yes | 🟡 | detection hooks 📄; SIEM ⬜ |
| A.8.24 | Use of cryptography | Yes | ✅/📄 | crypto-key-management.md |
| A.8.25–8.28 | Secure development | Yes | 🟡 | secure-sdlc.md; CI gates ⬜ |

Items marked ⬜/🟡 map to open work items in
[`../compliance-implementation-plan.md`](../compliance-implementation-plan.md).
