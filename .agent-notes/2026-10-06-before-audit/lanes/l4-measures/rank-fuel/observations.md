<!-- CAVEAT LECTOR: written by Claude (Opus 5.5), agent audit-rank-fuel, for the before audit at e0956b25d. -->

# Observations from the rank-fuel audit

Each entry is for triage after the audit. None is briefed for building.

## 1. The board measures peak heap, not allocation volume (design proposal)

The refill defect (`defect-rank-refill.md`) is invisible to every judged
currency of the board. Scan and touch stay flat, and peak heap stays flat
because each refilled buffer is freed before the next one is allocated. Its
allocation volume grows linearly per byte. `alloc_meter::Stats` already
reports that volume as `bytes_allocated`, but the board's `amp_board` example
reads heap through `peak_alloc` (`crates/before/examples/amp_board.rs:72-76`),
which reports only current and peak.

I measured, but did not run the board itself, that the volume per input byte
grows at each doubling on `freeze-parade` (×1.96), `dense-suffix` (×1.57 at
`p = 2048`), `freeze-pos`, `promo-rearm`, and `arming-train`. A volume reading
fitted by the board's exponent rule would have caught this defect.

Proposal: report `bytes_allocated` per input byte as a board column, or fold
it into the scope's fuel-board proposal. Allocation volume is deterministic
and needs no wasm guest.

## 2. Every flagged rank-family exponent shares one cause (verified for `rank`, `distance`, `lag`; inferred for the rest)

The scope listed fuel exponents above 1.15 on `freeze-parade` for
`ranked_encode`, `ranked_encode_rank`, `ranked_cmp`, `ranked_decode`,
`ranked_serde_deserialize`, `version_lag`, and `version_distance`. All of
these reach `Integrator`. I measured the refill on `rank`, `distance`, and
`lag(v, w)`: the ratios are identical, ×1.958 and ×1.959. The reverse lag
`w.lag(v)` stays linear, because `w` dominates `v` and nothing is
integrated. The remaining entries compute a rank or a rank comparison
through the same sweep, so I infer they share the cause. They should be
re-read after the fix, not root-caused separately.

## 3. The zero-range tree's per-write logarithm reaches beyond `Sum` (lead for the `suanpan` and measures lanes)

`ZeroRanges` charges `O(log R)` per accumulator write once two gaps coexist
(`defect-rank-sum-log.md`). Several operations documented as `O(n)` stream
writes into `suanpan` accumulators, among them `min_ticks`, `tick`,
projection, placement, `range_minima` and the region reader
(`crates/before/src/version/…`). Any of them whose accumulator ingests an
operand with zero 32-bit digit runs may carry the same hidden factor. I did
not examine them.

## 4. The scope's multiplication hypothesis is refuted (verified)

The scope suspected unbalanced schoolbook products or sums of products whose
widths do not shrink. Both claims were tested against every `add_product` operand pair,
modelled with `num-bigint` 0.4.8's `mac3` dispatch, across
`freeze-parade`, `dense-suffix`, `wide-arming`, `plateau-puncture`,
`freeze-pos`, `promo-rearm`, `lone-freeze`, `weight-comb`, and two
`arming-train` ladders. The modelled total stays below 0.28 of a single
full-width product at every size, with no upward trend. The deferred
reduction's modelled cost grows by about 0.4 per byte per doubling on
`arming-train`, the `log n` levels its design predicts.

Calibration: forcing the reduction to split at the leftmost leaf, a
reversible mutant since reverted with the diff verified identical, makes the
same model double per doubling and overtake a full-width product by
`k = 256`.

## 5. A sound usize-width argument (no finding)

`crates/before/src/rank.rs:1019-1027` reasons that `suanpan`'s documented
panic for digit positions beyond `usize` (from shift `2^37` on 32-bit
targets) cannot fire from `Sum`, because exponents are bounded by the bits
of an addressable input. The argument holds: decoded exponents count bits
actually read, and version-derived exponents are bounded by tree depth. The
behavior that differs by target sits behind an input no target can hold.
That is the "infeasible work" case the contract tolerates.

## 6. Comments that encode the refill's mistaken premise (part of the refill fix)

`integral.rs:270-271` and `deferred.rs:60-61` say that replacing an
accumulator avoids `reset()`'s scan of the zero prefix. The replacement moves
that cost into the next write instead. The refill fix note restates both
comments.
