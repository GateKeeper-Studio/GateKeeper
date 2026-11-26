// Security-sensitive identity provider: this crate is 100% safe Rust. `forbid`
// (stricter than `deny` — a nested module cannot re-`allow` it) guarantees no
// `unsafe` block, `unsafe fn`, or `unsafe impl` can be introduced without an
// explicit, reviewed removal of this line. See docs memory-safety checklist §1/§24.
#![forbid(unsafe_code)]
// Each use case keeps its unit tests in `<action>/tests.rs` as
// `#[cfg(test)] mod tests { … }` (the convention in CLAUDE.md), which clippy reads as
// a `tests::tests` module named like its parent. Intended, so not linted.
#![allow(clippy::module_inception)]

pub mod application;
pub mod bootstrap;
pub mod domain;
pub mod infraestructure;
pub mod presentation;

pub mod utils;

#[cfg(test)]
pub mod testing;
