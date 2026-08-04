# before — Interval Tree Clocks

A guidepost, not a manual: the documentation of record is the rustdoc and,
for the algorithms, the ITC 2008 paper (`reference/itc2008.md`). Read the
crate docs for the model (`Party`/`Version`/`Clock`, the Law of
Disjointness) and the public `implementation` module for the design essay,
`version/skyline.rs` for the stored coding and its operation kernels, and
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

- No `unsafe` (`#![forbid(unsafe_code)]`); the test-only stack guard's
  platform stack manipulation lives in the `stacker` dev-dependency.
- No library traversal recurses on tree depth: every deep walk is iterative,
  with `O(1)`, bit, or heap stacks documented where they live
  (`idbits::skip_subtree`, the `codec::tree` and `codec::text` parsers, the
  id walks `sum`/`covers`/`is_disjoint` and `diff`'s complement arm in
  `party::ops`, the skyline kernels — the fused fill walk and its pre-scan
  carry suspended ancestors on the explicit bit stacks in
  `version/skyline/fill.rs`). A walk that must recurse routes each recursive
  call through `crate::recurse::descend!`, which grows the stack onto the
  heap before a deep input can overflow — today those are only test
  surfaces: the oracle bridge and the test-local recursive witnesses beside
  it (`recurse.rs`'s module doc holds the inventory and the keep decision).
  The depth-100k `clock::tests::deep_tree_stack_safety` test is the proof.
- `decode` strictly rejects non-canonical input; byte-equality is what
  `Eq`/`Hash` rest on.
- `Party`/`Clock` are `!Clone`; `Version` is `Clone`. Don't add `Clone` to
  the first two, or borrowing `BitOr` overloads for `Clock` (either would
  duplicate a party).
- The public API is stable; don't add to or reshape it without explicit
  direction. If you believe something should be added, or doing so would
  make things more elegant, efficient, or usable, please suggest this.
