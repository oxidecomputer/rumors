# rumors — project notes

A guidepost, not a manual: it orients and points, and the documentation of
record is the rustdoc. Keep this file small and accurate; when detail needs a
durable home, put it in the docs and point there.

## Orientation

`rumors` is a Rust library for unordered gossip with redaction: a CRDT-backed
set of messages that peers replicate and keep convergent, reconciling over the
wire by exchanging only what differs. It is built on `crates/before`, an
Interval Tree Clock library whose crate docs define `Party`, `Version`, and
`Clock`. Read the crate docs (`src/lib.rs`) first, then the module docs for
the area you are changing.

## Verifying a change

The `justfile` is the source of truth for verification: `just --list` is the
tour, and the comment above each recipe says what it checks and why.

- Iterate with `just check`, `just test <filter>`, `just clippy`, and
  `just fmt`.
- After editing crate-level rustdoc, run `just readme`: the READMEs are
  derived, never hand-edited.
- Before every commit that touches anything the gate checks, get `just gate`
  fully clean. No gate leg reads `.agent-notes/`, so a commit confined there
  needs no run.
- Bisect `main` with `git bisect start --first-parent`. A merged branch can
  carry a commit whose new test fails until a later commit on it fixes the
  defect; `--first-parent` tests only merges and commits made on `main`.

`rust-toolchain.toml` provisions the stable toolchain on first use; the
justfile's header says which other tools the recipes need. Both toolchains are
pinned deliberately; the comments on `rust-toolchain.toml` and on the
justfile's `nightly_toolchain` give the reason and the bump procedure.

## Writing tests

- Unit and protocol tests live in a sibling file: `mod tests;` in the source,
  `tests.rs` next to it. Cross-peer suites are binaries in `tests/`, built on
  the fixtures in `crates/rumors-testkit` (its `common` module doc maps them).
- Give every test a doc comment stating the behavior and invariant it
  protects. The gate's `testdoc` checks that the comment exists; review holds
  it to the standard.
- Every test must protect behavior that could plausibly regress: exercise a
  boundary where a wrong implementation would matter, never an enum
  constructor, an error pass-through, or rendered prose for coverage's sake.
- When the claim is a family (a boundary, an ordering, a schedule), test it as
  a proptest invariant or exhaust the finite domain. A point unit test's name
  and prose state the exact case it protects, never a family-wide claim.
- Leave every test, helper, fixture, and strategy you touch more legible:
  understand how it establishes its invariant, then simplify its names,
  structure, and prose to match. Preserve its coverage and semantics unless it
  is wrong, vacuous, or weaker than its claim; then fix it and call out the
  behavioral correction for review.
- Commit every proptest seed file that appears, and never strip one from a
  diff. Seeds replay only from the path proptest derives; `tests/main.rs`
  explains that path, and `tests/seed_liveness.rs` fails any seed proptest
  would not read.

## Writing documentation

- Give every item a doc comment stating its purpose, private items, trait
  impls, and test helpers included; add detail only where it helps.
- Write public rustdoc for the developer *using* the library, following the
  Diátaxis quadrants where they apply, and never mention what only a reader of
  the source can see. Write private docs and comments for the maintainer who
  needs to *understand*, *orient*, and *modify*.
- Describe public lifecycle operations as joining, gossiping, and leaving a
  network. Internal peer identity is explained in `Bookmark` and in maintainer
  or protocol docs that need the mechanism; ordinary API use should not
  require it.
- Document a module's purpose, invariants, and guarantees without binding the
  reader to its internal structure.
- Prefer plain, teaching-register prose (_Style: Lessons in Clarity and
  Grace_) to self-invented jargon; define any term you must introduce.
- Inside private modules, `pub` and `pub(crate)` are both accepted; neither
  marks an item as external API.

## Hard rules

No gate leg checks these; review does.

- Nothing in the tree refers to code that no longer exists: no "formerly", no
  "was removed", no deleted API names. Restate prose in terms of what is;
  provenance lives in git history.
- Code may cite the Lean artifact by *theorem or definition name*, never by
  file path, when a kernel-checked statement backs the claim, and still states
  the invariant inline.
- The model of record is uniform-hash and authenticated-honest-peer: the
  transport is pre-authenticated and authorized, and an authorized peer already
  holds write authority over the set. Hostile peers are off-model, so no
  design or cost argument rests on adversary economics, and fail-fast checks
  detect conformance bugs rather than enforce security. Add no machinery to
  guarantee termination against non-conforming peers or links (applications
  own deadlines for those waits); preserve progress under conforming traffic
  and propagate detected failures promptly.
- Never let two independently `seed`ed universes interact. Within a universe,
  the linearity of parties is the invariant everything rests on (see
  `before`'s `Party` docs).
- Redaction leaves no tombstones: a deletion is honored by observing versions,
  so reason about it in version ceilings and floors, not markers.
- The `insta` snapshots and the exact byte counts in
  `tests/protocol_overhead.rs` pin the wire and storage formats. Re-accept
  them only for a deliberate, owner-ruled change to a format or to the capture
  renderer, named in the re-accepting commit, never to accommodate drift. Once
  a release ships, a wire format change means a new protocol version, never a
  mutation of a released one.
  - A bookmark snapshot that moves because the encoding changed is an on-disk
    format change: bump `BOOKMARK_FORMAT_VERSION` in the same commit.
  - A snapshot that moves only because its test fixture changed is a fixture
    change; say so in the commit.
  - To re-accept: `just test-all`, then `cargo insta review` (from
    `cargo install cargo-insta`), then commit the updated `.snap` files.

## Your own notes

You may leave durable notes and other artifacts of exploration in
`.agent-notes/`; `.agent-notes/AGENTS.md` sets the conventions. Notes are
dated and unaudited, never documentation of record: verify a note against the
tree before relying on it.
