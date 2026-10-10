<!-- CAVEAT LECTOR: the test-coverage review of 8c2282903, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict: changes required (test coverage)

I tested the branch at `8c2282903` with 15 hand-written mutants behind one `MT_MUTANT` switch, all in one build. Each mutant ran against the full `before` + `suanpan` suite (828 tests), the board's `acceptance` run, and `worst-cases-check`, plus release-profile probes.

- **Killed:** 4 mutants (2, 5, 9, 10), plus mutant 13, which only the worst-case pin catches, by accident (finding 5).
- **Survived everything:** 10 mutants.
  - One survivor (mutant 11) is a **value** defect: `min_ticks` returns the wrong number on a shape I constructed, and no committed generator ever reaches that shape.
  - The other nine are cost defects that the board and meters cannot see.

The value-coverage gap and the missing big-integer time meter both block. The owner's section 6 named that meter as an instrument that "must exist first", and the branch rebuilds `min_ticks` without it.

I committed nothing. The slot is restored: `git status` and `git diff` are both empty at `8c2282903`, the probe file is deleted, and the box copy is resynced with no mutant left in it. Full board logs stay on the box under `~/src/rumors-slot-52/target/mt-board/`.

## Blocking findings, most severe first

### 1. A sign error in the coefficient stack ships with every test passing (mutant 11)

**Where:**
- `crates/before/src/version/measure/min_ticks/minima/heights.rs:426` (`push_i128`) and `:433` (`pop_i128`)
- the generator in `crates/before/src/version/measure/min_ticks/minima/tests.rs:16-33`

**Mutant:** `pop_i128` returns `(code >> 1) as i128`, which drops the zigzag sign, so −n decodes as n−1.

**Suite result:** all 828 tests pass (rerun as `MT_MUTANT=b`, to rule out an artifact of my switch described in the evidence section): `828 tests run: 828 passed, 3 skipped`.

**Reach of the committed generator:** I instrumented both functions to count negative values.
- `PROBE committed-generator reach: of 512 cases, 0 pushed and 0 popped a negative stacked coefficient`
- A larger search, 4,096 cases of up to 400 nodes, also passed under the mutant.

**The mutant is not equivalent.** I constructed a shape where it is wrong:
- An outer record and an inner record share a reference.
- The inner record settles with `c` closes, more than its reference has leaves.
- A new record then makes the newer top referenced, and a freeze buries it, which pushes the negative coefficient.
- The shared reference has to sit above reference 0, because reference 0's chunk is always zero.

Release results:
```
=== NEGR 0   PROBE negshape pre=true c=3 pushed=1 popped=1 agrees=true   (also c=5, c=10)
=== NEGR b   PROBE negshape pre=true c=3 pushed=1 popped=1 agrees=false  (also c=5, c=10)
```
In a debug build the same mutant trips the `settle` assertion at `heights.rs:375` ("a nonempty binary tree has one more leaf than internal nodes") on the `pre=false` variant.

**Closing test**, in `minima/tests.rs`:
- A deterministic property over `c` and the lift: "`min_ticks` equals the tree oracle when a reference above the bottom one stacks a negative coefficient and later pops it."
- Add "a negative coefficient was stacked and popped" to `min_ticks_matches_the_tree_oracle_across_reference_paths`'s path-liveness floors, with the generator extended to reach it. Its current floors (`MAX_BURIED >= 2`) prove that stacking happens, but not with a negative value.

### 2. The regression `a0068b6f0` fixes is guarded by nothing (mutant 3, and mutant 4 alongside it)

**Where:** `heights.rs:91` and `:462`.

**Mutants:**
- Mutant 3: `INLINE_CHUNK_BITS` → 64, the round-1 behaviour.
- Mutant 4: the bound set to 0, so every buried chunk moves whole.

**Results:**
- Suite: 828 passed under each.
- Board: `5545 green / 0 red` at every scale under each.
- Worst-case pin: clean under each.

**Why nothing fires:** the commit's own shape, a 2^65 step alternating with a one-tick step, is `WP(65, d)`. The board samples only `WP(320, d)`. Release probe readings, against the board's ceiling of `1024 + 18n`:

```
m=0 WP(65,4096)  bytes=35841 peak=468000 per_byte=13.06 ceiling18=within
m=3 WP(65,4096)  bytes=35841 peak=744496 per_byte=20.77 ceiling18=OVER  wide_chunks=2047
m=4 WP(65,4096)  bytes=35841 peak=744432 per_byte=20.77 ceiling18=OVER
```

**Closing test:** sample `wide-step-pairs` on the board at `e = 65` as well as at 320. The 320 sample crosses the threshold; the 65 sample is the copied regime's worst case. Equivalently, a heap check in `tests/meter` on `WP(65, d)` at two depths would do. Either one fails both mutants at about 20.8 B/B.

### 3. Lesson 5.2 work is invisible (mutants 6, 12, 8; section 6 instrument 1 is absent)

The board counts only `suanpan` digit touches and encoded bits scanned. Work in `num-bigint`, `IntegerStack`, and the `Vec`s escapes every check. The branch states a time contract in prose that no check protects:
- "A record is written once and read once … no suspension or resumption moves it" (`minima.rs:38`)
- "a pop returns the entry without copying" (`integers.rs:10`)

**Mutant 6, design D's re-read.** `IntegerStack::push` reads the wide entry below it (`separate.last()`) into a `BigUint`. On design D's cross-prefix comb (a `2^(64k)` floor under `k` narrow sites), the limbs read grow quadratically while every board currency stays flat:
```
m=0 comb(k=250)  touches=5289  peak=18656  allocs=15
m=6 comb(k=250)  touches=5289  peak=18656  allocs=765   mt_limbs=62750
m=6 comb(k=2000) touches=42039 peak=144880 allocs=6015  mt_limbs=4002000
```
Suite: 828 passed. Board: `5545 green / 0 red`. Pin: clean.

**Mutant 12, a hidden memmove.** `IntegerStack.separate` uses `insert(0)`/`remove(0)`, which still behaves as a stack, but each operation shifts every element. Elements shifted: 4,096 at `WP(320, 128)` and 65,536 at `WP(320, 512)`, quadratic, with every reading identical. Suite: 828 passed. Board: green. Pin: clean.

**Mutant 8, one allocation per settlement.** Each settlement builds a `BigUint` product, as round 1 did. Allocations on `JR(40, d)`:
- baseline: 17 at every depth from 1,000 to 128,000
- mutant: 1,951 → 248,015

Peak heap and touches are identical. Suite: 828 passed. Board: green. Pin: clean.

**Cheapest committed checks:**
- (a) An allocation-count test in `tests/meter`: "`min_ticks`' allocation count on `JR(40, d)` and on design D's comb does not grow with `d`." `alloc_meter::Stats::allocations` already exists, and the counts are constant today (17 and 15). This kills 8, and 6 as a side effect of its allocation.
- (b) A non-allocating re-read like 12 needs the owner's instrument 1: a fuel ladder with the comb, JR, and WP shapes, or a `cfg(test)` limb-and-element counter in `IntegerStack` and `Records` with a linearity assertion on the comb and WP.

Per section 5.3, the meter should come with a committed demonstration that a known-bad mechanism fails it; mutant 6 or 12 can serve.

### 4. The `seam_stop` band's doc and failure message misstate what it measures

**Where:** `crates/before/tests/meter/version_scaling/minimum_boundaries.rs:234-245` and `:311-316`.

The doc says the stop and the control "part only at the stack", so "the difference is therefore the boundary-dominates hop itself". The re-pin commit `594cb7359` attributes the move to leaf-offset work that differs between the two runs: 12,981 against 7,169 touches at base.

Mutant 2 (record trigger off) changes no range-tracker code, yet fails this band with a message naming the range tracker:
```
seam_stop: the stop-minus-control difference 11964 exceeds its band ceiling 7962 — a per-hop read of the surviving boundary's width is back in the stopping arm
```

The band is useful, since it is one of only two checks that kill mutant 2. The doc and message should say that the difference also contains `min_ticks`' leaf-offset reads, freezes, and merges.

## Non-blocking findings

### 5. The WP test pins a literal, not the board's constant (mutant 13)

**Where:** `minima/tests.rs:90` uses `build2(320, d)`; the board uses `WIDE_STEP_PAIRS_STEP_BITS` (`family.rs:264`).

**Mutant:** the board's step set to 200, below the 256-bit bound.
- Suite: 828 passed.
- Acceptance: green.
- Only the worst-case pin fails, by accident, on two unrelated rows: `rank_checked_sub x heap … live worst wide-step-pairs` and `count_clone x heap …`.

The assertion `buried + 2 >= d / 2` is also one weaker than its doc ("at every second level past the first pair"): the measured counts are exactly `d/2 − 1` (63 at d=128, 255 at d=512).

**Fix:** reference the board's constant, and assert it exceeds both `INLINE_CHUNK_BITS` and `PACKED_BITS`.

### 6. Constant-factor wins are not locked in (mutants 1, 4, 7)

Each passes the suite, the board, and the pin.

| Mutant | What it changes | Measured effect |
|---|---|---|
| 1 | `RECORD_SLACK_BITS` → 32 | JR heap 4.7 → 9.0 B/B |
| 4 | every buried chunk moves whole | JR heap 12.3 B/B; 17.9 on the small input, against 18 |
| 7 | first height kept in the bottom reference, the pre-`eb02a764e` behaviour | `hugeleaf` heap 2.5 → 3.1 and touch 0.3 → 0.6 per byte; `bigroot` heap 1.9 → 2.3 |

The doc on `RECORD_SLACK_BITS` (`heights.rs:67-81`) cites sweep figures that no test checks.

**Closing tests**, both in `tests/meter`:
- "`min_ticks`' peak heap on `JR(40, d)` stays within measured ×1.25 at two depths" (about 5.9 B/B). It kills 1, 2, and 4.
- A touch ceiling on `hugeleaf`. It kills 7.

### 7. Nothing shows what the 256-bit stored-width trigger does (mutant 15)

**Where:** `min_ticks.rs:81`. Disabling it passes the suite, the board, and the pin; the only visible effect is `wide-step-pairs` touch moving from 1.1 to 1.3 per byte.

This needs your ruling: dissolve it (the builder suspects it is redundant), or add a test naming what it protects.

### 8. The narrower-side merge choice has one guard (mutant 5)

Mutant 5 makes the merge read the wider chunk. Only `latent-ladder` catches it: the board reports touch exponent 1.74, and the `skyline_min_ticks_latent_ladder` meter fails.

The `freeze-position` and `promotion-rearm` meter docs (`freeze_schedules.rs:395-401`, `:443-447`) describe the narrower read, but both tests pass under the mutant.

### 9. A sibling test now watches only one of two triggers

`zero_drift_never_freezes` (`measure/tests.rs:375`, pre-existing) watches only `FREEZE_HITS`, while `min_ticks` now has two freeze triggers. Adding `RECORD_FREEZES` would make "a drift that cancels exactly never trips the freeze trigger" cover both.

### 10. 32-bit targets never run the new stacks

`wasm32-pins` has no `min_ticks` check, so the new integer stack and record stack never run on a 32-bit target. This is a residual gap.

## Section 6 instruments

1. **Big-integer time meter:** absent. Finding 3 shows it.
2. **`jump-rising-spine` on the board:** present. It kills mutant 2: `RED version_min_ticks jump-rising-spine … heap 24.5/B`.
3. **A sampled input past every threshold:** partial.
   - `WP(320)` crosses `INLINE_CHUNK_BITS` (`wide_chunks=63/255`) and `PACKED_BITS` (wide records are present).
   - The copied regime's worst case is not sampled (finding 2).
   - The stored-width trigger's crossing is unverified (finding 7).
4. **Exact capture and compare:** present, but it predates the branch (`e17997ce1`).
5. **Real-scale probe:** exists only on `scratch/*` branches; nothing is committed.

## Killed mutants

| Mutant | What it changes | Killed by |
|---|---|---|
| 2 | record trigger off | the oracle test's liveness check ("the generator never reached record freezes") and the `seam_stop` band |
| 5 | merge reads the wider chunk | `latent-ladder` only (finding 8) |
| 9 | settlement drops its final carry | 9 tests |
| 10 | wrong coefficient sign on merge-to-newer | 14 tests |

The `seam_stop` band and the four lowered meter ceilings keep positive liveness floors.

## Corroborated builder claims

My readings agree with the builder's report:
- JR 4.69 B/B
- WP(320) 6.2 B/B
- `JR(40, 128000)`: 17 allocations plus 142 reallocations, consistent with "193 at depth 512,000"

## Evidence, verbatim

- **Baseline suite:** `Summary [ 34.983s] 828 tests run: 828 passed, 3 skipped`.
- **Board baseline:** `amp-board: 5545 green / 0 red (5545 cells)` ×3; `worst-case pin: clean (272 pinned rows verified at 2 sampling scales)`.
- **A switch artifact, not kills:** ids 10 and up first appeared to fail `amp_board_smoke::shard_protocol_round_trips`. Reading `MT_MUTANT` allocates a string inside the first measured cell, and that test requires byte-identical heap readings between a whole and a sharded board. Every two-digit id failed it and no single-digit id did. Rerun under single-letter ids, all five passed it.
- **No landing check run**, as briefed.

## Files

- Scratch logs and notes: `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-mt-coverage/` (`NOTES.md`, `suite.log`, `suite2.log`, `probe.log`, `negr.log`, `board6.log`, `wcc.log`, `wcc2.log`)
- Test and code files cited:
  - `/Users/oxide/src/rumors-slot-52/crates/before/src/version/measure/min_ticks/minima/heights.rs`
  - `/Users/oxide/src/rumors-slot-52/crates/before/src/version/measure/min_ticks/minima/tests.rs`
  - `/Users/oxide/src/rumors-slot-52/crates/before/tests/meter/version_scaling/minimum_boundaries.rs`
  - `/Users/oxide/src/rumors-slot-52/crates/before/src/testing/meter/board/family.rs`
