<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), reviewer review-rank-refill, of fix/rank-integrator-refill at 4f7f00eee. -->

# Review of `fix/rank-integrator-refill` (tip `4f7f00eee`, base `382b07b5c`), round 1

## 1. Verdict

**Changes required, prose only.** I approve the code. Watermark-based `reset` is sound, the fix is complete for this mechanism, and both bands fail at base for the claimed reason. Four prose findings block: one in private rustdoc, one in a test constant's doc, one dangling reference, and one commit message that omits measured regressions. Each is a one- or two-line repair.

Before starting, I confirmed the slot at `4f7f00eee` with a clean tree, four unsigned commits (`aab639a95`, `7c9cc90f3`, `50fdd253b`, `4f7f00eee`), and `382b07b5c` an ancestor. The slot is still clean at that SHA. My scratch worktree and its build directory (`/Volumes/forge/build/7d/7b260e085b27e2`) are removed.

## 2. Blocking findings, most severe first

**B1. The deferred reduction's "superlinear, therefore" step is invalid as worded.** `crates/before/src/version/measure/integral/deferred.rs:132-134`
- The doc says the backend's cost "is superlinear … The costs of successively smaller products therefore sum to the same order as the root's product."
- I derived that the argument needs *polynomial* superlinearity: `M(n)/n^(1+ε)` nondecreasing for some ε > 0.
  - Each level's combined weight is at most `n`, and a node at depth `j` weighs at most `n·2^(-j/2)`.
  - With `M(n) = n^α`, the levels sum geometrically to `O(n^α)`.
  - With `M(n) = n log n`, which is superlinear, they sum to `Θ(n log² n) = Θ(M(n) log n)`, and the "therefore" fails.
- I verified that `num-bigint` 0.4.8 (the locked version) tops out at Toom-3 (`src/biguint/multiplication.rs:96,282`), so `M(n) = Θ(n^log₃5)` and the conclusion holds today.
- Commit 3 exists to pin down what `M(n)` means, so the private doc should name the property it relies on. Otherwise a future FFT multiply in `num-bigint` would silently break the bound's derivation.
- Suggested wording: "The backend multiplication, `num-bigint`'s, costs polynomially more than linear (Toom-3, `Θ(n^log₃5)`, at large sizes), so the products' costs fall geometrically down the tree and sum to the order of the root's product."

**B2. The DS allocation band's size rationale and commit 1's message misstate the base measurement.** `crates/before/tests/meter/version_scaling/skip_adequacy.rs:589-593`, and the body of commit `aab639a95`.
- The constant's doc says that at the touch band's size (500) the width buffers are "too small a share of the reading for one doubling under the ×1.25 slack to resolve that scaling". Commit 1 says the growth there "stays inside the x1.25 slack (x1.20)".
- I measured at `aab639a95`, with a scratch test calling `alloc_meter::measure(|| v.rank())` on `Shape::DenseSuffix.build2(p, p)`:
  ```
  REVIEW DS p=500 alloc=1483144 bytes=119843
  REVIEW DS p=512 alloc=1584712 bytes=122716
  REVIEW DS p=1000 alloc=3716552 bytes=239531
  REVIEW DS p=1024 alloc=3952296 bytes=245276
  REVIEW DS p=2048 alloc=11045272 bytes=490396
  ```
  Under `assert_flat`'s test, 500 to 1000 is **×1.2537 (fails the band)**, 512 to 1024 is ×1.2478 (passes), and 1024 to 2048 is ×1.3978 (fails).
- The "×1.20" is the 512 to 1000 ratio, an adjacent-row ratio from the fixer's ladder (`scratchpad/fixer-rank-refill/ladder-base.log`).
- "Smallest power of two" holds; the clause about 500 is false, though only by 0.3%.
- Keeping 1,024 is still the right choice, because 500 sits at the slack's edge.
- Restate the doc without the false clause, and with the owner's ruling in mind: it justifies a size by the base's behavior. For example: "Larger than `DENSE_SUFFIX_SMALL`, so the width buffers are a large enough share of the reading for a doubling to resolve how they grow." Correct "(x1.20)" in the message when the branch is rewritten.

**B3. Commit 2's message leaves board regressions unaccounted for.** Commit `7c9cc90f3`.
- It reports the two bands' touch rises and the tick falls. It omits these moves in the fixer's own `verify/board-moves.tsv`:
  - **Peak heap +11.6% on `freeze-pos`** for `version_rank`, `distance`, `lag`, `ranked_cmp`, `ranked_encode_rank`, and `ranked_serde_deserialize` (e.g. `17680 -> 19736`). Also +0.68% on `arming-train` and +0.30% on `promo-rearm` across the rank family.
  - **Rank-family touch rises well above the bands' +1%**: `arming-train` +9.5% (`+35386` at 372,743 bytes) and `hoisted-window` +8.3% (`+20483` at 246,636).
- I derived the mechanisms; they are not stated anywhere in the branch.
  - **Heap.** `FP(k)`'s right spine puts region `j` at weight shift about `2k − j`, so segment positions *fall* through the sweep. Fresh buffers shrank with them, while the kept buffer stays at the first segment's length, about `2k/32` digits. At 604,241 bytes, `k` is about 3,950, so the kept buffer is about 2 KB, matching the +2,056 bytes. The rise is linear (`+1032 -> +2056` per doubling) and inside the `O(n)` space bound.
  - **Touch.** Each new `reset` of `segment_width` or the deferred width immediately follows a scaled read of the same accumulator: `close_segment` at `integral.rs:294` and `push` at `deferred.rs:56`. So the reset clears at most that read's span plus about two digits. That bounds the rise by a constant factor of pre-existing read work, consistent with the board's linear rises.
- Add both moves and their mechanisms to the message. Separately, the sentence "measured x1.25 on this tree: freeze parade 209,048 and 417,944 bytes…" reads as if those were the ceilings. They are the readings, and the ceilings are ×1.25 of them.

**B4. Commit 4 leaves "the band" without an antecedent.** `crates/before/src/testing/meter.rs:2215-2218`
- The rewrite replaced "The `skyline_flatness` freeze-parade band … measures" with "The meter suite's rank bands over this family…", in the plural.
- The paragraph still ends "re-derived in closed form by this module's tests and the band", which no longer names a band.
- It means the touch band `skyline_rank_freeze_parade_is_flat_per_unit` (the new allocation band's doc says so). Name it.

## 3. Prose findings against Part IV (non-blocking unless marked above)

- `crates/suanpan/src/accumulator/digits.rs:199`: "clearing covers that one digit" would be clearer as "clearing covers only position `highest_nonzero`".
- `crates/suanpan/src/lib.rs:81-83` and the README: `R` is defined as starting at "the lowest position written since creation or the last reset". After `x <<= s`, the literal reading starts `R` at the pre-shift position, which over-states the cost. That is the safe direction, so this is not wrong. "A shift moves the written span with the value" would make it exact.
- Commit messages are imperative, name the component, and commit 2 is shaped Previously/Now with readings. Commit 4 has no body, which is fine.

## 4. Non-blocking suggestions

**Behavior-preserving, adoptable now:**
- **One helper for the clamped watermark.** `lowest_written.min(highest_nonzero)` appears three times with the same meaning: `digits.rs:146` (`copy_into`), `digits.rs:204` (`reset`), and `digits/read.rs:47` (scaled read). A private `fn written_floor(&self) -> usize`, documented once ("every digit below is zero; never above the top"), would make all three evidently the same quantity.
- **`integral.rs:93-97`:** state the argument beside the claim: each reset follows a scaled read of the same accumulator, so it clears no more than that read's span. Also name the space trade: the kept buffers hold up to the scale's digits for the whole sweep, inside the `O(n)` space bound, and this is what moves `freeze-pos` peak heap.
- **Test name.** `scaled_read_costs_the_written_span` (`metered.rs:33`) now also pins `reset`, but its name says only "scaled read". Renaming is optional; `skip_adequacy.rs` cites the name, so a rename ripples.

**Out of this branch's scope (needs a disposition from you):**
- **`skyline_flatness`.** It is a dead name: no test or module carries it. It survives in about 15 doc comments, including this family's own `registry.rs:701` and `family.rs:337`, plus `meter.rs:2168,2347,3105`, `meter/tests.rs:178,209`, and `registry.rs:688,718,752,773`. Commit 4 fixes one site. I recommend queueing one sweep for the simplification pass rather than growing this branch.

**Checked, no finding:**
- **`regions.rs:291-292`** resets on `Max` but replaces on `Min`. This is deliberate and documented at `Extremum::min`. The register is written only at shift 0, so there is no cumulative-shift refill.

**Process note:** the fixer's scratch holds known-bad-variant and `mutant-reset0` experiments (`fixer-rank-refill/variants.sh` and `mutant-reset0.sh`). Nothing committed relies on them, and the fixer's notes record your mid-task instruction to drop that work. I injected no defects myself.

## 5. What I checked and found sound

**Item 1, soundness of `reset` from the watermark.** I read every writer of `digits`, `lowest_written`, and `highest_nonzero`: `add_at`, `normalize_nonzero_from`, `compact_until_order_known`, `negate`, `copy_into`, `activate`, `clear`, `reset`, `Value::shift_left`'s replacement, and `clone_normalized_into`.
- `add_at` lowers the watermark before writing.
- Normalization starts at or below the watermark with a zero carry, so the zero prefix stays zero.
- Compaction re-deposits through `add_at`, lowering the watermark by at most one digit.
- `copy_into` clamps the watermark into the copied prefix.
- `<<=` rebuilds through `activate` and `add_digits`.
- Zeros above `highest_nonzero` hold at every operation boundary.
- `Digits::assert_invariants` (`digits/tests.rs:8-49`) checks both zero regions, and checks all retained digits zero when idle (that is, after `reset`). `complete_surface_matches_bigint` applies it after every step of histories that include `Reset`; the clone tests apply it after `clone` and `clone_from`.
- I corroborated this by construction with a scratch exhaustive test (`scratchpad/review-rank-refill/reset_review.rs`).
  - It ran every history of up to 5 steps, from both scalar and digit starts, over 13 operations: adds and subtracts at digits 0, 3, 5, and 9, compaction, scaled and unscaled reads in place, negation, `<<= 32`, `clone_from` from a high cancelled source, and `reset`.
  - After each step it checked the invariants and the value against a `num-bigint` oracle, then checked reuse after a final reset.
  - Result: `REVIEW histories=402234 resets_above_zero=144793 resets_above_top=4334`, PASS. The two counters show it reached resets with the watermark above digit 0 and above `highest_nonzero`.
- The existing properties also passed at `PROPTEST_CASES=4000`: surface, pooled reuse, representation, and clones (`7 tests run: 7 passed`).

**Item 2, completeness.**
- Every `Accumulator::new()`, `mem::take`, `mem::replace`, and `reset` site in `before/src` and `suanpan/src`, and every shifted write, writes at shift 0 (the values' own widths), with three exceptions.
- The exceptions are the integrator (`total`, which persists; `segment_width`; and the deferred width) and `Rank::sum_iter`'s documented geometric `<<=`.
- So the two sites fixed are the only per-event rebuilds followed by a write at a shift taken from the sweep's position.

**Item 3, board moves.**
- **Tick falls: corroborated.** The DS row at 980,637 bytes is `DS(4000, 4000)` and saves 110,574 touches, 27.6 per block. That is 19 + 9 digits: the unwritten prefixes below the `2^608` arm climb (`A = 32·19`) and the `2^290` drift climb (`P = 290`) that `tick`'s gap resets now skip.
- The rank rises and heap rises are explained under B3.

**Item 4, public prose.** `O(R)` is a true upper bound: within one digit of compaction, and never under-counted. It is stated at the user's altitude, in italics, and consistent with `W`, `G`, and `Q`. No public doc names the backend: `version::measure` is private, and `suanpan`'s `num-bigint` mentions are features and alternatives.

**Item 5, tests.**
- I re-ran commit 1 at `aab639a95`: both bands fail for the claimed reason, with readings identical to the message (`4386968/50124 -> 17162392/100173` and `3952296/245276 -> 11045272/490396`, `status 100`).
- At the tip, the `before` meter suite passed `68 tests run: 68 passed`, and the readings match commit 2 (`209048`, `417944`, `2364480`, `4724528`, and `pure_comb … large=1107/1639B`). The `suanpan` metered pins pass.
- The ceilings are exactly ×1.25 of those readings, and the per-byte claims (about 4.17 and about 9.64) hold.
- The liveness floor is sound: `Rank`'s `Display` renders binary, and `BigUint` storage is at least `ceil(bits/8)`.
- The FZ and DS structural claims agree with the generators' docs: spine at `64k`, `k − 1` freezes, one level per block node, and two freezes plus one deferral per block.

## 6. Evidence and verification

- **Instruction conflicts.**
  - `common.md` forbids local cargo runs; your prompt allows targeted local runs, and I followed the prompt (niced, `CARGO_BUILD_JOBS=4`).
  - The reviewer brief asks me to re-run the gate; your prompt says a landing check is already running on ox-east-1 at the tip. I did not start another, so I have no landing-check result of my own to compare with `baseline.md`.
- **Logs**, all in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-rank-refill/`:
  - `run-base.log`: bands at commit 1, `status 100`.
  - `run-ladder-base.log`: the DS ladder.
  - `run-suanpan-tip.log`: scratch exhaustive test and metered pins, `status 0`.
  - `run-surface-hi.log`: properties at 4,000 cases, `status 0`.
  - `run-meter-tip.log`: meter suite at the tip, `status 0`.
  - `reset_review.rs`: the scratch test.
