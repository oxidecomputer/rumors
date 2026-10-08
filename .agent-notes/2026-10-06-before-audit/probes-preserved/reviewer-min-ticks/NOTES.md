# reviewer-min-ticks: resumption record

Task: round-1 review of fix/before-min-ticks-heap in /Users/oxide/src/rumors-slot-16,
tip 38202ca6 (verified HEAD == tip, clean, at start). Base 5065caeb; demo commit ba094bbe.
Caps: <=4 focused runs on box (>=1 debug), no landing check unless code changed. Commit nothing.

## Established (by reading)
- Fold correctness: narrow() moves the newest leaf's +1 coefficient to a new prefix whose
  component is the leaf's whole offset (== live); add_offset_to after observe adds 0. Sound.
- offset_fits_inline and inline() use the same predicate (i32::try_from), so after narrow the
  offset never spills. Values are robust to the boundary's placement (spill path is exact).
- Floor premise (1): add_leaf calls live.to_bigint(); small form touches stored_digit_count()>=1,
  digit form loops digits[0..=highest] (>=1). Verified by reading suanpan. Denominator
  Version::shape().count() is public API.
- Condition (2): regression cost is exactly k*r: fixer runB 27692-22572 = 5120 = 1024*5.
- Condition (5): ba094bbe (this branch) added ~53 numeric row comments in worst.rs plus the
  WORST_RANKINGS doc requiring them.
- Heap meter (PeakAlloc) lives only in integration test binaries; the #[cfg(test)] narrowed
  entry is unreachable from them.

## Hypothesis under test
- Spill regime (prefix >= 2^23) is heap-RED: each spilled contribution = 48 B slot in a doubling
  Vec; on a +1-step suffix (~6 bits/level) that is far over 20 B/B. Probe: narrowed bound 1
  on JR-like and WS-like spines, plus one real-scale run (2^23+ wide levels, then unit suffix).

## Scratch edits (MUST revert; prove with git diff empty)
- see below as they are made

## Runs
- Scratch edits applied (saved as scratch-probe.patch + reviewer_probe.rs): cfg(test)->cfg(any(test,
  feature="meter")) on 3 narrowed-bound items; REVIEWER_MUTANT env switches (1: offset added before
  observe in loop; 2: narrow skips coefficient decrement); testing::meter::reviewer_probe module;
  tests/reviewer_probe.rs (PeakAlloc).

## Run 1 (release, run1-release.log)
- CONFIRMED spill regime RED on production code: REAL wide=2^23+16 unit=2^25: 91,226,247 B,
  peak 6,135,222,240 = 67.25 B/B vs ceiling 1,824,525,964. Prefix alone (unit=0): 15.34 B/B green.
- Narrowed bound reproduces at small size: JR inline=1 125.1-125.7 B/B (== D1), WS inline=1 26.1 B/B,
  JR-after-64 inline=64 115-125 B/B. Production at same sizes green (13-14 B/B).
- seam_stop: diff 8892 -> 6164 (control +3066, stop +338), band (5904, 9840) doc stale
  ("part only at the stack", "record x0.75/x1.25"); not attributed. Per-hop 5-digit survivor read
  (+5120) still exceeds band ceiling (11284 > 9840) by arithmetic.

## Run 2 (debug, run2-debug.log): 24/24 pass incl. band demo + phase probe
- seam_plunge rise == reanchor reset (3077); replace_live -> 19495 (parent 19503). Removable.
- hoisted-window/plateau-puncture/reveal-hifloor rise == offset_add phase (~1 touch/leaf).
- Worktree reverted: git diff empty, status clean, HEAD 38202ca6. Patches: scratch-final.patch,
  band-demonstration.patch, reviewer_probe.final.rs. Remote ~/src/rumors-slot-16 still holds the
  probe file until next sync.
- DONE: report next.
