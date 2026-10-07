# Observations (lane L7)

Each item gives its location and enough detail to triage it without me. "Verified" means run on ox-east-1; "read" means established by reading only.

## Constant factors

**O1. A top digit of exactly 2 makes every comparison cost 6 touches, forever.** Verified (`l7_probe_repeated_comparison_cost`): after the first comparison, 1,000 more `cmp_zero` calls on `1 * 2^2048` and `3 * 2^2048` cost 1 touch each, while on `2 * 2^2048` each costs 6. Mechanism (`crates/suanpan/src/accumulator/digits/sign.rs:57-100`):
- The top digit 2 is below `COMPARISON_DECIDED` (3), so the scan clears it and reads the digit below, deciding at `2 * 2^32`.
- The collapse zeroes that digit and re-deposits `2^33`.
- `add_at` must recenter the deposit (`|2^33|` is not below the digit limit), carrying 2 back into the vacated top digit. The representation is a fixed point, and four of the six touches are churn.

The 2026-09-01 review recorded this as suanpan-17 and routed it to a performance lane; it is still present. Candidate: skip the collapse when the scan stops exactly one digit below the old top (nothing below the top was compacted), which leaves a two-read comparison. This changes touch counts, so it needs the owner's ruling. It matters to `before` wherever a running difference sits at such a value while many regions are compared.

## Debug-build cost amplification (read; affects test time, not release behavior)

**O2. Three `debug_assert!`s cost time proportional to retained storage on hot paths.**
- `crates/suanpan/src/accumulator/digits/read.rs:57` scans `digits[..start]` on every scaled read. That is the low prefix the scaled read exists to skip, which `before`'s `ScaledWidth::read` relies on.
- `crates/suanpan/src/accumulator/digits.rs:94-97` scans the whole retained buffer on every small-to-digits activation, so a pooled accumulator that once grew wide pays its full retained length at every reset-then-spill cycle.
- `crates/before/src/version/range_minima/boundary.rs:43` clones a whole accumulator (cost proportional to retained allocation) to check a boundary's sign.

The representation suites (`representation.rs`, `digits/tests.rs`) already hold the first two invariants after every step. The September review disputed a similar O(n) debug assert (suanpan-21) against an owner ruling on assertion cost. Owner-gated.

## Documentation drift

**O3. `Rank::sum_iter`'s comment cites a superseded panic condition.** `crates/before/src/rank.rs:1035-1043` says the accumulator documents "a panic at digit positions past `usize` (`shift / 32 > usize::MAX`, so from `shift = 2^37` on a 32-bit target)". suanpan's `# Panics` now state the landing condition: "a nonzero contribution would land at or beyond `usize::MAX`", where the landing is `shift / 32` plus the operand digit's offset. The comment's conclusion (unreachable from this fold) still holds by its exponent bounds. The fix is a self-contained comment edit; it belongs with whichever lane owns `rank.rs` if not mine.

**O4. A garbled comment in a metered pin.** `crates/suanpan/src/accumulator/tests/metered.rs:283-284`: "The shifted operand towers over the receiver, so the / The difference is negative, so compute its magnitude operand-first." Two drafts of one sentence are spliced together.

## Test quality (September items still open; read and verified by grep)

**O5. `scaled_read_costs_the_written_span` asserts a ceiling of 16 where the exact count is small** (`crates/suanpan/src/accumulator/tests/metered.rs:35-39`; the September review traced 3). The file header says every pin is exact.

**O6. `tests/amortized_sequences.rs` has no liveness floor** (`crates/suanpan/tests/amortized_sequences.rs:41-50`). The mixed-second-difference check passes vacuously when the meter reads all zeros (`0 <= 0`). This was September's suanpan-39.

## Representation dependence (design notes)

**O7. Every public observation of value is history-independent** (lane lead 1). Verified by the pool model: 20,000 programs, each checked after every step against an exact oracle (`S/long1.log`). Five observables are representation-dependent by documented design: `stored_bits`, `stored_digit_count`, `is_known_zero`, whether `cmp_zero_stable_under` answers `Some`, and the `(magnitude, shift)` split of `scaled_signed_magnitude`. Shift totality is the one place where representation leaks into panic and space behavior; see Q1.

**O8. The zero-range map is used as a stack except on writes inside a gap.** Read: `record_gap` inserts only above every existing range, and both `take_below` callers (`trim_high_zeros`, and the sign scan) only ever remove the topmost remaining range, since every range above the scan position has already been consumed. Only `remove_written` touches interior ranges. Any redesign answering F1 can exploit this: the logarithmic bookkeeping comes from interior lookups alone.
