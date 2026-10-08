# builder-writer-retention: resumption notes

Task: S1 (lane L1 round 1) + O9 (version writers). Worktree
/Users/oxide/src/rumors-slot-33, branch fix/before-writer-retention, base 1550aecb
(verified HEAD == base, tree clean, at start).

## Established
- Mechanism: BitsWriter::finalize -> Bytes::from(Vec) keeps capacity; when
  len != cap, bytes 1.11.1 also allocates a 24-B Shared header. len == cap goes
  through into_boxed_slice (no realloc, no header).
- peak_alloc 0.3.0 realloc = alloc new + copy + free old, so shrink C->L costs
  transient +L at the finalize moment (minus the 24-B header saved).
- O1 (decode slicing: Clock/Span decode share one buffer) is a distinct
  mechanism; the auditor left it unbriefed as a design trade; no owner ruling.
  Task prompt's goal names it as its example. Decision pending (see below).

## Background jobs
- parent board: local log parent-board.log (acceptance + worst-cases at 1550aecb).

## Next
- Read the existing capacity test, meter.rs heap tests, remaining writers.
- After parent board finishes: implement shrink in BitsWriter::finalize.

## Progress (update 1)
- Fix: BitsWriter::finalize shrinks when slack > len (writer.rs); fork.rs comment reworded.
- Tests: party/tests.rs + version/tests.rs "result retention" blocks (retention_excess
  collector, 4 deep families + 2 arbitrary properties). Pass on fix (run-fix-2.log).
- Calibration: swap `if false && ...` -> all 6 fail (calib-noshrink-2.log; unshrunk
  every case fires: calib-noshrink-unshrunk.log). Swap reverted, git diff verified.
- No proptest seed files appeared on the box.
- parent-board.log = parent acceptance+worst-cases. Next: tip-board.log, diff.
- O1 (decode slicing) not yet addressed; decide after movement table.

## Board finding (update 2)
- Variant A (shrink when slack > len): tip-board.log; acceptance FAILS:
  party/clock_forks_full x scatter heap exponent 0.90 -> 1.48 (ceiling 1.15).
  515 heap constants down, 103 up (realloc transient: peak_alloc realloc = new+copy+free).
  Worst-case map flips incl. regression-caused (version_span: concurrent-pair 2.01->2.49).
- Probe (tests/zz_scratch_forks_probe.rs, UNCOMMITTED scratch): cliff at n=2048, 4-byte
  shares keep cap 8 + 24-B Bytes Shared header (slack 4 == len -> no shrink).
- Saved variant A diff: variant-a-slack.patch (excludes untracked probe).
- Next: variant B (slack>0 && slack + header > len), probe + board.

## Variant B (update 3)
- B: shrink when slack>0 && slack + 3 words > len. tipB-board.log: acceptance GREEN,
  854 heap constants down, 192 up; regression-caused flips (version_span concurrent-pair,
  span_*_all scatter). movementB.txt / classB.txt.
- bytes 1.11.1: exact-cap Bytes promotes on first clone (allocates Shared) -> header deferred.
- DECISION: stop rule (A ceiling, B regression flips) -> no commit. Worktree left dirty at
  variant B (patch: variant-b-header.patch; variant A: variant-a-slack.patch).
- Running landing check on uncommitted B: landing-B.log.

## Final state (update 4)
- Landing check on uncommitted B (landing-B.log): tests 765/765 (+6 new), snapshots 142/142,
  docs/surface/wasm ok, board acceptance 5311 green but worst-cases-pin: 13 drifts; lints
  failed on 2 doc summaries -> fixed, lints leg re-run clean (lints-B.log).
- No commits. HEAD 1550aecb. Worktree dirty with variant B; patch variant-b-header.patch.
- Report delivered to coordinator: blocked on owner ruling.
- Committed 64deebe5 (fix+tests), 6e68c5e9 (re-pin), both G. Landing check at tip: landing-tip.log
- Review repairs: rebuilt as 7a6dba39 (fix+tests+repair) and 2dbdec95 (re-pin), both G; tip diff vs 6e68c5e9 == reviewer repair.diff; verify log repair-verify.log (gate-lints, clippy x2, 7 retention tests + 2 matched by filter: pass). Stray file /tmp/x1 (a git diff) created by mistake, not deleted.
