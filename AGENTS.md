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
  contract, and bootstrap/retire semantics: crate docs (`src/lib.rs`).
- The transport: sessions run over a `Link` — a control byte stream plus a
  supply of independent, lazily opened data streams. The contract (which
  the deadlock-freedom argument rests on) is in `src/link.rs`; the
  `conformance` cargo feature ships the public validation suite for
  caller-built links; `design/streaming-wire-deadlock.md` records why the
  contract exists and the deadlock analysis behind it.
- The tree (sparse Merkle radix trie, path compression, content-addressed
  leaves, the memo/version-bounds design): module docs in `src/tree.rs` and
  `src/tree/typed/`.
- The mirror protocols: module docs in `src/tree/mirror/` — `alternating/`
  (V1, full-level alternation; the streaming protocol's behavioral oracle)
  and `streaming/` (V2, fixed-memory; its module doc maps the layers:
  backend materiality, the type-level phase schedule, the walk and the
  proxy, the window, the wire vocabulary, the leaf conversion boundary).
- ITC semantics (`Party`, `Version`, `Clock`, the Law of Disjointness):
  `before`'s crate docs and `crates/before/CLAUDE.md`.

## Commands

The `justfile` is the source of truth for verification: every artifact in
the workspace has a recipe there, and the comment above each recipe explains
what it checks and why. `just --list` is the tour. Run `just gate` and get
it fully clean before every commit.

## Contributing a change

One-time setup: everything `just gate` shells out to is a stable Rust
toolchain (1.85 or later, for edition 2024) with clippy and rustfmt, a
nightly toolchain (merged doctests), `just`, `cargo-nextest`, `cargo-rdme`,
and python3 with bash (the `tools/` linters). `just ci` additionally wants
the `wasm32-unknown-unknown` target, `wasm-pack`, `cargo-fuzz`, and
node/npm.

1. Iterate with the inner loop: `just check`, `just test <filter>`,
   `just clippy`, `just fmt`.
2. If you edited crate-level rustdoc, run `just readme`: the READMEs are
   derived, never hand-edited.
3. Write tests to the conventions below, and commit any proptest seed
   files that appear.
4. Run `just gate` and get it fully clean before every commit: it adds
   everything the inner loop skips (the justfile's tier comments map it).
5. Sweep your prose against the hard rules below before asking for
   review; nothing in the gate checks them mechanically.

## Writing tests

- Unit and protocol tests live in a sibling file: `mod tests;` in the
  source, `tests.rs` next to it. Cross-peer behavioral suites are category
  binaries in `tests/`, built on `tests/common` (its module doc maps the
  categories).
- Give every test a doc comment stating, in English, the behavior and
  invariant it protects. The gate's `testdoc` checks that the comment
  exists; review holds it to the standard.
- When the claim is a family (a boundary, an ordering, a schedule), state
  it as a proptest invariant; the shrunk counterexample then rides along
  as a committed seed. A point regression may stay a unit test.
- A failing proptest writes a seed file automatically: under
  `proptest-regressions/<module path>.txt` for `src/` tests, and next to
  the binary for `tests/` suites. Commit every seed file that appears,
  wherever it appears; never strip one from a diff.

# Writing style

- When writing user-facing documentation (all public rustdoc comments), consider
  first *who is reading it* (the developer wanting to *use* the library) and
  what they *need to know*. Hew to the quadrants of the Diataxis framework where
  applicable.
- When writing maintainer-facing documentation (all private rustdoc comments and
  internal code comments), consider first *who is reading it* (the developer
  wanting to *understand*, *orient*, and *modify* the library) and what they
  *need to know*.
- Documentation should respect the underlying abstraction boundaries of the
  objects it documents. For example, when documenting a module, specify its
  invariants, constraints, purpose, and guarantees, but eschew over-binding
  definitions as to its internal structure; when writing public rustdoc, do not
  refer to functionality which cannot be seen by someone who is not looking at
  the source code.
- At all levels of structure, when writing all prose, think about how to clearly
  present the information, both concise and pedagogically. Eschew needless and
  especially self-invented jargon except where it is clearly defined and serves
  an expository purpose. Consider the precepts of _Style: Lessons in Clarity and
  Grace_ as you craft prose.

## Hard rules

- Nothing in the codebase refers to code that no longer exists — no
  "formerly", "superseded", "was removed", no deleted API names in any
  prose. Excise or re-denominate; provenance lives in git history and
  the design plans' decision records.
- Code may cite the Lean artifact by *theorem or definition name* (never
  by file path — Lean refactors orphan paths) when a kernel-checked
  statement backs the claim, with the invariant still stated inline;
  the model's design document (`formal/MODEL.md`) and proof-effort
  progress notes (`formal/PROGRESS.md`) are never cited from code.
- The model of record is uniform-hash, authenticated-honest-peer:
  transport is pre-authenticated and authorized, and an authorized peer
  already holds write authority over the set, so hostile-peer regimes
  are off-model — no design or pricing argument may rest on adversary
  economics. Violation/fail-fast machinery is a conformance bug
  detector, not a security boundary.
- Never let two independently-`seed`ed universes interact; within a universe,
  linearity of parties is the invariant everything rests on (see the crate
  docs' safety rules).
- Commit every proptest seed file (`proptest-regressions/**` and
  `tests/*.proptest-regressions`); never strip them from diffs.
- `tests/gossip_snapshot.rs` and the `insta` snapshots pin the wire format
  byte-for-byte; re-accept them only after a deliberate protocol change,
  which means a new protocol version, never a mutation of an existing one.
  To re-accept deliberately: `just test-all`, then `cargo insta review`
  (install: `cargo install cargo-insta`), then commit the updated
  `tests/snapshots/*.snap`.
- Redaction leaves no tombstones: deletion-honoring rides on version bounds.
  When reasoning about it, think version ceilings/floors, not markers.
