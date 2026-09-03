<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T168 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P1 lane `p1-envelope`, follow-on: the protocol-overhead grid

## Goal

The crate doc prices the protocol's metadata per message with one
figure at one shape. The sweep in `tests/dispute_wire.rs` showed the
figure is a function of the shared history (about 5 KB per message at
256 shared messages, 13 KB at 65,536, for a two-message session), and
the `gossip_fixed` benchmark shows insertions and redactions cost
differently by direction. The invariant this follow-on lands: the
protocol's wire overhead, per direction, is a pinned function of the
session's shape (shared history, insertions on each side, redactions on
each side), measured with a zero-length byte-string payload so the
bytes are protocol overhead alone, and every qualitative claim the crate
doc makes about that cost is asserted as a trend over the pinned grid.

## What to build

- **The grid.** A test binary (`tests/protocol_overhead.rs`, its own
  binary so its slow-timeout override, if one is needed, is scoped) with
  one test per cell. A cell is `(shared, left_inserts, right_inserts,
  left_redacts, right_redacts)` and pins two numbers exactly: bytes
  from left to right, and from right to left. Axes run in powers of ten
  (0, 1, 10, 100, ...), scaled as far up as the budget below affords.
  The grid is sparse: not the product of the axes but the sweeps that
  answer the doc's questions. At minimum: shared history swept at each
  of the divergence shapes (1, 0), (1, 1), (10, 10), (100, 100), with no
  redactions; insertions swept one-sided and two-sided at two shared
  sizes; redactions swept the same way (redacting shared elements) at
  two shared sizes; and the mixed shape, one side inserting while the
  other redacts. State the design as a table in the module doc, each
  sweep beside the question it answers.
- **Per-direction counting.** The counting link in `dispute_wire.rs`
  shares one counter; split it so each direction is counted alone, and
  keep the total as their sum.
- **Cached intermediates.** Build each shared corpus once and derive
  every cell at that size from it (clone or re-fork), so the suite's
  cost is one build per shared size plus the sessions. Prove the cache
  changes no byte: one test at a small shared size compares a cell
  measured from a fresh build with the same cell measured from the
  cached one (fixed sign, so no measurement gates the design; the
  equality is the proof).
- **Shape claims as tests.** Over the pinned grid, assert the trends
  the crate doc states: at fixed divergence, cost per divergent element
  is non-decreasing in shared history; whatever the measurements show
  about redactions against insertions and about direction asymmetry is
  stated as a trend only if the grid supports it at every point,
  otherwise reported as a finding, never asserted by hand.
- **The crate doc derives from the grid.** The regime paragraph's
  numbers come from named cells; `operating_envelope_figures_are_one_line`
  extends to the new figures. Draft the restated paragraph (the crate
  doc is Finch's to word) and annotate it as a draft for him; the
  existing envelope cells become the first rows of the grid rather than
  a second table.
- **Payload.** The zero-length byte string (the one-byte CBOR string
  `0x40`); say in the module doc why it, and not a unit payload, is the
  protocol-overhead measurement.

## Rulings and rules

- T168 (this follow-on); T17 (figures derived by committed tests); T151
  and T157/T161 (no `cases`, no rejecting strategies); T141 (prose).
- The pins are wire-format pins under the same re-accept rule as the
  snapshots: moved only by a deliberate, owner-ruled format change
  named in the commit; the failure message says so. AGENTS.md's pin
  paragraph names the binary.
- Every test has a doc comment stating its invariant; a cell test's
  doc names its cell and the sweep it belongs to.
- Public rustdoc names no source path and no undefined referent.

## Resource discipline

Base: the envelope lane's tip after its round-2 repairs land and its
gate is clean; this follow-on is further commits on `triage/p1-envelope`
with annotation rows. Build and run on the illumos box under the lane
caps (`CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`, the zero rejection
budgets), never with a pset; the gate under the mutex with the pid file.
Budget for the suite: every cell test under 60 s on the box so none is
flagged slow, the binary under three minutes in total at 32 threads;
scale the axes up to where that holds and record the largest size each
axis reaches and why it stops there. Never iterate on timings; measure
once per design change. Commit at logical units with `--no-gpg-sign`
while the signer is locked; report the sha, the grid table with its
numbers, the trends asserted and those refused, and the draft paragraph.
