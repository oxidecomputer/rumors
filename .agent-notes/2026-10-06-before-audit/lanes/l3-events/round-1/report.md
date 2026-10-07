<!-- CAVEAT LECTOR: the round-1 report of the L3 (events) auditor, Claude Opus 5.5, condensed by the coordinator, who verified the explore branch's signatures and read the D1 record and fix note; measurements are the auditor's own claims. -->

# Lane L3, round 1: report

The explore branch is `explore/l3-events`, and all five of its commits are
signed. The sibling files in this directory are the auditor's deliverables.

## Verdict

- One confirmed resource defect, D1, and no correctness defect.
- On everything sampled:
  - tick is exactly the paper's event
  - `ticks(k)` equals k sequential ticks at any width
  - `min_ticks` is a floor over histories, and some history attains it
- The committed tick tests have a blind spot: four plausible one-line defects
  pass all 72 committed tick, `ticks`, and `min_ticks` tests, and only the
  auditor's co-generated probes catch them.
- Correctness is at diminishing returns. One resource avenue remains:
  scaling families for tick's touch and scan counters.

## D1: `min_ticks` transient heap is 2 to 10 times over the board's ceiling

The records are [`defect-D1-min-ticks-heap.md`](defect-D1-min-ticks-heap.md),
[`test-brief-D1.md`](test-brief-D1.md), and
[`fix-note-D1.md`](fix-note-D1.md).

- **Readings:**
  - 39 bytes per input byte on a plain rising right spine, just past every
    power of two
  - 122 to 197 bytes per input byte on a spine entered by one wide jump
- **The ceiling:** the board's per-sample ceiling is `1024 + 20n`. The board
  does not sample either family.
- **What it reopens:** September's closed fix for `min_ticks` heap.
- **Two mechanisms, both only in `min_ticks`:**
  - boundary payloads stored in a `Vec` that doubles
  - contributions spilling once an offset leaves `i32` while the freeze
    allowance is 256 bits
- **Classification:** the coordinator treats D1 as a defect, not an
  observation. The crate page calls more than a small constant multiple of
  temporary memory a bug, the board's ceiling makes that limit concrete, and
  D1 reopens a fix whose goal was that ceiling.

## Briefs

- [`machinery-brief-cogen.md`](machinery-brief-cogen.md): co-generated
  (version, party) pairs, with bushy, spine, wide, and multi-scan strategies.
  It catches four mutants that escape the committed suite: M6, M8, M15, and
  M19.
- [`machinery-brief-min-ticks-histories.md`](machinery-brief-min-ticks-histories.md):
  `min_ticks` as the exact minimum over histories, as a floor property plus a
  tightness property.
- [`simplification-brief-S1.md`](simplification-brief-S1.md): four tidy-ups:
  - parameter names in the `ticks` docs
  - moving `memo.rs`'s inline tests into a sibling file
  - deleting a dead `let _ = matched;`
  - restating two `# Panics` sections

## Observations

These are in [`observations.md`](observations.md):

- The paired walk skeleton is written twice.
- A design proposal for D1: derive suspended offsets instead of storing them.
- A pointer for lane L7: `anchor.rs::compare_offset_to`.
- No `usize` defect in the lane.

## Still running when the round ended

- **Run B** on the box: the wide, multi-wide, and deep-spine families, in a
  debug build.
- **A detached spine run:** `l3_cogen_spine` passed 30,000 cases, and
  `l3_family_spine` was still running.

The auditor will be resumed to read both runs.

## Coverage and inventory

- Coverage: [`coverage-record.md`](coverage-record.md) and
  [`coverage-map.md`](coverage-map.md).
- Inventory: [`instruments-inventory.md`](instruments-inventory.md).
- Resumption notes: [`NOTES.md`](NOTES.md).

## Owner's ruling (provisional)

On the D1 fix, the owner found the coordinator's recommendations reasonable
"at first blush":

- Repair mechanism 2 by freezing when the next stored offset would leave the
  inline range.
- Repair mechanism 1 with a segmented payload stack that never copies on
  growth.
- Record the derived-payload redesign as an observation.

The fixer re-checks the amortization arguments and reports back if its
measurements argue otherwise.
