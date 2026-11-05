# Memory-Safety Posture

This document records how the `server/` crate implements the Rust memory-safety
verification checklist, what is enforced automatically, and what is deliberately
deferred. It doubles as the **exception register** (checklist §24.12): any accepted
advisory or unsafe dependency is listed here with a justification.

> The strongest honest claim: *the server is written entirely in safe Rust, unsafe
> boundaries do not exist in first-party code, dependencies are continuously
> assessed, and CI prevents unreviewed regressions.* We do **not** claim it is
> "CVE-proof" or "guaranteed secure."

---

## What is enforced

| # | Item | How | Status |
|---|------|-----|--------|
| §1 | Forbid `unsafe` in first-party code | `#![forbid(unsafe_code)]` on all three crate roots (`src/lib.rs`, `src/main.rs`, `src/bin/create_admin.rs`) | ✅ enforced by compiler |
| §2 | No unsafe/FFI constructs | `rg '\bunsafe\b\|extern "C"\|no_mangle\|asm!'` over `src/` returns nothing | ✅ clean |
| §4 | Dependency advisories, bans, licenses, sources | `deny.toml` + `cargo deny check` in CI | ✅ CI gate |
| §5 | Commit + verify `Cargo.lock` | `server/Cargo.lock` tracked; every CI cargo step uses `--locked` | ✅ |
| §8 | Strict compiler/Clippy/test gates | `cargo fmt --check`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo doc` in CI | ✅ CI gate |
| §16 | Leak-prone patterns | `rg 'unbounded_channel\|Box::leak\|mem::forget'` over `src/` returns nothing | ✅ none present |
| §17 | Bounded request body | Axum's default 2 MB body limit is in effect — no `DefaultBodyLimit::disable()` anywhere | ✅ default intact |
| §19 | Pin + verify toolchain | `rust-toolchain.toml` pins `1.96.0`; CI prints `rustc/cargo --version --verbose` | ✅ |
| §20 | Supply chain | `cargo audit` + `cargo deny` on every PR; lockfile committed | ✅ CI gate |
| §22 | CI pipeline | `.github/workflows/rust-ci.yml` (build-test, supply-chain, geiger) | ✅ |
| §23 | PR safety checklist | `.github/pull_request_template.md` | ✅ |

CI compiles against a live Postgres service because the crate uses **sqlx
compile-time-checked queries with no offline cache** — `cargo check/clippy/test/doc`
all need the schema present, so the workflow applies migrations first.

---

## Deferred (documented, not yet automated)

These need extra setup or produce noise on the current code; tracked here so their
absence is a decision, not an oversight.

| # | Item | Why deferred |
|---|------|--------------|
| §3 | `cargo geiger` | Included in CI as a **non-blocking** job (`continue-on-error`). Establish an approved unsafe-dependency baseline before making it a gate. |
| §8 | `deny(clippy::unwrap_used / expect_used / panic)` | ~349 `unwrap`/`expect` in non-test first-party code. Denying crate-wide would be a large, mechanical refactor. Address incrementally (deny per-module as code is cleaned), not in one sweep. |
| §9 | Miri | No first-party `unsafe`, so Miri has nothing first-party to check. Worth running only if unsafe/FFI is ever introduced. |
| §10 | Sanitizers (ASan/LSan/TSan) | Same — valuable at FFI/unsafe boundaries, of which there are none today. |
| §11 | Loom | No hand-rolled synchronization primitives; concurrency is via Tokio + sqlx pool. Revisit if custom lock-free code appears. |
| §12–13 | Fuzzing / property tests | High value for the token/JWT/redirect-URI/PKCE parsers. Recommended next step: `cargo fuzz` targets for the smallest parse/validate units, plus `proptest` for "parsing never panics / codes consumed once / redirect URIs matched exactly". |

---

## Exception register

Accepted RustSec advisories and reviewed unsafe dependencies. Each row has a
matching `ignore` entry in both `deny.toml` (`[advisories].ignore`) and
`.cargo/audit.toml`. Add a row before accepting anything new.

| Advisory / crate | Reason accepted | Date | Revisit when |
|------------------|-----------------|------|--------------|
| `RUSTSEC-2023-0071` — `rsa` Marvin timing sidechannel | No upstream fix. Transitive via `jsonwebtoken`'s `rust_crypto` backend; this IdP signs/verifies **ES256 + HS256 only**, so the RSA path is never exercised. | 2026-07-16 | RustCrypto/RSA ships a constant-time release |
| `RUSTSEC-2026-0173` — `proc-macro-error2` unmaintained | Build-time proc-macro (dependency of `validator_derive`); not present in the runtime binary. No security impact. | 2026-07-16 | `validator` migrates off it |

**Fixed by lockfile bump (not accepted, resolved):** `anyhow` 1.0.102→1.0.103
(RUSTSEC-2026-0190 unsound `downcast_mut`), `quinn-proto` 0.11.14→0.11.16
(RUSTSEC-2026-0185 high — remote memory exhaustion, a §17 concern), `spin`
0.9.8→0.9.9 (yanked).

---

## Running the checks locally

```bash
cd server

# Compiles need a live schema (no sqlx offline cache):
docker compose up -d
export DATABASE_URL="postgres://gatekeeper:gatekeeper@localhost:5432/gatekeeper"
sqlx migrate run

cargo fmt --all --check
cargo check --all-targets --all-features --locked
cargo test  --all-targets --all-features --locked

# Supply chain (no database needed):
cargo install cargo-audit cargo-deny --locked   # one-time
cargo audit
cargo deny check
```
