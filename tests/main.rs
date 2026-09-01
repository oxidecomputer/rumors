//! The persistence anchor that centralizes the integration suites'
//! proptest seeds.
//!
//! Proptest's default persistence (`SourceParallel`) walks up from a
//! failing test's source file to the first directory containing `lib.rs`
//! or `main.rs`, then writes under `<that directory's parent>/
//! proptest-regressions/`, mirroring the source path below the anchor.
//! A tree with no anchor above `tests/` falls back to scattered sibling
//! `<suite>.proptest-regressions` files instead. This deliberately empty
//! test binary is the anchor: with it, every `tests/<suite>.rs` seed
//! resolves to `proptest-regressions/<suite>.txt`, in the same central
//! directory as the `src/` suites' seeds.
//!
//! `tests/seed_liveness.rs` reconstructs the same resolution from the
//! filesystem, so it enforces this layout: a seed committed anywhere
//! proptest would not read it — a sibling file included — fails the
//! sweep.
