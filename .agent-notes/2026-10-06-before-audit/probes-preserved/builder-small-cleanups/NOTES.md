# builder-small-cleanups resumption notes

Worktree /Users/oxide/src/rumors-slot-30, branch fix/before-small-cleanups, base 97798357 (verified clean at start).
Brief: /Users/oxide/src/rumors/.agent-notes/2026-10-06-before-audit/coordinator-briefs/small-cleanups.md

## Findings (verified by reading at 97798357)

1. recurse.rs para 2-3: wrong. Claims test-surface recursion "lives" in bridge + witnesses "beside it",
   and that "the guard is what lets [the paper-shaped oracle] meet deep inputs safely". Tree oracle has
   no descend! anywhere (grep); tree.rs "Operating envelope" says unguarded by design. Guarded sites:
   bridge emit_ev/read_id/read_ev, raise/tests.rs rec (deep-tested at 4096), deferred/tests.rs recursive_depth.
   ACTION: restate without enumerating sites.
2. union: DROP as briefed. Whole tree oracle is deliberately unguarded (tree.rs envelope); union's callers
   (party/tests.rs, clock/tests.rs assert_join_all_matches_all_models) feed ARB_DEPTH=4 generators or 4-5 forks,
   and the same inputs go through unguarded oracle join_all/is_disjoint/sum. Guarding union alone contradicts the
   oracle's doc and protects nothing. Crude scan: dozens of unguarded test recursions (scratch selfrec.py).
   Report: AGENTS.md rule ("deliberately recursive test helper routes each descent through descend!") vs
   tree oracle envelope + practice -> owner question. Also bridge emit_id unguarded while siblings guarded;
   bridge guards ineffective end-to-end (from_oracle_* call oracle is_normal first; to_oracle_version calls
   normalized; oracle trees' Drop recursive).
3. clock/tests.rs:640 "recursive Debug pretty-printer": wrong. Debug = debug_struct of Party/Version, each
   `{:#b}` of Bits; Bits Binary = flat while loop (bits/storage.rs). ACTION: restate comment.
4. width.rs span*4: cannot overflow. S = max_depth of a stored stream (unary run read => 1 bit/level);
   widths <= 2^S (disjoint segment sums); balanced digit indices 0..=S/32+1 => span <= S/32+2;
   4*span <= S/8+8 <= stored bytes + 8 <= isize::MAX + 8. ACTION: checked_mul + proof expect. No stop.

## Next
Edit items 1, 3, 4 (one commit each), cargo fmt locally, focused checks + landing check on box.

## Progress
Commits (all G): 5cdbd5fb item1 recurse.rs; 36a42437 item3 clock/tests.rs; 86a3b7fb item4 width.rs checked_mul.
Item 2 dropped (no commit).
Calibration done: mutant checked_mul(2) -> 9/27 measure tests fail (width.rs:261 range end index 8 out of range for slice of length 6); reverted, git diff clean.
Then landing check -> landing.log.
Landing check running at 86a3b7fb -> landing.log
