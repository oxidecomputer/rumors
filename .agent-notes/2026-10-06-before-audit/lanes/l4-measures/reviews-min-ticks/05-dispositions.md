<!-- CAVEAT LECTOR: the fix builder's report at ac7b99900, with a disposition per finding, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Fix round report: `fix/min-ticks-transient-heap`

Every review finding is now fixed, settled, or held for you. The final tip is **`ac7b99900469af127e81b6d9291bae67c5fe7019`**: 18 signed commits on `e0956b25d`, in worktree `/Users/oxide/src/rumors-slot-53`, with a clean tree. The landing check at that tip is clean (verbatim at the end). Nothing was pushed, and the owner's checkout was not touched.

I left two local tags in the slot, `pre-autosquash-r3` and `pre-autosquash-r3b`. Each autosquash produced a tree identical to its tag; I checked both with `git diff`.

### Commits (oldest first)
```
4909c0bfe Add the jump-entered rising spine to the before amplification board
8d7b9e52c Keep min_ticks minima as records over merged references
e3626646b Count min_ticks' first height once and merge in one read
0e875e681 Store min_ticks record offsets at their exact width
01663b934 Copy buried min_ticks chunks up to 256 bits, not 64
0ff2b098f Sample min_ticks' whole-chunk references on the board      (coverage 5 folded in)
882b1e5ef State the measured trade behind min_ticks' record slack
75fd8ec47 Re-pin the seam_stop band to min_ticks' narrower offsets   (coverage 4 folded in; message amended)
5d144e517 Re-pin the worst-case map for the two new min_ticks families
b7d51a1e9 Lower min_ticks' meter ceilings to its measured readings
aef15248d Report distinct range minima as begin and end events       (structure 1, p3)
b8e216d6b Freeze min_ticks' live change unread past its width bound  (structure 2 / coverage 7, p1b)
7b6510b04 Tidy min_ticks' settlement and stack bookkeeping           (structure 3, p2)
e3660b9a7 Pass min_ticks' leaf offsets as sign and limbs             (structure 5)
437b548c3 Gather min_ticks' cost argument into one ledger            (structure 4)
3d33488a8 Test min_ticks with a negative coefficient stacked and popped (coverage 1)
1e7966b92 Pin min_ticks' heap, allocation, and touch readings        (coverage 2, 3a, 6)
ac7b99900 Name the band that detects min_ticks' merge side           (coverage 8)
```
All commits are prefixed `before:` except `4909c0bfe`. All 18 are signed (`G`).

### Disposition per finding

**Correctness**
1. The false premise in the `min_ticks.rs` comment: **fixed in `e3626646b`**. The comment now says every height is h0 plus a signed offset, and h0 counts once because a binary tree has one more leaf than internal nodes.
2. The top-reference retention claim in `heights.rs` (the module doc and the commit message): **fixed in `8d7b9e52c`**. The top stays while unreferenced, and settling its last record only marks it unreferenced.
3. The `INLINE_CHUNK_BITS` doc: **fixed in `01663b934`**. It now says "twice its working width" and states the relation to `PACKED_BITS`, which is structure item 6.
4. The cherry-picked message (old `ab0e0f839`, now `4909c0bfe`): **fixed**. It names `MAX_HEAP_BYTES_PER_INPUT_BYTE` instead of quoting 20, and it uses 6 bits per level, giving about 94 bytes per level.

**Coverage**
1. Mutant 11 (negative stacked coefficient): **fixed in `3d33488a8`**.
   - `a_negative_stacked_coefficient_settles_exactly` is a deterministic oracle check over 3–23 closes and lifts of 5–399.
   - The generator mixes in `negative_coefficient_tree`.
   - The oracle property's liveness floors now include "a stacked negative coefficient" and "a popped negative coefficient".
   - Mutant 11 kills both tests.
2. WP(65) regime: **fixed in `1e7966b92`**, as a heap meter in `tests/meter` rather than a board sample. It checks WP(65, d) at d = 2,048 and 4,096.
   - Readings: 234,064 and 467,984 bytes (13.06 B/B). Ceilings: 292,580 and 584,980.
   - Mutant 3 (copy bound 64) reads 372,320 at the small depth and fails.
   - Mutant 4 (always whole) reads 372,256 and fails.
3. (a) Allocation counts: **fixed in `1e7966b92`**. Counts on JR(40, d) and JR(320, d) do not grow from d = 8,000 to 16,000 (17 and 30 at both depths).
   - Deviation: I used JR(320, d) instead of design D's cross-prefix comb. The comb is not a registry shape, and adding one means adding a board family. JR(320) has the property the comb was built for: the outermost record's 321-bit offset is stored on its own beneath every narrow push.
   - Mutant 6 reads 40,020 and 80,020 allocations and fails.
   - Mutant 8 reads 7,767 and 15,517 and fails.
   - The first version of mutant 8 that I wrote allocated nothing at runtime on JR's one-limb products: the count stayed at 17. It was therefore not a real cost, so I rewrote the mutant with `BigUint::new(digits) * count`, which does allocate.
   - (b) **held**; see below.
4. The seam_stop band doc and failure message: **fixed, folded into `75fd8ec47`**. Both now name the stop's leaf-offset reads, freezes, and merges beside the boundary-dominates hop. The amended message says the band fails with the record trigger off even though no range-tracker code changed.
5. The WP test: **fixed, folded into `0ff2b098f`**.
   - It reads the board's `WIDE_STEP_PAIRS_STEP_BITS`, which is now `pub(crate)` and re-exported `#[cfg(test)]` from `board`.
   - It asserts the step exceeds `INLINE_CHUNK_BITS` and `PACKED_BITS` as two separate asserts. Clippy's `redundant_comparisons` rejects the `&&` form because both constants are 256; that is what failed the first landing check.
   - The bound is now `buried >= d/2 − 1`.
   - Mutant 13 (step 200) fails with "the board's 200-bit step no longer buries chunks whole".
6. Constant-factor ceilings: **fixed in `1e7966b92`**.
   - JR(40, d) peak heap at d = 8,000 and 16,000 reads 23,328 and 46,880; ceilings 29,160 and 58,600. It kills mutant 1 (45,056), mutant 2 (122,880) and mutant 4 (61,728).
   - The hugeleaf touch meter at b = 32,000 and 64,000 reads 2,503 and 5,003; ceilings 3,129 and 6,254. It kills mutant 7, which reads 5,003 at the small width.
7. See structure 2.
8. Mutant 5 (the merge reads the wider side): **docs corrected in `ac7b99900`**.
   - Measured movement: freeze-position 69,001 → 69,025 touches, promotion-rearm 398,557 → 411,619. Both stay inside their bands, so the bands cannot be made to catch it without a different shape.
   - Only the latent ladder fails: its marginal goes from 3,584 / 3,584 to 8,183 / 12,791.
   - The two freeze docs now state what they detect and point to the ladder. The ladder's doc and failure message now name a reference merge beside the undercut decision. That failure message misattributed the cause in the same way coverage finding 4 did.
9. `zero_drift_never_freezes`: **settled by structure 2**. Both triggers go through `freeze()`, which bumps `FREEZE_HITS`.

**Structure**
1. p3: payload dissolved in `aef15248d`.
2. p1b: the fast path inside `add_leaf` landed in `b8e216d6b`, with `a_live_change_far_wider_than_its_delta_freezes_unread` as the check. Disabling the fast path fails only that test.
3. p2: `7b6510b04`.
4. The cost ledger: `437b548c3`.
5. The per-leaf `BigInt` round trip: dropped in `e3660b9a7`. The capture shows 18 heap decreases and no rises.
6/7. Done in earlier commits:
   - The relation between the two thresholds is in the `INLINE_CHUNK_BITS` doc.
   - `HEIGHT_FREEZE_ALLOWANCE_BITS` moved to `measure/integral.rs`.
   - The second tap style is gone. Counting taps use `bump`; the two maximum taps (`MAX_READ_EXCESS`, `MAX_BURIED`) keep `.with(max)`, because they record a maximum rather than a count. I left that difference deliberately.

### Held for you (not built)
- **Coverage 3(b)**, the only check that catches mutant 12 (the hidden memmove).
  - A counter inside `IntegerStack` cannot see `Vec::insert(0)`'s internal shifting, so it would catch mutant 6 but not 12.
  - What would work is a fuel ladder: export `min_ticks` from the existing fuzzfit/wasmtime guest, run it at doubling sizes of JR(320, d), WP(320, d) and a comb, and assert that fuel per input byte stays flat. Fuel is deterministic, so this needs no noise threshold.
  - It would need a guest entry for registry shapes, a harness test, a calibrated flatness factor, and a committed demonstration that mutant 12 fails it.
- **Coverage 10**, `wasm32-pins`.
  - It needs a guest export running `min_ticks` on shapes that reach both stacks: JR(320, d), WP(65, d), WP(320, d), and `negative_coefficient_tree`.
  - Each value would be compared against its closed form or the native result, so the 32-bit `usize` paths in `IntegerStack` and `PackedU64Stack` run once.
  - It would need a guest function, fixtures, and one harness test.

### Correctness probes (the reviewer's `review_probe.rs` and `run-tip.sh`, run once at the tip, then removed)
- **Differential sweeps:** seeds 2–5 at 3,000 cases each all passed, about 1,500 of them per seed also checked against the tree oracle. The counters were all live (for example, seed 2: `record_freezes=427136 unread_freezes=294816 merges_newer=363233 merges_older=358719 wide_chunks=317969 max_buried=977`).
- **Cost probes:** none of the 238 cells exceeded the `1024 + 18n` ceiling, and **no reading grows**.
  - **H1, the answer starting wide (hugeleaf comb):** touches per tooth stay about 15 for every N from 64 to 65,536. For example, at k = 1,024, N = 64 gives 15,369 touches and N = 65,536 gives 20,487. The difference is about 5 touches per digit of N, paid once, so per-leaf adds into the wide answer do not carry across its width. H1 is refuted.
  - **Entry-jump widening:** at fixed leaf count, raising b from 300 to 19,200 adds the same touches at 2,000 and at 8,000 leaves (+6,202 on shape 0), so the cost is additive and has no width × cycles term. Heap also grows additively, by about 26.5 KB, the same at both leaf counts; that is about 11 bytes per byte of the jump. The worst density was 5.12 B/B.
  - **Across the 256-bit thresholds:**
    - JR: 4.66–4.87 B/B for every b from 31 to 1,000 at d = 8,000.
    - WP: 4.18 B/B at e = 255, then 7.20 at e = 256, where whole chunks take over; 6.27 at e = 320.
    - STEP s = 64 to 66 is the densest regime at 13.0–13.4 B/B, and it is constant from d = 1,000 to 8,000.
    - Small-step pairs WPS e = 64 read 13.56 B/B at d = 4,000, 16,000 and 64,000.
  - In the WP and STEP regimes, allocations grow linearly, about 2 per wide level, which the input bounds. The committed allocation meter covers only JR.

### Board movements
- The final capture was taken at `0a26fcfcd`. It differs from the tip only in `minima/tests.rs`; I checked with `git diff`.
- Against the previous capture (`c4`): `16635 rows: 16635 unchanged`. The new commits move nothing on the board.
- Against the `8c2282903` reference: `13 growing, 4 uneven, 18 constant, 16600 unchanged`. Every moved row is `version_min_ticks`, and **none rises**: no positive delta anywhere.
  - **Touch decreases**, attributed to the unread fast path: freeze-pos −8,191 @604,241; promo-rearm −8,192; dense-suffix −8,193; freeze-parade −4,094; arming-train −1,025; wide-step-pairs −512.
  - **Heap decreases**, attributed to p2 plus `LeafHeight`: plateau-puncture −20,480; wide-tooth-comb −19,200; ascend-cliff and ascend-plateau −1,008; memo-oscillating −1,024; cancelling-chain −512; mask-drift −440; and smaller constant shifts.
- Neither the worst-case map nor any ceiling needed re-pinning; the board leg of the landing check is clean.

### Mutant results (string swaps, each checked restored by `git diff`)
| Mutant | Killed by |
|---|---|
| 1 slack 32 | JR heap meter |
| 2 record trigger off | JR heap meter (plus the earlier oracle liveness check and seam_stop) |
| 3 copy bound 64 | WP(65) heap meter |
| 4 always whole | JR and WP(65) heap meters |
| 5 merge reads wider | latent ladder only (docs corrected) |
| 6 push re-reads the entry below | allocation meter, JR(320) |
| 7 first height in the bottom reference | hugeleaf touch meter |
| 8 BigUint product per settlement | allocation meter, JR(40) |
| 11 zigzag sign | both negative-coefficient tests |
| 13 board step 200 | WP test step assert |
| 12 memmove | survives everything; held as 3(b) |

### Other notes
- Two unrelated tests leaked under load earlier in this round (`family_pairs_agree` and `every_family_reaches_board_operations`). Both passed.
- The first landing check, at `0a26fcfcd`, failed only on lints: clippy's `redundant_comparisons` on the combined `&&` assert. I folded the split into `0ff2b098f` and re-ran the check.

### Landing check at `ac7b99900` (verbatim)
```
ac7b99900469af127e81b6d9291bae67c5fe7019
audit-check: ok      docs (48s)
audit-check: ok      surface (49s)
audit-check: ok      lints (53s)
audit-check: ok      tests (171s)
audit-check: ok      board (205s)
audit-check: ok      wasm (273s)
audit-check: clean in 274s; logs in target/audit-check-logs
```

### Files
- New meter: `/Users/oxide/src/rumors-slot-53/crates/before/tests/meter/version_scaling/minimum_resources.rs`
- Changed docs: `/Users/oxide/src/rumors-slot-53/crates/before/tests/meter/version_scaling/minimum_boundaries.rs` and `/Users/oxide/src/rumors-slot-53/crates/before/tests/meter/version_scaling/freeze_schedules.rs`
- WP test: `/Users/oxide/src/rumors-slot-53/crates/before/src/version/measure/min_ticks/minima/tests.rs`
- Scratch, all under `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/proto-min-ticks-parked/`:
  - mutant logs: `res-mutants/`
  - probe logs: `probes/`
  - board comparisons: `c5-compare.txt`, `c5-vs-c4.txt`
  - landing checks: `landing3.log` (the failed one) and `landing4.log`
  - notes: `NOTES.md`

The session now pauses for the owner.
