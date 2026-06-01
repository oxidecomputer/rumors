//! Compile-fail guards for the crate's linearity and locked-API invariants
//! (COV-5). `Party` and `Clock` are deliberately not `Clone`, and the `|`
//! (`BitOr`) operators consume their operands by value rather than borrowing —
//! both properties are *type-level* guarantees the design relies on. These
//! tests pin them: each `tests/ui/*.rs` case must FAIL to compile, with its
//! expected diagnostic recorded in the sibling `*.stderr`.
//!
//! Run with `cargo test -p itc --test compile_fail`. To regenerate the
//! `.stderr` expectations after an intentional change, run with
//! `TRYBUILD=overwrite`.
//!
//! `trybuild` `.stderr` output can drift between rustc versions; these
//! snapshots were captured under rustc 1.93.1. If a future toolchain reformats
//! the diagnostics, overwrite the snapshots and confirm the *cause* of each
//! failure is unchanged (still a missing `Clone` / a use-after-move / a missing
//! `BitOr`).

/// The invariant: every forbidden linearity-violating pattern is rejected by
/// the compiler. Cloning a `Party` or a `Clock`, reusing a `Clock` after it has
/// been consumed by `|`, and borrowing `&Clock | &Clock` all must fail to
/// compile.
#[test]
fn linearity_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
