# before — Interval Tree Clocks

Safe-Rust ITC: packed `BitVec` storage, transient fixed-width working form for
mutation, linear-typed API. The references are the ITC 2008 paper
(`reference/itc2008.md`) for the algorithms and the code itself for the design.

## Commands

- Build:  `cargo build -p before`
- Test:   `cargo nextest -p before --all-features --release`
- Lint:   `cargo clippy -p before --all-targets --all-features -- -D warnings`
- Format: `cargo fmt -p before`
- Verification gate (run all three, must be clean, before every commit):
  fmt → clippy → test.

## Workflow (always)

- Build order: implement the oracle and make its property suite (Appendix D) pass
  before writing ANY implementation code.
- TDD: write the phase's tests FIRST and confirm they fail before implementing.
  Do not write implementation ahead of its tests.
- Make minimal, scoped changes; do not refactor unrelated code.
- One commit per phase (or logical sub-step), descriptive message.
- When unsure between two approaches, stop and explain both rather than guessing.

## Hard rules (always)

- No `unsafe` (crate has `#![forbid(unsafe_code)]`). `stacker` (a dependency)
  does the platform stack manipulation; the crate itself stays unsafe-free.
- Every tree traversal RECURSES on depth and MUST route each recursive call
  through the `crate::recurse::descend!(depth, call)` macro, which grows the
  stack onto the heap (via `stacker`) before a deep, unbalanced input can
  overflow. Never recurse on tree depth without the guard. Exceptions, all still
  overflow-safe: (1) the `pending: i64` span scans in `idbits::skip_subtree` /
  `Builder::copy` loop with an `O(1)` stack; (2) the codec decode parsers
  (`codec::tree::parse_id`/`parse_ev`) stay iterative with inline-`SmallVec`
  stacks — measured faster than recursion for gamma-heavy parsing, the SmallVec
  keeps shallow inputs allocation-free, and the heap spill preserves overflow
  safety; (3) test-only walks in `testing/` keep their own explicit stacks. The
  depth-100k `clock::tests::deep_tree_stack_safety` test is the overflow proof.
- `decode` strictly rejects non-canonical (non-normal-form) input; canonical
  byte-equality is relied on for `Eq`/`Hash`.
- `Party`/`Clock` are not `Clone`; `Version` is. Don't add `Clone` to the first two
  or borrowing `BitOr` overloads for `Clock` (would duplicate a party).
- The public API is stable; don't add to or reshape it without explicit direction.
