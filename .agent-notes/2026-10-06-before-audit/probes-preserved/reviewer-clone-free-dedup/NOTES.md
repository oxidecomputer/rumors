# reviewer-clone-free-dedup resumption record
Task: round-1 review of simplify/clone-free-dedup-and-moves, /Users/oxide/src/rumors-slot-23, tip ab548daf (verified HEAD, clean at start).
Cap: 4 box runs. Swaps via swap.py (apply|revert NAME); probe copied to crates/before/examples/zz_reviewer_probe.rs (untracked; delete at end).
## Established
- celldiff + own mydiff.py: parent->dedup heap-only falls 62/92/94, no rises, no text; dedup->moves 0.
- non-24 falls only in version_join_all/meet_all (owned `rest`); weave 4.0/1 fall 3400 > 16*24: commit-msg explanation incomplete (owned inputs freed a step earlier, inferred).
- meet never yields Right (Outcome::of verified). extreme/assign_extreme private; Cow From impls predate branch.
- Hypotheses: (A) filter saves log k kernels for [x, d*k]; (B) offset dup pairs: one kernel per dup; (C) lookahead holds one extra lazily-decoded input during combine (regression vs parent?); (D) span Merged/Merged arms clone owned right endpoint.
## Runs
1. tip + adapter-test swap + probe -> run1-tip.log (started)
2. planned: no-filter swap + probe + board capture + before nextest
3. planned: parent 6efb4562 probe (C attribution)
4. planned: span-merged-moves patch probe (D)
- Box stray file: ~/rev-capture-clone-free-dedup.sh in agent home (my scp, outside worktree; not deleted per rules; report it).
## Run log
1. DONE run1-tip.log: adapter exhaustive test PASS, owned_inputs test PASS. Probe VACUOUS (fixture normalized to uniform leaf; my error). Fixed fixture (varied tick counts) in scratch probe.
2. RUNNING run2-nofilter.log: no-filter swap (applied, revert after!) + fixed probe + amp_board capture (target/bed/rev-nofilter-*.cells on box; scp back) + full before nextest.
3. planned: tip + span-merged-moves swap + probe -> tip readings for A,B,C,E-join; patched D, E-span_all.
4. planned: parent emulated by `git diff HEAD 6efb4562 -- crates/before/src | git apply` (revert with apply -R; verify git diff empty) + probe -> C, E attribution.
- Untracked in worktree to delete at end: crates/before/examples/zz_reviewer_probe.rs, zz-rev-capture.sh.
- run3 done (run3-table.tsv). Now: switches reverted (diff empty); to-parent.diff APPLIED (revert: git apply -R to-parent.diff).
- run4 done; to-parent reverted; untracked probe+script removed; worktree verified clean.
## Conclusions (final)
- Q1: keep filter (A: tip const 4172 scan vs filter-free 4172*(1+log2 k); B: filter-free 2x scan). Board blind (tip->nofilter 0 moves, all 3 scales); suite blind (695 pass filter-off).
- BLOCKING: lookahead regression: C-lazy_sum_owned parent->tip peak +126/+650/+2748 at widths 256/1024/4096.
- D/E: span merged moves and balanced_fold merged move are heap-neutral where reached.
- Report delivered via handback.
## Round 2: repair verified (r2-run-repair.log), reverted, worktree clean.
