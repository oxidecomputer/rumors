# itc — Interval Tree Clocks

Safe-Rust ITC: packed `BitVec` storage, transient fixed-width working form for
mutation, linear-typed API. The full design and execution plan is in
`IMPLEMENTATION_PLAN.md` — it is frozen; follow it, don't redesign.

## Commands

- Build:  `cargo build -p itc`
- Test:   `cargo test -p itc --all-features`
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
- Never mutate integers in the packed form in place; arithmetic happens only in the
  fixed-width working form, repacked at the batch boundary.
- `decode` strictly rejects non-canonical (non-normal-form) input; canonical
  byte-equality is relied on for `Eq`/`Hash`.
- `Party`/`Clock` are not `Clone`; `Version` is. Don't add `Clone` to the first two
  or borrowing `BitOr` overloads for `Clock` (would duplicate a party).
- The public API matches `IMPLEMENTATION_PLAN.md` Appendix B exactly.

## Layout

- `src/lib.rs`        crate root, re-exports, errors, `#![forbid(unsafe_code)]`
- `src/party.rs`      `party::Party` (+ packed id ops)
- `src/version.rs`    `version::{Version, Batch}` (+ event codec, working form, ops)
- `src/clock.rs`      `clock::{Clock, Batch}`
- `src/codec.rs`      bit I/O, Elias-gamma integer code, encode/decode + validation
- `src/oracle.rs`     `#[cfg(test)]` reference oracle — mirrors the target API; ground truth
- `tests/`            property tests (proptest) + the differential harness

## Testing

- Property tests via `proptest`; generate values via ops from a seed (always valid,
  normal-form, party-disjoint). The oracle (mirrors the API) must pass its property
  suite (Appendix D) first. The differential harness then checks impl vs oracle by
  **structural** agreement — lower both to oracle trees — after every op; the byte
  codec is tested separately. Keep deep-tree (≥100k) and decode-fuzz tests in CI.
- Tests live in a sibling file (`mod tests;` in the source, `tests.rs` sibling),
  not inline `mod tests { ... }` blocks.
