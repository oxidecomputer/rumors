# The coordinator's four leads (round 2)

## 1. `resident_space_comparison_is_scoped_by_shape` fails once under load

**Verdict: a real race in the instrument, but not with a concurrent test.**
The contaminating allocation comes from libtest's own main thread.

Evidence (verified unless marked):

- The recorded failure (`builder-join-all-multiplicity/gate-1-legs/workspace.log`,
  box load about 500) is the release check, not a ratio:
  `assertion 'left == right' failed: the measured value must release every allocation it owns`,
  `left: 4729`, `right: 4609`: 120 bytes live after the measured value was
  dropped. Five other recorded runs pass.
- The test is alone in its binary (`crates/before/tests/representation_space.rs`)
  with its own `#[global_allocator] static HEAP: PeakAlloc`, and nextest runs
  each test in a fresh process, so no other test shares the counter.
  Production code holds no lazily allocated state (its only `thread_local!`
  is a `#[cfg(test)]` const `Cell`).
- How nextest invokes the binary on the box (probe test `l8_env_probe`):
  `ARGS [..., "--exact", "l8_env_probe", "--nocapture"]`,
  `RUST_TEST_THREADS Err(NotPresent)`, `PARALLELISM Ok(192)`,
  `THREAD Some("l8_env_probe")`. libtest therefore takes its concurrent path
  and runs the test on a spawned thread.
- libtest 1.97.1 (`library/test/src/lib.rs:451-464`), concurrent path: after
  `run_test` spawns the test thread, the main thread runs
  `running_tests.insert(id, RunningTest { join_handle })` and
  `timeout_queue.push_back(TimeoutEntry { id, desc, timeout })`, both first
  insertions into empty collections, so both allocate. Normally that finishes
  before the test takes its baseline; when the main thread is preempted right
  after the spawn, it lands inside the measurement window and the
  process-wide counter shows it. With `concurrency == 1` the main thread only
  blocks in `rx.recv()` (lines 429-437) and nothing races.
- Inferred, not verified: that the 120 bytes are exactly those two
  allocations (I did not compute their layouts).

The same race reaches every heap check made in a libtest process with
`PeakAlloc`: `crates/before/tests/meter.rs` (for example
`meter/identity_fast_paths.rs`, peak heap against
`EQUAL_JOIN_PEAK_HEAP = 440`) and `crates/before/tests/amp_board_smoke.rs`
(`retaining_results_does_not_allocate_measured_heap` asserts a heap reading
of exactly `"0"`). The board itself is unaffected: its `amp_board` shards are
custom `main`s with one thread.

**Proposed instrument fix**, cheapest first:

1. Run libtest with one thread under nextest: `RUST_TEST_THREADS=1` in the
   environment of the nextest invocations (or a `.cargo/config.toml` `[env]`
   entry). Each nextest process runs one test anyway, so this costs nothing,
   and libtest's single-thread path does not allocate on its main thread while
   the test runs (verified in the 1.97.1 source). Weakness: it depends on
   libtest internals staying that way.
2. Robust: attribute heap to the measuring thread. Replace the process-wide
   `PeakAlloc` readings in these three binaries with a counting allocator that
   counts only allocations made by a thread inside a measurement scope (a
   maintained crate, per the zero-`unsafe` rule; I have not vetted one). Then no
   harness thread can enter a reading.

A calibration for either: a test that spawns a thread allocating in a loop
while the measurement runs must still read exact values.

## 2. `STOPPING_DIFF_BAND` reading rose from 7,872 to 8,892

**Attributed (verified): commit `fdd1bf47` ("Simplify exact accumulation"),
mechanism: a debug-only `clone().cmp_zero()` positivity check added to
`Boundary::from_positive` for every boundary, counted by the touch meter
because the meter suites run in the dev profile with debug assertions on.**

- Readings of `diff_large` (the stop-minus-control touch difference at the
  larger run, `tests/meter/version_scaling/minimum_boundaries.rs`), measured
  on ox-east-1 from `git archive` exports in the probe tree (logs
  `seam/<sha>.log`): pin commit `63d01d90`: 7,867 (the pinned record 7,872 is
  the band's centre: `(5_904, 9_840)` is ×0.75 and ×1.25 of it); audit base
  `58285ca5`: 8,892.
- Bisect over the 25 first-parent commits between them that touch either
  crate (`seam/bisect.txt`): `eff7de3e` 7,867; `fdd1bf47` 8,892 (first new).
- The diff of `fdd1bf47` in `range_minima/boundary.rs`: the old
  `from_positive` checked the sign only while materializing a value of at
  most two digits; the new one runs `#[cfg(debug_assertions)] { let sign =
  difference.clone().cmp_zero(); debug_assert_eq!(sign, Ordering::Greater,
  "boundaries are positive"); }` for every boundary. Each stopping hop leaves
  a wide surviving boundary, so the check adds about one touch per hop:
  1,025 at `k = 1,024`.
- Corroboration: the debug-assert builder's deletion (`builder-debug-asserts/deletion.diff`)
  removes exactly this block, and its runs read 7,867 again.

**Instrument finding.** The band's ceiling is the record ×1.25, which admits
up to 1,968 extra touches, about 1.9 per hop. Its own failure message says
the ceiling exists to catch "a per-hop read of the surviving boundary's
width". A one-touch-per-hop read rose 13% and passed. A tighter ceiling
denominated in hops (for example `record + k / 2`) would have caught it.

## 3. nextest flags passing tests as LEAK under load

**Verdict: harmless to every verdict; the flag reflects scheduling, not the
tests (inferred from the evidence below; I did not reproduce a leak).**

- nextest marks a test leaky when its stdout or stderr stays open longer than
  the leak timeout (100 ms by default) after the test process exits. That
  requires some other process to hold the pipe's write end.
- The flagged tests own no such process. Across the audit's logs the flags
  land on purely computational tests in all three crates
  (`before version::tests::rank_formatting_behaves_as_text`,
  `suanpan ... new_and_default_hold_zero`, many `rumors` units), and
  intermittently: the coordinator's data point is
  `before version::measure::tests::arbitrary_arming_trains_agree` flagged on a
  branch that changes only one suanpan test file, unflagged in that slot's
  previous run, box load about 81 at the start. Neither `before` nor
  `suanpan` tests spawn processes.
- How a pipe end can outlive its test: std spawns through `posix_spawn` on
  illumos (`library/std/src/sys/process/unix/unix.rs:438-452`), and a
  just-spawned process holds copies of every descriptor open in nextest at
  that moment (other tests' pipe ends included) until it execs, when
  close-on-exec drops them. With many spawns in flight on a loaded box, that
  window can exceed 100 ms.

Remedy (observation): set `leak-timeout` in `.config/nextest.toml` (for
example `"1s"`) so the flag marks a handle that truly lingers.

## 4. `arb_party_family` spends 94% of cases on `join_all`'s error path

**Verdict: the claim holds and understates the skew. No multi-input success
is ever drawn, though a law covers success with regular shapes.**

Measured (`census-family.log`, 20,000 draws each, fixed seed):
`arb_party_family` succeeds in 5.88% of draws, all at arity 0 (1,104 of
1,104) or 1 (72 of 1,115), and 0 of the roughly 17,700 draws at arity 2 to
17. `arb_clock_family` succeeds in 5.55%. Picks are drawn with replacement
from a pool of at most four arbitrary parties, so any repeat overlaps, and
distinct arbitrary parties are disjoint only 13.9% of the time (census).

Consequence: `party_join_all_matches_all_models` and
`clock_join_all_matches_all_models` never compare a successful multi-input
`join_all` with the models. Multi-input success is exercised only by
`party_join_all_reunites_forks_at_any_width` and its clock twin, which build
`forks(width)` shares: correct width coverage (0 to 17, across the fold's
carry boundaries), but only regular, balanced split shapes. No mutant in the
campaign demonstrates a failure behind this, so it is a reach gap, not a G.

Proposed rebalance: briefs/machinery-disjoint-families.md.

## Owner's ruling on the PeakAlloc race (question 91)

Factor `rumors`' counting allocator (`tests/support/allocation.rs`,
`MeteredSystem`) out into a shared crate that both `rumors` and `before`
depend on as a dev-dependency, and use it in `before`'s three heap-measuring
test binaries. The no-`unsafe` rule guards production code; `unsafe` in a
test-only allocator is fine. The libtest race probe
(`builder-peakalloc-single-thread/zz_libtest_race_probe.rs` in the session
scratchpad) is the calibration the fix must pass.
