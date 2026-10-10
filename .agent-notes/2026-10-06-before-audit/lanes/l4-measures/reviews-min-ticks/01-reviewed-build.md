<!-- CAVEAT LECTOR: the build report for the reviewed tip 8c2282903, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict

**`fix/min-ticks-transient-heap` is rebuilt as clean commits, rebased onto #122's tip `e0956b25d`, and passes the landing check on every leg.**

| Measure | Base (#122 + family) | Fix tip |
|---|---|---|
| `jump-rising-spine` heap (board) | 125.0 B/B | 4.69 B/B |
| Worst `min_ticks` board heap | 125.0 (JR) | 12.7 (`propagate-seam`, itself down from 15.6) |
| 91 MB stepped spine, peak heap | 5,247,074,352 (57.52 B/B) | 329,383,936 (3.61 B/B) |
| 91 MB stepped spine, allocations | 178 | 319 (round 1: 81.3 million) |

- Every `min_ticks` value matches on all inputs.
- Every remaining rise is a linear density shift, and each is attributed below.
- Item 6 was dropped per your correction, and the cherry-pick is unchanged.

## Commits

Ten signed commits on `e0956b25d`, branch `fix/min-ticks-transient-heap`, worktree `/Users/oxide/src/rumors-slot-53`:

1. `ab0e0f839` Add the jump-entered rising spine to the board. This is the cherry-pick of `08573e15`. Its WIP note is replaced by an accurate note that the worst-case pin is re-pinned at the end of the branch.
2. `f6491efe0` `before: Keep min_ticks minima as records over merged references` (the redesign, as reviewed in round 1).
3. `eb02a764e` `before: Count min_ticks' first height once and merge in one read`
4. `bb3115f83` `before: Store min_ticks record offsets at their exact width`
5. `a0068b6f0` `before: Copy buried min_ticks chunks up to 256 bits, not 64`
6. `7893aa6ca` `before: Sample min_ticks' whole-chunk references on the board` (adds the `wide-step-pairs` family).
7. `4c63b6972` `before: State the measured trade behind min_ticks' record slack`
8. `594cb7359` `before: Re-pin the seam_stop band to min_ticks' narrower offsets`
9. `e19bc519e` `before: Re-pin the worst-case map for the two new min_ticks families`
10. `8c2282903` `before: Lower min_ticks' meter ceilings to its measured readings`

Every commit compiles on #122 with no warnings. Commits 2 through 5 each passed clippy (`-D warnings`) and the lib tests before the rebase. From commit 2 until commit 8, the `seam_stop` band reads below its floor, and commit 2's message says so.

The probe and phase tools never land. They live on throwaway branches you can delete: `scratch/min-ticks-parked-probes` (the old exploration branch, renamed), `scratch/pmtp-base-phases`, `scratch/pmtp-proto-phases`, `scratch/pmtp-base2`, and `scratch/pmtp-tip2`.

## Items

**Item 1: the wide-chunk regime.**
- Buried reference chunks are now copied at their exact width up to `INLINE_CHUNK_BITS` (256 bits). Only wider chunks move whole as an accumulator.
- `reference-dense e=65` falls from 20.7 to **13.4 B/B** (base 19.2, ceiling 18).
- The new `wide-step-pairs` family, WP(320, d), crosses this threshold at every second level. It also crosses the 256-bit bound for separately stored record offsets.
- All 276 of its board cells are green; `min_ticks` reads 6.2 B/B on it.
- A test pins that `min_ticks` takes the whole-accumulator form at least once per pair and agrees with the oracle.

**Item 2: every rise attributed.** The tip-versus-base comparison on #122 has 39 growing, 20 uneven and 43 constant rows, all in `version_min_ticks`, with no scan row moving. Every growing difference doubles when the input doubles, so none is superlinear. All 20 uneven rows are decreases. The constant rows are fixed shifts: +1 touch on many families (the final settlement reads the top chunk once), +30 touches on `comb-scatter`, +120 bytes of heap on `hoisted-window`. I attributed the rises with phase-split touch counters and allocation-peak hooks on base and prototype.

*Removed:*
- **`hugeleaf` and `bigroot`, touch and heap.** The cause was extra passes over the wide first height, which also stayed alive beside the answer. The first height now enters the answer once, moved in by value (commit 3).
- **`freeze-pos` touch.** The cause was a second read of the narrower chunk in each merge, now fused into one read (commit 3).
- **Heap on `tooth-tail`, `latent-ladder`, `memo-fanout` and `mask-drift`.** The cause was wide record offsets sharing one doubling bit stack, whose reallocation briefly holds old and new storage. Wide offsets now get their own exact allocation, and products stream into the total without a buffer (commit 4). All four are now below base.
- **`cross-prefix` and `wide-tooth-cycle` heap.** The cause was a retained wide decode buffer, removed by the same change.

*Inherent, remaining:*
- **Heap +0.27 / +0.28 / +0.25 / +0.14 B/B on `promo-rearm` / `dense-suffix` / `arming-train` / `wide-arming`.** Wide frozen heights are kept as `suanpan` accumulators, which store 32-bit digits in 64-bit words, where base kept exact `BigInt` components. The peak-growth hooks put the growth in the leaf-advance and freeze steps. Merges need the accumulator's amortized carries: a `BigInt` can ripple a carry across its full width on alternating-sign merges, and converting on every bury would make the wide-tooth cycle quadratic.
- **Touch on the hole families (+0.20/B), `staircase` (+0.10), `ascend-cliff` (+0.11), `latent-ladder` (+0.04), `propagate-seam` (+0.03) and `memo-churn` (+0.02).** This is the record-slack trade: in the sweep, these return to base readings (or within 0.02) at slack 8 or more, while JR's heap rises to 7.4 B/B at slack 8.
- **`freeze-parade` touch +0.10/B.** A merge writes the sum into the wider chunk, so it costs three passes (read, product, sum) against base's two. That extra write is what releasing references costs, and it buys this family's heap drop from 0.99 to 0.12 B/B.

**Item 3: the slack sweep.** I swept 1, 2, 4, 8, 16 and 32 bits on every board family at the top of the ladder, plus the probe ladders, on the final code.
- At 4 bits JR reads 4.69 B/B, against 7.4 at 8, 9.0 at 16, and at 32 a figure that keeps growing with depth (9.0 at depth 128,000, 14.8 at 512,000).
- At 2 and 1 bits the hole families read 15.2 and 16.0 touches/B, against 14.6 at 4 bits and base's 14.4.
- No board family's heap falls more than 0.17 B/B below its slack-4 value.
- The value stays 4, and the constant's doc states this evidence.

**Item 4: allocations.** The per-pop `BigUint` is gone: offsets decode onto the stack or come back in their own allocation, and products stream limb by limb. On the 91 MB input the tip makes 319 allocations. Base makes 178, and round 1 made 81,264,958. JR at depth 512,000 makes 193.

**Item 5: re-pins.**
- **`seam_stop`:** the band becomes (4,776, 7,962) around a measured 6,369, and its ceiling falls from 34,170 to 30,413. The phase counts attribute the move. Leaf read-plus-add work on the stop falls from 12,981 to 7,061 touches, because the record trigger keeps offsets narrow that base left multi-digit. The freezes and merges that achieve it cost 860. The range tracker's share is identical on both sides.
- **Worst-case map:** 60 drift cells are re-pinned, with a movement record. JR takes over the touch cells of the rank, lattice, span, clock and codec operations. The rank-fold rows are a real lead (×1.31, 10.6 against the harmonic spine's 8.1 touches/B); the rest are near ties. Both families join two exact-tie sets. No `version_min_ticks` cell changed its worst family.
- **Neither moved under #122**, so neither needed a second re-pin.

**Meter ceilings you asked me to lower (commit 10).** Four `min_ticks` ceilings now sit at measured × 1.25, rounded up:
- `MIN_TICKS_FREEZE_POSITION_CEILINGS` → [87,500, 175,000], from readings of 70,000 / 140,000 (base 73,004 / 146,004).
- `MIN_TICKS_PROMOTION_REARM_CEILINGS` → [500,697, 1,001,322].
- `MIN_TICKS_PURE_COMB_CEILINGS` → [2,712, 5,408].
- `MIN_TICKS_REVEAL_COMB_CEILINGS` → [12,924, 25,834].

The attribution: base reads a frozen height at the freeze and again in its final product, while the fix reads it once.

**What #122 moved.** On the `min_ticks` rows it moved only heap, and only downward: 12 base rows by −4 to −8 bytes and 13 tip rows by −4 to −16. No touch or scan reading moved, and the real-scale readings are byte-identical to the pre-rebase ones.

## Readings whose verdict changes at the corrected ceiling of 18 B/B

- **Only one:** base's `reference-dense e=65` (19.22 B/B) passed under 20 and breaches at 18.
- **Above 18 under either ceiling:** base's `jump-rising-spine` (125.0) and round 1's `reference-dense e=65` (20.7), plus `stepped b=65`, below.
- Every final-code reading other than `stepped b=65` is at or below 13.4.

## Item 7: `stepped b=65` (out of scope)

It reads 27.1 B/B on base and 22.0 on the fix, so it breaches the ceiling either way. The fix's part shrank, from offsets that were spilled on base and are now packed. What remains is about 370 bytes per level against an input of about 17 bytes per level.

My inference, not directly measured on this shape: the cause is `RangeMinima` holding one whole `Accumulator` per wide boundary in a doubling `Vec`. The fix stores only about 86 bits per record there and freezes once per 32 levels.

No family samples it, as you directed.

## Landing check at `8c2282903`, verbatim

```
audit-check: ok      surface (40s)
audit-check: ok      docs (150s)
audit-check: ok      lints (167s)
audit-check: ok      board (209s)
audit-check: ok      wasm (270s)
audit-check: ok      tests (358s)
audit-check: clean in 358s; logs in target/audit-check-logs
```

- **tests:** `Summary [ 34.744s] 828 tests run: 828 passed, 1 skipped`; the `rumors` snapshot filter: `142 tests run: 142 passed (1 slow)`.
- **docs:** 193 + 15 doctests passed.
- **surface:** `214 public function-like items = 134 board-covered + 80 excepted`.
- **board:** `amp-board: 5545 green / 0 red (5545 cells)` and `worst-case pin: clean (272 pinned rows verified at 2 sampling scales)`.
- **wasm:** 12/12, 25/25, 43/43.

The baseline table was recorded at an older commit, so its counts differ from these by #122's changes and by this branch. Within `before`'s lib suite, this branch adds the records, integers and two `min_ticks` tests plus the WP shape property, and removes the old contributions test.

The first landing check run failed only on `doclint`: six doc summaries in my new docs and tests were over 220 characters. I split them, squashed each fix into the commit that introduced it, and confirmed the tree is byte-identical to the fixed state before re-running.

## Things you should know

- **Rebuilt history and provenance.**
  - The intermediate commits are reconstructed states, each checked as above.
  - Measurements quoted in commits 3 to 5 were taken on development states bracketing each change, and the messages say so.
  - The tip itself was fully measured, before and after the rebase.
- **One nextest LEAK.** Under load, `version::order::tests::family_pairs_agree` (outside `min_ticks`) once ended "leaky": it passed but held handles open past exit. It did not recur.
- **Simplification left to you: the 256-bit stored-width trigger.** It is probably redundant now, since the record trigger is strictly tighter on value width and reads compact stored width. I left it because it sits on #122's lines; removing it is a follow-up for you to rule on.
- **Possible cleanup in `RangeMinima`.** Its payload type parameter is now `()` at every in-crate user. I kept it, because its lifecycle tests guarantee the stack order the records rely on. Dissolving it is a possible cleanup, not done.

