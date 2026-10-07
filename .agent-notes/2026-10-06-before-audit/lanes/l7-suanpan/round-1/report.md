<!-- CAVEAT LECTOR: the round-1 report of the L7 (suanpan) auditor, Claude Opus 5.5, condensed by the coordinator. The coordinator reproduced D1 independently and read every record; other results are the auditor's own claims. -->

# Lane L7, round 1: report

The explore branch is `explore/l7-suanpan` at `880aa203`. The sibling files
in this directory are the auditor's records and briefs.

## Verdict

- The round produced three confirmed findings and one question.
- No value-level defect turned up in `suanpan`'s arithmetic, comparisons,
  normalization, reads, or conversions.
  - A pool model ran 20,000 programs and checked every step against an exact
    oracle. It catches 8 of 9 injected value mutants; the ninth trips
    production's own `debug_assert`.
  - The amortized touch bounds held under random programs, a hill-climbing
    adversary, and shifts rescaled by 2 to 8 times.
- `suanpan`'s value correctness is at diminishing returns. How its costs
  compose with `before` is not yet exhausted.

## Findings

- **D1 (high, 32-bit only):** [`D1-limb-index-wrap.md`](D1-limb-index-wrap.md).
  - **Behavior:** a limb stream longer than `usize::MAX` wraps its landing
    position on 32-bit release builds. It silently adds a value at the wrong
    scale where the docs promise a panic.
  - **Cause:** the limb index in `Digits::apply_limbs` comes from
    `Iterator::enumerate`, whose `usize` counter wraps.
  - **Fix:** counting in `u128`, which preserves the API.
  - **Coordinator check:** reproduced independently on the box, exit 100,
    `L7 case4 (index 2^32) outcome: Failed(WrongValue)`.
- **D2 (low):** [`D2-reserve-digits-panic.md`](D2-reserve-digits-panic.md).
  - **Behavior:** `reserve_digits(usize::MAX)` panics with "capacity
    overflow" and has no `# Panics` section.
  - **Repair chosen by the coordinator:** the record's option 1, making the
    hint best-effort.
- **F1 (medium, cost):**
  [`F1-cost-composition-log-factor.md`](F1-cost-composition-log-factor.md).
  - **Behavior:** `before`'s `O(n)` decode and comparison pay `suanpan`'s
    per-update logarithmic factor when an accumulator holds many zero ranges.
    Wasm fuel per byte rises across five sizes, while a dense control stays
    flat.
  - **Owner-gated:** the choice between restating the bounds and redesigning
    is question 3 in `QUESTIONS.md`.
- **Q1:** [`Q1-zero-shift-representation.md`](Q1-zero-shift-representation.md).
  - **Behavior:** shifting a redundantly stored zero retains or attempts
    space proportional to the shift, and on wasm32 it panics. A known zero
    does neither.
  - **Owner-gated:** question 4 in `QUESTIONS.md`.

## Briefs

- [`MB1-touch-bound-property.md`](MB1-touch-bound-property.md): a touch-bound
  property over arbitrary programs, with embedded gap-split rounds.
  - It catches dropped zero-range remnants, which survive all 64 existing
    `suanpan` tests.
  - It is the instrument the September triage ruled for, which never landed.
- [`S1-read-digits-high-part.md`](S1-read-digits-high-part.md): the readout's
  one-digit high part, stated directly instead of drained by a loop.

## Observations

These are in [`observations.md`](observations.md):

- **O1:** a top digit of exactly 2 makes every comparison cost 6 touches.
  - Touch totals are no longer an API promise (`crates/suanpan/src/lib.rs`),
    so this is a constant-factor improvement the audit may build.
  - Round 2 writes its brief.
- **O2:** three debug assertions cost time proportional to retained storage
  on hot paths. This is question 5 in `QUESTIONS.md`.
- **O3:** a stale panic condition in a comment in `rank.rs`. This goes to
  the docs branch.
- **O4 to O6:** test-quality items in `suanpan`'s metered suites. They go to
  a test-hygiene branch.
- **O7 and O8:** design notes. Every observation of value is
  history-independent, and the zero-range map is used as a stack except on
  writes inside a gap.

## Coverage

See [`coverage.md`](coverage.md) and [`NOTES.md`](NOTES.md).
