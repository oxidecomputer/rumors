# builder-trait-coherence: resumption record

Worktree /Users/oxide/src/rumors-slot-31, branch audit/trait-coherence-and-hole-subtracts,
base 97798357 (verified HEAD == base, clean, at start).

Briefs: .agent-notes/2026-10-06-before-audit/lanes/l8-adequacy/round-1/briefs/
  machinery-trait-impl-coherence.md, simplification-hole-subtracts.md

## Established (by reading the tree at base)

- Version `|`/`&`/`|=`/`&=` matrices: already oracle-pinned (version/tests.rs
  join_matrix_matches_oracle etc.). Version cmp matrix: compare_matrix_matches_oracle.
- Version/Party/Clock Debug: pinned by snapshots (testing/snapshots.rs).
- Ranked Hash == Version hash: pinned in ranked_carries_own_rank.
- Version Hash = Bits Hash = <[u8]>::hash(as_bytes()); Party same.
- Gaps to close (commit 1):
  - Count: + (4 cells), += (2), Sum (2); Debug == Display  -> count/tests.rs proptests
  - Rank: same family -> RANK_TRIPLE laws
  - Version/Party Hash == byte hash -> VERSION_SOLO / PARTY_SOLO
  - Ranked Debug contains "Ranked" and version Debug -> VERSION_SOLO
  - Floor/Ceiling Debug == their Query's Debug (add impl docs) -> VERSION_SOLO
  - From<&Query> copy -> VERSION_TRIPLE over neutral/down/up query lists
  - OwnVersion cmp matrix all cells -> extend own_version_cmp_matches_materialized
    and own_version_pair_cmp_matches_materialized with a cmp_cell_agrees helper
  - Span version-receiver cells (V op &S, &V op S) -> extend 3 span laws' point blocks
  - Clock | Version all cells + |= both -> extend anonymous_join_merges_versions
  - size_hint soundness: shape walks (Plateaus/Regions/Overlay/Cells), exactness
    for PartyForks/ClockForks small counts
- Deliberately not tested: suanpan Accumulator/ZeroRanges Debug (no format promised).
- Commit 2: delete Sealed::hole_subtracts (4 sites in causally/polarity.rs).

## Background jobs / artifacts

- Parent readings at 97798357 (tree verified clean on box): scratch readings/
  parent-{acceptance,worst-cases,amp-board}.txt; acceptance = 5311 green / 0 red.
  MISTAKE: these were written to ~/readings-slot-31 on the box (outside my
  worktree dir). Report it; do not delete it.
- Commit-1 code written (laws + count tests + Floor/Ceiling Debug docs);
  clippy fixed (needless_borrows allow on the OwnVersion laws).
- mutants.py / swap.py in scratch: apply|revert by ID; one mutant per law
  group per run (driver stops at first failing law per group).
  Run A: M1 M2 M12 M19 M4 M16 M15 M17 M14 M3
  Run B: M6 M10 M13 M20 M5 M18
  Run C: M8 M11
  Run D: M9

## State after coordinator scope correction (round-2 brief: Hash only)

- d653bb83 (broad laws) -> 735b6b2e (narrowing follow-up; net = hash laws only).
  `git reset --hard` to base was DENIED by the classifier; do not retry.
- 23ad7c9b: hole_subtracts deleted. Calibrated: Down::hole_demand strict->NotBefore
  fails 4 suites; reverted, diff empty.
- Hash calibration: M1+M2 and Bits mutant each fail version/party_hash_is_the_byte_hash.
- Landing check + tip readings running in background (scratch landing.log);
  tip readings in box ~/src/rumors-slot-31/target/tip-readings/.
- Then: scp tip readings, diff vs scratch readings/parent-*, report.

## Next

1. Parent readings at 97798357 on the box (board acceptance, worst-cases, amp-board).
2. Write commit 1 code; mutants; commit.
3. Commit 2; calibration mutant on remaining Sealed method; commit.
4. Tip readings + landing check.

## Done

Landing check clean at 23ad7c9b (logs: landing-logs/). Readings parent==tip (p-*.txt vs t-*.txt, 0 differing lines). Report handed back.

## Round 3 (coordinator: restore four documented-contract laws)

d85c2f3d committed (signed). Calibrated (restore-mut1/2.log). #43 merge-tree clean (tree 7b7540c4); scratch merge commit ebe49774 on no branch; 65/65 pass (compose43.log). Slot back on branch. Landing check running -> landing2.log.

Landing check clean at ef3c364f (landing-logs-3/). d85c2f3d failed doclint (landing2.log); fixed in ef3c364f.

## Round 4 (review repairs)
Commits f6dfc2d7 34432e5c 3af476ec 9700d27e 726ea0da 1ac485f7 (a). (b) skipped per owner ruling. Calibrations: r2-runX.log, r-mut1.log. Verify: r-verify.log. Landing -> landing4.log
