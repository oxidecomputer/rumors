# auditor-l8 resumption record

Lane L8 (adequacy of the instruments). Worktree
/Users/oxide/src/rumors-audit-l8-adequacy, branch explore/l8-adequacy,
base 58285ca5. Scratch: this directory (S below). On the box the worktree
syncs to ~/src/rumors-audit-l8-adequacy (R below).

## Rules specific to this run (beyond the briefs)

- Never sync a change under `crates/` while a mutation group is queued: each
  group copies R's source tree when it starts. Probes and injections go in
  the detached copy `l8/probe/` (rebuilt by `l8/mkprobe.sh`, which also
  inserts the `l8_probe!` recorder from `l8/probe-*.rs.inc`).
- nextest's `-j` means `--test-threads`; use `--build-jobs` for builds.
- My share is 24 jobs. Killing processes is denied to me; the coordinator
  stops process A itself (see below).
- Coverage ran once on the Mac by owner exception; do not rerun it.

## Background jobs (box)

- Process A: bash pid 17176 running target/l8/campaign.sh, cargo-mutants pid
  17180; log R/target/l8/campaign.log; outputs R/target/l8/campaign-<group>;
  queue version, bits, party, rank, span, suanpan, rest; 6 jobs, 12 build
  tokens, 2 test threads. Started 14:13Z. The coordinator stops it at
  "=== group party start".
- Process B: bash pid 13652 running target/l8/campaign2.sh; log
  R/target/l8/campaign2.log; outputs R/target/l8/campaign2-<group>; queue
  suanpan, rest, span, rank, party; 4 jobs, 6 tokens, 2 test threads.
  Started 14:27Z.
- Both: `l8/mutants-common.sh` (PROPTEST_RNG_SEED=8008, no debuginfo,
  TMPDIR=R/target/l8/tmp, --timeout 600, full before or suanpan suite,
  excludes src/testing/ and test files).
- Throughput measured ~17 s/mutant for A under load 50-190.
- Split survivors locally: `S/split-survivors.sh '<remote mutants.out>' <label>`.

## Established

1. Mutant inventory: 3919 (S/mutants-list.txt). Pilot on count.rs (full
   suite): 3 missed (Debug, Add<&Count> for Count, Sum<&Count>);
   survivors/before__count.txt. Pilot2 on ranked.rs (lib tests only): Debug
   missed; rerun in the rank group for the full-suite verdict.
2. count_display x heap pin drift: findings/count-display-heap.md. Probe
   l8/nbprobe (stock vs FAST_DIV_WIDE=false num-bigint) reproduces the box's
   board readings and shows both platform dependence and a 0.1% near-tie on
   the non-x86 setting. Table: S/nbprobe-compare.txt.
3. Branch coverage (Mac, approved): coverage/NOTES.md, coverage/SUMMARY.txt,
   per-module files. 97.8% lines, 95.4% branch outcomes. Notable: Rank::decode
   reader rejection paths never run; hole_subtracts is dead; Rank::accumulate
   runs only in wasm32 pins; trait-impl spellings unexecuted.
4. Generator census (S/census.log, l8_census.rs on the explore branch):
   arbitrary operands shallow and small; independent version pairs never
   Equal; ~65% of trace joins/syncs skipped, 64% of sends self-sends;
   organic picks same clock 55%.
5. Early campaign survivors (version, suanpan): Hash for Version -> ()
   (Eq/Hash law is vacuous for a constant hash); span_all fold arm
   (unreachable by design); suanpan reserve/reserve_digits (cost only),
   Debug impls, conversions.rs:46 `| -> ^` in primitive_parts (TO CHECK).

## Open hypotheses

- Trait-impl surface (operators, Debug, Hash, From) lacks a semantic
  instrument; candidate machinery brief: a "spellings agree" law group.
- Trace generator degeneracy loses interaction coverage; candidate machinery
  brief with a calibration experiment (inject a defect needing a
  non-sibling interaction; compare detection rates).
- Rank::decode chunk-boundary paths: candidate machinery brief.
- hole_subtracts: simplification brief (delete dead trait method).

## Also established (since the first draft)

6. Suanpan group done: 355 caught, 30 missed, 26 unviable; classified in
   survivors/INDEX.md. Normalize witnesses verified reachable through public
   ops (briefs/machinery-suanpan-normalize-bound.md, probe
   l8/l8_normalize_probe.rs).
7. Surface check calibrated 2/2; fold pins floor-only (fold mutant measured,
   14x scan bits, pins pass): findings/instrument-calibrations.md.
8. Detached workspaces lack a nextest config; verified a 200 s test is never
   terminated in wasm32-pins: briefs/machinery-detached-nextest-timeouts.md.
9. Census made reusable (eec46dbb): census functions over any iterator;
   baseline output S/census-baseline.txt.
10. New contract clause (usize invariance): report a line on whether the
    instruments can detect width-dependent behavior (pins: lengths/counts
    yes, positions no; meters only run on 64-bit).

## In progress / next

- DONE wasm32 pin calibration: findings/wasm32-pins-calibration.md (driver
  S/w32/run-injections.sh, table S/w32/injections.py where INJ_ALL holds the
  eleven injections and INJ the trap probes; verdicts S/w32/verdicts.txt).
- DONE briefs: briefs/simplification-hole-subtracts.md,
  briefs/machinery-rank-decode-reader.md.
- TODO suanpan survivors: normalize.rs:74-83 (loop body never runs per
  coverage), read.rs:90/99 drain loops (touch counts; prior review flagged),
  operators.rs:382 `|| -> &&` in shift_left, read.rs:115 (likely equivalent).
- TODO quantify worst-case pin fragility (runner-up margins) with
  `amp_board -- worst-cases` on the box (AMP_BOARD_SHARDS=8).
- Split survivors per group as they finish; analyze each for unreached vs
  reached-unchecked (probe tree), cluster, and write briefs.

## Round 1 ended (report returned ~15:40Z)

On resumption: re-read the briefs and this file; check both campaign logs;
split finished groups with split-survivors.sh; classify into
survivors/INDEX.md; extend briefs/machinery-trait-impl-coherence.md with
new class-T survivors; look for class-G survivors in bits, party, clock,
span, causally, rank (each needs a reachability probe in l8/probe and a
witness before briefing). Process A must be stopped by the coordinator at
"=== group party start".
