# itc — Interval Tree Clocks

Safe-Rust ITC: packed `BitVec` storage, transient fixed-width working form for
mutation, linear-typed API. The references are the ITC 2008 paper
(`reference/itc2008.md`) for the algorithms and the code itself for the design.

## Commands

- Build:  `cargo build -p itc`
- Test:   `cargo nextest -p itc --all-features --release`
- Lint:   `cargo clippy -p itc --all-targets --all-features -- -D warnings`
- Format: `cargo fmt -p itc`
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

- No `unsafe` (crate has `#![forbid(unsafe_code)]`).
- Every tree traversal is ITERATIVE (explicit stack / preorder cursor). No
  recursion on tree depth — deep inputs must not overflow.
- `decode` strictly rejects non-canonical (non-normal-form) input; canonical
  byte-equality is relied on for `Eq`/`Hash`.
- `Party`/`Clock` are not `Clone`; `Version` is. Don't add `Clone` to the first two
  or borrowing `BitOr` overloads for `Clock` (would duplicate a party).
- The public API is stable; don't add to or reshape it without explicit direction.
