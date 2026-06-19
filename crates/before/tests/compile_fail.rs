//! Compile-fail guards for the crate's linearity and locked-API invariants
//! (COV-5). `Party` and `Clock` are deliberately not `Clone`, the `|`
//! (`BitOr`) operators consume their operands by value rather than borrowing,
//! and the balanced-array splits ([`From<Party>`] / [`From<Clock>`] for
//! `[_; N]`) reject `N == 0` — all *type-level* guarantees the design relies on.
//! These tests pin them: each `tests/ui/*.rs` case must FAIL to compile, with
//! its expected diagnostic recorded in the sibling `*.stderr`.
//!
//! The single `tests/ui/pass/*.rs` case is load-bearing beyond its own
//! assertion. trybuild `cargo check`s a fail-only suite, but `cargo check` does
//! not monomorphize, so the zero-length splits' post-monomorphization `const`
//! assert never fires. One `pass` case flips trybuild to `cargo build`, under
//! which that guard surfaces.
//!
//! Run with `cargo test -p before --test compile_fail`. To regenerate the
//! `.stderr` expectations after an intentional change, run with
//! `TRYBUILD=overwrite`.
//!
//! `trybuild` `.stderr` output can drift between rustc versions; these
//! snapshots were captured under rustc 1.96.0. If a future toolchain reformats
//! the diagnostics, overwrite the snapshots and confirm the *cause* of each
//! failure is unchanged (still a missing `Clone` / a use-after-move / a missing
//! `BitOr` / the `N >= 1` const assert).
//!
//! [`From<Party>`]: before::Party
//! [`From<Clock>`]: before::Clock

/// The invariant: every forbidden linearity- or arity-violating pattern is
/// rejected by the compiler. Cloning a `Party` or a `Clock`, reusing a `Clock`
/// after it has been consumed by `|`, borrowing `&Clock | &Clock`, and
/// splitting either into a zero-length array all must fail to compile; the
/// non-empty array split must still compile.
#[test]
fn linearity_compile_fail() {
    let t = trybuild::TestCases::new();
    // Registers a `pass` case, so trybuild builds (not just checks) the suite —
    // required for the zero-length splits' monomorphization-time assert to fire.
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/*.rs");
}
