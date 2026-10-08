# builder-comparison-fixed-point: resumption notes

Worktree /Users/oxide/src/rumors-slot-39, branch simplify/suanpan-comparison-fixed-point, base b9ec1fbd (W tip). Verified HEAD == b9ec1fbd, clean, at start.

## Established
- S2 fixed point exists only for top digit +-2 with the digit below recentered ([0,2^31) for +2; [-2^31, 0] for -2, the brief omits 0 for -2). Given index+1 == top, partial = d_top*B + d_low, so recenter(partial).0 == d_low <=> .1 == d_top: the coordinator's "only one digit matches" mutant is EQUIVALENT (cannot fail any test). Plan: run it anyway to corroborate; real calibration = drop the adjacency conjunct (M1).
- Skip vs full path: digits, highest_nonzero identical; full path adds `index` to written (truncate(index) then insert_span(index, top)) and lowers lowest_written to index. Skip: written subset, lowest_written >=. Simulation relation (same digits, S' subset S, lw' >= lw) is preserved by every op; later touches only fall. Scaled read split (shift) may differ: allowed by scaled_signed_magnitude docs ("factor need not be maximal"); before's ScaledWidth uses only magnitude*2^shift.
- Full path can convert written's Below form prefix->mask->bitset (allocation when top-1 >= 64); skip avoids it -> board heap cells may fall.
- No board capture exists at W tip (board-mask.log is at 2fb7a9f7, fixer landing at b9b7e953; both trees differ from b9ec1fbd). Plan: my one board capture = W tip (test-only edits don't enter amp_board); my tip's cells from landing check's target/audit-check-logs/board.log on the box.
- O4 applies (metered.rs 284-285). O5 applies (<= 16; predicted exact 3, control 1003). O6 applies; floor = 4 touches/round (2 add_at deposits + 2 comparisons each >= 1 touch).

## Runs (log paths in this dir)
- run1-parent.log: W tip prod code + new tests. suanpan 70 run, 69 pass; S2 pin FAILS left 6000 right 2000 (c=2); c=1 = 1000 ok; c=3 unobserved (loop stopped). O5 exact 3 / 1003 pass; O6 floor passes. Then board acceptance (W tip cells) in same log.

## Next
S2 implemented locally in sign.rs (uncommitted). After board in run1 finishes: run2 = suite + lints with S2; mutants (coordinator one-digit, M1 drop adjacency); O6 zero meter; commit x2; landing check (board.log from target/audit-check-logs on box).

## Progress (later)
- run2-s2.log: S2 suite 70/70, doclint, testdoc, clippy both configs clean.
- Commits (signed): a8a7dc59 S2 (pin extended with adjacency negative case, folded in), 2be7e55a hygiene. Tip tree verified equal to pre-fold state.
- run3: coordinator's either-digit mutant SURVIVES (70/70) - equivalent, as argued.
- run4: no-adjacency mutant M1 SURVIVED original suite (70/70) -> extended the named S2 test with (2 @66, -(2^33-2) @65, 5 @64) case.
- run5: M1 now fails comparison_skips_only_a_write_back_that_restores_its_digits at metered.rs:629.
- run6: zero meter fails amortized floor: "2048 rounds at d = 32768 recorded 0 touches, below the floor of 8192".
- Next: landing check (run7-landing.log), then fetch target/audit-check-logs/board.log from box, diff cells vs run1-parent.log board section.
- run7-landing.log + landing-logs/: tests 763/763 (+1 skipped), snapshots 142, docs 193/3, surface 14, wasm 8/29/43, board 5311 green + exactly 2 count_display drift lines. Landing tip 2be7e55a tree e6f8509e.
- board-diff.txt: 30 moved cells: 24 small-scale touch density falls; 6 = 3 ladder touch exponents 1.00 -> 1.01 (arming-train span_place/precedence/contains), densities unchanged 1.5/B; raw counts not captured (inferred falls).
- Final (message-only amend of S2, tree unchanged): 28b15fd7 S2, 9926cab1 hygiene, both G. DONE; handed back.
- Review fold: efcf780a S2, ec17f661 hygiene (G), comments only vs 9926cab1; run8 fmt/doclint/testdoc clean.
