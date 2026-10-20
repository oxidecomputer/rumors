<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent audit-rank-fuel, for the before audit at e0956b25d. -->

# Defect: rank integration refills a zero prefix at every freeze and deferral

## Summary

`Version::rank`, `distance`, `lag`, and every operation that reaches the
same integrator do `Θ(n²)` work on `n` input bytes from the board's
`freeze-parade` family and four other registered families. The extra work is
not multiplication. At every freeze the integrator replaces its
segment-width accumulator with a fresh one, and at every deferral it replaces
its deferred-width accumulator the same way. The next width it adds lands at
the region's absolute bit position, close to the tree's maximum depth `S`, so
`suanpan` allocates and zero-fills the digit buffer from digit 0 up to `S/32`
once more. A version can fire `Θ(n)` freezes at a scale `S = Θ(n)`, so the
refills total `Θ(n²)` bytes.

Severity: medium. Under the reading that `M(n)` is the backend's
multiplication cost, which is how the crate's own family documentation reads
it, `Θ(n²)` breaches the documented `O(M(n))` bound. Under a schoolbook
reading, `M(n) = n²`, it stays inside the bound. Either way the derivation
that justifies the bound in `integral.rs` is false for this cost, and a
committed band claims a linearity that does not hold (see "Contract
clauses").

## Contract clauses

- `crates/before/src/version.rs:367` (`Version::rank`), `:435` (`distance`),
  `:470` (`lag`), `:1012` and `:1042` (the ranked encodings), and
  `crates/before/src/ranked.rs:71`, `:122`, `:158`, `:190`, `:219`, `:254`,
  `:365`: "`O(M(n))` time and `O(n)` space for `n` input bytes; `M(n)` is
  the cost of multiplying `n`-bit integers".
- `crates/before/src/version/measure.rs:15`: "Rank, distance, and lag take
  `O(M(n))` time in `n` input bits".
- The argument for the bound, `crates/before/src/version/measure/integral.rs:91-93`:
  "Each input delta therefore pays for the accumulator changes and width
  checks it causes. Freezes and deferrals move a wide value only after input
  established that width; relative segment widths are bounded by the tree
  structure traversed inside them." Relative widths are bounded, but the
  absolute position where a fresh accumulator stores them is not, and no
  input pays for that position once per freeze.
- The committed band `skyline_rank_freeze_parade_is_flat_per_unit`
  (`crates/before/tests/meter/version_scaling/skip_adequacy.rs:176-191`)
  states "rank is linear on the freeze parade". The band measures digit
  touches only, which stay flat while the refill grows quadratically, so the
  band is weaker than its claim.

## Mechanism

1. `Integrator::freeze` ends with `self.segment_width = Accumulator::new();`
   (`integral.rs:272`), and `DeferredIntegral::push` ends with
   `self.width = Accumulator::new();` (`deferred.rs:62`). The comments beside
   them (`integral.rs:270-271`, `deferred.rs:60-61`) give the reason: a fresh
   accumulator avoids the scan of the zero prefix that `reset()` performs.
2. The next `Integrator::interval` adds the region's width through
   `segment_width.add_shifted_limbs(weight_shift, ...)` (`integral.rs:199-200`),
   where `weight_shift = S − depth`. For a shallow region that is close to
   `S`. The next `DeferredIntegral::add_width` likewise adds a segment width
   at its scaled position.
3. A fresh accumulator's first wide write reaches `Digits::add_at`, which
   grows the buffer with `self.digits.resize(position + 1, 0)`
   (`crates/suanpan/src/accumulator/digits.rs:205`). That allocates and
   zero-fills every digit from 0 to `S/32`. `suanpan` documents this growth
   as its `G` term ("A large shift can make `G` large even for one input
   limb", `crates/suanpan/src/lib.rs:82`), so `suanpan` behaves as
   documented. The integrator pays `G = Θ(S)` once per freeze or deferral
   rather than once per traversal.
4. Replacing the accumulator therefore moves the cost the comment meant to
   avoid out of `reset`'s clearing loop and into the next write's `resize`.
   Digit touches stay flat, because touch counts digit writes and the scaled
   read skips the prefix through `lowest_written`. Peak heap stays flat,
   because each old buffer is freed before the next one is allocated. Only
   allocation volume, fuel, and time see the refill.

For `FZ(k)`, the parked-unit spine sets `S = 64k`. The shallow block of `2k`
wide leaves fires `k − 1` freezes, and each new segment starts at weight shift
`S − 2 − log2 k`. Each freeze refills `2k` digits of 8 bytes, `16k` bytes, so
the sweep refills about `16k²` bytes against an encoded input of about `98k`
bytes. The digit is a `Vec<i64>` element on every target, so the refill is the
same on 32-bit and 64-bit targets.

## What the scope's fuel slope reflects

The scope's fuel per byte on `version_rank × freeze-parade` equals a constant
plus 8 times the zero-filled digits per input byte, at every rung of its
ladder, within 3 fuel. The constant, 913, is fitted at the first rung alone;
the per-byte slope of 8 is not fitted at all, since Wasmtime 48 charges
`memory.fill` one fuel per byte and each digit is 8 bytes.

| `k` | Input bytes | Scope's fuel per byte | 913 + 8 × fill per byte | My fuel per byte, base | My fuel per byte, fix candidate |
|---|---|---|---|---|---|
| 512 | 50,124 | 997 | 996.8 | 995.4 | 906.3 |
| 1,024 | 100,173 | 1,081 | 1,080.6 | 1,079.5 | 906.6 |
| 2,048 | 200,269 | 1,251 | 1,248.2 | 1,247.2 | 906.7 |
| 4,096 | 400,461 | 1,587 | 1,583.4 | 1,582.5 | 906.7 |
| 8,192 | 800,845 | 2,256 | 2,253.8 | 2,253.0 | 906.8 |
| 16,384 | 1,601,614 | 3,598 | 3,594.7 | 3,593.9 | 906.8 |

The multiplications play no part. On `FZ(k)` every product multiplies one
32-bit digit by a parked height of at most 307 bits, `k − 1` of them, for
`5k` limb operations under a model of `num-bigint` 0.4.8's `mac3` dispatch.
The scope's two suspicions do not hold. No product is unbalanced and done
schoolbook, and no sum of products keeps its width: on every family I
probed, the products' modelled total stays below 0.28 of a single full-width
product at every size, with no upward trend (see the coverage record).

## Reproduction

ox-east-1 was down, so everything ran on this Mac at `e0956b25d`, under
`nice -n 10` with `-j 8` and 8 test threads. The task prompt authorized this
in place of `common.md`'s rule that every build runs on the box.

1. Allocation volume, with no probe code in the tree. Copy
   `scratch_rank_fill.rs` from my scratch directory into
   `crates/before/tests/`, then:

   ```
   cd /Users/oxide/src/rumors-slot-56 && NEXTEST_TEST_THREADS=8 nice -n 10 cargo nextest run --locked -j 8 -p before --features meter -E 'binary(scratch_rank_fill)' --no-capture
   ```

   The test prints rather than asserts, and exits 0. At base it prints:

   ```
   PROBE FZ rank 512->1024: 4386968/50124 -> 17162392/100173  per-byte 87.522 -> 171.328  ratio 1.958
   PROBE DS rank 2048->4096: 11045272/490396 -> 34667864/980636  per-byte 22.523 -> 35.352  ratio 1.570
   PROBE DS rank 512->1024: 1584712/122716 -> 3952296/245276  per-byte 12.914 -> 16.114  ratio 1.248
   PROBE FZ distance 512->1024: 4387168/50200 -> 17162592/100250  per-byte 87.394 -> 171.198  ratio 1.959
   PROBE FZ lag 512->1024: 4387168/50200 -> 17162592/100250  per-byte 87.394 -> 171.198  ratio 1.959
   ```

2. Fill counts across a ladder. Apply `probe-only.patch`, copy
   `rank_fuel_probe.rs` into `crates/before/examples/`, then:

   ```
   nice -n 10 cargo build --locked --release -j 8 -p before --example rank_fuel_probe --features meter,touch-meter
   target/release/examples/rank_fuel_probe freeze-parade 4 14 rank
   ```

   Zero-filled digits per input byte: 0.339, 0.658, 1.308, 2.615, 5.233,
   10.47, 20.95, 41.90, 83.80, 167.6, 335.2 for `k = 16 … 16,384`. They
   double at every doubling. Touch per byte is 0.831 throughout, and peak
   heap per byte is about 0.98. The logs are `fz-rank.tsv`, `fz-pair.tsv`
   and `attrib.tsv`.

3. Fuel. In `fuelprobe/` in my scratch directory, build the guest with
   `cargo build --offline --release -p fuelprobe-guest --target wasm32-unknown-unknown --target-dir target`
   and the host with `cargo build --offline --release -p fuelprobe-host --target-dir target`.
   Then run `target/release/fuelprobe-host guest-base.wasm 0 6 14`, where
   op 0 is `rank` on `FZ` and op 1 is `distance`. The output is in
   `fuel.tsv`.

## Failure family

The refill appears when both of these hold:

- The sweep freezes or defers `Θ(n)` times. A freeze needs the drift to be
  wider than the newest delta by more than 256 bits, so one wide stored code
  can fire one freeze.
- The region after each freeze or deferral has a weight shift of `Θ(n)`.
  This holds when a deep part of the tree sets the common scale `S` and the
  freezing happens in shallow regions.

Measured, with zero-filled digits per byte doubling at each size doubling:

| Family | Fill per byte across the ladder | Site |
|---|---|---|
| `freeze-parade` | 1.31 → 83.8, `k` 64 → 4,096 | segment width only |
| `freeze-pos` | 0.028 → 1.73, `k` 64 → 4,096 | segment width |
| `promo-rearm` | 0.126 → 3.19, `k` 64 → 4,096 | both, about 2:1 |
| `dense-suffix` | 0.140 → 3.30, `p` 64 → 4,096 | both, about 2:1 |
| `arming-train`, `w = g = 32` | 0.66 → 140, `n` 8 → 2,048 | both, about 2:1 |

It disappears when freezes are `O(1)` (`lone-freeze`), when no freeze fires
(`weight-comb`), or when every segment starts at a small weight shift. The
worst case is `Θ(n²)`: a sweep has `O(n)` freezes and deferrals, and
`S ≤ n`.

## Root cause

The integrator re-creates two accumulators per freeze or deferral, and so
pays `suanpan`'s growth term `G` each time, the distance from digit 0 to the
write position. The input funds only the first growth to a given position.

## API and formats

The fix candidate below preserves the public API and every wire and storage
format. It changes private integrator state and the bounds of `suanpan`'s
internal `reset` loop.

## Question for the owner

Which `M(n)` does the documented bound mean? I recommend fixing this whatever
the answer, because the derivation it rests on is false and the fix is small.
The answer decides whether this record is a bound breach or a broken
derivation inside the bound. Evidence for the backend reading:

- `crates/before/src/testing/meter.rs:1899-1902`, the `wide_arming`
  constructor's rustdoc, calls the reduction's one wide product "priced at
  the multiplication bound" and says "A per-digit schoolbook charge instead
  pays `Θ(w · d)` digit work … and is quadratic at `w = d`". The family was
  designed to catch a quadratic cost as a failure.
- `crates/before/src/version/measure/integral/deferred.rs:130` speaks of "the
  integer backend's superlinear multiplication bound".

## Test brief (for a demonstrator)

**Invariant.** Integrating a version's rank allocates a bounded number of
bytes per input byte, however many freezes and deferrals its sweep fires: no
segment and no deferral pays again for the zero prefix below its absolute
position.

**Form.** Two two-scale bands in the style of the neighbouring `skyline_*`
bands, each over a named generator family at a size and its double. These
bands are point pairs because the claim is flatness across a doubling, as
their neighbours state theirs.

**File.** `crates/before/tests/meter/version_scaling/skip_adequacy.rs`, beside
`skyline_rank_freeze_parade_is_flat_per_unit`. `alloc_meter` is already
linked by the meter suite: `tests/meter.rs` defines `peak_heap` with
`alloc_meter::measure`.

**Band 1, the segment-width site.**

- *Construction.* `Shape::FreezeParade.build1(k).version()` at
  `k = RANK_FREEZE_PARADE_SMALL` (512) and `2k`. Reuse
  `rank_freeze_parade_run`, which already checks the family's closed-form
  `min_ticks`, or a sibling of it. Measure the rank call with
  `alloc_meter::measure(|| v.rank())` and record `stats.bytes_allocated`
  beside the existing touches. Keep the touch reading and the allocation
  reading in separate calls, so neither meter's setup enters the other.
- *Liveness floor.* `bytes_allocated` must be at least the bytes of the
  returned rank's own numerator, which the operation has to allocate. State
  that derivation in the floor's assertion message. If the band cannot read
  the numerator's limb count through the public API, use a bound that is
  provably below it, and say why it is.
- *Assertion.* `assert_flat("rank_freeze_parade_allocated", "byte",
  (small.allocated, small.bytes), (large.allocated, large.bytes))`.
- *Failure on the base commit.* `alloc_meter` readings are deterministic.
  These are the scratch run's numbers, from the dev profile in which the
  meter suite runs:
  `skyline_comb_rank_freeze_parade_allocated: per-byte cost grew more than x1.25 across the size doubling: 4386968/50124 -> 17162392/100173`

**Band 2, the deferred-width site.**

- *Construction.* `Shape::DenseSuffix.build2(p, p).version()` at `p = 2048`
  and `4096`, the board's `DS(p, p)` diagonal: 490,396 and 980,636 encoded
  bytes.
- *Assertion.* `assert_flat("rank_dense_suffix_allocated", "byte", …)`, with
  the same liveness floor.
- *Failure on the base commit.*
  `skyline_comb_rank_dense_suffix_allocated: per-byte cost grew more than x1.25 across the size doubling: 11045272/490396 -> 34667864/980636`
- *Why it is a separate band.* With only the segment site repaired, this band
  still fails at 6,832,144/490,396 → 17,853,392/980,636, ×1.307. So it
  witnesses the deferral site independently. Do not use smaller sizes: at
  `p = 512 → 1024` the base ratio is 1.248, inside the ×1.25 slack. The pair
  `4096 → 8192` widens the segment-only margin (×1.47 predicted, not
  measured) at twice the input.
- *Runtime.* My scratch test ran all five cases at base in 0.229 s in the dev
  profile.

**Optional third band.** `distance` over `FZ` at the same sizes, since
`Integrator::pair` is a separate entry into the integrator. Its base ratio is
×1.959.

**Prose.** Once the allocation leg exists, restate the existing band's doc
(`skip_adequacy.rs:176`) so that each leg's sentence says what that leg
establishes.

**Ceilings.** The fixer, not the demonstrator, adds absolute allocation
ceilings at both scales, measured ×1.25 on the fixed tree as the touch
ceilings are. The fixed readings are 209,048 and 417,944 bytes for `FZ`, and
4,724,528 and 9,444,080 bytes for `DS`.

**Residual blind spot.** A repair that clears the prefix with a loop that
neither allocates nor records touches would pass both legs. Only fuel sees
such a loop. The scope's fuel-board proposal would close this gap.

## Fix note

**Recommended: the smallest complete fix, measured as a prototype.**

1. In `suanpan`, `Digits::reset` (`digits.rs:166-176`) clears
   `digits[min(lowest_written, highest_nonzero) ..= highest_nonzero]` rather
   than `digits[..= highest_nonzero]`. Every position below `lowest_written`
   is zero. `digits.rs:26` states that invariant, the scaled read relies on
   it, and the representation tests check it after every step. A reset then
   costs the written span rather than the working width.
2. `Integrator::freeze` calls `self.segment_width.reset()`, and
   `DeferredIntegral::push` calls `self.width.reset()`. Each accumulator
   keeps its buffer, so a later write at the same or a lower position grows
   nothing, and growth beyond the highest position reached so far happens
   once per traversal.
3. Restate the comments at `integral.rs:270-271` and `deferred.rs:60-61`: the
   reset clears only the span this segment or interval wrote.
4. Restate `suanpan`'s documented reset cost from `O(W)` to the written span
   (`crates/suanpan/src/lib.rs:100`, `crates/suanpan/src/accumulator.rs:257`),
   since the integrator now relies on it.

Measured on the prototype (`fix-only.patch`, uncommitted, now reverted):

- Zero-filled digits per byte are flat on all five families (`fixproto.tsv`),
  and allocation volume per byte is flat: 4.17 on `FZ`, 9.63 on `DS`.
- Fuel per byte is flat: 903 to 907 for `FZ` rank at `k = 64 … 16,384`,
  1,157 to 1,175 for distance.
- Touches rise slightly, from 0.831 to 0.841 per byte on `FZ` rank, because
  each reset now clears its segment's span. That stays far below the board's
  ceiling of 24 per byte, and the pinned worst families for `version_rank`
  (`arming-train` for heap, `harmonic` for touch) are unaffected. Peak heap on
  `arming-train` rises about 0.7% at the top of my ladder, because the two
  width buffers are retained; `arming-train` is already that cell's pinned
  heap worst.
- `suanpan`: 94 of 95 tests pass. The one failure is the pinned reset cost
  moving down, as the fix intends, and the fixer re-pins it:
  `metered.rs:442`, "reset at 64 held digits: one touch per digit (the shift
  grew the span by one)", left 64, right 65.
- `before`: 90 of 90 tests pass under
  `-E 'test(/measure|rank|ranked|integral|deferred|width/)'`.

This prototype check is a feasibility test, not the landing check, which
remains the fixer's.

**Alternative: structural and larger.** Give `suanpan`'s digit storage a base
offset, so that a buffer spans only `[lowest_written, highest]`. A fresh
accumulator's first write at a high position would then allocate `O(1)`, and
`G` would measure growth of the written span rather than of the absolute
position. That fixes every caller with this pattern at once, but it touches
every indexing site in `suanpan` and collides with the
`fix/suanpan-uniform-amortization` rework.

**Entry points that share the mechanism.** Every caller of `Integrator`:

- `Rank::of_version`
- `Integrator::pair_rank`, behind `distance` and `lag`
- `Integrator::pair_order`, behind `Version::rank_cmp` and `Ranked`'s `Ord`
- the ranked encodings, which compute a rank

The region reader's `Extremum::min` also replaces its register
(`crates/before/src/version/io/regions.rs:292`), but it writes at shift 0,
so its refill is bounded by the width of the delta that caused it. It is not
affected.

**Base.** The mechanism is the same at `bf2ba264`, where the scope measured:
`freeze` and `push` replace their accumulators there too, and `suanpan`'s
`add_at` grows through the same `resize`. Base a fix branch on the tip of
`fix/suanpan-uniform-amortization`, or on `main` after it lands, since both
change `suanpan`'s `digits.rs` and the integrator.
