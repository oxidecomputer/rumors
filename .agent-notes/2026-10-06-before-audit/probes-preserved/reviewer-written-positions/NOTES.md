# reviewer-written-positions: resumption record

Task: review fix/suanpan-zero-range-bitset in /Users/oxide/src/rumors-slot-24,
commits fe2525bb..b9b7e953 (b1551ec0, 2fb7a9f7, b9b7e953). No commits; repairs
as diffs here. Cap: four remote runs, no landing check.

## Established (by reading)
- HEAD b9b7e953, tree clean at start.
- Re-pins: cmp_worst.py over fixer worst-mask.log: the 7 changed rows are
  exactly the log's disagreements with parent pins, minus the 2 baseline
  count_display lines; message numbers match worst-parent.log/worst-mask.log.
  Each flip follows from arming-train's fall alone.
- FINDING A (blocking): Digits::normalize returns early on Equal
  (normalize.rs:39-41) before the set replacement (:92), so a bitset sized to
  the old width survives normalize of a zero value. Parent's scan cleared
  ZeroRanges on a zero collapse, so parent held O(1). Breaks owner ruling
  "in every case" and lib.rs:95-97 / accumulator.rs:235-237 O(Q) promise.
  No before/rumors caller of normalize -> repair moves no pin.
- FINDING B (prose): skip_adequacy.rs:8,65,82,264 "recorded zero range(s)";
  fuzzfit harness lib.rs:32 "record many zero ranges" -- untouched stale prose.
- FINDING C (prose): BAND_PERCENT claims smallest rise "on any family's
  decode and <= cells"; kb3.log shows control decode/<= at ~+1.93..1.96%,
  InBand. Should say sparse.
- Prose: written.rs uses "run" for both the top run of the set and runs of
  unwritten zeros (lines 21, 26), and digits.rs/sign.rs say "run of unwritten
  zeros".

## Runs
1. (planned) stage A: new zero-normalize test fails at tip; mutants M1..M7 on
   written.rs against the model test only; restore + rebuild.
2. (planned) stage B: fix applied; new test + suanpan suite + clippy.

## Next
Restore local edits after runs; confirm `git -C slot-24 diff` empty.

## Runs done (2 of 4)
1. run1.log: stage A test FAILS at tip ("463 words for 1 digits", representation.rs:275);
   mutants M1..M8 all caught by the model test; cases before first catch:
   M1 2, M2 11, M3 0, M4 0, M5 139, M6 129, M7 0, M8 86. Final clean rebuild
   errored (--no-fail-fast with --no-run); touched written.rs locally so the
   next sync rebuilds.
2. run2.log: repair applied: suanpan 69/69, clippy -D warnings clean, private
   docs -D warnings clean. Reverted; tree clean.
Repairs: repair-normalize-zero.diff (verified), repair-prose.diff (comment-only,
not executed). Tree clean after both reverts.
## Next: write report.
