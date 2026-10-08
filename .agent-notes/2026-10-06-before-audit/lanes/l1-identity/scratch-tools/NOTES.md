# auditor-l1 resumption notes (lane L1, identity)

Worktree `/Users/oxide/src/rumors-audit-l1-identity`, branch `explore/l1-identity`,
base `58285ca5`. Scratch: this directory (`S`). Box copy: `~/src/rumors-audit-l1-identity`.
Remote runs: `on-illumos.sh <wt> 'unset CARGO_TARGET_DIR; nice -n 10 cargo nextest run -p before --all-features --locked --build-jobs 24 -j 24 ...'`
(nextest `-j` is test threads; `--build-jobs` caps the build).

## Explore commits

- `03213585` discovery suite `crates/before/tests/audit_l1/` (model.rs, gen.rs,
  history.rs, main.rs).
- `1b1c16e7` + deep.rs, overlay.rs, wide.rs.
- `f3456ae7` left-spine deep inputs; `L1_CASES` env overrides case counts.
- Uncommitted: `tests/audit_l1_retention.rs` (heap-retention probe, PeakAlloc).

## The suite (black-box, public API only)

`model.rs`: party as sorted disjoint dyadic intervals (u128 at 2^-126). Own
encoder/decoder of canonical bytes (decoder asserts canonical); `fork` = hull
characterization; `shares(n)` = ceil-left/floor-right recursion; `regions()`;
`same_multiset` = pointwise multiplicity equality.
`gen.rs`: arbitrary parties = random ownership of pieces between random dyadic
cut points (incl. clustered points), depth to 80; `arb_disjoint(k)` colors pieces.
Distribution measured (`stats-1.log`): ~50% depth > 32, many have >= 32 two-child
branches, unary chains to 64+; arbitrary pairs 15% disjoint, 50% nested.
Properties: set algebra (is_disjoint, covers, without, join, shape) on arbitrary /
disjoint / nested pairs; fork; sync (arbitrary + disjoint; untouched on Err);
forks partial drain + exact size_hint; array split N in {1..9,13,16,17} (Party,
Clock); Clock::forks; join_all success; join_all failure multiplicity
conservation; Clock::join_all regions+versions conservation; sync_all vs model
and unchanged on Err; rule-respecting histories (fork, forks+take, split, join,
join_all, sync, sync_all, encode/decode moves) checked vs model every step;
overlay = refinement of regions by version plateaus; wide counts (to 2^100,
around usize::MAX) exact shares + sound hints; deep probe (depth 100k, 256 KiB
stack, both spines, combs, zigzag).

## Established

- Suite passes at default counts and at 100k cases/property (`long-1.log`;
  forks_wide_counts timed out at 100k by model cost, passes at 10k in 73 s,
  `long-2.log`).
- Calibration (`calibration.md`, `calib/*.log`, driver `calibrate.py`, lists
  `muts1..4.py`): 21 mutants caught by the suite; git diff clean after each run.
- Existing suite vs mutants: M19/M20 (join_all error path returns the rejected
  group twice; Party and Clock) PASS all 610 lib tests, caught only by my
  multiplicity properties (`calib-run-3.log`). M26 (recursive removal descent
  in forks) passes existing suite, caught only by deep probe (`calib-run-6.log`).
  M25 (tail-recursive region descent) passes EVERYTHING at the workspace's
  `profile.dev.package.before.opt-level = 2`; caught by deep probe only at
  opt-level 0 (`run-m25-opt0.log`). Unmutated tree passes all 34 deep tests at
  opt-level 0 (`run-opt0-deep.log`, target dir `target/opt0` on the box).
- Refuted: existing union checks are NOT blind to dropping regions (M08, M11,
  M18, M21 caught by existing tests, `calib-run-2.log`, `calib-run-3.log`).
- Retention (observation, 4 sizes, `retention-2.log`, `retention-6.log`):
  1-byte results of join / without / Clock::join / forks(1) residual retain heap
  ~ input size (75 KB at d=100k); fork control retains 1 B. Cause: BitsWriter
  finalize keeps Vec capacity; join/without reserve |a|+|b|, removal sizes the
  residual like the share. Party from Clock::decode retains the whole clock
  buffer after the version is dropped (Party::decode_prefix slices the buffer).
- D1 (low): Distant fork plans never report an exact size_hint; width-dependent
  (`d1-repro.log`, explore commit `1a5f5f8b`, brief `briefs/D1-...`).
- Committed generators measured (`stats-committed.log`): depth <= 4, <= 4 bytes.
- Cost probe (`cost-1.log`, `tests/audit_l1_cost.rs`): all ratios flat.
- Later explore commits: `8074f098` retention probe, `1a5f5f8b` D1 probe,
  `c54c195c` committed-generator stats, `26cbae22` cost probe.

## State at end of round 1

No background jobs running. Deliverables in `briefs/`, `observations.md`,
`inventory.md`, `coverage.md`, `calibration.md`. Report returned. If resumed:
merge any fix branch for D1 into explore and rerun the suite; next lines of
work are listed at the end of `coverage.md`.

## MB2 sizing (resumed task)

- Merged `audit/deep-surfaces` (#34) into explore (`0a61a4cd`); sizing drafts
  `fa7cfe0f` (siblings and without_constructed at 2^18, Part B
  `deep_identity_stack_safety`, Part C `deep_tree_remaining_surfaces_stack_safety`).
- One sizing run (`sizing-1.log`): 11 deep tests pass at 2^18; Part B 2.14 s,
  Part C 4.32 s, siblings 0.72/1.61/0.16 s; build 2m15s at load 206.
- Revised brief committed on explore: `.agent-notes/2026-10-06-before-audit/lanes/l1-identity/round-1/MB2-deep-identity-probe.md`
  (commits `4551c52c`, then the narrowing commit). Coordinator copies it to main.
- Corrections found: round-1 MB2's "686 of 686" was 666 committed + 20 explore
  tests (M26 failed only the explore probe); SB1 cites `Clock::join_all` at
  `src/clock.rs:270-288`, actually `:292`.
