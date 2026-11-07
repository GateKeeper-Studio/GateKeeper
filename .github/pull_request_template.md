<!-- Describe the change and link any issue. -->

## Summary

## Rust safety review (server/ crate)

<!-- Checklist §23. Tick what applies; strike through (~~…~~) with a reason what doesn't. -->

- [ ] No new `unsafe` code was introduced (`#![forbid(unsafe_code)]` still holds).
- [ ] Any required unsafe is isolated and carries a `SAFETY:` comment.
- [ ] `cargo fmt --all --check` passes.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --all-targets --all-features` passes.
- [ ] `cargo audit` reports no unaccepted advisories.
- [ ] `cargo deny check` passes.
- [ ] `cargo geiger` reviewed for new unsafe dependency usage.
- [ ] `Cargo.lock` changes reviewed; new deps/features justified.
- [ ] Build scripts, proc macros, git deps, and native libs reviewed.
- [ ] Parsers / attacker-controlled inputs (tokens, JWT, redirect URIs, headers) have tests.
- [ ] Resource limits are bounded (body size, timeouts, pool sizes, channel capacity, caches).
- [ ] No new unbounded background task or channel can grow indefinitely.
- [ ] Cancellation and cleanup behavior tested where relevant.
