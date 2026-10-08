# reviewer-deep-identity: resumption record

Task: round-1 review of audit/deep-identity-probe in /Users/oxide/src/rumors-slot-32
(tip aad3577b, base 20da73ad, #34). Verified at start: HEAD aad3577b, clean, three
commits signed (G). Cap: three box runs, no landing check, no commits; leave slot clean.

## Mutants (mutate.py here; apply backs up to ./orig, revert copies back)
All mutants compiled into one build behind crate::reviewer_mutant(NAME), read from
REVIEW_MUTANT env once. NONE = control. M26, M26S (=builder's M26-stored), MCPD
(=builder's MC-place-drop), PAMT (new: min_ticks leaf fold as recursion with a
16-byte shell frame, fold state passed back through fold_step's return).
`apply depth100k` additionally sets STACK_SAFETY_DEPTH = 100_000.

## Runs
1. [launched] run1.log: tip + switched mutants, whole `-p before --all-features` per mutant,
   plus disassembly of the recursive shells. Worktree currently MUTATED; revert after run 1
   finishes (or before run 2 re-apply with depth100k).
2. [planned] depth100k + PAMT whole suite (expect all pass), M26S/MCPD deep tests at 100k.

## Run 1 results (run1.log, tip, all 12 before binaries, 696 tests)
- NONE: 696 pass. M26: deep_identity + 5 amp_board_smoke abort (builder confirmed).
- M26S: ONLY deep_identity_stack_safety aborts. MCPD: ONLY deep_tree_remaining_surfaces aborts.
- PAMT: ONLY deep_tree_min_ticks_stack_safety aborts (Part A test).
- illumos x86_64 keeps frame pointers (push rbp; mov rbp,rsp). M26S frame 32 B; M26 frame 96 B.
- Reverted after run 1, git diff clean; re-applied with depth100k for run 2 (worktree MUTATED).
## Run 2 DONE: PAMT 696/696 pass at 100k; fold_rest frame = 16 B (push rbp; call; ret).
  M26S and MCPD also caught at 100k (frames >= 32 B). Reverted; diff clean; files touched for mtime.
  REPORT SENT next. Box copy of slot still holds mutated source until next sync.
## Run 2 (was) run2.log: depth 100k, PAMT whole suite, M26S/MCPD stack_safety tests, fold_rest disasm.
## Composition: merge-tree aad3577b vs 4ecfbcfd: one conflict line, clock/tests.rs Debug comment
  (branch "`Debug` renders both deep encodings." vs cleanups "Formatting a deep clock with `Debug` must not overflow either."). main: clean.

## Established
- Box: no RUST_MIN_STACK/RUSTFLAGS in env, wrapper, audit-check, audit-reserved; ulimit -s 10240.
- before builds opt-level 2 in dev (Cargo.toml profile.dev.package.before).
- Deep tests run only natively (CI ubuntu-latest x86_64, illumos box, Mac aarch64);
  wasm32-pins guest drives no deep trees.
