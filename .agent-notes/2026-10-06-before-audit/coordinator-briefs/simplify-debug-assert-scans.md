<!-- CAVEAT LECTOR: a simplification brief written by the coordinator (Claude Opus 5.5) from the L7 auditor's observation O2, after Finch ruled to delete these assertions under a mutation check. -->

# Simplification brief: delete three debug assertions that scan whole buffers

Kind: self-contained simplification. It changes no public API or format, and
no release-build behavior.

## The owner's ruling

Delete the three assertions below, on one condition: show by targeted
mutation that the committed tests still catch every violation each assertion
would have caught. The committed tests are the home for these invariants.
In debug builds, the assertions cost time proportional to retained storage on
hot paths. That slows exactly the deep and wide tests the audit adds.

## The three sites

1. `crates/suanpan/src/accumulator/digits/read.rs`, in `read_digits`:
   `debug_assert!(self.digits[..start].iter().all(|&digit| digit == 0));`.
   - It scans the low prefix that a scaled read exists to skip, on every
     scaled read.
   - `before`'s `ScaledWidth::read` relies on that skip.
   - Keep the `O(1)` assertion on the line above it,
     `start <= self.highest_nonzero`.
2. `crates/suanpan/src/accumulator/digits.rs`, in `Digits::activate`: the
   `debug_assert!` that all retained digits are zero.
   - It scans the whole retained buffer every time a small accumulator spills
     into digit form.
   - So a pooled accumulator that once grew wide pays its full retained
     length on every reset-then-spill cycle.
3. `crates/before/src/version/range_minima/boundary.rs`, in
   `Boundary::from_positive`: the `#[cfg(debug_assertions)]` block that clones
   the whole accumulator to check that its sign is `Greater`.
   - The clone costs time proportional to the retained allocation, once per
     boundary.

## What to verify

For each site:

- **Name the invariant.** State the invariant the assertion checks, and name
  the committed test or tests that check it. The L7 auditor reports, and the
  coordinator has not verified, that `suanpan`'s representation suites
  (`representation.rs`, `digits/tests.rs`) check the first two after every
  step. For the third, look in the range-minima and tick tests.
- **Prove a test catches it.** By reversible string swap, inject a production
  defect that breaks the invariant, with the assertion already deleted. Show
  that a committed test fails, and quote the failure verbatim.
  - For site 1: leave a nonzero digit below `start`.
  - For site 2: skip zeroing on deactivation.
  - For site 3: let a non-positive difference reach `from_positive`.
- **If no committed test catches it**, stop and report for that site. The
  assertion stays until a test holds the invariant, and the coordinator asks
  an auditor for a machinery brief.
- **Check the meters.** Debug-build board or meter readings that include
  these scans may fall. If a committed ceiling covers one, capture the fall
  by lowering the ceiling, per the pinned-instrument rules.

## Where comments go

Where an assertion is deleted, add no comment that says it was deleted.
Where the invariant matters to a reader, state it in the function's doc or a
comment, and name the test that holds it.
