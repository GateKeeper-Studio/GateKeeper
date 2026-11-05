# Documentation index

Authoritative, current documentation for the `server/` crate (Gatekeeper — a
multi-tenant OAuth 2.1 / OIDC identity provider). Start here.

## Current reference docs

| Doc | What it covers |
|---|---|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Clean-Architecture layers, the dependency rule, project structure, core building blocks, persistence/transactions, cryptography, domain map, request lifecycle, testing strategy |
| [`FLOWS-OAUTH2-OIDC.md`](FLOWS-OAUTH2-OIDC.md) | OAuth 2.1 / OIDC roles, client model, endpoints, Authorization Code + PKCE flow, token exchange, refresh-token rotation & reuse detection, signing keys / JWKS |
| [`FLOWS-AUTHENTICATION.md`](FLOWS-AUTHENTICATION.md) | First-party login SPA flow, MFA (email OTP / TOTP / WebAuthn), registration & email confirmation, password reset / forced change, platform-admin auth, request authentication & RBAC |
| [`FLOWS-WEBHOOKS.md`](FLOWS-WEBHOOKS.md) | Outbound user-lifecycle events: the transactional outbox, HMAC-signed delivery contract, consumer verification, at-least-once/retry semantics, SSRF hardening of subscription URLs, dispatcher operations |
| [`ARCHITECTURE-REVIEW.md`](ARCHITECTURE-REVIEW.md) | Security and architecture review (findings S1–S10, A1–A6, C1–C4), what was implemented, and the checklist to follow before deploying |
| [`WEBHOOK-CONSUMER-GUIDE.md`](WEBHOOK-CONSUMER-GUIDE.md) | Integration guide for the *other* service: registering a subscription, the receiver algorithm, signature verification in five languages, idempotent projection into your own user table, secret rotation, backfill, troubleshooting |

The agent guide for the crate is [`../CLAUDE.md`](../CLAUDE.md).

## Compliance & governance

A living governance/evidence set for the LGPD/GDPR/ISO 27001/SOC 2 program — these
documents map to controls implemented in code and each names an owner + review
cadence (so they are maintained, not historical):

- [`compliance/`](compliance/) — policy/evidence set (data-processing register, data
  classification, retention schedule, access-control policy, crypto key management,
  incident-response plan, DPIA, monitoring/availability, secure SDLC, statement of
  applicability). Start at [`compliance/README.md`](compliance/README.md).
- [`compliance-implementation-plan.md`](compliance-implementation-plan.md) — the
  umbrella program plan and work-item tracker the `compliance/` set references.
