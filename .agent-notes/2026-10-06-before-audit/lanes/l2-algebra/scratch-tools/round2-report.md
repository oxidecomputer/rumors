# Lane L2, round 2: L8's survivors against the L2 probe, and the tie order

<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), auditor for lane L2, as the record of a bounded task the coordinator set on 2026-10-07. -->

## Verdict

- **No survivor is caught.** The probe passes against every algebra-relevant
  survivor from L8's version, span, and rest groups (18 mutants) and against
  two tie-order mutants of my own. So no survivor argues for folding the
  probe into the committed suite on mutation evidence.
- **No misclassification.** Wherever the probe reaches a mutant L8 classed E
  or U, it confirms that no value changes on 3,200 generated cases per
  mutant. A pass cannot prove reach for the U arm or for `writer.rs:155`, so
  those two rows give no evidence either way.
- **The tie order is settled.** All four `CursorSet` implementations produce
  the same verdicts in any tie order. Order changes only internal state and
  work in the query filter, never a verdict, and I give the input where it
  does.
- **The ready branches change no result.** This is established by reading
  and by a run of the probe on a trial merge of the two branches (below).

## How the survivors were run

- Base: `main` `1745d377`, merged into `explore/l2-algebra` (merge
  `5c53b5d26`). `main` changed no file of this lane's code since the round-1
  base `58285ca5`, so L8's diffs apply unchanged.
- Patches: `crates/before/tests/l2_probe/mutants/00`–`17`, extracted from
  `lanes/l8-adequacy/round-2/survivors/diffs/{version,span,rest}.md` (patch 08
  regenerated because the markdown dropped a blank context line), plus `18`
  and `19` (tie order reversed).
- Loop: `crates/before/tests/l2_probe/mutants/run.sh`, run once on ox-east-1
  at `b2dff4dc8` with
  `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; bash crates/before/tests/l2_probe/mutants/run.sh`.
  Each patch is applied, the probe run, the patch reverted, and
  `crates/before/src` checked against HEAD (the loop stops otherwise).
- Per mutant: the 16 parallel copies of `algebra_matches_model` at
  `PROPTEST_CASES=200` (3,200 tapes, fresh random each run, no failure
  persistence) plus `deep_surfaces_are_stack_safe` and
  `deep_fork_halves_are_stack_safe`.
- Every log shows one rebuild of `before` and `18 tests run: 18 passed`. The
  baseline passed the same way, and every revert verified (`restored=yes`).

## Results

"Reached" is my reading of the call graph from the probe's entry points; I
did not instrument coverage.

| # | Survivor | L8 class | Reached by the probe | Probe |
|---|---|---|---|---|
| 00 | `version.rs:162` `Hash for Version` → `()` | T\* | yes (hashes every equal pair) | passes: its only hash assertion, equal values hash equally, holds for a constant hash. L8's coherence brief is the right instrument. |
| 01 | `version.rs:776` `&=` → `\|=` (`span_all`, `(Input, Merged)` arm) | U | no evidence of reach | passes. Were the arm reached, `lo \|= a` would break the meet, which the probe checks on every `span_all`. Passing over 3,200 cases with up to 40 items is consistent with U. |
| 02 | `version.rs:777` `\|=` → `&=` (same arm) | U | as 01 | passes |
| 03 | `io/regions.rs:122` `peek_flip` → `0` | C | yes (projection materialization, masked comparisons) | passes: disables the bulk skips only |
| 04 | `io/regions.rs:122` `peek_flip` → `1` | C | yes | passes. A skip needs an unowned mask region, which always lies at depth ≥ 1, so `1 >` that depth never holds. |
| 05 | `io/writer.rs:155` `>` → `>=` (`take_left`) | C | plausible, not measured: needs a cascade collapse whose left code is exactly 63 bits (delta magnitude in `[2^30, 2^31)`; the generator emits `2^31 − 1`) | passes |
| 06 | `io/writer.rs:255` `>` → `==` (`splice_continuation`) | ? | **no**: only `tick` reaches `copy_subtree_remainder` (`tick.rs:794`, `tick/output.rs:119`), and the probe's cases never tick | passes; says nothing. This stays with the events lane. |
| 07 | `io/writer.rs:255` `>` → `>=` | E | no | passes; says nothing |
| 08 | `io/writer.rs:410` `\|` → `^` (`push_bits`) | E | yes (every narrow payload) | passes |
| 09 | `overlay.rs:142` `>` → `>=` (`advance_set`) | E | yes (masked comparisons and span placement, whose equal-depth ties are frequent) | passes |
| 10 | `projection.rs:110` capacity `+` → `*` | C | yes | passes |
| 11 | `projection.rs:355` `!=` → `==` (`others_deepest`) | C | yes (the block skip then never fires) | passes |
| 12 | `projection.rs:375` `block_skip` → `()` | C | yes | passes |
| 13 | `version/shape.rs:174` `>` → `>=` (`advance_refinement`) | E | yes (`combine`, `Clock::shape`) | passes |
| 14 | `fold.rs:27` `==` → `!=` | C | yes (every fold of two or more inputs) | passes: the counter degenerates to a left fold of the same values |
| 15 | `fold.rs:32` `+=` → `*=` | C | yes | passes, as 14 |
| 16 | `span/wire.rs:136` `>` → `>=` | G | entry point yes (`Span::decode` of valid hulls), distinguishing input no | passes; says nothing. The G needs a buffer holding only a lower endpoint, which the probe never builds. L8's brief covers it. |
| 17 | `span/algebra.rs:410` delete `!` (`fold_endpoints`) | C | no (the probe calls no span-algebra fold) | passes; says nothing |
| 18 | tie order reversed in `projection.rs`'s `CursorSet` priority | (mine) | yes | passes |
| 19 | tie order reversed in `place.rs`'s `CursorSet` priority | (mine) | yes | passes |

Survivors I did not run, and why:
- L3's and L4's code: `measure/*`, `min_ticks/*`, `range_minima/*`, `tick/*`.
- `place/filter.rs`: the query filter, which the probe does not reach.
- `version/instrument.rs` (class I): meter-only entry points.
- `shape.rs` `size_hint` (class T): advisory, and the probe asserts no
  `size_hint`.
- The remaining `rest` entries (borsh, serde, recurse) are not lattice
  operations.

## The tie order in `CursorSet` walks (`projection.rs:420` and its siblings)

My filed round-1 records (`lanes/l2-algebra/round-1/`) do not mention
`projection.rs:420`, so the question may have come from another lane.
It is settled here regardless.

There are four implementations: `projection::Comparison`, `place::Cursors`,
`place::filter::MemberCursors`, and `place::filter::SpanCursors`.
`overlay::advance_set` drives each one.

**Claim.** In every implementation, the verdict is the same for every order
of tied steps.

**Argument**, verified by reading at `1745d377`:

1. **The same slots step whatever the order.** `advance_set` steps the deepest
   slot, then every other slot whose depth is at least the returned flip
   level.
   - The flip level is a property of the boundary point, not of the slot
     that stepped first. `debug_assert_eq!(tied, flip)` enforces this.
   - A slot's depth changes only when that slot itself steps.
   - Dropping a bound, settling an endpoint, and a hook's `Fate::Drop` all
     happen in the read phases between `advance_set` calls, never inside a
     step (`filter.rs:343-366` and `:585-620`, `place.rs` `walk`).
2. **Every step only adds into running quantities.** Each `step` folds its
   crossing's delta into accumulators, by add or subtract: `diff` and the
   height integrators, the per-bound differences, `probe_height`, and
   `bound`. Accumulator addition is exact and commutative, so after a tie
   round every accumulator holds the same value in any order.
3. **No verdict is read inside a tie round.** Signs are read only by `read`
   and `resolve`, after `advance_set` returns.

**The one order-sensitive operation does not reach a verdict.** In the
filter, `Comparison::fold` (`filter.rs:183-190`) inspects the current widths
of the probe and bound heights. It drops the cached private difference when
they are not `Close`, and both heights already include the stepping side's
own delta at that point. So the order of a tie decides whether the cache
survives the round. The verdict cannot change: the absolute heights are
always updated, and `read` recomputes the sign exactly through
`compare_heights` when the cache is gone.

**The input where order matters**, which changes work only. Constructed from
the code; I did not run it.
- In a two-bound query walk (`MemberCursors`, slot order
  `[probe, bounds…]`), take a probe and a bound whose heights both occupy 10
  base-2^32 digits and are `Close`.
- At one shared boundary, the probe rises to 12 digits and the bound to 14
  digits. A shared boundary means both leaves end there at the same flip
  level.
- Probe first, as committed: the probe's fold sees widths (12, 10), which is
  `Close`, so the cache is kept; the bound's fold sees (12, 14), still
  `Close`, so the cache is kept and updated.
- Bound first: the bound's fold sees (10, 14), which is `BoundFarWider`, so
  the cache is dropped, and the next `read` rebuilds it.
- The verdict is identical; the reversed order does one extra
  `compare_heights`.
- The same holds for `SpanCursors`.

**Empirical support.** Mutants 09, 18, and 19 change the tie order in the
walks the probe reaches: `advance_set`'s choice among equally deep slots, and
the masked and placement priorities. Each passes 3,200 cases. The probe
does not reach the filter's two walks, so for them this record rests on
the argument above.

The doc comment at `projection.rs:417-419` ("Every fold is a commutative sum,
so this order cannot affect the verdict") is accurate.

## The ready branches

Three branches were named:
- `simplify/lattice-one-sweep` (`cdba4b5f6`) touches only `lattice.rs`.
- `simplify/lattice-entry-delegation` (`6efb45624`) is an ancestor of the
  third.
- `simplify/clone-free-dedup-and-moves` (`ab548dafd`) touches `version.rs`,
  `fold.rs`, `span/algebra.rs`, and meter tests.

**By reading,** no result in the table would change:
- None of the 20 mutated sites is in `lattice.rs`.
- On the clone-free branch these are textually unchanged, only shifted:
  - the `span_all` `(Input, Merged)` arm (01, 02), still unreachable by the
    same counter discipline, because `balanced_try_fold` is unchanged;
  - `balanced_try_fold`'s weight logic (14, 15);
  - `advance_set` (09);
  - `Hash for Version` (00).
- The other sites are in files the branches do not touch.
- Site 17 moves: the branch rewrites `fold_endpoints`' duplicate filter. The
  probe does not reach it on either tree.

**By running** (box run 2), with `crates/before/tests/l2_probe/mutants/run_on_tree.sh`
at `1023d3ea8`:
- The tree was `1745d377` plus an uncommitted trial merge of
  `simplify/clone-free-dedup-and-moves` and `simplify/lattice-one-sweep`.
  The merge was clean, six source files differed from HEAD, and I aborted it
  afterward.
- The probe passed 19 tests each time: `algebra_matches_model` and the 16
  copies at 200 cases, plus both deep tests.
- That held for the unmutated baseline and for mutants 01, 02, 14, and 15,
  the survivors in files the clone-free branch rewrites.

So on about 3,400 generated cases, the independent model finds no semantic
change in the three ready branches.

## Records and commands

- Commits on `explore/l2-algebra`:
  - `5c53b5d26`: merge of `1745d377`.
  - `b2dff4dc8`: patches and `run.sh`.
  - `1023d3ea8`: `run_on_tree.sh`.
  - This record.
- Logs, in my scratch directory: `round3/` and `round3-mutants.log`
  (run 1); `round3-branches.log` and `round3-branches/` (run 2).
