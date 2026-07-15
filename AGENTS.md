# rumors — project notes

A guidepost, not a manual: the documentation of record is the rustdoc. Read
the crate-level docs first, then the module docs named below. Keep this file
accurate and small; when detail belongs somewhere durable, put it in the docs.

## Orientation

`rumors` is a Rust library for unordered gossip with redaction: a CRDT-backed
set of messages that peers replicate and keep convergent, reconciling over
the wire by exchanging only what differs. It is built on `crates/before`, an
Interval Tree Clock library (`crates/before-viz` visualizes the clocks).

- The model (membership as custody), the `Peer`/`Rumors` split, the session
  contract, bootstrap/retire semantics: crate docs (`src/lib.rs`; blocking
  wrapper in `src/sync.rs`).
- The tree (sparse Merkle radix trie, path compression, content-addressed
  leaves, the memo/version-bounds design): module docs in `src/tree.rs` and
  `src/tree/typed/`.
- The mirror protocols: module docs in `src/tree/mirror/` — `alternating/`
  (the materialized implementation, the streaming protocol's behavioral
  oracle) and `streaming/` (fixed-memory streams; its module doc maps the
  layers: backend materiality, type-level phase schedule, session walks,
  the conversion boundary at the leaves).
- ITC semantics (`Party`, `Version`, `Clock`, the Law of Disjointness):
  `before`'s crate docs and `crates/before/CLAUDE.md`.

## Commands

The `justfile` is the source of truth for verification — every artifact in the
workspace has a recipe there; `just --list` shows them all. The tiers:

- Inner loop: `just check`, `just test <filter>`, `just clippy`, `just fmt`.
- The gate before every commit: `just gate` (fmt → doclint → testdoc →
  readme-check → clippy → docs and docs-internal with `-D warnings` → tests →
  doctests), all clean. The `docs-internal` pass renders private items, so
  intra-doc links inside private modules rot loudly instead of silently. `doclint`
  (`tools/doclint`) fails when a doc comment's first paragraph — the summary
  rustdoc shows in index tables — outgrows a one-liner; move the rest below a
  blank `///` line. `testdoc` requires every Rust test to explain the behavior
  and invariant it protects. `readme-check` (`tools/readme`) re-derives each
  crate's README from its crate-level rustdoc (via cargo-rdme, intra-doc links
  stripped) and fails on drift; run `just readme` after editing crate docs.
- `just all`: the full no-rot sweep — adds what the gate never touches (the
  `before` feature matrix, the wasm target, bench builds, the fuzz targets,
  the viz bundle).

## Hard rules

- Never let two independently-`seed`ed universes interact; within a universe,
  linearity of parties is the invariant everything rests on (see the crate
  docs' safety rules).
- Commit `proptest-regressions/**` seed files; never strip them from diffs.
- `tests/gossip_snapshot.rs` and the `insta` snapshots pin the wire format
  byte-for-byte; re-accept them only after a deliberate protocol change.
- Redaction leaves no tombstones: deletion-honoring rides on version bounds.
  When reasoning about it, think version ceilings/floors, not markers.
