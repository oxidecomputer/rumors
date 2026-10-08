# reviewer-deep-surfaces: resumption notes

Task: review audit/deep-surfaces (3d14c9fe, base d5e80103), round 1. Commit nothing.
Worktree /Users/oxide/src/rumors-slot-01 (verified HEAD 3d14c9fe, clean, at start).
Brief: .agent-notes/2026-10-06-before-audit/lanes/l2-algebra/round-1/machinery-deep-surfaces.md
Builder evidence: scratchpad/builder-deep-surfaces/

## Plan
1. just gate on clean tree first (background) -> gate.log. No wrapper calls (they sync!) while it runs.
2. Read code: shape.rs, lattice.rs, clock/tests.rs deep tests, coverage.rs row.
3. After gate: M1, M2 (builder's), plus own M3 (Cells) / M4 (Overlay or Regions). Restore, git diff empty.
4. Frame-size/stack argument check.

## Established
- Gate started ~15:49 (box clock) on clean 3d14c9fe, log $S/gate.log (bg task bbku6y3je). No wrapper calls until done.
- Verified in 1.97.1 rust-src on box: libtest spawns each test via thread::Builder::new().name(..) (no stack_size);
  std uses RUST_MIN_STACK else DEFAULT_MIN_STACK_SIZE = 2 MiB (sys/thread/unix.rs:26). Nothing in tree/box env sets RUST_MIN_STACK.
- probe.rs/probe2.rs (scratch; logs probe.log, probe2.log), rustc 1.97.1 -C opt-level=2 on illumos:
  * illumos x86_64 keeps frame pointers: a walker with one value live across the call = 32 B/level
    (ret, rbp, rbx, pad) -> overflows at 100k already.
  * `1 + acc(n-1)` (source non-tail) is compiled to a loop: survives depth 1e8. => test doc's
    "a walk that makes one non-tail recursive call per level overflows here" is inexact (prose finding).
  * could not build a bare 16 B frame on illumos (TLS spills regs); 16 B min stays an ABI argument.
- Sibling doc + coverage row: "deep_tree_* ... is the proof [every library walk is iterative] at depth" overclaims:
  party without_constructed (party/tests.rs SCALES incl 100_000) pins `without` at depth outside deep_tree_*;
  builder's own notes list un-driven walks (Party::without/covers, Clock::absorb, Span::intersect) - check.
- Sibling doc line 552 now ~130 cols (was already long) - prose nit.
- GATE (mine): workspace FAILED at 302s: 1862 passed, 2 TIMEOUT (>180s) in rumors, untouched by branch:
  rumors tree::mirror::streaming::remote::codec::tests::bounded_corpus_manifest_snapshot,
  rumors::dispute_wire table_corpus_has_similar_protocol_overhead. Box load avg 411-481 at the time.
  Copy: $S/gate-workspace.log. Builder's workspace leg: 1864/1864 passed (10 slow).
  TODO after gate: isolated run of those two tests (diagnostic) -> isolated.log.
- covers: Comparison::holds shared with is_disjoint (driven deep). encode_rank = rank().encode() (driven). So
  un-driven-at-depth entry points: Party::covers (own entry only), Party::without driven in party tests at 100k (outside deep_tree_*).
- !! LOCAL TREE DIRTY: shape.rs carries reviewer mutants (env REVIEW_MUTANT=cells|cells-tail|regions|overlay).
  Restore: cp $S/mut/shape.rs.orig /Users/oxide/src/rumors-slot-01/crates/before/src/shape.rs ; then git diff must be empty.
  Then sync a clean tree (any wrapper call) so the remote copy is restored too.
- Board leg FAILED at 452s (expected per baseline; verify 5311/0 + two drift lines). Wasm leg pending.
- shape.rs RESTORED locally (clean). Gate 1 final: FAILED after 463s: board fuzz workspace. board 5311/0 + exact 2 drift lines;
  fuzz libfuzzer #error; wasm 25/43/8 ok; workspace 1862/1864, 2 rumors TIMEOUTs (load 411-514).
- Coordinator: every remote cmd starts `unset CARGO_TARGET_DIR; export NEXTEST_TEST_THREADS=24;`; rerun once under cap.
- Gate 2 (rerun under cap) started -> $S/gate2.log (bg bce7o4kxa). No wrapper calls until done.
- Next: re-apply mutants (python3 -I $S/mut/apply_shape.py <shape.rs>), build before tests --no-run, run new test with
  REVIEW_MUTANT=cells|cells-tail|regions|overlay (ulimit -c 0), then full before suite per mutant; also M1, M2 (builder's swap.py).
- Coordinator: gates now via 'pset-run -n 24 -- just gate'. Gate2 started before that; if it times out, rerun with pset-run.
- Gate2: FAILED after 771s: board fuzz workspace; same 2 rumors TIMEOUTs (180.758s, 180.630s), 1862 passed. Gate3 via pset-run -> gate3.log
- gate3 pset-run failed to start (psrset Device busy, exit 125, no set left). Doing mutants now; gate retry last. shape.rs MUTATED again.
- Gates now via ~/bin/audit-reserved just gate (never pset-run). Mutants done: see mut-mine.log
- M1/M2 rerun: each aborts only new test (693/694). Tree restored. Gate4 via audit-reserved -> gate4.log
- Shape mutants results (mut-mine.log): cells -> new test SIGABRT + 5 amp_board_smoke SIGABRT (board_runs_to_completion overflowed);
  cells-tail -> 694/694 pass (TCO'd loop); regions -> only new test (693/694); overlay -> only new test (693/694).
- Gate4 queued in audit-reserved (~11 waiters). Waiter bg task bz7t4hlf9. Then: write report.
- Findings draft: B1 test doc "non-tail recursive call" wording (acc probe); B2 sibling doc + coverage row overclaim
  ("every library walk ... proof" ; Party::covers entry, without pinned in party/tests.rs); nits: line 552 133 cols.
- Gate4 queued process killed (never started) per coordinator; landing check audit-check -> check.log
- Landing check (audit-check, reserved): FAILED after 1130s: board only; board 5311/0 + exact 2 drift lines; tests 758/758 + 142/142. Tree clean (DIFF-EMPTY) at 3d14c9fe. DONE: report sent.

## ROUND 2 (tip 20da73ad)
- B1/B2 prose read in diff 3d14c9fe..20da73ad. M3 read: fair (loop left runs, non-tail recursion per right subtree; passed 694 at 3d14c9fe).
- TREE DIRTY: M3 applied + probe test appended to clock/tests.rs. Restore: cp $S/mut/clock-tests.rs.r2orig and shape.rs.r2orig; git diff empty.
- Then landing check (fixed audit-check) -> r2-check.log
- R2 M3+probe done (r2-m3.log): probe PASS, new test only SIGABRT 694/695. Tree restored DIFF-EMPTY. Landing check -> r2-check.log
- R2 check: FAILED board tests; tests = 758/758 before; rumors 141/142, bounded_corpus_manifest_snapshot TIMEOUT 180.070s. Diagnostic single-test reserved run -> r2-corpus.log
- R2: corpus test isolated on reserved cores PASS 93.683s at tip. Verdict approve. DONE.
