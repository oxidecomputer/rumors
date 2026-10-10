<!-- CAVEAT LECTOR: the correctness review of eedff98d, written by a Claude (Opus 5.5) agent and archived verbatim by the audit coordinator. Its claims are unverified unless the README says otherwise. -->

## Verdict: changes required

The code computes correct values everywhere I looked. Every value differential I built passes, and the landing check at `eedff98d6` is clean against the baseline. The branch needs changes for two reasons:
- **False cost claims about `bits()`.** Three constructed sequences contradict its public rustdoc.
- **One ruling-conformance question about `bits()` near a power of two**, which is the owner's call.

The builder's account of the heap movement is also materially wrong. That is a correction to the evidence the owner will rule on, not a code defect; it gets its own section below.

## Blocking findings

### 1. `bits()`: "a repeated read before the next update takes O(1) time" is false

Locations:
- `crates/suanpan/src/accumulator.rs:472-474` (public rustdoc)
- `crates/suanpan/src/accumulator/digits/bits.rs:11-13` ("the `normalized` flag lets every repeat before the next update answer from that digit alone")
- the commit message of `eedff98d6`

Mechanism:
- The normalized form of `2^(32k)` is `[0, …, 0, 1]`, and its top digit is 1.
- Any compacting read sees a top partial below 3, folds the top two digits into one, and writes the result back with `add_at`. The compacting reads include `sign()`, `is_zero()`, `acc > 0u8`, `==`, and `Display`'s own comparison with zero. The write is at `digits/sign.rs:93-97`.
- `add_at` clears `normalized` (`digits.rs:207`). The stored form becomes `[0, …, 0, 2^32]`.
- The next `bits()` finds the top three partials (`B`, `B²`, `B³`) each straddling a power of two. It falls back and renormalizes all `k` digits.

The flag discipline itself is sound: I found no write that leaves the flag set falsely. The defect is that reads undo normalization.

Constructed input: `add_shifted_limbs(32k, [1])` (and `[2]`), then `bits()`, `bits()`, `is_positive()`, `bits()`. Probe P1 measured touches:
```
P1 2^(32k) k=1000: first=1010 repeat=1 sign=5 bits_after_sign=1006 repeat=1 cmp_u8=6 bits_after_cmp=1006 display=1006 is_zero=5 bits_after_display_and_is_zero=1006
P1 2^(32k) k=2000: first=2010 repeat=1 sign=5 bits_after_sign=2006 ...
P1 2^(32k) k=4000: first=4010 repeat=1 sign=5 bits_after_sign=4006 ...
P1 2^(32k+1) k=1000: first=1011 repeat=1 sign=6 bits_after_sign=1011 ... cmp_u8=12 bits_after_cmp=1011
```
- **Observed:** a repeat read costs Θ(k) whenever any read intervenes, with no update in between.
- **Expected per the doc:** O(1).
- **Why the pin misses it:** `bit_lengths_cost_their_top_digits` uses `2^(32k) − 1`. That value's normalized top digit is `2^32 − 1`, which a sign query leaves alone.

Suggested repair (a design proposal): make reads of a normalized form read-only. When `normalized` is set, the top digit has the value's sign, and the top two digits give `|partial| ≥ B ≥ 3`. So `compact_until_order_known` can decide without writing, and reads would never undo normalization. The alternative is to weaken the doc.

### 2. `bits()` near a power of two pays Θ(Q) again after every O(1) update (ruling conformance; owner's call)

The documented per-call bound, amortized O(log(W+1) + Q), is met. However, the #122 ruling says "Every `Accumulator` operation is uniformly amortized, so a caller never has to think about it", and "normalize whatever we actually touch, so we don't have to re-do it".

Construction: `2^(32k)`, then repeat 100 rounds of `+= 1; bits(); -= 1; bits()` (probe P2):
```
P2 k=1000 rounds=100: bits loop 202200 (2022 per round); sign loop 404 (4 per round)
P2 k=2000 rounds=100: bits loop 402200 (4022 per round); sign loop 404 (4 per round)
P2 k=4000 rounds=100: bits loop 802200 (8022 per round); sign loop 404 (4 per round)
```
`bits()` returns O(1) output, yet re-pays the whole width on every call. An amortized alternative appears to exist; this is a sketch I have not built. Decide `|V| ≥ 2^m` in place: deposit `−2^m`, compare with zero through the compacting scan, then deposit `+2^m` back. That costs two writes plus an amortized scan, and later scans skip the gap through the recorded zero range.

### 3. The fallback condition is misstated in three places

Locations:
- `crates/suanpan/src/lib.rs:93`: "at or just below a power of two"
- `crates/suanpan/src/accumulator.rs:468-469`: "unless the magnitude lies within three units of its lowest read digit of a power of two"
- `digits/bits.rs:10-11`: "within `3·B^i` of a power of two"

The exact condition is that the top-three partial `P` satisfies `2^m − 2 ≤ |P| ≤ 2^m + 2`. The magnitude can therefore lie up to about 4.01 units away on either side and still fall back. Probe P3:
```
P3 k=1000: above-power fallback 1010 touches; control 8; four-units case 1006 touches (distance x1000 in units of B^(k-2) = 3999)
P3 k=2000: above-power fallback 2010 touches; control 8; four-units case 2006 touches (...= 3999)
P3 k=4000: above-power fallback 4010 touches; control 8; four-units case 4006 touches (...= 3999)
```
- **Above the power:** `2^(32k) + 2^(32(k−3))` falls back, contradicting "at or just below". The control `2^(32k) + 2^(32(k−2))` decides in 8 touches.
- **Four units away:** `2^31·B^k + 2·B^(k−2) + (2B−1)·B^(k−3)` falls back. Its redundant digit `2B − 1` was built by depositing `[0xFFFF_FFFF]` and `[1<<32]` at the same position. It lies 3.999 units of `B^(k−2)` above `2^(32k+31)`, contradicting "within three units".

## Corrections to the builder's report (evidence the owner rules on)

**"44 grow linearly, with slope at most +0.067 heap bytes per input byte" is false.** The builder's own `cap/compare2.log` lists these slopes:

| Row | Slope (heap bytes per input byte) |
|---|---|
| `query_coverage_many` × `meet-shade` | 9.868 |
| `query_coverage_many` × `scatter` | 8.784 |
| `query_contains_many` × `meet-shade` | 7.018 |
| `query_contains_many` × `scatter` | 5.344 |
| `version_min_ticks` × `propagate-seam` | 1.401 |
| `query_*_wide_*` | 0.21 to 0.60 |

At the tip, `query_coverage_many` × `meet-shade` reads 89.7 bytes per input byte against its 109.0 ceiling. Everything is green, but the movement is far larger than reported.

**The mechanism is not "a borrow-flag word".** Measured at the tip, against a shadow struct with the base's field types:

| Type | Base | Tip | Cause |
|---|---|---|---|
| `Accumulator` | 112 | 128 | an 8-byte flag plus 8 bytes of padding, because `i128` is 16-byte aligned |
| `Option<Accumulator>` | 112 | 144 | the `RefCell` hides `Value`'s niche, so the `Option` needs its own tag |

The new `normalized` bool costs nothing: it fits in existing padding, and `Value` is 112 bytes at the tip. The rank rows' +8,208 bytes is 513 × 16, so it is 16 bytes per held accumulator, not 8.

**The 18 "uneven" rows are mostly not the `RefCell`.** The counterfactual:
- **Patch:** I restored one- and two-limb fast paths in `interop::biguint` (`interop.rs:132`): `BigUint::from(u64)` for one limb and `BigUint::from(u128)` for two, as the base's `before` conversion had.
- **Capture:** I recaptured the board with the patch in place.
- **Same tree as the builder:** my unpatched tip capture equals the builder's `tip2.cap` exactly (`16083 rows: 16083 unchanged`).

Comparison results:
```
affine vs tip:  44 growing, 18 uneven, 579 constant, 15442 unchanged
tip vs fast:    13 uneven, 343 constant, 15727 unchanged   (deltas −1 to −8)
affine vs fast: 44 growing, 5 uneven, 243 constant, 15791 unchanged
```
- **The temporary buffer:** it accounts for 13 of the 18 uneven rows and 336 of the 579 constant rows. The interop always builds a temporary `Vec<u32>`. Under `num-bigint` 0.4.8, which stores one digit inline, the base conversion of a one-limb value allocated nothing.
- **The layout growth:** what remains after the patch moves in multiples of 16 bytes, which is the layout growth.
- **Unexplained:** three rows still move by +37 bytes at a single size (`version_tick`, `version_ticks`, and `clock_tick` × `pure-comb`). My inference, which I have not traced, is that the peak moved to a different moment.

The direct heap measurement compares `BigInt::from(&acc)` with the base conversion on the same accumulator:
```
HEAP limbs=1 digit_mode=false: interop peak=8 allocs=1; base peak=0 allocs=0
HEAP limbs=2 digit_mode=false: interop peak=48 allocs=4; base peak=32 allocs=3
HEAP limbs=3 ...: interop peak=72 allocs=4; base peak=72 allocs=4
```
Every `BigInt::from(&acc)` of a one- or two-limb value in `before`'s per-region read paths now pays one extra allocation. That is a constant-factor regression; the fix is the eight-line patch above. The builder's claim that no touch reading moves holds: only heap rows moved.

## Prose findings against Part IV

Prose style is out of my scope. The false statements are findings 1 and 3. One test doc is also inaccurate:
- `crates/before/src/version/measure/tests.rs`, the doc of `zero_drift_never_freezes`, says neither fold's freeze trips. The test asserts only `integral::FREEZE_HITS`, so the `min_ticks` freeze is never checked. That is a claim stronger than the assertion; I'm passing it to the coverage reviewer.

## Verified correct (by construction or by reading)

**`RefCell` borrow discipline (by reading).** No borrow is held across caller code:
- `Display` and `Hash` release the borrow before writing to the sink or hasher.
- `Debug` renders into a `String` first.
- `Sum`'s `next()` runs between borrows.
- Limb iterators run under `get_mut`.

`ptr::eq` guards `Ord`, and every comparison form goes through `Ord`. The borrowed-left operators read sequentially. The interop has no operation that takes two accumulators.

**Comparison loop (by reading).** It terminates: a refusal leaves the asked side at most two digits wider than the other, because compaction stops at index ≤ `b_digits`, plus one digit for carry. The far-wider decision is sound, since the unread side is below 2.01·B^(f+1). The small-value path always decides when it leads by more than two digits.

**`bits` fast path and flag (by reading).** The interval argument holds: `low·B^i < |V| < (high+1)·B^i`. The `normalized` flag is cleared only by `add_at` and set only on zero or normalized forms. Negation preserves it correctly, and no path leaves it set on a form that isn't normalized.

**Differentials, all passing.** Each check builds a fresh stored form: limbs, primitive, cancelled top, double negation, redundant low digits, and digit mode.
- `Display` against `num-bigint` for 16 format specs, including `{:+060}`, `{:*^65}`, `{:#}`, `{:.3}`, and `{:^+071}`.
- `PartialOrd`, `PartialEq`, and `TryFrom` for all 12 primitive widths, with `MIN`, `MAX`, `i128::MIN`, `u128::MAX ± 1`, and their neighbors.
- `Ord`, `Eq`, and `Hash` over 363,096 fresh pairs across forms.
- Interop at zero, negative values, and redundant zeros (`BigUint::try_from` → `Ok(0)` or `Err(NegativeValueError)`).
- Refusal-path comparisons, including both sides cancelled.
- `bits()` near `2^m` for m ≤ 330, offsets −5..5, both signs: 43,560 checks, each with a repeat read and a read after `sign()`.

**`before` (by reading and compile probe):**
- `switch_delta` is equivalent: `sign(D)·|D|` and `−sign(D)·|D|` equal `D` and `−D`.
- `rank`'s `expect` is unreachable except through a `suanpan` bug, because the sum only adds nonnegative limbs and shifts.
- `min_ticks`' `expect` rests on the existing nonnegativity invariant. Decode rejects negative leaf heights, but that is my inference, not a proof. A violation previously returned `|answer|` silently in release builds; it now panics.
- Removing the zero-drift freeze case is exact. Every `None` arm of `close_segment` leaves `parked` one digit wide, so the old `defer_parked(None)` reset could no longer fire, and a zero-drift freeze would settle `P·w` early and continue exactly.
- The only changed signatures are `jump` (`&mut` became `&`) and `Signed::of` (`mut` dropped).
- A compile probe shows every public `before` type keeps `Send`, `Sync`, `RefUnwindSafe`, and `UnwindSafe`. The fork iterators already lacked `UnwindSafe`, because they hold `&mut Party`.
- Only `before` and the wasm guest depend on `suanpan`.

## Non-blocking observations

- **Redundant reads in `tick.rs`.** Lines 663, 691, and 709 call `self.gap.sign();` or `self.height.sign();` and discard the result, right before `BigInt::from(&…)`, which compacts by itself. Each costs one touch. `projection.rs` had the same calls removed in commit 1.
- **`Debug` flags.** `Debug` no longer honors `{:x?}`: it renders through `format!("{:?}")`, where the base wrote through the caller's formatter.
- **Operand forms.** The ruling says "every operand form", but borrowed primitive operands are absent (`acc + &5`, `AddAssign<&i64>`, `Sum<&u64>`), and so is primitive-left arithmetic. `num-bigint` provides these. Only primitive-left *comparisons* were deliberately dropped, for type inference.
- **Hash input width.** `Hash` writes a `usize` length prefix, so its input bytes differ between 32- and 64-bit targets, as they do for `BigInt`. This is a usize-invariance note only; equal values still hash equally within a process.
- **Band re-pin.** `STOPPING_DIFF_BAND` moved from (5,900, 9,834) to (8,973, 14,955). That is the owner's Q125 re-pin, re-derived over #38, and the ready entry already notes that it differs from the ruling's (9,742, 16,237).
- **Surface leg.** It reads 214 = 134 + 80 excepted, against a baseline of 211 = 134 + 77. The new `suanpan` items are module-excepted, so none is board-covered. That's for the coverage reviewer.
- **Pre-existing.** Shifting a digit-mode cancelled zero (`Value::shift_left` via `mem::replace`) returns it to the small representation. That contradicts the module doc's "stays there until reset" (`accumulator.rs:3-7`). It predates this branch: base `a591f15ca` does the same.
- **The ready entry is stale.** #122's entry in `QUESTIONS.md` still describes the two-commit state (`539870ba`, `373e7541` on `ca02dea2`), not the four commits under review.

## Evidence

**Base verification:** `git -C /Users/oxide/src/rumors-slot-51 rev-parse HEAD` → `eedff98d65309eaeeb333d7ad84ccc025d39740b`.

**Landing check** (one run, at the tip):
```
on-illumos.sh /Users/oxide/src/rumors-slot-51 'unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24; ~/bin/audit-reserved ~/bin/audit-check'
audit-check: ok      docs (13s)
audit-check: ok      lints (16s)
audit-check: ok      surface (49s)
audit-check: ok      tests (104s)
audit-check: ok      wasm (153s)
audit-check: ok      board (202s)
audit-check: clean in 202s; logs in target/audit-check-logs
exit 0
```
Counts, which match the baseline's shape with the branch's added tests:
- `before` + `suanpan`: `815 tests run: 815 passed (1 leaky), 1 skipped`. The leaky marker is nextest's `LEAK` on `version::tick::tests::materialize_is_a_noop_once_built`, a handle-leak note, not a failure.
- Snapshots: `142 tests run: 142 passed (1 slow), 594 skipped`.
- Doctests: `193 passed` and `13 passed`.
- Surface: `14 tests run: 14 passed`.
- Board: `5361 green / 0 red`; `worst-case pin: clean (272 pinned rows verified at 2 sampling scales)`.
- wasm: `12 / 25 / 43` passed.

**Probes:** built with `--no-run` separately from the run. `cargo nextest run --locked -p suanpan --features touch-meter,num-bigint -E "test(/zz_review|^p[0-9]/)"` → `12 tests run: 12 passed`. `cargo nextest run --locked -p before --all-features --test zz_review_probe` → `2 tests run: 2 passed`.

**Layout probe:** `LAYOUT Digits=80 ZeroRanges=32 Value=112 Accumulator=128 Option<Accumulator>=144 RefCell<Value>=128 BaseDigits(shadow)=72 BaseAccumulator(shadow)=112 Option<BaseAccumulator>(shadow)=112 align(Value)=16`.

**Board captures:** `cargo run --locked --release -p before --example amp_board --features touch-meter,scan-meter,serde,borsh -- capture|compare …`, with the outputs quoted above.

**Cleanup:** every probe was a reversible edit or an untracked file. After restoring, `git -C /Users/oxide/src/rumors-slot-51 diff --stat` is empty, `git status --short --untracked-files=all` is empty, and HEAD is still `eedff98d6`. A final sync left the box's copy clean as well. Two capture files remain inside the slot's build directory on the box, at `~/src/rumors-slot-51/target/review122/{tip,fast}.cap`.

## Files

All in `/private/tmp/claude-506/-Users-oxide-src-rumors/b638bbfa-77c5-4e67-a88c-bf0a7948c0cb/scratchpad/review-122-correctness/`:
- `probe-suanpan.rs`, `probe-before.rs`: the probes
- `apply-fastpath.py`: the counterfactual patch
- `landing.log`, `landing-logs/`: the landing check
- `probe-run1.log`, `probe-run2.log`: probe output
- `compare.log`: the board comparisons
- `NOTES.md`: resumption notes
