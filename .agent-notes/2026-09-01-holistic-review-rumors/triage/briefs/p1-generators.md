<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T161 in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# P1 lane: no proptest strategy rejects

## Goal

A proptest that rejects draws has a hidden ceiling on its case count:
proptest's rejection budgets (1024 global, 65536 local) are totals per
run and do not scale with `PROPTEST_CASES`, so a property that rejects a
constant fraction of draws aborts with "Too many global rejects" at some
count, and a CI job that raises the count silently loses the suite. The
invariant this lane lands (T161): every strategy in `rumors` is total,
so nothing rejects, and the gate proves it at runtime by running every
property with both budgets at zero. After this lane the release CI job's
count is bounded by wall time alone.

## Ground rules

- **Base.** Your worktree's HEAD must equal `<base sha>` before you
  start (the coordinator fills it: the `triage/p1-proptest-ci` tip after
  its repair round). Run `git -C <worktree> rev-parse HEAD`. If HEAD is
  an ancestor of `<base sha>`, fast-forward; if it has diverged, stop
  and report. Never call EnterWorktree; operate on the worktree through
  `git -C <path>` and absolute paths, one shell invocation at a time.
- **The ruling is the specification.** T161 is quoted below; where this
  brief and it disagree, the ruling wins. Sites are derived from the
  census grep in the Mechanism, never from line numbers.
- **Goal beside mechanism.** Where the ruling's mechanism and the goal
  above come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the item open; do not work around: a
  rewrite that changes what a property asserts (a generator must produce
  the same population the filtered one did, or a superset that the
  property still holds on; say which at each site); anything that moves
  an `insta` snapshot or a committed seed's replay (a committed seed must
  still replay after its strategy changes, or the change is a stop);
  any change to a public signature; anything that contradicts a ruling
  in `triage/rulings.md`.
- **Negative controls.** The check carries a committed `--self-test`
  case per spelling that a known-bad fixture fails; the runtime tripwire
  is demonstrated once by a reversible mutation (one `prop_assume!`
  restored at a site) whose gate failure line the commit message quotes
  verbatim.
- **Where you build.** The illumos box (`ox-east-1`, per the
  `building-on-illumos` skill; the wrapper syncs a real checkout) is
  where this lane builds, tests, and gates; the Mac runs no gate. Every
  remote command exports `CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`
  and runs unbound in the general pool; never `pset-run`; `--locked` on
  every cargo call. The gate of record is `on-illumos.sh <worktree>
  'export CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32; just gate'`, one
  run per commit series, redirected to a log under
  `<scratchpad>/p1-generators/` and polled with short foreground checks.
  `fuzz` is expected red there (libFuzzer has no illumos port) and
  counts as clean when it is the only failure: quote that line. Never
  run `just all` or `just ci`; the release recipe is run alone on the
  box for the one high-count run below. Hold a launch while the box's
  one-minute load average is above about 150.
- **Resource discipline.** Build with `cargo nextest run --no-run`
  before running; during iteration run only the binaries whose
  strategies you changed; one high-count release run at the end, once.
  Keep every working file under the scratchpad directory named above.
  Never delete anything outside your worktree; on a full disk, stop and
  report.
- **Commits.** One commit per logical unit (the check; the sweep per
  file family; the set strategies; the recipe and workflow variables as
  the last commit, since the justfile and `.github/` are shared root
  files), its message naming the ruling. Commit every proptest seed file
  that appears. Prose speaks in the present tense; comments use spaced
  double-hyphens, never em-dashes; every test keeps a doc comment
  stating its invariant. Apply `PROSE.md`'s three tests to every
  paragraph you touch. Annotations per WORKFLOW.md: one row per changed
  region in `triage/annotations/p1-generators.tsv`, in your own words
  for Finch.
- **Self-retirement.** After the final commit, from inside the
  worktree: `cargo metadata --no-deps --format-version 1 | jq -r
  .build_directory`, then delete that directory and the worktree's
  `target/` if a Mac build ever happened. Leave the worktree in place.
- **Report.** Landed, stopped, or open per item; the commit shas; the
  acceptance evidence (command and decisive output, verbatim); the site
  table (file, the rejecting form before, the total construction after,
  and whether the population is identical or a stated superset); the
  high-count run's result. Your report is data: the coordinator verifies
  the acceptance against the tree at the reported sha.

## The ruling

> (T161, quoted from ../rulings.md; read it there.)

## Mechanism, in order

1. **The check, one commit.** Extend `tools/caselint` (the proptest-ci
   lane's check; keep its name and shape) with the rejecting spellings:
   `prop_assume!`, `.prop_filter(`, `prop_filter_map`, and a collection
   strategy call (`hash_set(`, `btree_set(`, `hash_map(`, `btree_map(`)
   whose size range has a nonzero minimum. Roots `benches examples src
   tests` now; `crates` joins when `before/p2-generators` merges (the
   coordinator widens it; say so in the recipe comment). Self-test
   cases: one fixture per spelling fails; a `btree_set(.., 0..=8)` and a
   `sample::subsequence` pass. The tree at base fails the new rule (the
   known-bad); the sweep commits take it to zero.
2. **The sweep.** Census at base:
   `grep -rn 'prop_assume!\|\.prop_filter(\|prop_filter_map' src tests benches`
   (nine sites: `src/tree/tests.rs`, `src/tree/mirror/streaming/tests/capacity.rs`,
   `src/tree/mirror/streaming/remote/codec/tests.rs` (four),
   `src/tree/mirror/streaming/remote/codec/decode/tests.rs`,
   `tests/partition.rs`, `tests/multi_peer.rs`) and
   `grep -rn 'hash_set(\|btree_set(\|hash_map(\|btree_map(' src tests | grep -v ', 0\.\.'`
   (ten sites, all `btree_set` over bytes or fixed byte arrays). Each
   becomes a total construction: a non-empty length from `1..=n`; a
   distinct pair as an element and a nonzero offset; "not present"
   drawn from the complement; a well-formed `WireSignal` built by the
   strategy from its valid parts rather than filtered; a run within
   `PAYLOAD_CHUNK_LEN` sized before its bytes are drawn; a set of k
   distinct elements as `proptest::sample::subsequence(<the full
   range>, k)` (for `uniform32` arrays over a small alphabet, draw
   distinct arrays by drawing a subsequence of indices and mapping).
   Where the filtered population was a strict subset the property
   depended on, the generator produces exactly it; where the property
   holds on the superset, say so in the site table. A committed seed
   under a changed strategy must still replay (`cargo nextest run` of
   that binary shows the seed's case passing); if a seed cannot replay,
   stop on that site.
3. **The runtime tripwire, one commit, last.** The justfile's `test`,
   `test-all`, and `test-release` recipes export
   `PROPTEST_MAX_GLOBAL_REJECTS=0 PROPTEST_MAX_LOCAL_REJECTS=0`, with a
   recipe comment stating why (the ruling's argument in two sentences);
   the CI test jobs (`ci.yml`) export the same. No `ProptestConfig`
   sets either field; extend the check's `cases` rule to
   `max_global_rejects`/`max_local_rejects` literals. Negative control:
   restore one `prop_assume!` reversibly and run its binary; quote the
   `Too many global rejects` abort at the first rejection.
4. **The high-count run, once.** `PROPTEST_CASES=16000 just test-release`
   on the box with the meter binary excluded as the recipe already does
   (the proptest-ci lane's shape): every property runs to completion or
   fails on a real counterexample; no `Too many ... rejects` anywhere
   (`grep -c 'Too many' <log>` is 0). Any real counterexample is a
   finding: commit its seed, report it, do not fix the property.

## Stops

A committed seed that cannot replay under its rewritten strategy; a site
whose filtered population cannot be generated directly without changing
the property's claim; anything the ground rules name.
