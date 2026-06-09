# rumors — project notes

A guidepost, not a manual: the documentation of record is the rustdoc. Read
the crate-level docs first, then the module docs named below. Keep this file
accurate and small; when detail belongs somewhere durable, put it in the docs.

## Orientation

`rumors` is a Rust library for unordered gossip with redaction: a CRDT-backed
set of messages that peers replicate and keep convergent, reconciling over
the wire by exchanging only what differs. It is built on `crates/before`, an
Interval Tree Clock library (`crates/before-viz` visualizes the clocks).

- What a `Known` is, the public API, bootstrap/retire semantics: crate docs
  (`src/lib.rs`; synchronous wrapper in `src/sync.rs`).
- The tree (sparse Merkle radix trie, path compression, content-addressed
  leaves): `src/tree.rs` and `src/tree/typed/`.
- The mirror protocol: module docs in `src/tree/traverse/mirror/` —
  `local.rs` (state machine, asymmetry matrix), `protocol.rs` (type-level
  phase schedule), `message.rs` (wire format), `remote.rs` (preamble,
  framing, party hand-off).
- ITC semantics (`Party`, `Version`, `Clock`, the Law of Disjointness):
  `before`'s crate docs and `crates/before/CLAUDE.md`.

## Commands

The `justfile` is the source of truth for verification — every artifact in the
workspace has a recipe there; `just --list` shows them all. The tiers:

- Inner loop: `just check`, `just test <filter>`, `just clippy`, `just fmt`.
- The gate before every commit: `just gate` (fmt → clippy → tests → doctests),
  all clean.
- `just all`: the full no-rot sweep — adds what the gate never touches (the
  `before` feature matrix, the wasm target, rustdoc `-D warnings`, bench
  builds, the fuzz targets, the viz bundle).

## Hard rules

- Never let two independently-`seed`ed universes interact; within a universe,
  linearity of parties is the invariant everything rests on (see the crate
  docs' safety rules).
- Commit `proptest-regressions/**` seed files; never strip them from diffs.
- `tests/gossip_snapshot.rs` and the `insta` snapshots pin the wire format
  byte-for-byte; re-accept them only after a deliberate protocol change.
- Redaction leaves no tombstones: deletion-honoring rides on version bounds.
  When reasoning about it, think version ceilings/floors, not markers.
