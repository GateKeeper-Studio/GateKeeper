# Secure Development & Change Management

> **Work item:** WI-12 · **Frameworks:** ISO A.8.25–A.8.31 · SOC 2 CC8.1/CC3 · GDPR Art. 25
> **Owner:** Engineering lead · **Review:** annually.

## Change management

- All changes via pull request; **at least one reviewer** (segregation of duties,
  SOC 2 CC8.1). No direct pushes to the default branch.
- Reversible DB migrations: every migration has `.up.sql` **and** `.down.sql`
  (already the repo convention).
- Tag releases; the changelog / merged-PR history is the change-management evidence.

## CI gates (required)

| Gate | Command | Purpose |
|---|---|---|
| Tests | `cargo test` | Functional correctness (in-memory, no DB) |
| Lints | `cargo clippy --all-targets -- -D warnings` | Code quality |
| Format | `cargo fmt --check` | Consistency |
| Dependency CVEs | `cargo audit` | Vuln management (ISO A.8.8) |
| Licenses / supply chain | `cargo deny check` | Supply-chain (A.8.28) |
| Secret scanning | e.g. gitleaks | Prevent secret leakage |
| Frontend | `npm run check && npm run build` (in `dashboard/`) | Dashboard integrity |

A reference workflow lives at `.github/workflows/ci.yml` _(to be added)_.

## Secure design by default (GDPR Art. 25)

- Data minimization: audit `metadata` is PII-minimal; exports exclude secrets/hashes.
- Deny-by-default authorization (see [access-control-policy.md](access-control-policy.md)).
- Compile-time-checked SQL (`sqlx::query!`) reduces injection risk.
- Short-lived, rotating, hashed tokens with reuse detection.

## DPIA

A Data Protection Impact Assessment for the IdP processing is maintained in
[dpia.md](dpia.md) (GDPR Art. 35) and reviewed on material change.
