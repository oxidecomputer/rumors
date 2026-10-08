<!-- CAVEAT LECTOR: a builder brief written by the coordinator (Claude Opus 5.5), collecting small findings that came up in passing during the audit. -->

# Builder brief: small cleanups

Kind: corrections to prose that contradicts the code, plus one arithmetic
guard. One commit per item, so the owner can take any subset. No public
API, wire format, or value changes.

Each item below is the coordinator's reading. Verify each one against the
tree before changing anything, and drop any item that turns out to be
wrong, saying why.

1. **`crates/before/src/recurse.rs`, module doc.** The paragraph after the
   list of guarded sites reads as though the tree oracle's traversals are
   guarded. Check what is actually guarded, and restate the paragraph so it
   says exactly that.
2. **A test-only `union` recurses unguarded.** `crates/before/AGENTS.md`
   requires that no traversal recurse on input-controlled depth unless it
   runs under the stack guard. The test-only oracle `union` recurses
   without the guard. Find it (it is in the tree oracle under
   `src/testing/oracles/`). Then either route it through the existing guard
   (`descend!`, or whatever `recurse.rs` provides), or show that its depth is
   bounded by construction and say so at the site. Show by a deep input,
   within the existing deep-test conventions, that the guarded form
   survives depth.
3. **`crates/before/src/clock/tests.rs:640`.** The comment calls the `Debug`
   printer "recursive". Read the `Debug` implementation. If it isn't
   recursive, restate the comment to say what the test guards: that
   formatting a deep clock must not overflow.
4. **The measures lane's O9.** `crates/before/src/version/measure/integral/width.rs:240`
   allocates `vec![0; span * 4]`. The lane's record is at
   `lanes/l4-measures/round-1/observations.md`. Decide whether the
   multiplication can overflow on a 32-bit target for an input that
   reaches it, and what happens if it does: a wrapped, too-short buffer
   would be a correctness bug, while a panic is merely an allocation-limit
   panic. Use `checked_mul` with an `expect` message that is a one-line
   proof, or show it cannot overflow and say so at the site. If a wrapped
   length can reach a read or write, stop and report: that is a defect,
   not a cleanup.

## Verification

One landing check at the tip. Expect the post-board-pin baseline, plus any
deep test that item 2 adds.

## Owner's ruling: remove the stack guard (question 68)

Remove `recurse.rs`, its `descend!` macro, and the `stacker` dev-dependency,
in one branch after #40, #72, and #74 land (each adds or edits a use). Every
test helper that recursed through the guard recurses directly and states its
depth bound at the site, where the bound holds by construction; the one deep
test (`deep_spines_grow_identically`) runs its reference probe on a thread
with an explicit stack size. `crates/before/AGENTS.md`'s rule becomes: no
library traversal recurses on input depth; a test helper recurses only on
input bounded by construction, stated where it recurses; a test that needs
more depth runs on a thread with an explicit stack size. The docs branch's
`RED_ZONE` item disappears with the constant.
