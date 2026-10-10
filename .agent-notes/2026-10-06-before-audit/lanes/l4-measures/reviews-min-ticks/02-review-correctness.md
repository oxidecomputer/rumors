<!-- CAVEAT LECTOR: the correctness review of 8c2282903 (handed back before its cost probes ran), written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict

**Provisional approve on correctness, with the review incomplete.** I was told to hand back before my cost probes produced output, so the cost items below are open. The value-correctness verdict rests on runs that finished. No blocking defect in the branch's own code was found. Two private comments state something false about the code; they are listed under "Doc and comment findings" and should be fixed before landing. The overall goal ("heap under the ceiling at every input size") is not met, because of a heap regime in `RangeMinima` that predates this branch and that the builder ruled out of scope. Details are under "Goal-level finding".

The worktree is restored. `/Users/oxide/src/rumors-slot-53` is at `8c22829038e92a56efced84a29f9908b42e1b6d1`, `git status --short` is empty, and `git diff --stat` is empty. A remote build or test launched by my last script may still be running on the box under `~/src/rumors-slot-53`. Its local driver was killed and the probe file it needed is gone, so any later step fails harmlessly.

## Landing check at the tip (verbatim)

Command: `on-illumos.sh /Users/oxide/src/rumors-slot-53 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; ~/bin/audit-reserved ~/bin/audit-check'`

```
8c22829038e92a56efced84a29f9908b42e1b6d1
audit-check: ok      docs (14s)
audit-check: ok      lints (14s)
audit-check: ok      surface (49s)
audit-check: ok      tests (106s)
audit-check: ok      wasm (155s)
audit-check: ok      board (202s)
audit-check: clean in 202s; logs in target/audit-check-logs
```

Summary lines from the box's leg logs:
- `Summary [  33.686s] 828 tests run: 828 passed, 1 skipped`
- `Summary [  69.009s] 142 tests run: 142 passed (1 slow), 594 skipped` (the `rumors` snapshot filter)
- `amp-board: 5545 green / 0 red (5545 cells)`
- `worst-case pin: clean (272 pinned rows verified at 2 sampling scales)`
- `surface totality: 214 public function-like items = 134 board-covered + 80 excepted`
- doctests: 193 and 15 passed
- wasm: 12/12, 25/25, 43/43

These match the builder's report. `baseline.md` was recorded at `9a90b767` (759 tests, 211 items); the differences come from #122 and this branch.

## What I verified, and how

**By reading the code at the tip**
- **Records stay in stack order.** I checked `SubtreeMinima`'s handling of every `RangeMinima` event: suspend, equal arming, negative arming, undercut, `Close::Lower`/`Retired`. Every settlement pops the top record, and the record stack always holds one more entry than there are positive boundaries. So the stack-order invariant the design rests on holds.
- **References stay referenced.** Every reference below the top keeps a live record: a freeze merges an unreferenced top at once, and settling the first record at the second reference removes it. That justifies the `expect` in `settle_record` and the end-of-sweep `buried == 0`.
- **The coefficient transfer is correct in both directions** (`heights.rs:300-328`): the sign handling, the narrower-side choice, and `remove_second` re-reading the third coefficient before `charge_older`.
- **The `IntegerStack` push/pop split is consistent**, given minimal limbs: `iter_u64_digits`, and `signed_magnitude` documented as minimal.
- **`WP(320, d)` crosses both 256-bit thresholds by construction.** It freezes `W+1` (321 bits) at every second level, so the chunk moves whole, and stores 321-bit record offsets, so they get separate allocations.
- **No ghost references** to `contributions` or `HeightPrefixes` remain in `crates/before`.

**Differential runs at the tip (passed)**
- I wrote a reversible probe, `review_probe.rs`, saved at `<scratch>/review-mt-correctness/review_probe.rs`. It builds normal-form trees in an arena and computes `min_ticks` independently, as the sum of normal-form bases. The reference itself agrees with the tree oracle on every tree under 400 nodes.
- Swarm sweep: 400 cases, shapes from right spine to left spine to random, up to 4,000 leaves, jump widths drawn from classes between 0 and 530 bits, equal runs, exact jump reversals. Result: `PROBE differential cases=400 seed=1 oracle_checked=222`, passed.
- Targeted spines: 1,440 cases. Entry jumps were 2, 31–33, 63–65, 200, 255–257, 288, 289, 320 and 600 bits. Steps came from eight classes, undercuts every 0, 3 or 17 levels, four shapes, depths 260–900; zero steps give close counts well past 255. Passed.

**Mutation calibration (all four mutants detected; file restored)**
Each mutant was one string swap in `heights.rs`, run against both differential probes, then reverted. `restored True` was checked by `diff` against a saved copy. Every mutant failed both tests:
- older-neighbour product sign: exit 100
- third coefficient read after the merge: exit 100
- below-top settlement charged to the top: exit 100
- freeze drops an unreferenced top's coefficient: exit 100

Logs are `mutant-*.log` in my scratch directory.

## Doc and comment findings (false statements about the code)

1. **`crates/before/src/version/measure/min_ticks.rs:44-47`.** The comment says "Every leaf and every subtree minimum lies at least the first leaf's height above zero". That is false: leaves can sit below the first leaf, which is why offsets and chunks are signed. The conclusion is still right, for a different reason: the coefficients (leaves minus closes) sum to one, so h0 enters exactly once. The premise should be restated.
2. **`crates/before/src/version/measure/min_ticks/minima/heights.rs:19-23`.** The doc says "A reference is retained only while a live minimum record refers to it … When the last record at a reference is settled, that reference is the second-highest one". Both parts are false for the top reference. The top is retained while unreferenced, and settling its last record only clears `top.referenced` (`settle_record`, lines 265-268). Commit `f6491efe0`'s message makes the same claim.
3. **`heights.rs:86-90` (`INLINE_CHUNK_BITS`), minor.** The doc says "digits that take twice the value's bits". The code tests working width (`stored_bits`), which can far exceed the value's bits after cancellation. It should say twice the working width.
4. **Commit message `ab0e0f839` (the cherry-pick), minor.** It says "The board's heap ceiling is 1024 + 20 bytes per input byte"; at this base `ceilings.rs` says 18. It also gives "about 5 encoded bits per level", while the same message and the family doc say 6 (125 B/B × 6/8 is about 94 bytes per level, not 78).

## Goal-level finding (predates the branch; not in its diff)

**What remains.** `RangeMinima` stores any positive boundary wider than 64 bits as `Boundary::Wide(Accumulator)` in `Vec<Accumulator>`. That is a whole accumulator struct plus a digit buffer per suspended level (`range_minima/boundaries.rs:80-91`, `boundary.rs:18-23`). So the 64-bit word is a fixed-width field whose crossing changes the per-element cost: lesson 5.1's pattern.

**Is it sampled?** No board family holds such boundaries at every level. The builder says the same ("No family samples it").

**Reading.** The builder reports `stepped b=65` at 22.0 B/B on the tip and 27.1 on base, against the ceiling of 18. Their remaining-cost estimate is about 370 bytes per level against about 17 input bytes per level. That figure is consistent with the struct-per-boundary layout. **I did not measure it:** my `STEP s=65` probe never ran.

**Who else it affects.** `RangeMinima` is shared by `version/tick.rs` and `version/tick/prescan.rs`, so the same regime likely reaches the tick walk. That is an inference, not measured.

**Status.** This is not a regression: the tip improves on base. Section 6's requirements 2 and 3 are stated "for every valid Version", so the `min_ticks` heap problem stays open past this branch.

## Not completed (cost probes I wrote but did not get to run)

Each would close a hypothesis left open in this review. The probe is saved at `<scratch>/review-mt-correctness/review_probe.rs` (tip) and `review_probe_base.rs` (base variant without the tip's counters). The driver script is `run-tip.sh`. It needs reinstalling as `crates/before/src/version/measure/min_ticks/minima/review_probe.rs` plus `#[cfg(test)] mod review_probe;` in `minima.rs`.

- **Wide receiver for the running total (H1).** `answer` now starts as `Accumulator::from(first_height)` (`min_ticks.rs:48`). Every per-leaf add and every settlement product therefore lands in a receiver whose working width is about `bits(h0)`. `suanpan` documents `O(L·log(W+1))` for such adds; base kept `h0` out of the total until the end. `review_probe_cost_hugeleaf_comb` compares touches per leaf across `N` = 64 to 65,536 bits. If the probe turns up growth, run the same probe on base to attribute it. My expectation, unverified, is that the log factor lives in the zero-range map and does not fire for shift-0 adds.
- **Lesson 5.2 white-box check.** `review_probe_cost_entry_jump` builds identical trees whose entry jump alone widens from 300 to 19,200 bits. Touches growing with jump width times cycle count would mean a surviving wide value is being re-read. By reading, every read of a stored wide value either destroys it or is bounded: a merge reads only the narrower side and drops it; records are read once; chunks under 256 bits are copied at constant cost; reads of the live change are bounded by the 256-bit stored-width trigger.
- **Heap across the 256-bit thresholds.** `review_probe_cost_spines` and `review_probe_cost_small_pairs` cover JR at b from 31 to 1,000, WP at e from 200 to 512 (including 254–257, where whole chunks and separate record offsets are densest), WP at small e (the most freezes per input bit), and `STEP s` from 31 to 320. My estimate for WP near e = 257 is about 7 B/B, under 18.
- **Larger differential sweeps** (seeds 2–5, 3,000 cases each) did not run.
- **Allocation claim.** The builder reports 193 allocations for JR at depth 512,000 and 319 on the 91 MB input. That is plausible: the locked `num-bigint` 0.4.8 stores single-digit values inline (`BigDigits::from_digit`). I did not measure it.

## Non-blocking observations

- **`STOPPING_DIFF_BAND` re-pin** (`tests/meter/version_scaling/minimum_boundaries.rs:246`). The new reading of 6,369 touches over k = 2,048 hops is about 3.1 per hop. That fits the doc's description (one domination read plus a three-digit terminal fold) better than base's 11,964. A per-hop read of the surviving boundary (about 3 digits × 2,048) would still exceed the new ceiling of 7,962, so the band still detects its stated failure (arithmetic, not run).
- **Intermediate commits 2–7** read below the `seam_stop` floor, as commit 2's message says, and commit 1 is red on the board by design. This is harmless under the first-parent bisect convention.
- **Simplification candidate (agrees with the builder).** The 256-bit stored-width freeze trigger (`min_ticks.rs:81`) looks redundant. The read in `add_leaf` compacts `live` to its value's width, and the record trigger then bounds that width. I have not demonstrated this.
- **The four lowered `MIN_TICKS_*` ceilings** match ×1.25 of the readings in commit `8c2282903`'s message. I did not re-measure them, but the landing check passes against them.

## Files

- `<scratch>` here is `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad`.
- Logs: `<scratch>/review-mt-correctness/landing.log`, `landing-summary.txt`, `diff1.log`, `mutant-*.log`
- Resumption record: `<scratch>/review-mt-correctness/NOTES.md`
