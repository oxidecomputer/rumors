<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from ruling T162 in ../rulings.md; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# P2 lane: two anomalies under deep geometry

## Goal

The collision schedule (`p1-collision-mode`, T23) reruns the suite under
deep trie geometry, and its first sweep found two behaviors the shallow
hash geometry never showed. Both are correctness-class until explained:

1. **An understated-length lie is not surfaced on one side.** In
   `proxy/tests/declarations.rs::understated_version_bytes_fail_the_session`
   under `RUMORS_PATH_SCHEDULE=1`, the lie heard on the right yields the
   typed `Decode(OversizedVersion { declared: 0, actual: 4 })`, as the
   contract says; heard on the left, the left endpoint reports
   `Stream(SupplyClosed { .. UnexpectedEof, "peer link is gone" })` and
   the right `Send(Frame(EncodeError { .. SupplyLength .. BrokenPipe }))`.
   The contract: a peer's malformed frame is reported as the typed decode
   error at the endpoint that decodes it, never masked by the transport
   failure that follows it. Either an error-attribution race among the
   many concurrent streams a deep session opens, or the check is missed
   on one side.
2. **A session's poll count is not reproducible.** In
   `streaming/tests.rs::cancelled_session_leaves_no_residue`, once, the
   measured session length differed between the strategy's measurement
   and the test's run for the same input (three leaves under a shared
   31-byte prefix, every path memoized beforehand) within one process.
   The premise it breaks: under the closed-world poller with in-memory
   links, a session's schedule is a function of its input. The
   streaming session holds no `HashMap` and every production
   `tokio::select!` is `biased`, so those are not the source; candidates
   are the `FuturesUnordered` in `streaming/tasks.rs` (its poll order
   follows wake order) and anything ordered by an address.

The invariant this lane restores or refutes, for each: the typed error
reaches its endpoint under every geometry; the schedule is a function of
the input under every geometry. A refutation that shows the contract was
never what the docs say is a stop, not a rewrite.

## Ground rules

- **Base.** `triage/p1-collision-mode` at `7858b35e` (the mode's recipe and
  marks); the coordinator names the sha at launch. Verify
  `git -C <worktree> rev-parse HEAD`; if HEAD is an ancestor, fast-forward;
  if diverged, stop and report. Never EnterWorktree; `git -C` and absolute
  paths, one shell invocation at a time.
- **Instruments before cures.** For each anomaly, first a deterministic
  reproduction committed as a failing test (a point construction under the
  schedule, or a seeded loop that fails by name within a bounded number of
  runs, with the count stated), then the diagnosis with the mechanism
  quoted from the code, then the fix, then the test passing. A fix without
  the failing test landing first is a deviation.
- **Stops.** Report and leave open: a fix that changes a public signature
  or public rustdoc contract; a moved `insta` snapshot or committed pin;
  a diagnosis that the contract as documented was never held (owner's
  ruling); anything contradicting `triage/rulings.md`.
- **Where you build.** The illumos box via `on-illumos.sh <worktree>
  '<command>'` with `export CARGO_BUILD_JOBS=32 NEXTEST_TEST_THREADS=32`
  inside every remote command, unbound, never `pset-run`, `--locked` on
  every cargo call; the schedule needs `NEXTEST_EXECUTION_MODE=process-per-test`,
  which nextest sets. One `just gate` at the end, backgrounded to a log
  under `<scratchpad>/p2-deep-geometry/` and polled; `fuzz` red alone is
  clean; also `just test-collision` once at the end, whose three expected
  failures (7, 8, 9 of T162) must drop to (7) alone. Never `just all` or
  `just ci`.
- **Resource discipline.** Build with `--no-run` first; iterate on the
  two binaries only; loops for (2) capped at 200 runs per configuration,
  release profile, stated in the report. Keep every file under the
  scratchpad directory. Never delete anything outside your worktree.
- **Commits.** One per logical unit (the failing test; the fix; the test
  passing may be the same commit as the fix when the test is the
  instrument), messages naming T162 and the anomaly. Every proptest seed
  committed. No `cases` in any config (T151); no rejecting strategy
  (T161). Prose in the present tense, spaced double-hyphens, every test
  with a doc comment; `PROSE.md`'s three tests on every paragraph you
  touch. Annotations in `triage/annotations/p2-deep-geometry.tsv` per
  WORKFLOW.md.
- **Self-retirement** of caches after the final commit (resolve the forge
  dir from inside the worktree first); leave the worktree.
- **Report.** Per anomaly: the reproduction (command, decisive output),
  the mechanism (code quoted), the fix, the test's before/after lines,
  and the `test-collision` summary; stops as a numbered block with
  recommendations; findings no entry covers.

## Mechanism, in order

1. **Reproduce (1).** Run the declarations test under the schedule with
   `--no-capture`, both lie sides, twenty times; record which side masks
   and how often. Then read the left endpoint's decode path for a
   supply-length lie (`remote/adapter/decode.rs`, the codec's version
   decoding, `remote/proxy/work`'s error attribution among concurrent
   streams) and find where the typed error is lost: a race in which the
   transport failure from the peer's abort arrives before the decode error
   is published, or a side that never decodes the lied frame. Commit the
   deterministic reproduction.
2. **Fix (1).** The typed error is published before, or in preference to,
   the transport failure it causes; the fix's shape is yours within the
   session's existing error-precedence rules (read `remote/error.rs` and
   the mirror's error docs first).
3. **Reproduce (2).** The residue test's shrunk input is in the lane's
   `sweep2-gate1.log` from line 505 (scratchpad `p1-collision-mode/`);
   build it as a point test that measures the session length N times in
   one process under the schedule and asserts all equal; loop until it
   fails or 200 runs pass, and report the count. Instrument the poll
   schedule (a trace of which task polls in which order, gated behind the
   test) to diff two unequal runs.
4. **Fix (2).** Make the schedule a function of the input: replace the
   address- or wake-order-dependent ordering with a deterministic one, or
   document and pin the precise premise if the order is legitimately
   free and the test's measurement is what must change. The second is a
   stop.
5. **Both fixed:** `just test-collision` shows only T162 (7) failing;
   `just gate` clean.
