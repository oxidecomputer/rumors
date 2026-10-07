<!-- CAVEAT LECTOR: the round-2 report of the L7 (suanpan) auditor, Claude Opus 5.5, condensed by the coordinator, who verified the explore branch's signatures and read the F1b and Q1 records; measurements are the auditor's own claims. -->

# Lane L7, round 2: report

The explore branch is `explore/l7-suanpan` at `4b277b0b`, and all its commits
are signed. The sibling files in this directory are the round's deliverables.

## Verdict

- One new low-severity defect.
- The zero-shift fix and test brief, now unconditional since the owner ruled
  on question 4.
- Design evidence for the cost composition (F1), which is the owner's
  decision: question 22.
- A constant-factor brief.
- A cross-check of the adequacy lane's survivors.
- `suanpan`'s value correctness remains at diminishing returns.

## Findings and briefs

- **Zero shift:** [`Q1-fix-and-test-brief.md`](Q1-fix-and-test-brief.md).
  - **Fix:** `shift_left` checks for zero first. An operand that would extend
    the receiver gets a read-only zero scan.
  - **Test:** a property over generated redundant zeros, plus one 32-bit
    guest case.
  - **Predicted pin rises:** shift rows rise by 1 touch, and the
    `add_shifted`/`sub_shifted` rows rise from 4 to 6. That needs the
    owner's approval: question 23.
- **U1-a (low, new):** [`U1-usize-width-sweep.md`](U1-usize-width-sweep.md).
  - **Behavior:** `cmp_zero_stable_under` skips its compaction on 32-bit
    targets for widths of `32·2^32` bits or more.
  - **Cause:** `usize::try_from(adjustment_digits - 1).ok()?` at
    `crates/suanpan/src/accumulator.rs:285-286`.
  - **Evidence:** verified on 64-bit and wasm32.
  - **Fix:** keep the threshold in `u64`.
  - **The rest of the sweep:** `suanpan` is otherwise `usize`-invariant.
- **F1b:** [`F1b-design-evidence.md`](F1b-design-evidence.md).
  - **Root cause:** the `BTreeMap` of zero ranges costs `O(log R)` in three
    places:
    - a lookup on every write
    - insertion and removal at the top
    - splits
  - **Measurements:** every `before` operation grows per byte on three
    sparse families, measured at five sizes against dense controls.
  - **Candidate designs:**
    - *Tags (T):* fix only the first of the three costs.
    - *Written-position bitset (W):* replaces the map with a 64-ary
      hierarchical bitset of written positions. It is flat on every family
      and operation, and 1.3 to 3.7 times cheaper at the largest sparse
      sizes. It strengthens `suanpan`'s documented bounds, and every
      committed touch pin is unchanged. It costs up to 18% more on
      range-free `before` inputs, and 29% to 44% on `suanpan`-only
      microbenchmarks, before tuning.
- **S2, a constant factor:**
  [`S2-comparison-fixed-point.md`](S2-comparison-fixed-point.md). The
  comparison fixed point at a top digit of 2 drops from 6 touches to 2.
  - **Pins:** no pin, ceiling, or ranking moves.
  - **Board:** 5311 green.
  - **Random programs:** never more touches, and 110 of 1,000 programs
    used fewer.

## Adequacy cross-check

- The touch property (MB1's prototype, with its embedded gap rounds) catches
  the three range-dropping `compact_storage` survivors, Z3 to Z5. The
  `take_below` survivors are unreachable by reading.
- The pool model catches the `normalize.rs:74` survivor, N1. The shrink-loop
  survivors still need the adequacy lane's brief.

## Inventory and observations

- Inventory: [`inventory.md`](inventory.md).
- Observations: [`observations.md`](observations.md).
- Resumption notes: [`NOTES.md`](NOTES.md).
