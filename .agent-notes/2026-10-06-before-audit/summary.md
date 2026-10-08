<!-- CAVEAT LECTOR: written by Claude (Opus 5.5) for Finch at the end of the audit's build phase, 2026-10-08, from the ready entries, the lane records, the landed commits, and the review reports. Unaudited: verify any claim against the cited branch, commit, or record before relying on it. -->

# What the audit of `before` and `suanpan` achieved

The audit set out to do three things in the `before` and `suanpan` crates,
starting from `main` at `455e97de` on 2026-10-06 (see [`README.md`](README.md)
for the plan and [`baseline.md`](baseline.md) for the starting state):

1. find and fix correctness defects;
2. find and fix asymptotic performance defects, taking constant-factor wins
   where they came cheaply;
3. improve test coverage along the way.

This document records the outcome as of 2026-10-08, when every ruled plan item
had been built and reviewed. It is written for a reader who has not seen the
audit's conversation, and it carries the substance of each result itself,
because the ready entries in `QUESTIONS.md` (git-excluded) are deleted as their
branches land.

## Contents

1. [Summary](#1-summary)
2. [How the audit ran](#2-how-the-audit-ran)
3. [Correctness](#3-correctness)
4. [Performance](#4-performance)
5. [Test coverage and instruments](#5-test-coverage-and-instruments)
6. [What was stopped, withdrawn, or not built](#6-what-was-stopped-withdrawn-or-not-built)
7. [What remains](#7-what-remains)
8. [Index of every branch](#8-index-of-every-branch)

## 1. Summary

- **Correctness.** The audit found six defects in shipped code. One fix has
  landed on `main`: suanpan's limb-stream index wrapped on 32-bit targets
  and returned a wrong value. The other five are fixed on reviewed
  branches:
  - a fork iterator's inexact size hint;
  - a panicking capacity hint;
  - a zero's shift retaining space in proportion to the shift;
  - a 32-bit stability query skipping its promised compaction;
  - sealed results retaining their writers' spare capacity.

  A seventh defect, `Version::min_ticks`'s transient heap, is documented and
  deliberately left open. Beyond those, the audit corrected about thirty
  documentation statements that contradicted the code, including suanpan's
  cost table.
- **Performance.** Two asymptotic defects are fixed on branches:
  - suanpan's zero-range map, whose cost grew superlinearly on sparse
    workloads;
  - the zero-shift space defect above.

  The sealed-result fix bounds retention at twice the encoded length.
  suanpan's cost table is re-derived with a written proof, removing the
  logarithmic factor from every row but one, and one constant per-call cost
  in the bitset is removed. Constant-factor wins and simplifications
  ride along.
- **Test coverage.** A mutation campaign over 3,370 mutants found the
  committed suite killing 91.1% of viable mutants. The audit closed every
  surviving cluster that a reachable wrong value could hide in. It added:
  - 32-bit pins;
  - stack-safety tests at depth 2^18 for every public entry point that
    walks a tree;
  - new properties for touches, ticks, serde, and readout classes;
  - board and gate instruments.

  A further 161 instruments the audit built are catalogued for the owner to
  choose from.
- **State.** Three changes are on `main`. Forty-five ready entries, covering
  forty-six branches, wait for review in the order of notice 96, each
  signed, independently reviewed, and checked on ox-east-1.

## 2. How the audit ran

**Lanes.** Eight auditors each examined one area, in rounds, on their own
`explore/*` branches, and wrote findings as briefs for other roles:

| Lane | Area | Records |
|---|---|---|
| L1 identity | `Party`, `Clock`, forks, joins | [`lanes/l1-identity/`](lanes/l1-identity/round-1/report.md) |
| L2 algebra | the `Version` lattice and folds | [`lanes/l2-algebra/`](lanes/l2-algebra/round-1/report.md) |
| L3 events | ticks and `min_ticks` | [`lanes/l3-events/`](lanes/l3-events/round-1/report.md) |
| L4 measures | `Rank`, `Count`, measures | [`lanes/l4-measures/`](lanes/l4-measures/round-1/report.md) |
| L5 spans | `Span`, `Query`, coverage | [`lanes/l5-spans/`](lanes/l5-spans/round-1/report.md) |
| L6 codecs | binary, borsh, and serde codecs | [`lanes/l6-codecs/`](lanes/l6-codecs/round-1/report.md) |
| L7 suanpan | the `suanpan` accumulator | [`lanes/l7-suanpan/`](lanes/l7-suanpan/round-3/report.md) |
| L8 adequacy | whether the tests catch wrong code (mutation testing) | [`lanes/l8-adequacy/`](lanes/l8-adequacy/round-2/report.md) |

**Roles.** A *demonstrator* turned each defect into a failing test on a fresh
branch, and a *fixer* repaired it on top. A *builder* implemented machinery
and simplification briefs. A *reviewer*, independent of both, reviewed every
branch adversarially, in rounds, until its findings stopped being defects. The
role briefs are in [`briefs/`](briefs/common.md).

**Verification.** Every branch's tip passed a landing check on ox-east-1
(`~/bin/audit-check`) with six legs:
- tests (including the `rumors` snapshots);
- lints;
- docs (rustdoc both ways with warnings denied, plus doctests);
- the public-surface census;
- the amplification board, with its worst-case ranking pin;
- the wasm32 pins, fuzz-fit, and fuelscape.

Each branch's readings were compared with [`baseline.md`](baseline.md):
759 tests, 142 snapshots, 193 + 3 doctests, a 5,311-cell board passing
with clean pins, and wasm 8/25/43.

**Rulings.** The owner ruled on more than a hundred questions. Rulings are
recorded beside the finding they decide, under "Owner's ruling" in its lane
record; the plan-level rulings are in [`README.md`](README.md). Two rulings
shaped everything:
- correctness is judged by the contract clause breached, never by how likely
  the input is;
- after a period of new-instrument construction, the owner stopped work on
  new instruments against imagined threats, while keeping the instruments
  already built as an asset ([`instrument-rescue.md`](instrument-rescue.md)).

## 3. Correctness

### 3.1 Defects in shipped code

Each was demonstrated by a test that fails at its base before any fix.

| Defect | Contract breached | Fix | State |
|---|---|---|---|
| suanpan limb index wraps on 32-bit | `add_shifted_limbs` panics on an unaddressable landing; on wasm32 a limb past index 2^32 instead landed at a wrapped position and returned a wrong value | index counted in `u128` | **landed**, `0b23cc36` (pin) and `c9a0ea68` (fix) |
| Fork iterators' size hint | `PartyForks` and `ClockForks` hinted `(0, None)` for the last `usize::MAX` shares of a count wider than twice `usize::MAX` | remainder held as `BigUint`; hint exact whenever it fits `usize` | #43 |
| `reserve_digits(usize::MAX)` panics | documented as a value-neutral hint with no `# Panics` section, it panicked with "capacity overflow" | owner-ruled API change to `reserve_bits(u64)`, which saturates and ignores an unsatisfiable request | #28 |
| A zero's shift retains space | a zero stored as cancelling digits, shifted by 33,554,432 bits, kept 1,048,578 digits, and trapped on wasm32 | one read-only zero scan, only when a deposit would extend the buffer | #48 |
| 32-bit stability query | on 32-bit, `cmp_zero_stable_under` returned early above `32 · 2^32` bits and skipped the compaction its rustdoc promises (the answer was right) | the bound stays in `u64` position space | #50 |
| Sealed results retain capacity | sealing kept a writer's whole reservation; a 1-byte party could keep a 138-byte allocation alive | sealing releases slack when it would exceed the stream, so a sealed stream holds at most twice its encoded length | #106 |

Records:
- the limb index: [`lanes/l7-suanpan/round-1/D1-limb-index-wrap.md`](lanes/l7-suanpan/round-1/D1-limb-index-wrap.md);
- fork hints: [`lanes/l1-identity/round-1/D1-fork-size-hint-width.md`](lanes/l1-identity/round-1/D1-fork-size-hint-width.md);
- `reserve_digits`: [`lanes/l7-suanpan/round-1/D2-reserve-digits-panic.md`](lanes/l7-suanpan/round-1/D2-reserve-digits-panic.md);
- the zero shift: [`lanes/l7-suanpan/round-1/Q1-zero-shift-representation.md`](lanes/l7-suanpan/round-1/Q1-zero-shift-representation.md)
  and [`round-2/Q1-fix-and-test-brief.md`](lanes/l7-suanpan/round-2/Q1-fix-and-test-brief.md);
- the stability query: [`lanes/l7-suanpan/round-2/U1-usize-width-sweep.md`](lanes/l7-suanpan/round-2/U1-usize-width-sweep.md)
  and [`round-3/U1a-stability-width-test-brief.md`](lanes/l7-suanpan/round-3/U1a-stability-width-test-brief.md);
- retention: [`lanes/l1-identity/round-1/S1-result-buffer-retention.md`](lanes/l1-identity/round-1/S1-result-buffer-retention.md).

### 3.2 A defect left open: `min_ticks`'s transient heap

`Version::min_ticks` can hold about 125 bytes of temporary heap per input byte
on a constructed input (a rising spine entered by one jump), against the
board's general ceiling of 20. Five repair designs were built and measured,
and each failed in one of three ways:
- one recreated the defect past a threshold the board cannot reach;
- two cut heap at the cost of time that is quadratic on some input;
- two stopped on small rises in fitted growth exponents.

The owner stopped the work (question 87). `main` keeps its implementation, and
the defect, what a fix must achieve, and which instruments must exist first
are recorded in
[`../2026-10-08-min-ticks-transient-heap/README.md`](../2026-10-08-min-ticks-transient-heap/README.md).
The lane record is
[`lanes/l3-events/round-1/defect-D1-min-ticks-heap.md`](lanes/l3-events/round-1/defect-D1-min-ticks-heap.md).

### 3.3 Documentation that contradicted the code

- **#85**, 23 commits, one per item, collected from every lane
  ([brief](coordinator-briefs/docs-branch.md)). Among them:
  - the n-ary folds' cost contract;
  - strict versus non-strict order in `Version`'s docs;
  - `Rank`'s normalization invariant as the code keeps it;
  - `Count`'s conversion to `usize` depends on the target;
  - the coverage soundness law checks soundness only;
  - `Decode::TrailingBits` covers filler inside a value;
  - the borsh serializers documented;
  - `Version`'s causal ordering is `PartialOrd`, not `PartialEq`;
  - `Clock::from_parts` over a stale version can reissue stamps;
  - `BitsWriter::repeat`'s `expect` states the proof its caller supplies.

  `FusedIterator` for the fork iterators is the one addition, which the owner
  approved.
- **#117**, suanpan's cost table. Three statements were false:
  - growth was defined by the observed increase in working width, which a
    cancelling shifted operand defeats;
  - retained space was stated per operation, which geometric buffer growth
    defeats;
  - `reset` and `normalize` named the wrong logarithmic term.

  See section 4.
- **#92.** A wasm32 pin's doc claimed to catch a narrowed cursor position,
  which it cannot.
- **#72.** `recurse.rs` claimed the tree oracle is guarded, which it is not by
  design; a test comment called a flat loop recursive.
- **#53.** `Decode::Io`'s doc did not say that a refused buffer growth is
  reported through it.
- **#51.** `Span`'s `Deserialize` doc named only the binary decoder.

### 3.4 Defects caught inside the fixes

The reviews caught defects in the repairs before any reached the owner. For
example:
- The written-position set in #86 kept a bitset sized to the old width after
  normalizing a value to zero, which breaks the O(`Q`) space promise.
- #48's docs said shifting zero never panics, which an invalid count
  falsifies.
- #106's tests were first too weak to fail the rejected design.
- #117's derivation had two false steps and one missing inference.
- #118's first cost argument could not be checked operation by operation.

Each was fixed on its branch and re-reviewed.

## 4. Performance

### 4.1 Asymptotic

- **suanpan's zero ranges (#86, with the ladder #52).** The accumulator kept
  an ordered map of the zero ranges between written digits, so that a
  descending scan could skip them, and on sparse workloads the map's cost
  grew superlinearly. #52 first committed a fuel ladder that measured the
  growth: 72 cells, measured in wasm fuel, with a ±2% band. #86 then replaced
  the map with a set of written positions, held inline as a prefix, then a
  one-word mask, then a 64-ary bitset, converting in one direction only.
  - Every sparse ladder row is flat after the fix.
  - At r = 4,096, oscillating `min_ticks` costs 53.7% to 66.6% less fuel, and
    interior cells 23.5% to 62.5% less.
  - No board heap reading rises by more than 0.1 B/B, 159 fall, and no scan
    or touch reading moves.

  Records: [F1](lanes/l7-suanpan/round-1/F1-cost-composition-log-factor.md),
  [F1b](lanes/l7-suanpan/round-2/F1b-design-evidence.md),
  [F1c](lanes/l7-suanpan/round-3/F1c-machinery-zero-range-fuel-ladder.md),
  [F1d, with the owner's rulings](lanes/l7-suanpan/round-3/F1d-fix-design-W.md).
- **A zero's shift (#48):** retained space proportional to the shift becomes
  constant (section 3.1).
- **Sealed results (#106):** retention is bounded by twice the encoded length.
  On the board, 854 heap readings fall and 192 rise; the largest fall is
  `party_forks_full` × `meet-shade`, from 10.7 to 1.2 B/B. The rises come
  from a 24-byte shared header that is now allocated at an input's first
  clone, so the board charges it to the first operation that clones; no
  program holds more heap.
- **suanpan's cost table (#117).** It is re-derived against the new code, with
  the argument in a private, doc-only module, `accumulator/digits/costs.rs`,
  written as a potential-function proof. From `main`'s table to the branch:

  | Operation | `main` | Branch |
  |---|---|---|
  | `+=` / `-=` a primitive | amortized O(log(W+1)) | amortized O(1) |
  | `L` limbs | amortized O(L log(W+1) + G) | amortized O(L) at shift 0; O(L + G + log(W+1)) at a nonzero shift |
  | An accumulator | amortized O(A log(W+1) + G) | amortized O(A) at shift 0; O(A log(W+1) + G) at a nonzero shift |
  | Comparison with zero | amortized O(log(W+1)) | amortized O(1) |
  | `normalize` | O(W + Q log(Q+1)) | O(W + Q) |

  The one remaining logarithmic factor is real and tight on a constructed
  input: operand digits `[2^32, −1]` repeated at a shift of `32s + 1`, which
  cancel above a high bitset and climb its levels on every pair. The owner
  accepted it (question 112), because it is a base-64 logarithm of a digit
  position: at most 11 levels on 64-bit targets. The owner also ruled that
  growth is measured by an update's span (question 113), and that retained
  space is stated as amortized wherever storage grows (question 114).
- **Bitset growth (#118).** `Bitset::cover` visited every level on each
  growing call. It now stops at the first level already long enough. Level
  visits per growing call fall from 2.98, 3.98, and 4.75 at three, four, and
  five levels to 2.02. The cost argument now pays for each call within the
  potential method, with a new table-potential term `max(0, 2·len − cap)`,
  and depends on no width of `usize` (question 116).

### 4.2 Constant factors

- **#53:** `Rank::decode_stream` held a long fraction three times at its peak:
  in its group buffer, in a combined image, and in `from_bytes_be`'s copy. It
  now holds it once, and a rank with no fraction builds no image
  ([record](lanes/l6-codecs/round-1/defect-rank-decode-wasm32-growth.md)).
- **#89:** a comparison skips a write-back that would restore the same two
  digits, so each repeated comparison on such a value costs 2 touches instead
  of 6 ([S2](lanes/l7-suanpan/round-2/S2-comparison-fixed-point.md)).
- **#36:** join and meet share one short-circuit ladder, and no clone runs
  before the short-circuit checks
  ([record](lanes/l2-algebra/round-1/simplification-lattice-entry-delegation.md)).
- **#107:** the n-ary folds' duplicate filter holds the next input instead of
  cloning the last, which avoided a 24-byte shared count on the caller's
  value. Owned right operands of `|`, `&`, `|=`, and `&=` move instead of
  cloning ([brief](coordinator-briefs/heap-dedup-and-owned-operands.md)).
- **#61:** `Query::coverage` decides a crossed clamp by `floor <= ceiling`,
  building only the one clamped endpoint its holes need, and only when holes
  exist ([brief](lanes/l5-spans/round-1/brief-refine-partial.md)).
- **#38:** three debug-assert scans whose cost grew with retained storage are
  deleted, each after a committed test was shown to hold its invariant
  ([brief](coordinator-briefs/simplify-debug-assert-scans.md)).

### 4.3 Simplifications

- **#30:** one overlay sweep replaces two copies in the `Version` lattice
  ([record](lanes/l2-algebra/round-1/simplification-lattice-one-sweep.md)).
- **#76:** `Clock::sync_all` merges through `join_all`'s own steps instead of
  a parallel implementation
  ([SB1](lanes/l1-identity/round-1/SB1-sync-all-merge-via-join-all.md)).
- **#63:** `Rank`'s `+` and `checked_sub` align by `u64` shifts, deleting a
  route chosen by `usize` width and the suanpan path that ran only on 32-bit
  targets ([brief](coordinator-briefs/simplify-rank-single-alignment.md)).
- **#67:** three codec cleanups:
  - one padded-prefix step;
  - bit counts typed `u64`;
  - an unreachable early return deleted.

  Records:
  [padded prefix](lanes/l6-codecs/round-1/simplification-padded-prefix.md),
  [bit counts](lanes/l6-codecs/round-1/simplification-bit-count-types.md),
  [splice](lanes/l6-codecs/round-1/simplification-dead-splice-branch.md).
- **#27:** suanpan's readout reads its high part, bounded in `[−3, 2]`,
  directly ([S1](lanes/l7-suanpan/round-1/S1-read-digits-high-part.md)).
- **#83:** `hole_subtracts`, a method with no caller, is deleted.

## 5. Test coverage and instruments

### 5.1 The mutation campaign

The adequacy lane ran cargo-mutants 27.1.0 over the full test suites. The
campaign:
- wrote 3,370 outcomes, finding the suite killing 91.1% of viable mutants;
- classified 249 survivors, 21 of them reachable wrong values that no test
  detected.

Sources: the [round-2 report](lanes/l8-adequacy/round-2/report.md) and
[survivor index](lanes/l8-adequacy/round-2/survivors/INDEX.md).

Branches that close the surviving clusters:
- **#66** ([brief](lanes/l8-adequacy/round-2/briefs/machinery-suanpan-normalize-bound.md)):
  six `normalize` survivors, among them one that reads 2^65 back as zero.
  An exhaustive test over boundary digits of one to four positions checks
  value, width, a later deposit, and the full digit invariants, with a
  liveness floor.
- **#78** ([brief](lanes/l8-adequacy/round-2/briefs/machinery-rank-decode-reader.md)):
  eleven rank-decoder survivors in its `Interrupted` guards and
  end-of-input check. A scripted reader interrupts and fails, against an
  exact oracle.
- **#82** ([brief](lanes/l8-adequacy/round-2/briefs/machinery-range-minima-near-boundaries.md)):
  two range-minima survivors (a subtraction changed to an addition). Values
  are drawn near open minima.
- **#83** ([brief](lanes/l8-adequacy/round-2/briefs/machinery-trait-impl-coherence.md)):
  empty `Hash` bodies, which passed every test. New laws hash every value as
  its byte view, which the owner made a public promise (question 79). It
  also adds `Debug` laws under formatter flags, and a `ClockForks` hint law.
- **#40 and #81**
  ([MB1](lanes/l1-identity/round-1/MB1-join-all-multiplicity.md)): a failed
  `join_all` that returns one identity twice passed every test, because the
  checks compared only unions. The `# Errors` contracts now promise each
  point's owner count. Model tests and the registry laws, which the fuzz
  workspace also runs, check that count.

### 5.2 32-bit targets

- **#29:** pins `Rank`'s `Sum` across the 32-bit alignment limit
  ([MB](lanes/l4-measures/round-1/machinery-wasm32-rank-sum-pin.md)).
- **#54:** pins both public routes to suanpan's direct deposit at digit
  index 2^32 ([MB2](lanes/l7-suanpan/round-3/MB2-add-shifted-wasm32-landings.md)).
- **#31:** the wasm32 pins tell a panic from an allocation abort. Before,
  both trapped alike, so a pin expecting a panic passed on running out of
  memory ([MB](lanes/l4-measures/round-1/machinery-wasm32-trap-diagnosis.md)).
- **#58:** derives the pins' protocol decoders from their declared numbers,
  so a renumbering at landing cannot leave a stale decoder arm
  ([brief](coordinator-briefs/wasm32-pins-check-roundtrip.md)).
- **#92:** records in the validation index what no pin reaches.
- **#50, #48, and the landed limb-index fix** each add a 32-bit pin with
  their fix.

### 5.3 Stack safety

- **#34:** drives the shape walks, the concurrent hull, the folds, and every
  shape iterator at depth, on left and right spines
  ([brief](lanes/l2-algebra/round-1/machinery-deep-surfaces.md)).
- **#75:** sets one depth, `STACK_SAFETY_DEPTH = 2^18`, which leaves at most
  8 bytes of a 2 MiB test stack per level, below any real frame. It drives
  every remaining walk-bearing public entry point at that depth
  ([MB2](lanes/l1-identity/round-1/MB2-deep-identity-probe.md)).

### 5.4 New properties and exact tests

- **#37:** touches stay linear in priced work over arbitrary programs on
  three accumulators, with derived constants
  ([MB1](lanes/l7-suanpan/round-1/MB1-touch-bound-property.md)).
- **#74:** tick parties and versions are co-generated region by region and
  checked against the recursive oracle, with a census that keeps each
  strategy on its regime
  ([brief](lanes/l3-events/round-1/machinery-brief-cogen.md)).
- **#51:** serde's human-readable leniency, which the owner chose, is stated
  and tested: keys in any order, byte keys, sequences, and position keys,
  with rejection of missing, repeated, or extra fields
  ([brief](coordinator-briefs/test-serde-record-framing.md)).
- **#64 and #27:** every reachable final-carry class of suanpan's readout is
  tested for exact value and touches, with a proof that the one remaining
  class is unreachable
  ([MB3](lanes/l7-suanpan/round-3/MB3-readout-high-part-classes.md)).
- **#36:** a heap check holds "no clone before the short-circuit" at every
  arm of the ladder.
- **#107:** folding a run of shared copies must scan exactly what one copy
  scans, for all 21 public entry points that reach the filter.
- **#118:** the existing property test was shown to reach every bitset growth
  case and to fail four deliberately wrong early exits.

### 5.5 Instruments and the gate

- **#52:** the zero-range fuel ladder (section 4.1).
- **#94:** a board row for a bounded query with no holes. No board row
  reached #61's case before, so all 15,933 heap readings were identical
  across it.
- **#95:** two developer modes on the board. `capture` writes every cell's
  exact readings, and `compare` classifies each cell's difference as
  unchanged, a constant shift, growing, or uneven. Rounded exponent diffs
  had twice stopped fixes on fixed-cost changes, and once missed a 40% flat
  rise.
- **#93:** the gate runs the worst-case ranking pin even when board
  acceptance fails. Before, drift went unreported and was attributed to the
  next passing commit.
- **#115:** every measured board ceiling follows one rule,
  `C = ceil(1.25 · max((reading − intercept) / denominator))` over every
  judged sample (questions 65 and 101). Four ceilings tighten, three loosen
  to the rule, and the rest already met it. A deliberately broken
  `min_ticks` adding 5 B/B fails the new heap ceiling of 18 and passed the
  old 20.
- **#110:** `crates/alloc-meter`, a per-thread counting allocator for tests,
  factored out of `rumors`. It removes a flaky heap test: the old
  process-wide counter included libtest's main-thread allocations
  (question 91).
- **#57, and `20519931` (landed):** hung tests are terminated in the detached
  workspaces, and the root limit is 300 seconds
  ([brief](lanes/l8-adequacy/round-2/briefs/machinery-detached-nextest-timeouts.md)).
- **`ddfabe4c` (landed):** the board's worst-case ranking for
  `count_display × heap` is declared target-dependent, because `num-bigint`
  converts decimals differently on x86 ([finding](lanes/l8-adequacy/round-1/findings/count-display-heap.md),
  [brief](coordinator-briefs/board-count-display-pin.md)).

### 5.6 Instruments built but not yet folded in

The lanes built many more probes, models, generators, and meters on their
`explore/*` branches. [`instrument-rescue.md`](instrument-rescue.md) ranks and
categorizes all 161 of them by the coverage each would add, with nine
decisions for the owner. The preserved copies are in `lanes/*/scratch-tools/`,
`probes-preserved/`, and the `archive/rescue-*` branches.

## 6. What was stopped, withdrawn, or not built

- **The `min_ticks` heap fix:** stopped by the owner (section 3.2). The
  demonstration branch `fix/before-min-ticks-heap` is kept, because the
  write-up cites its demonstration commit `08573e15`.
- **The crate page's linearity section:** withdrawn (question 111). The owner
  writes it. The draft, `docs/crate-page-identity-linearity` (`78a180b1`),
  is kept as a reference. Two of its corrections are recorded for that
  pass in the identity lane's report
  ([`report.md`](lanes/l1-identity/round-1/report.md)):
  - `Party`'s docs omit `FromStr` as an escape hatch from linearity;
  - the decode warnings omit two forms of coexistence.
- **The codecs lane's 32-bit buffer-growth branch:** reduced to #53 by the
  owner's ruling that neither crate promises any input size. The branch is
  deleted, and its 32-bit borsh check is kept on `archive/rescue-8208efaeb`.
- **Running libtest single-threaded for the heap tests:** not built. The
  premise was false: libtest's main thread still allocated inside the
  window. The per-thread allocator (#110) replaces it.
- **Not built, by the owner's caution against unbounded instrument work:**
  - a rebalanced party-family generator (a reach gap with no demonstrated
    failure);
  - a lone-endpoint padding test (it would pin a decode precedence the
    contract does not promise);
  - an opt-level-0 stack-safety leg;
  - two lower-ranked survey candidates
    ([`instrument-survey.md`](instrument-survey.md)).

## 7. What remains

- **Review and landing.** The owner reviews the 45 ready entries in notice 96's
  order, starting with #58. The coordinator lands each approved branch by
  signed cherry-pick, rebases what follows, and re-verifies. Decisions inside
  entries:
  - #83's squash;
  - #85's fuelscape edit, `RED_ZONE` doc, and `sync_all` proposal;
  - #86's `reset` retention reading;
  - #95's "fading" class;
  - #110's `HeapMeter` reshape and redundant `rumors` test;
  - #115's enforcement of the ceiling rule;
  - #117's `Sum` wording.
- **After landings:**
  - remove `descend!`, `recurse.rs`, and `stacker` once #40, #72, and #74
    land (question 68);
  - a second documentation pass, for #85's deferred items, the survey's
    verification-map gaps, and the `follow-ups.md` items marked for it;
  - one full `just gate` at `main`;
  - retirement of the slots, worktrees, and box state.
- **Leads outside the audit's finished scope**, in [`follow-ups.md`](follow-ups.md):
  - whether `before`'s integral measure inherits suanpan's logarithmic
    factor, since it adds mixed-sign accumulators at nonzero bit offsets;
  - suanpan costs no meter counts (bitset words; a cancelled shifted
    transient's zero-fill);
  - an audit of `rumors`' share-receiving paths for merging events before
    ticking.

## 8. Index of every branch

Slot paths are `/Users/oxide/src/rumors-slot-NN`. "Order" is the position in
notice 96; "goal" is the audit goal the branch mainly serves (C correctness,
P performance, T test coverage, D documentation).

| Order | Entry | Branch | Tip | Slot | Goal | In one line |
|---|---|---|---|---|---|---|
| 1 | #58 | `audit/wasm32-pins-check-roundtrip` | `bfe4fcad` | 21 | T | wasm32 protocol decoders derived from their numbers |
| 2 | #43 | `fix/before-fork-size-hint` | `b34bb0c8` | 12 | C | exact fork size hint past `usize` |
| 3 | #28 | `fix/suanpan-reserve-digits-hint` | `19154cd1` | 05 | C | `reserve_bits(u64)` replaces a panicking hint |
| 4 | #63 | `simplify/rank-single-alignment` | `7f2a80a2` | 20 | P | `Rank` aligns without a `usize` route |
| 5 | #48 | `fix/suanpan-zero-shift` | `fc4743aa` | 15 | C, P | a zero shifts in constant space |
| 6 | #50 | `fix/suanpan-stability-width` | `cf99aef9` | 18 | C | 32-bit stability query compacts as promised |
| 7 | #51 | `fix/before-serde-record-framing` | `0e63cfc7` | 04 | T, D | serde leniency stated and tested |
| 8 | #83 | `audit/trait-coherence-and-hole-subtracts` | `1ac485f7` | 31 | T | trait laws; `hole_subtracts` deleted |
| 9 | #52 | `audit/suanpan-zero-range-fuel-ladder` | `fe2525bb` | 17 | T | fuel ladder for zero-range cost |
| 10 | #86 | `fix/suanpan-zero-range-bitset` | `b9ec1fbd` | 24 | P | written-position set replaces the zero-range map |
| 11 | #89 | `simplify/suanpan-comparison-fixed-point` | `ec17f661` | 39 | P | no-op write-back skipped; test fixes |
| 12 | #53 | `simplify/before-rank-decode-image` | `38b4d9a5` | 14 | P, D | rank fraction held once; two doc corrections |
| 13 | #78 | `audit/rank-decode-reader` | `7995a948` | 35 | T | rank decoder against interrupting readers |
| 14 | #61 | `simplify/span-refine-partial` | `89afe1d4` | 22 | P | crossed clamp decided by `floor <= ceiling` |
| 15 | #94 | `proposal/board-neutral-coverage` | `94dba951` | 44 | T | board row for a hole-free bounded query |
| 16 | #30 | `simplify/lattice-one-sweep` | `cdba4b5f` | 03 | P | one lattice sweep |
| 17 | #36 | `simplify/lattice-entry-delegation` | `6efb4562` | 02 | P | one short-circuit ladder, clone last |
| 18 | #107 | `simplify/clone-free-dedup-and-moves` | `a2868797` | 23 | P | clone-free duplicate filter; owned moves |
| 19 | #76 | `simplify/clock-sync-all-via-join-all` | `c44a4203` | 34 | P | `sync_all` through `join_all`'s steps |
| 20 | #40 | `audit/join-all-multiplicity` | `72d534bf` | 11 | T, D | `join_all` errors conserve owner counts |
| 21 | #81 | `audit/registry-multiplicity-laws` | `2feb6db0` | 37 | T | registry laws count owners |
| 22 | #27 | `audit/suanpan-readout-high-touch`, `simplify/suanpan-read-high-part` | `9193e030`, `5c0d1c78` | 07 | T, P | high-part touch pin; high part read directly |
| 23 | #64 | `audit/suanpan-readout-class-table` | `a56e688c` | 25 | T | every final-carry class tested |
| 24 | #67 | `simplify/codec-cleanups` | `81f3d682` | 28 | P | three codec simplifications |
| 25 | #106 | `fix/before-writer-retention` | `2dbdec95` | 33 | C, P | sealed results release spare capacity |
| 26 | #38 | `simplify/debug-assert-scans` | `919c752f` | 08 | P | three debug scans deleted |
| 27 | #31 | `audit/wasm32-trap-diagnosis` | `cc9e0ecd` | 09 | T | panic told apart from allocation abort |
| 28 | #92 | `proposal/wasm32-compare-pin-reach` | `1c8e1c5e` | 42 | D | compare pin's doc made true |
| 29 | #54 | `audit/suanpan-deposit-value-wasm32-pins` | `eb07570d` | 13 | T | deposits at digit 2^32 pinned |
| 30 | #29 | `audit/wasm32-rank-sum-pin` | `0c22bd66` | 10 | T | `Rank`'s `Sum` across 2^32 pinned |
| 31 | #34 | `audit/deep-surfaces` | `20da73ad` | 01 | T | stack safety for shape walks and folds |
| 32 | #75 | `audit/deep-identity-probe` | `eef22551` | 32 | T | stack safety at 2^18 everywhere |
| 33 | #37 | `audit/suanpan-touch-bound` | `7f777ce5` | 06 | T | touch-bound property |
| 34 | #66 | `fix/suanpan-normalize-width-shrink` | `7cb1800b` | 27 | T | exhaustive `normalize` test |
| 35 | #74 | `audit/events-cogen-and-tidy` | `183a094a` | 29 | T | co-generated tick pairs |
| 36 | #82 | `audit/range-minima-near-boundaries` | `5b885a21` | 36 | T | range minima near open minima |
| 37 | #57 | `audit/detached-nextest-timeouts` | `1ceb1184` | 19 | T | hung detached tests terminated |
| 38 | #93 | `proposal/worst-case-pin-always` | `c806f12e` | 40 | T | ranking pin runs whatever acceptance says |
| 39 | #95 | `proposal/board-exact-compare` | `3a50ac40` | 43 | T | exact board comparison |
| 40 | #115 | `audit/board-ceiling-one-rule` | `e937662e` | 26 | T | one rule for every board ceiling |
| 41 | #110 | `audit/shared-counting-allocator` | `c30ad499` | 47 | T | per-thread counting allocator |
| 42 | #72 | `fix/before-small-cleanups` | `4ecfbcfd` | 30 | D, C | true stack-guard prose; checked buffer length |
| 43 | #117 | `docs/suanpan-time-bounds` | `5971ca16` | 46 | D, P | suanpan's cost table re-derived |
| 44 | #118 | `fix/suanpan-cover-growing-levels` | `6f55a19a` | 49 | P | bitset growth stops at the first level with room |
| 45 | #85 | `docs/audit-corrections` | `8c3a17ba` | 38 | D | 23 documentation corrections |

Landed on `main` during the audit:

| Commit | Change |
|---|---|
| `0b23cc36`, `c9a0ea68` | suanpan's limb-stream index: pin, then fix |
| `ddfabe4c` | `count_display × heap` ranking declared target-dependent |
| `20519931` | nextest's root terminate limit raised to 300 seconds |
