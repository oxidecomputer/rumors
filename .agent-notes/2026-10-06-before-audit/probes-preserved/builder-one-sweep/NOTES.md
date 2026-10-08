# builder-one-sweep resumption notes

Task: simplification brief `.agent-notes/2026-10-06-before-audit/lanes/l2-algebra/round-1/simplification-lattice-one-sweep.md`.
Worktree /Users/oxide/src/rumors-slot-03, branch simplify/lattice-one-sweep, base d5e80103 (verified HEAD, clean).
Touch only crates/before/src/version/lattice.rs (another builder edits version.rs).

## Established
- Board meters: heap/scan/touch only; OrderState::fold touches none. Wasm fuel (fuzzfit bands) is the only
  counter that can see the extra fold in emit; bands are log fits with 0.2 slack, so gate won't flag it.
- Readings check: dump-readings.sh (scp'd to box at ~/src/rumors-slot-03/target/dump-readings.sh) dumps
  sorted `cell` lines at scales 0.01/1.0/4.0 into target/readings-<label>/.

## Background jobs
(none yet)

## Next
1. base readings dump (label base) -> 2. implement sweep<N> -> 3. change readings dump, diff
4. calibration: inject defect into new sweep, run lattice tests -> 5. fuel measure? -> 6. gate, commit

## Progress (update 1)
- Base board readings dumped on box: ~/src/rumors-slot-03/target/readings-base/{3f847ae147ae147b,3ff0000000000000,4010000000000000}.cells (5311 cells each). Log: dump-base.log.
- lattice.rs restructured locally (uncommitted): sweep<const N>, Emission at module level, emit/hull_bits delegate;
  module doc restated; panics clause restated (join has no # Panics section; old text cited it).
- Running: fuzzfit calibrate at BASE on box (ssh, no sync) -> box target/bands-base.rs; log calibrate-base.log.
  Box tree must NOT be synced until this finishes (it is at base).
## Next
- after calibrate-base: sync change, dump readings (label change), diff; calibrate at change -> bands-change.rs; diff.
- calibration (defect injection into sweep), focused tests, clippy, gate, commit.

## Progress (update 2)
- calibrate at base done: scratch bands-base.rs (box target/bands-base.rs). Committed bands.rs already differs
  from a fresh base calibrate (380 diff lines) -> pre-existing drift, report as observation; compare base-run vs change-run.
- Change synced to box (box tree now = change). clippy -p before all-targets all-features clean (clippy-1.log).
- Running (bk7bm3ut7): focused nextest (focused-1.log) then dump-readings change (dump-change.log).
## Next
- diff readings-base vs readings-change on box; calibrate at change -> bands-change.rs, diff vs bands-base.rs
- defect injections (3): drop loop directions.fold; pass old_side as next (never switch); opening depth min.

## Progress (update 3)
- Board readings: base vs change byte-identical at all 3 scales (5311 cells; 1338 lattice-reaching each). dump-change rebuilt before (verified "Compiling before").
- focused-2: 68 passed incl. all version::lattice::tests (log focused-2.log).
- calibrate-change (bands-change.rs) is VOID: guest wasm not rebuilt (mtime trap: lattice.rs local mtime 14:53:41 older
  than base guest build 14:54:39). Must touch lattice.rs and rerun calibrate at change, then diff vs bands-base.rs.
## Next
- defect injections (3) -> restore (git diff vs committed-intended state) -> touch lattice.rs -> calibrate change -> gate -> commit.

## Progress (update 4)
- Committed 85220545 (signed) on simplify/lattice-one-sweep.
- Injections done (logs inject-{relation,noswitch,depth}.log): 6/11, 9/11, 10/11 lattice tests fail; all restored, git diff empty.
- Running bdur0r04y: calibrate at change with touched lattice.rs -> bands-change2.rs; diff vs bands-base.rs (log calibrate-change2.log).
## Next
- verify guest rebuilt in calibrate-change2.log; then just gate (background) -> compare with baseline.md; optional l2 probe run.

## Progress (update 5)
- calibrate-change2 (guest rebuilt, commit 85220545): fuel ROSE vs base: version_join ~+0.9% @128 bits to +0.2% @15k; meet similar;
  also clock_join/recv/send/sync, join_all/meet_all; tick bands moved tiny both ways. => measurable constant -> remedy.
- Remedy (uncommitted): SignFold trait (OrderState folds, () ignores); sweep<N, S: SignFold>(.., signs: S) -> ([Version;N], S).
  Deviation from brief's const TRACK_ORDER bool: avoids meaningless Option<Ordering> return for emit.
- Running bapi19sw2: clippy-2.log, calibrate-change3.log -> bands-change3.rs diff vs bands-base.rs.
## Next
- if fuel == base: rerun board dump (change3), focused tests, commit (amend? no: new commit or fold into one), gate.

## Progress (update 6)
- SignFold variant (signfold-experiment.patch in scratch) did NOT recover fuel (slightly worse); backed out, tree == 85220545.
- Fuel rise of 85220545 vs base (fitted lines): join +0.88%..+0.23%, meet +0.88..+0.29, join_all +0.98..+0.47,
  meet_all +1.15..+0.71, clock_join +0.60..+0.16, clock_sync +0.73..+0.21; clock_send/tick ~-0.25% (codegen shift).
- DECISION: stop per builder.md (raised deterministic counter) and report; run gate on 85220545 for the coordinator.
- Running: gate (gate.log).

## Progress (update 7)
- gate #1 (gate.log, unreserved cores, load ~500): audit/internal-docs/surface/docsrs/doctest/wasm ok; board = baseline (5311 green, 2 drift lines);
  fuzz = baseline libfuzzer; workspace: 1862 passed, 1 TIMEOUT rumors::dispute_wire table_corpus_has_similar_protocol_overhead.
  Isolated rerun also timed out (rerun-timeout.log, load 364).
- Coordinator: gates must run `pset-run -n 24 -- just gate` with NEXTEST_TEST_THREADS=24. Running gate #2 (gate2.log).

## Progress (update 8) -- FINAL
- Reserved-core gate: pset-run failed 6 times (gate2.log, gate3.log, gate3-attempts.log): "psrset: cannot assign processor N: Device busy".
  No leftover pset of mine (only set 1 exists). Tree clean at 85220545 (signed).
- Reporting: stopped on fuel rise (brief remedy tried, refuted); gate under reserved cores blocked by pset contention.
## update 9: running gate via ~/bin/audit-reserved (gate4.log)
## update 10 FINAL: gate4 (reserved) matches baseline; amended message -> 7681765965c8691dcb9f9d98b05475dfbadf03a1 (signed, tree == 85220545)
## update 11: amended to cdba4b5f (doc sentence + fuel paragraph); landing check -> landing-check.log
