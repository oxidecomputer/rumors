<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T101, T128, T130, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: the closed-world poller everywhere, and families where the claim is a family

> **Ruled after drafting.** T159 items 6 and 10: streaming-tests-23 takes the default count; `pollster` leaves the manifest when no site survives.

## Goal

Every session-driving test runs under the closed-world poller, so a
stall fails as `Stalled` in milliseconds instead of hanging to nextest's
kill (the four pollster sites and the five wall-clock negatives in
`gossip_when.rs`); and every family claim is a family: a proptest where
the space is open, an enumeration where the roster is finite, a
`proptest!` member rather than a hand-driven `TestRunner` (T128), and
one deterministic tripwire where the property is unreachable (T101).
Effort: high.

## Ground rules

These apply to every P4 lane; the lane sections below add to them and
never relax them.

- **Base.** Your worktree's HEAD must equal `<base sha>` before you start.
  Run `git -C <worktree> rev-parse HEAD`. If HEAD is an ancestor of
  `<base sha>`, fast-forward; if it has diverged, stop and report. Never call
  EnterWorktree; operate on the worktree through `git -C <path>` and
  absolute paths, one shell invocation at a time.
- **The review documents are the specification.** Each member entry below
  quotes its Resolution and Acceptance verbatim from the topic document in
  `.agent-notes/2026-09-01-holistic-review-rumors/`; the entry's full record
  (evidence, construction, demonstration) is under `### <id>:` there and in
  `evidence/`. Line anchors are at the reviewed commit `9e5784fb`; re-anchor
  from the quoted evidence, never from the line numbers.
- **Goal beside mechanism.** Where a quoted resolution and the goal above
  come apart, the goal wins, and the discrepancy is reported.
- **Stops.** Report and leave the entry open; do not work around: anything
  that moves an `insta` snapshot or a committed pin; any change to a public
  signature or public rustdoc contract the resolution does not name;
  anything that contradicts a ruling in `triage/rulings.md`; any deviation
  from the stated resolution; anything this brief marks as a stop. A stop on
  one entry does not block the others.
- **Negative controls.** Every repaired instrument lands with a committed
  demonstration that a known-bad artifact fails it. The constructions in
  `evidence/witness.md` and each entry's Construction line are those
  artifacts; convert each into a committed test (`should_panic`, an asserted
  `Err`, a `--self-test` case, or a reversible mutation whose observed
  failure the commit message records verbatim).
- **Resource discipline.** Build with `cargo nextest run --no-run` before
  running; during iteration run only the binaries the entry names; never
  iterate on a timing measurement. One full `just gate` before each commit,
  run in the background redirected to a log under `<scratchpad>/p4-drivers/`,
  polled with short foreground checks (the foreground command cap is ten
  minutes). Keep every working file under that directory. `just all` and
  `just ci` are run once each at the end, not per commit.
- **Commits.** One commit per logical unit, its message describing the
  change and naming the entry ids and ruling. Commit every proptest seed
  file that appears. Prose speaks in the present tense: no reference to code
  that no longer exists, no dated rationale at a declaration site. Comments
  use spaced double-hyphens, never em-dashes; every test has a doc comment
  stating its invariant. Never delete anything outside your worktree; if the
  disk fills, stop and report.
- **Self-retirement.** After the final commit, from inside the worktree:
  `cargo metadata --no-deps --format-version 1 | jq -r .build_directory`,
  then delete that directory and the worktree's `target/`. Leave the
  worktree in place.
- **Report.** For each entry: landed, stopped, or open; the commit sha(s);
  the acceptance evidence (the command and its decisive output, verbatim).
  Then anything left open and why. Your report is data: the coordinator
  verifies each entry's Acceptance against the tree at the reported sha
  before the ledger records it. Report what you could not do rather than
  working around it.
- **Prose.** `PROSE.md` in this directory binds this lane (ruling T141): every
  paragraph you touch passes its three tests (altitude, concision,
  legibility), and the diff is net shorter in prose unless your report says
  what the added sentences buy.
- **Machine.** The illumos box (`ox-east-1`, per the `building-on-illumos` skill) is
  where a lane builds, tests, and gates. The wrapper syncs the Mac
  worktree to `~/src/<worktree basename>` on the box and runs one command
  there with its own target directory, so lanes do not collide; cargo
  runs `--locked` there; nothing is edited or committed on the box. A
  clean gate on the box is the gate of record for a commit; the Mac runs
  no gate (Finch's ruling). The box gate is `on-illumos.sh <worktree> 'just gate'`. One leg is
  expected red there and counts as clean when it is the only failure:
  `fuzz`, because libFuzzer has no illumos port (`FuzzerPlatform.h`
  refuses the target); a lane quotes that line and runs no fuzz build
  elsewhere (Finch's ruling: fuzzing is CI's). clippy's
  `missing_const_for_thread_local` misfires on illumos, where
  `thread_local!` expands through the OS-keyed path; `before` carries an
  illumos-scoped crate-level allow, so a lane based before that landed
  either rebases or passes `RUSTFLAGS="-A clippy::missing_const_for_thread_local"`
  in the remote command for that one run. Two legs that
  pin toolchain-derived numbers may fire on the box if its toolchains
  differ from the pinned ones; a lane reports such a leg with both numbers
  rather than re-pinning anything. `tools/memwatch` is deleted by the
  `p1-memwatch` lane, which runs first and unstacked; `p1-gate` rebases
  onto it. Benchmarks whose committed baselines are the Mac's run on the
  Mac, once, on a quiet machine. Clock guard, checked before every box
  run: rsync preserves mtimes and cargo's rebuild detection is
  mtime-based, so a box clock ahead of the Mac by more than a couple of
  seconds means a green build of stale code; on skew, a lane either runs
  with a fresh target directory on the box (a cold build, no stale
  artifact to trust) or waits, and says which. Stepping the box's clock is
  admin work on a shared machine and is Finch's, never a lane's. The
  builder cap counts Mac builders only; on the box (96 cores, 1 TiB) there
  is no lane cap, only the load: hold a launch while the one-minute load
  average sits above about 150 on 192 threads, and keep wall-time
  measurements under `pset-run` to one at a time, announced in the merge
  queue first.
- **Out of scope.** The formal tier (`lean`, `eventdag`, `muxprobe`, everything under
  `formal/`) and `before`'s bench judge (`bench-judge`,
  `bench-judge-tripwire`) are never run or edited by a rumors lane; a
  recipe that composes them (`all`) is exercised by its other legs
  individually. A rustdoc on a Rust-side literal derived from the Lean
  artifact is Rust prose and may be edited where a ruling names it.

## Mechanism, in order

1. **Pollster, one commit per file**: testing-infra-18,
   remote-proxy-tests-8, tests-observation-8,
   tests-resource-link-window-10, tests-disruption-handshake-14. If no
   site survives, `pollster` leaves `[dev-dependencies]` in the closing
   commit, which is the strongest check (a reintroduced
   `pollster::block_on` fails `cargo check --locked`); if a site must
   stay (`tests/handshake.rs`, if its fake-peer tests are not
   session-driving), the allow list is stated in the commit and the
   oracle is `git grep -n pollster -- src tests Cargo.toml` against it.
   Known-bad: tests-resource-link-window-10's Construction (a `Pending`
   with no wake) fails in under a second with `Stalled`.
2. **Families**: mirror-common-14 (the arm-hit count quoted once),
   remote-capture-atlas-24, streaming-backend-window-38,
   streaming-tests-16, -17, -23, tests-disruption-handshake-16,
   tests-observation-7, verification-infra-17, remote-codec-26. Each
   proptest sets no `cases` (T151); each seed that appears is
   committed. Known-bad: the Constructions (an `unreachable!` past the
   magic check is hit; `Batch::commit` returning `true` unconditionally
   fails the `Changes` property).
3. **T101**: one deterministic case; the docs restated.

Closing oracle: the pollster grep above; `git grep -n TestRunner -- src/tree/mirror/streaming/tests/local_eq.rs`
empty; `faults.rs` has one `proptest!` block; T151's `caselint` leg
clean.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **remote-proxy-tests-6** (low; T101; T101: one deterministic case over `early_first_child_dispute_pair` asserting `reordered == 0`; the `== 0` assertion stands (T21)). `src/tree/mirror/streaming/remote/proxy/tests.rs:113-122`. Resolution: Ungated: rewrite the `REORDER_BATCH` and helper docs to state that no inversion is reachable under the joined-endpoint driver and that the counter's zero is the assertion; settle "provably" versus "verified empirically" by stating the mechanism argument once and calling the zero an observation that pins it. Gated, the owner's choice: keep the property as is; or keep the tripwire as one deterministic case over `early_first_child_dispute_pair` asserting `reordered == 0`, relying on `wide_symmetric_accepts_match_local` for the wide property and the conformance suite's `ReversingAcceptor` for the decorator; or restructure the driver so accepts can batch and flip the tripwire to `> 0`. Acceptance: the helper docs and the test doc agree that no inversion is reachable and say why; either the case count is one with the reason stated, or the current count is defended in the `CASES` doc.
- **streaming-tests-23** (medium; T128; T151 forbids `with_cases(32)`: the property takes proptest's default count; report the site at removal). `src/tree/mirror/streaming/tests/local_eq.rs:144-254`. Resolution: Move the body into the file's `proptest!` block as `fn free_insertions_are_invisible_to_the_local_view(spec in arb_divergence())` with `prop_assert!`s (`#![proptest_config(ProptestConfig::with_cases(32))]` if the case count matters), delete `CASES`, `runner`, `nondegenerate`, and the two imports at 24-25, and retitle the doc from "NONDEGENERACY, counted" to the invariant. Acceptance: no `TestRunner` in local_eq.rs; the test is a `proptest!` member; a forced failure writes to `proptest-regressions/tree/mirror/streaming/tests/local_eq.txt`.
- **tests-disruption-handshake-14** (medium; T130; the entry's five sites are in `gossip_when.rs`, which `p2-vanish-liveness` edits: after its merge). `tests/gossip_when.rs:180-184`. Resolution: Replace each negative window with `assert!(matches!(run_to_quiescence(futures::future::join(a_sessions.next(), b_sessions.next())), Err(Quiescence::Stalled)), "...")` (rumors::testing::Quiescence is public), moving the affected tests onto `common::wire::block_on` if calling the poller inside a runtime reads awkwardly; keep the 10 s DEADLINE only on positive waits, which fail loudly on expiry. Rewrite lines 4-6 to say what is true: cues are hand-fed and negative checks are stall-detected, with wall-clock deadlines remaining only as watchdogs on positive waits. Acceptance: no `from_millis(100)` remains in the file; each former negative names `Quiescence::Stalled`; the module doc no longer claims "no timers anywhere". Construction: In a scratch copy make `Trigger::Tick(Some(Gossip::WhenChanged))` in src/peer/gossip.rs always initiate and insert `std::thread::sleep(Duration::from_millis(150))` in the driver's first poll; suppression_swallows_echoes_not_news passes at line 182 today, while the quiescence form fails on the first run.
- **mirror-common-14** (low; T132; the arm-hit count is a one-time observation quoted in the commit, not a committed counter). `src/tree/mirror/handshake/tests.rs:257-263`. Resolution: Supplement the strategy with a prefix-valid arm, weighted heavily: `prop_oneof![1 => any::<[u8; V2_PREAMBLE_LEN]>(), 8 => any::<[u8; 19]>().prop_map(|tail| { let mut b = [0u8; V2_PREAMBLE_LEN]; b[..11].copy_from_slice(&V2_PREFIX); b[11..].copy_from_slice(&tail); b })]`, and optionally a third arm that also fixes the version byte at `Protocol::V2 as u8` so the network and intent arms are reached on most cases. Acceptance: a temporary `unreachable!()` inserted after the magic check (handshake.rs:119) is hit by the suite; the committed strategy reaches every `Err` arm of `decode` (checked once by counting arm hits). Construction: insert `unreachable!("reached past the magic check")` at handshake.rs:119 and run `arbitrary_bytes_never_panic` alone: it passes all 256 default cases at this commit.
- **remote-capture-atlas-24** (nit; T132). ``src/tree/mirror/streaming/remote/codec/tests.rs:89-92``: `arb_query`'s `btree_map(any::<u8>(), .., 0..=256)` cannot realize a 256-child fan (about 162 distinct keys) Resolution: generate the fan as a subsequence of the radix space
- **remote-codec-26** (nit; T132). ``src/tree/mirror/streaming/remote/codec/greeting.rs:35-44``: `KEYS`' deterministic order is asserted by hand and the greeting round-trip is three fixtures Resolution: a test that `KEYS` ascends under `(head_len, bytes)`; a round-trip proptest over versions, listings, and `u64` fields
- **remote-proxy-tests-8** (low; T132). `src/tree/mirror/streaming/remote/proxy/tests.rs:316-330`. Resolution: Rewrite both as `#[test] fn ... { let (a, b) = run_to_quiescence(reconcile(..)).expect("..."); ... }`, matching `symmetric_accept_handshakes_are_live`. Acceptance: `grep -n pollster src/tree/mirror/streaming/remote/proxy/tests.rs` is empty.
- **streaming-backend-window-38** (low; T132). `src/tree/mirror/streaming/window/tests.rs:359-362`. Resolution: Use a log-uniform strategy (for example `(0u32..64).prop_flat_map(|bits| { let lo = 1u64 << bits; lo..=lo.saturating_mul(2).saturating_sub(1) })`) for both tests, and draw asymmetric `(a, b)` pairs in `window_stays_inside_the_budget`. Acceptance: both strategies produce corpus sizes spanning 1..2^63 with roughly equal mass per octave; asymmetric pairs are drawn.
- **streaming-tests-16** (low; T132). `src/tree/mirror/streaming/tests/capacity.rs:299-312`. Resolution: Replace the tally with a loop over `[(4, 1), (6, 3), (8, 4), (12, 6)]` asserting `stalls(parents_of_three(p), c)` and `completes(parents_of_three(p), c + 1)` (using the three-way probe from streaming-tests-11), and likewise `for cap in 1..=6` in `parent_delay_single_parent_boundary`; reduce the comment to the law. Acceptance: every (P, C) pair named in the two tests' prose is asserted by the body, or absent.
- **streaming-tests-17** (low; T132; the orphaned `faults.txt` seed lines were disposed by T59 (`p3-seeds`): verify at base). `src/tree/mirror/streaming/tests/faults.rs:41-58`. Resolution: Give `Violation` (or a test-local const) and `GreetingLie` an `ALL` array; rewrite both tests as plain `#[test]`s with nested loops over `ALL x [true, false]` and `ALL x 0..=REPLY_PHASES x side`, where `REPLY_PHASES` is a named constant tied to the driver's schedule; keep `materialized_backend_failures_are_fail_fast` as a proptest. Acceptance: both tests are deterministic enumerations; faults.rs has one `proptest!` block; the orphaned seed lines have an owner ruling.
- **testing-infra-18** (low; T132; T146: `src/testing.rs` carries the harness-tests lane's poll-budget verdict; build on it). `src/tests.rs:49-53`. Resolution: Replace each session-driving `pollster::block_on(async { ... })` with `run_to_quiescence(async { ... }).expect("the closed in-memory session stays live")`; consider exporting that one-liner from `testing` as `block_on` with `#[track_caller]` and its poll budget stated in the doc, so `src/tests.rs`, `tests/common/wire.rs`, and the integration suites stop re-deriving it. Reword the doc at 664-665 to name the poller. Acceptance: `grep -n pollster src/tests.rs` returns only line 269 or nothing; the tests pass; a deliberately stalled variant fails with `Quiescence::Stalled` in under a second.
- **tests-disruption-handshake-16** (low; T132). `tests/gossip_when.rs:490-494`. Resolution: Add a proptest `drop_after in 0usize..N` (N from a metered clean run's poll count) that polls both drivers `drop_after` times, drops them, and asserts the union and atomicity invariants plus poison-then-recover; record whether `a.snapshot().len() == 2` at the drop and, when it is, assert the drop landed after the last data frame. Acceptance: a proptest over the drop point exists in tests/gossip_when.rs (or tests/lifecycle.rs) and passes; any shrunk failure persists to proptest-regressions/. Construction: Reuse the body of dropping_a_driver_mid_session_commits_nothing with the loop bound drawn from the strategy.
- **tests-observation-7** (low; T132). `tests/changes.rs:1-6`. Resolution: Add a proptest over `Vec<{Send(u64), SendAll(0..=3 values), RedactHeld(idx), RedactUnheld, Poll}>` that records `latest()` at each Poll and asserts `try_next() == Tick` iff `latest` differs from the last reported one, `Quiet` otherwise. Add two point tests: `redact` of a version the set never held and `send_all(std::iter::empty::<u64>())` both leave a reported signal `Quiet`. Acceptance: a mutant making `Batch::commit`'s closure return `true` unconditionally fails the new tests. Construction: In tests/changes.rs, `let rumors = Peer::<u64>::seed().sync_window_floor().into_rumors(); let mut changes = rumors.changes(); assert_eq!(changes.next().now_or_never(), Some(Some(()))); rumors.send_all(std::iter::empty::<u64>()).unwrap(); assert_eq!(changes.next().now_or_never(), None);` and the same with `rumors.redact(&Version::new())`. Today nothing in the suite makes either call with an observer attached.
- **tests-observation-8** (low; T132; if no `pollster` site survives, the dev-dependency leaves the manifest in this lane's closing commit). `tests/changes.rs:14-19`. Resolution: Make all seven tests plain `#[test] fn`, call `bootstrap_fork`/`wire_gossip`, and in the reclaim test use `rumors.try_into_peer().now_or_never().expect("an observer does not count against quiescence")`. Drop the pollster import from this file (handshake.rs and gossip_when.rs keep the dev-dependency unless they follow). This also removes changes.rs from the `testdoc` hole in tests-observation-37. Acceptance: `grep -c pollster tests/changes.rs` is 0; the seven tests pass unchanged in behavior.
- **tests-resource-link-window-10** (low; T132). `tests/latency_link.rs:28-43`. Resolution: Wrap both `check` calls in `tokio::time::timeout(SUITE_TIMEOUT, ...).await.expect("conformance suite ran past its liveness bound")` as the socket runners do, with a `SUITE_TIMEOUT` constant. Acceptance: both tests carry the timeout; a deliberately wedged pipe fails with that message rather than after 180 s. Construction: In a scratch copy of `latency::delayed_pair`, make one stream's `poll_read` return `Pending` without arranging a wake; today the test hangs until nextest kills it, with the timeout it fails immediately.
- **verification-infra-17** (low; T132; the routed-link property uses the conformance suite as oracle (T31/T68's routed link, `p2-link` unmerged): after its merge). `tests/changes.rs:3-6`. Resolution: add a Changes property over generated commit sequences and poll points asserting the coalescing law, and a routed-link property over generated connect/accept/label schedules against the in-memory network (the conformance suite as oracle); keep the point tests as named corners. Acceptance: each file carries a `proptest!` block whose doc states the family invariant.

## Hazards and stops

- `p1-proptest-ci` (unmerged) edits `capacity.rs`, `stats.rs`,
  `observe.rs`, the proxy tests, and adds `tools/caselint`;
  `p2-vanish-liveness` (unmerged) edits `gossip_when.rs` and
  `src/testing.rs`; `p2-link` (unmerged) edits the routed link
  (verification-infra-17's property). Launch after all merge.
- `faults.rs` is also `p4-testdocs`' (streaming-tests-18): testdocs
  first.
- `tools/testdoc` recognizes `#[pollster::test]` (T30); that pattern is
  harmless once the dependency is gone and is not removed here.
