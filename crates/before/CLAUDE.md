# before — Interval Tree Clocks

A guidepost, not a manual: the documentation of record is the rustdoc and,
for the algorithms, the ITC 2008 paper (`reference/itc2008.md`). Read the
crate docs for the model (`Party`/`Version`/`Clock`, the Law of
Disjointness), `version/event/mod.rs` for how the traversals work, and
`testing/` module docs for the differential-test architecture (recursive
oracle, function-space oracle, exhaustive small-scope, algebraic laws).

## Commands

The workspace `justfile` (repo root) is the source of truth for verification;
`just gate` before every commit, `just all` for the full sweep (which is what
builds this crate's feature matrix and fuzz targets). For a `-p before`-scoped
inner loop:

- Test: `cargo nextest run -p before --all-features`
- Lint: `cargo clippy -p before --all-targets --all-features -- -D warnings`
- Format: `cargo fmt -p before`

## Hard rules

- No `unsafe` (`#![forbid(unsafe_code)]`); `stacker` does the platform stack
  manipulation.
- Every traversal that recurses on tree depth must route each recursive call
  through `crate::recurse::descend!`, which grows the stack onto the heap
  before a deep input can overflow. The deliberate exceptions are iterative
  with `O(1)` or heap stacks and documented where they live
  (`idbits::skip_subtree`, the `codec::tree` parsers, test-only walks). The
  depth-100k `clock::tests::deep_tree_stack_safety` test is the proof.
- `decode` strictly rejects non-canonical input; byte-equality is what
  `Eq`/`Hash` rest on.
- `Party`/`Clock` are `!Clone`; `Version` is `Clone`. Don't add `Clone` to
  the first two, or borrowing `BitOr` overloads for `Clock` (either would
  duplicate a party).
- The public API is stable; don't add to or reshape it without explicit
  direction.
