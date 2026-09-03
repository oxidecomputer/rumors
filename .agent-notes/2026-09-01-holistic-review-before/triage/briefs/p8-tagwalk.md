<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling 115 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P8 lane: the id reader's tag cursor

## Goal

`IdReader::tag` (`crates/before/src/idbits.rs`) decodes each id node's
2-bit tag by calling `BitsView::bit(pos)` twice; each call asserts
`pos < live`, computes `pos / 8` and `pos % 8` in `u64`, indexes the byte
with a second bounds check, and shifts. On a wasm32 guest every `u64`
operation lowers to several instructions, so a deep fork chain's id,
nearly all tag nodes, decodes at about 101 fuel per bit against the fuel
law's 53, which is what the fuzzfit sentry found and the gate lane's
bisect traced to the commit that replaced bitvec's bit slice with the
crate-owned view. The invariant restored: the tag walk reads a node's
two tag bits with one gathered access and one bounds check per node, in
`usize` arithmetic, on the owned representation; every reading the walk
produces is unchanged; the recapture is measured, not argued.

## Ground rules

The standard ones: base the SHA the coordinator names (the gate lane's
tip); never EnterWorktree; touch nothing outside the worktree and the
lane's scratchpad; builds and tests on the illumos box through the
wrapper with `--locked`, `CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`,
no `pset-run`, no bench, no wall-time measurement; annotation rows per
changed region; report, never decide. Ruling 88: fuel and meter readings
are ceilings; an improvement is landed by tightening with attribution,
never a re-pin upward. Ruling 43: typed references. Rulings 109 and 113
for any test. Every prose change passes `PROSE.md`.

## The change

- `IdReader::At` keeps its interface (`new`, `at(bits, pos)`, `read`,
  `peek`, the recorded `pos` that `split`'s `build_split` resumes from)
  and gains a `usize` cursor (byte index and bit offset within the byte,
  or an equivalent) advanced as the walk moves, so `tag` reads both bits
  of the current node from the current byte with one shift and mask
  (crossing a byte boundary at most once per node, since a tag is two
  bits and nodes are consecutive), and checks bounds once per node
  against the view's live length. `BitsView::load_be` is the existing
  gathered-window primitive if a two-bit window through it is already
  cheap; measure before choosing. Positions stay `u64` at the interface
  (a recorded position is a bit offset); the cursor is derived from it
  once at `new`/`at` and kept in step.
- The scan meter's `record_bits(2)` per node is unchanged; touch and
  limb readings are unaffected by construction. Any other `.bit(` caller
  in the trusted-stream walks with the same shape (a loop of single-bit
  reads on consecutive positions) is a finding to report, not to change
  here.

## Acceptance

1. Correctness: the id suites (`idbits`, `party::ops`, the fill, grow,
   split, and diff tests, the differential and exhaustive suites over
   ids) pass unchanged; no test moves.
2. The measurement, at the parent and after, on the box: the fuzzfit
   harness's per-bit fuel for `ff_party_decode` over the calibration
   corpus (the harness's `diag` over its 512 programs, as the gate lane
   used) and on the seeded fork-chain program in
   `crates/before/fuzzfit/harness/proptest-regressions/enforce.txt`
   (13,742 fuel at 136 bits at the parent). Both numbers in the commit
   message, and the per-bit constant before and after. No band is
   re-pinned here (the fuzz lane calibrates once after rebasing onto
   this lane); the sentry will still read red on the committed seed at
   your tip if the band's law is still above the new constant, and that
   is expected: quote it.
3. The meter suite (`cargo nextest run --locked -p before --all-features
   -E 'binary(meter)'`) at the parent and after: every id-row reading
   byte-identical (`ID_JOIN`, `ID_COVERS`, `ID_DISJOINT`, `ID_WITHOUT`,
   and every row whose scenario decodes an id); a moved reading is a
   stop with both numbers.
4. `cargo clippy --locked -p before --all-features --tests -- -D
   warnings` and `cargo fmt --check` clean; one `just gate` at the end
   (the audit stream reads red on the base for the gate lane's `syn`
   stop and the wasm stream red on the committed seed; quote both as
   inherited; every other stream ok).

## Hazards and stops

- Any id-row meter reading moving; any test moving; any change to a
  public signature or rustdoc contract (the reader is `pub(crate)`);
  any `.bit(` caller changed beyond `IdReader`.
- The `assert!` in `BitsView::bit` is the panic policy's programmer-error
  guard; the cursor keeps an equivalent guard once per node, in the
  message's own words, never drops it.
