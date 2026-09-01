//! The persistence anchor that centralizes this harness's integration
//! proptest seeds: with a `main.rs` in `tests/`, proptest's default
//! `SourceParallel` persistence resolves every `tests/<suite>.rs` seed
//! to `proptest-regressions/<suite>.txt` at the package root instead of
//! a scattered sibling file. The full rationale is at the rumors crate's
//! `tests/main.rs`; the repository-wide sweep is
//! `tests/seed_liveness.rs`.
