# Surveyor resumption record

Task: instrument survey (brief `coordinator-briefs/instrument-survey.md`).
Deliverable: `surveyor/instrument-survey.md`. No commits. Box runs <= 4, on
`/Users/oxide/src/rumors-survey` (not yet created). Base: main @ 599bf7d3c.

Governing rule: eligible only with a named constructed/observed failure the
committed instruments miss; otherwise listed.

## Established
- Read common.md, auditor.md, brief, instruments.md, STATE.md final phase,
  README, baseline, before/AGENTS.md.

## Box runs used: 1 / 4
- Run 1 (background bkd16mov0, log surveyor/run1-fuel.log): worktree
  /Users/oxide/src/rumors-survey (main 599bf7d3c + scaffold.diff: compile-time
  SURVEY_MUTANT switches at packed_u64.rs:63, writer.rs:363, words.rs:78 x2;
  untracked survey-fuel.sh). Variants base/packed/splice/words_none/words_lt,
  each: guest+harness rebuild, fuzzfit nextest. Box logs in
  ~/src/rumors-survey/target/survey/. Purpose: do fuel bands catch the 5
  "invisible" C survivors? MUST revert scaffold (git diff empty) before
  `git worktree remove`.

## Classifications so far (verified unless noted)
- worst-case pin skipped when acceptance red: ELIGIBLE (observed: min_ticks
  demonstrator NOTES: landing check showed only expected red; separate pin run
  showed 64 drift lines incl 48 touch flips on 24 ops).
- bounded_corpus_manifest_snapshot: ELIGIBLE (2 TIMEOUTs at 180.5s:
  fixer-zero-shift/landing-logs/tests.log, coordinator/recheck/rumors-slot-02.log;
  passes 71-123 s). rumors test; nextest.toml doc claim false (S25 report).
- PeakAlloc: ELIGIBLE (4729 vs 4609 twice: builder-join-all-multiplicity gate-1,
  builder-rank-decode-reader landing/landing1). Note: L8's calibration
  (background allocating thread) only fits per-thread option, not
  RUST_TEST_THREADS=1.
- STOPPING_DIFF_BAND: dispute L8: band catches what msg names (width read >=3
  touches/hop vs slack 1.92/hop); missed rise was 1 touch/hop, debug-only
  (fdd1bf47 cmp_zero clone). Eligible-low or listed; #38 removes cause.
- num-bigint: ELIGIBLE, Q87 dep. Reviewer logs run1-branch (125751..8006001
  limbs) vs run2-parent (251..2001; different hook site). Smallest change:
  wasm-fuel ladder family (extends #52 machinery) sees num-bigint without hooks.
- Ceiling rule check: ELIGIBLE, Q65 dep (COMB 2.0->2.98 unnoticed; FOLD/QUERY
  docs don't reproduce values).
- Delta-heap compare: eligible? observed false stop triggers (W 314 fits; min_ticks
  records 5 fits), comparisons are ad hoc agent scripts on rounded output. No
  committed comparison tool. Board children already emit exact samples.
- fuzzfit bands drift: refit vs committed: ceilings up to +0.066 log10
  (ff_version_min_ticks), 8/50 >0.01; within ENFORCE_MARGIN 0.2. LISTED.
- Fold mutant vs board: L8 inferred by arithmetic (134 vs 17); never run.

## Next
- T1 wasm32 pins 4 missed narrowings vs ready branches
- L1 M19/M20/M26 vs #40/#81/#75; L3 M6/M8/M15/M19 vs #74
- S4 harmonic, S5 touch-bound read-only, S6, S8, S10, S21
- explore branches L1..L8
- verification map

## Progress 2
- Run 1 interim: base 25/25 pass (scaffold neutral); packed: guest sha changed,
  25/25 pass => fuel bands MISS packed_u64.rs:63. splice/words pending.
- Run 2 planned (after run 1): RUSTDOCFLAGS=-D warnings cargo doc --locked
  --workspace --no-deps (public, then --document-private-items) in
  crates/before/wasm32-pins and crates/before/surfacecheck; separate target dirs.
- T1: compare cursor narrowing (2 injections) missed at main + all ready branches
  (no branch touches the compare/join pin docs). Pin runtime 37-196 s. #57 sets
  20-min limit for wasm32-pins. Join-live wrap: value-neutral => doc fix only.
- L1 mutants: M19/M20 by #40+#81; M26 by amp_board_smoke at base (#75 reviewer).
- L3: #74 catches M6, M8, M15, M21; M19 caught round 1 (index OOB), not re-run after
  round-2 palette change; #74's index text names it.
- MB-2 (min_ticks histories): production-only +/-1 mutants also killed by committed
  differentials => LISTED (cuts dispatch queue item 1).
- harmonic: no record found; reconstructed from demonstrator-min-ticks NOTES
  ("rank-fold four (x1.28-1.32)" touch flips to jump-rising-spine); family lives
  only on stopped D1 demo.
- #61 neutral/hole-free coverage heap: no board row; #61's board heap identical.
- Validation index at main lacks: wasm32-pins (#31 adds), zero-range ladder (#52/#86
  add), worst-case pin, census floors, stack-safety tests, PeakAlloc heap checks,
  auto_traits, fuelscape_islands test, everything suanpan. testing.rs module doc does
  NOT index scaffolding though validation_index says it does. suanpan lib.rs: no
  Testing section; before lib.rs "## Testing" omits resource + 32-bit instruments.
- Ceilings at main: COMB doc "flat 2.0 B/B" + "split skyline builder" ghost; QUERY
  152 vs rule. Reviewer readings in reviewer-ceiling-rules/NOTES.md.
- Q87 and Q65 open. Owner ruled no input-size promise (#53) => 32-bit cost listed.

## Progress 3
- Run 1 DONE (exit 0): base 25/25; packed 25/25 (MISSED); splice 22/25, 3 fail
  ABOVE BAND ff_clock_fork (CAUGHT); words_none 25/25 (MISSED); words_lt 25/25
  (MISSED). Guest hashes all distinct. Scaffold reverted, git diff empty.
- Box runs used: 2/4. Run 2 (bg b3bf8w8oe, log run2-docs.log): survey-docs.sh in
  worktree root (untracked). Delete it before `git worktree remove`.
- Snapshot timeouts: >= 11 distinct runs timed out (180.0-181.5 s); passes 65.8-169.7 s.
- Draft eligible section: draft-eligible.md (needs: re-rank, WORDS_RESULT fill,
  S3 docs entry from run 2).
- harmonic/jump-rising-spine -> LISTED (no missed failure; reach only).
- threshold rule -> LISTED (failure on rejected design; owner already ruled the
  principle for min_ticks).

## DONE
- Run 2 DONE: wasm32-pins + surfacecheck rustdoc public/private, -D warnings: all pass,
  no warnings => detached-docs candidate LISTED.
- instrument-survey.md written (10 eligible, 37 listed). Drafts removed.
- Worktree /Users/oxide/src/rumors-survey removed (plain git worktree remove; clean).
- Box dir ~/src/rumors-survey (with target/) left for coordinator retirement.
