<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), reviewer review-range-minima-wide, of fix/range-minima-wide-boundaries at 3ea10554f. -->

## Verdict: approve, with no blocking findings

I found nothing wrong in `fix/range-minima-wide-boundaries` at `3ea10554f`. I checked the three storage forms with a `BigUint` model and found them exact. The heap is within the ceiling across every width band I measured. The touch-ceiling derivation matches the mechanism. The slot is untouched: its HEAD is `3ea10554f` and `git status` is clean. I did not run the landing check, because one is already running at the tip, so the gate result against `baseline.md` is still pending.

## Blocking findings

None.

## Prose findings (none block)

1. **Ragged manual wraps in rustdoc source.** Rendered rustdoc is unaffected, but these read sloppily in source.
   - `crates/before/src/version/range_minima.rs:20`: "Thus deep nesting does not" sits alone on a line.
   - `range_minima.rs:76`: one line runs to about 125 characters.
   - `crates/before/src/version/range_minima/boundary.rs:71-72`: "consumes it. A wider difference" then "therefore stays…".
2. **`boundaries.rs:20`, "Each form answers one cost."** The verb "answers" carries no proposition. Better: "The live form spares a shrinking boundary from being reread; the copied form… ; the whole form…".
3. **`boundaries.rs:22-23`, "at most the bounded width per push."** The phrase is unclear. Better: "each push copies at most one boundary, of at most `INLINE_BOUNDARY_BITS`."
4. **`boundaries.rs:39-41`, `INLINE_BOUNDARY_BITS`.** "`min_ticks` copies a buried reference's chunk up to the same width" should name its sibling, `INLINE_CHUNK_BITS` (`version/measure/min_ticks/minima/heights.rs:104`). That sibling's doc also states the caveat this one lacks: a copied value can be up to two bits wider than the bound. I measured `stored_bits(2^256) = 256`, so `2^256` is copied as five limbs.
5. **`range_minima.rs:17-19`.** "kept whole only when it is wide enough that…" describes the threshold without naming it. Link `INLINE_BOUNDARY_BITS`.
6. **`crates/before/tests/meter/version_scaling/minimum_boundaries.rs:66-69`, `DESCENDING_TOUCH_FLOOR`.** This is an open question; I did not check it. The fix removed "spilled", and the dying operand is now rebuilt through `Accumulator::from(u128)`, which keeps values below `2^96` inline. "its held digit span is exactly three" describes a digit-form operand, so it may no longer describe this one. The floor itself is safe (3,072 against a reading of 24,133). Only the stated mechanism needs confirming or restating.
7. **Fix commit message.** It says "three touches per copied boundary" in the board bullets and "four touches per copied boundary" for the ceiling. Both are true: the copy costs four, and consuming the rebuilt value costs one fewer, so the net is three. The message should say net versus gross. It also measures "against the base 6a8dab7cc", but the branch's parent is now `feeb760da`. `main`'s intervening changes touch only `Digits::reset` and rank's integrator, and every meter reading I re-ran at the tip matches the message exactly.
8. **"machine word" for the 64-bit boundary threshold** (`testing/meter/registry.rs:930`, `board/family.rs:274`, `testing/meter.rs:572`). On wasm32 a machine word is 32 bits. The repo uses the phrase this way elsewhere, so this is low priority; "64-bit word" would be exact.

## Simplifications

- **Behavior-preserving:** in `pop_copied` (`boundaries.rs:141-147`), the `count <= 2` fast path needs a comment explaining why it exists. A value of at most two limbs is rebuilt through `u128`, which keeps values up to `2^96` inline, with no digit buffer and no touches. That path would also mis-pop for `count == 0`. This is unreachable because boundaries are strictly positive, but `match count { 1 | 2 => …, _ => … }` would make that visible.
- **Behavior-preserving:** the `expect` message at `boundaries.rs:136`, "a copied boundary has a few limbs", should be its one-line proof: "at most five limbs, which fits `u64`".
- **Design proposal, low value:** no board family samples the first whole width.
  - `WS(257)` reads 8.04 B/B at reallocation peaks, against the pinned worst of 6.2 for `version_min_ticks × heap` (`wide-step-pairs`).
  - `wide-step-pairs` already exercises whole range boundaries at 321 bits, so a regression in the whole form would show there too.
  - A sample at `INLINE_BOUNDARY_BITS + 1` would only tighten the constant.

## Answers to your six questions

1. **The three forms preserve values exactly.**
   - **Stack order.** `store_top` writes the live boundary before the top zero run. `pop` reads the top run, then the live slot, then the columns. `push_positive` is the only writer of `live_wide`, and it always stores the old value first.
   - **The live slot.** It can hold only the innermost positive record.
   - **The `whole` tag.** It is pushed and popped only under a wide tag, so it stays in step with the other columns.
   - **Sign.** `push_copied` drops the sign, which is safe because every `Boundary` is strictly positive (`Signed::of` and `abs_diff`'s `Greater` arm).
   - **Covering property.** `nested_operations_preserve_minima_and_report_distinct_minima_in_stack_order` in `range_minima/tests.rs` reaches all three forms, since its shifts run to 259 bits with `i16` coefficients.
   - **My own model.** I wrote a `BigUint` model that drives word, live, copied, and whole forms; wide pushes in three accumulator forms, including a cancellation that inflates `stored_bits`; the close path's "pop a run, push back one less"; and the drop path's "lower, then push the remainder straight back". It passed 4,096 cases, plus a deterministic cover-then-pop sweep over every width from 1 to 400 bits.
2. **The heap claim holds across the three width bands.**
   - **At most 64 bits:** packed words, unchanged.
   - **65 to 256 bits:** one live accumulator in total, and each covered boundary becomes minimal limbs plus a count in the packed column.
   - **Over 256 bits:** a 128-byte slot (up to three times that at a `Vec` reallocation peak) plus digits, against at least about 64 input bytes per boundary.
   - **Measured on the whole band:** `WS(e)` at `e` from 257 to 320 peaks at **8.04 B/B** at `e = 257`, then falls with width (6.77 at 320; the record has 4.8 at 512). That is below 18, and below the 15 the derivation rule now gives.
   - **Not proven:** that the input pays for each boundary's width. I argued it from normal form plus the top compaction done by `Signed::of` and `abs_diff`; I did not prove it.
3. **The touch cost follows from the mechanism.**
   - **One copy per covering push** is true: `store_top` copies only `live_wide`.
   - **Four touches per copy** is also true, and I was wrong to suspect otherwise. My probe read 4 touches on a fresh `5·2^64`, 4 again after `sign()`, and 4 for a real cover. The sign read is not cached.
   - **Net rise.** The `seam_plunge` rise of 3,069 is 3 × 1,023: the innermost boundary stays live, and consuming each rebuilt boundary costs one touch less (the fixer's probe reads `abs_diff` 6 for the rebuilt form against 7 for the digit form).
   - **The surplus band is still sound.** Plunge and control both rise by 3,069, so the surplus stays at 1,792. The copies happen while arming, which both runs share. `seam_stop` is unchanged at 24,330.
4. **The fixer's attribution of the small heap rises holds.**
   - **`wide-step-pairs`:** +64 B at 41,601 B is about 513 boundaries at 321 bits, one tag bit each.
   - **`promo-rearm` and `arming-train`:** their arms are 608 bits and 32 digits wide, so their boundaries move whole before and after the fix, and the tag bit is their only new cost.
   - **`dense-suffix`:** I inferred it from the same slope, about `1e-3` B/B, and did not trace it.
   - **The −512 rows:** a lone wide boundary now stays live instead of occupying a four-slot `Vec`.
5. **Prose:** see the prose findings above.
6. **`MAX_HEAP_BYTES_PER_INPUT_BYTE = 18` is stale.**
   - **The rule** (`board/ceilings.rs:9-24`) is `ceil(1.25 × max((reading − 1024) / denominator))` over the rows judged against the default ceiling.
   - **Excluded rows:** query rows, deserialize rows (all below 7 anyway), and the comb-scatter projection.
   - **At the measured base `6a8dab7cc`:** the maximum is 12.693 (`version_min_ticks × propagate-seam`), giving 16.
   - **At the tip:** the maximum is 11.993 (`party_forks × ascend-plateau` and `ascend-cliff`), giving `ceil(14.99) = 15`.
   - **Caveats:** the captures come from the Mac, and 14.99 sits right at a rounding boundary.
   - I did not change the constant.

## What I checked and found sound

- The stack discipline and all five transitions: push, pop, close, drop, and remainder push-back.
- Rebuilding from limbs, including the `u128` path and five-limb copies.
- `is_empty` with the live slot.
- The rebase: `main` moved only `suanpan` reset and rank, which `RangeMinima` does not use.
- The demonstration, reproduced at the rebased `382b29be5`: 25.9 / 25.8 / 25.8 / 25.8 B/B, red at depths 260 to 2,080, matching the commit message.
- Every cited tip reading, reproduced exactly: plunge 24,133 and control 25,925; `seam_stop` 24,330; `WP(65)` peaks 52,096 and 104,064. Both `WIDE_STEP_PAIRS_HEAP_CEILINGS` values are exactly ×1.25 of those peaks.
- Deleting the counterfactual sentence from `WIDE_STEP_PAIRS_HEAP_CEILINGS`, which follows the owner's ruling.

## Evidence

All runs were local under `nice -n 10` with `CARGO_BUILD_JOBS=4`. The logs are in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-range-minima-wide/`.

- `probe.log`: `3 tests run: 3 passed` (the model, the width sweep, and the heap band, e.g. `RHP e=257 … d=4100 … per_byte=8.04`).
- `covering.log`: `22 tests run: 22 passed` at `PROPTEST_CASES=20000`.
- `diffops.log`: 55 passed and 2 timed out, at 3,000 cases. The two were `wide_pre_scans_tick_identically` and `multiple_wide_pre_scans_tick_identically`. Rerun alone at their default case count (`prescan.log`), they read `2 passed` in 6 and 8 seconds, so my raised case count caused the timeouts.
- `seam.log`: `10 passed`, with the `MEASURED` lines quoted above.
- `touch.log`: `RCT fresh=4 classified=4 cover=4`.
- `demo-probe.log`: `RHP64 d=260 bytes=4307 peak=111504 per_byte=25.9 … RED`.

The probe sources are in that directory as `review_probe.rs` and `review_heap_probe_board_depths.rs`. The whole-band heap probe's source was overwritten, but its output is in `probe.log`. The scratch worktree is removed, along with its build directory, `/Volumes/forge/build/be/46a3db4a3a6c35`.