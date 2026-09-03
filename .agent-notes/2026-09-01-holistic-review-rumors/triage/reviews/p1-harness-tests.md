<!-- CAVEAT LECTOR: review packet for lane p1-harness-tests, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-harness-tests

## Goal

The cross-peer test harnesses under `tests/` and the shared test
transport in `src/testing` are instruments; this lane repairs the ones
the review found could pass while the behavior they meter was broken: a
disruption harness whose serving tasks could panic unobserved, fault
schedules whose cuts mostly landed past the end of every connection,
generator populations that could stop producing the redactions the
suites exist to exercise, a reordering acceptor that never reordered,
and a quiescence guard with no verdict for runaway self-waking. Each
repaired instrument lands with a committed demonstration that a
known-bad artifact fails it.

## Rulings landed

- T13: the overlap shadow's Open-time fork is the model and the guard
  its consequence; documented as such, the fork point unchanged.
- T21: the reversing acceptor is deleted and the conformance suite's
  reordering tests run on the patient reordering acceptor, with a
  witness that it inverts a batch.
- T26: the disruption harness joins every serving task and fails the
  parent on any panic; the lifecycle generators pin their redaction
  populations.
- T28: fault-cut ranges are pinned two-sided against the measured
  cycle they must land inside; the quiescence guard returns a
  poll-budget verdict for runaway self-waking.

## Stack position

- Base: `51ccf03c` (main after the conformance merge and the `before` allow; the lane rebased there for its gate of record). Twenty-one commits.
- Parent: `main`
- Children: none

## Acceptance table

Every row is one the coordinator's verification runner ran against the
lane at `0c4b8635` (first pass) or a detached scratch worktree at
`54015761` (final pass, on the illumos box under a processor set), with
whole logs kept; mutations were reversible swaps restored to an empty
diff.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| `tests-disruption-handshake-7` | `43919d67` | `cargo nextest run -p rumors --all-features -E 'binary(disruption)'` | `14 tests run: 14 passed` |
| (negative control) | | join results swallowed in both wind-down loops, `serving_task_panics_fail_the_parent` | `FAIL ... test did not panic as expected` |
| `tests-lifecycle-15` | `22462391` | `-E 'binary(pairwise) \| binary(multi_peer)'` | `17 tests run: 17 passed` |
| (negative control) | | redaction weight 0 in both generators | exactly `action_population_contains_effectual_redactions` and `schedule_population_contains_redactions` FAIL; `15 passed, 2 failed` |
| `tests-common-12` (part 1) | `f4201699` | doc-only; read in the diff | the Open-time fork stated as the model |
| `testing-infra-12`, `conformance-18` | `7fc8eced`, `4af2eaf3` | `-E 'test(/^conformance::/)'`; `git grep ReversingAcceptor -- src tests` | `24 tests run: 24 passed`; grep empty |
| (negative control) | | `REORDER_PATIENCE = 0`, the three reordering tests by name | only `reordering_acceptor_inverts_a_patient_batch` FAILS: `left: [49, 50] right: [50, 49]` |
| `tests-disruption-handshake-3` | `c427facd` | `--no-capture -E 'test(max_child_cut)'` | `child cycle bytes per endpoint: bootstrap 784, session 1055, final gossip 1459, retire 1749`; PASS |
| (negative control) | | `MAX_CHILD_CUT = 4096` | `FAIL: MAX_CHILD_CUT (4096) is more than twice the child cycle's 1749 bytes` |
| `tests-disruption-handshake-17` | `25442f33` | `--no-capture -E 'binary(gossip_when)'` | `fixture session bytes: A wrote 188, B wrote 191, B read 188`; `19 tests run: 19 passed` |
| (negative control) | | `MAX_SEVER_CUT = 400` | `FAIL: MAX_SEVER_CUT (400) is more than twice the fixture session's 191 bytes` |
| `testing-infra-4` | `d9bc1809` | `-E 'test(runaway)'` | PASS |
| (negative control) | | guard returns `Stalled` | `FAIL: left: Err(Stalled) right: Err(PollBudget)` |
| all | `d9bc1809` (final code tree; `0c4b8635` adds annotation rows only) | `just gate` (lane logs `gate-*.log`) | clean at `43919d67` (572 s), `4af2eaf3`, `25442f33`, `d9bc1809` (179 s) |
| T144 (`6ed7210f`, `b8ed44cc`) | `54015761` | `tail -1 proptest-regressions/shadow_validity.txt`; `grep FORK_ROUNDS tests/common/overlap.rs`; the stop directory | seed `cc 3043e400…` committed; `FORK_ROUNDS = [2, 1]` stated at the model; `triage/stops/p1-harness-tests/` retired |
| (same) | `54015761` | the seven affected binaries under `pset-run -n 40` (box, scratch worktree) | `47 tests run: 47 passed`; `overlap_shadow_predicts_live_state` PASS; `survivor_parks_when_its_peer_vanishes_before_its_first_stream` PASS |
| (negative control, T144) | | both sides' views taken at `Open` again | `overlap_shadow_predicts_live_state` FAILS: `peer 2 observation set disagrees with the overlap shadow` (the live set holds 21 and 32; the shadow does not) |
| round 2, vanish floor (`58dd11c0`) | `54015761` | the vanish draw made to yield `None` (a zero weight panics at construction in proptest) | `vanishes_fire_in_the_generated_population` FAILS: `no endpoint of a sampled plan reached its vanish point`; the two-sided range pin still passes |
| round 2, deadline | `54015761` | `SESSION_DEADLINE` set to 1 ms, the two plan properties | both FAIL by name: `a session parked past SESSION_DEADLINE (1ms): a protocol deadlock`; no new seed row (the first committed seed already parks at 1 ms) |
| round 2, first-connect pin | `54015761` | the test's attributes and assertion | not ignored; asserts `Err(Quiescence::Stalled)`; doc names T143 and T145 |
| T143 hygiene | `54015761` | `grep -c 'process kills\|inter-process' tests/tcp_link.rs design/rumors-frame-fuzz.md` | 0 and 0 |
| `src/` footprint | `54015761` | `git diff --stat 51ccf03c...54015761 -- src` | four files: `src/conformance/link/tests.rs` and `src/testing/transport.rs` (the brief's own files, the reordering-acceptor entry), `src/testing.rs` and the proxy tests' doc lines (T146) |
| all | `82cc0257` (re-signed `58dd11c0`, byte-identical tree; `351ea27e` and `2ef3c26e` add annotation rows only) | `just gate` on the box (lane log) | seven streams ok; `fuzz` the accepted illumos leg |

## Fresh-eyes rounds

**Round 1** (surface correctness with operational validity), against
`0c4b8635`, by sha: no defect in the change's code. The reviewer traced
the serving-task tripwire (both panic sites carry the expected
substring; a pre-abort panic survives as `JoinError::Panic`), both cut
pins (two-sided, own messages, stated premises), the poll budget, the
reordering witness's schedule under the quiescence driver (the
conformance traffic pins the inversion; only the witness pins the
patient wait), the redaction pins, and the seed path. Four repairs
landed: the observation-28 stop's
deliverables (the failing meta-test patch, its seed, the run log, and a
README with the shrunk case and the two options) now live in the tree
under `triage/stops/p1-harness-tests/` instead of the session
scratchpad; the disruption wind-down gained a liveness floor
(`possible_losses == 0` on every fault-free plan, so the sharp
party-reconstitution check cannot go silent when the grace expires),
with a negative control that fails by name and one finding recorded: a
zero grace does not trip the pin, because on a clean plan every serving
session has already ended when the parent reaps its child, so the
premise is stronger than the doc first claimed; the child-cycle
envelope's dominance argument is now stated per direction from the
generator's constants (the envelope stocks both directions before every
connection), re-measured at `bootstrap 784, session 1369, final gossip
2194, retire 2750` against `MAX_CHILD_CUT = 3072`; and `MAX_POLLS`'s
doc keeps its band statement and drops the two one-off point counts.
The T141 prose pass followed as its own commit, net shorter. A seed
persisted while running the pin's negative control (a deliberate
mutation, not a failure of the tree) is kept out of the tree, its line
recorded in the lane's log.

Reviewer notes not acted on: the reordering witness implicitly pins
`REORDER_PATIENCE >= 5`, stated nowhere (a doc sentence would do; taste);
`SERVE_GRACE` is wall-clock, and its only cost on expiry is the loss the
new floor now makes visible.

## Stops

1. **Ruled: T143.** The inter-process family is dissolved into an
   in-process vanish fault; decisions 1 and 4 of the owner's queue closed
   with it. Landed in `e0601bc4` and `92b48c3a`.
2. **Ruled: T144.** The overlap shadow forks where the live session
   forks (after its preamble exchange, at its first poll), so the model
   reflects the code and the T13 validity meta-test lands; the stop
   artifact under `triage/stops/p1-harness-tests/` is its starting
   point. Landed: the shadow forks each side at the round the live
   session forks it (`FORK_ROUNDS = [2, 1]`, derived from the step
   order), the meta-test `overlap_shadow_predicts_live_state` with its
   seed, the stop directory retired; with the fork moved back to
   `Open` the meta-test fails on the recorded shape.
3. **Ruled: T146.** Two sites under `src/` outside the brief's file
   list (three doc lines in the proxy tests that would otherwise
   reference the deleted reversing acceptor; the poll-budget verdict in
   `src/testing.rs`, the entry's own site) stand. The lane's other two
   `src/` files (`src/conformance/link/tests.rs`,
   `src/testing/transport.rs`) are the brief's own.
4. **Ruled: T154.** Once the vanish draw was weighted so vanishes
   fire, survivors parked after mid-stream vanishes too, so the T143
   open item is wider than the handshake window. As landed, a park
   after a planned vanish is aborted and counted in the outcome's
   `parked` field (the `p2-vanish-liveness` lane's assertion target,
   T145 as widened by T154), and a park with no vanish planned still
   fails by name; the first-connect point is a positively stated pin
   of the current `Stalled` behavior, not ignored, that the fix flips.

## Reading order

### deviations from a stated resolution

- testing-infra-4 (T28) at `src/testing.rs:376` ([hunk](#hunk-12))
- testing-infra-12 (T21) at `src/tree/mirror/streaming/remote/proxy/tests.rs:514` ([hunk](#hunk-17))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:125` ([hunk](#hunk-54))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:318` ([hunk](#hunk-55))

### new tests and negative controls

- conformance-18 (T21) at `src/conformance/link/tests.rs:851` ([hunk](#hunk-11))
- testing-infra-4 (T28) at `src/testing.rs:426` ([hunk](#hunk-13))
- testing-infra-12 (T21) at `src/testing/transport.rs:823` ([hunk](#hunk-16))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:226` ([hunk](#hunk-73))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:847` ([hunk](#hunk-79))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:877` ([hunk](#hunk-79))
- tests-lifecycle-15 (T26) at `tests/multi_peer.rs:210` ([hunk](#hunk-83))
- tests-lifecycle-15 (T26) at `tests/pairwise.rs:246` ([hunk](#hunk-85))
- tests-observation-28 (T144) at `tests/shadow_validity.rs:165` ([hunk](#hunk-88))

### production edits

- testing-infra-4 (T28) at `src/testing.rs:423` ([hunk](#hunk-13))
- testing-infra-12 (T21) at `src/testing/transport.rs:666` ([hunk](#hunk-14))
- conformance-18 (T21) at `src/testing/transport.rs:728` ([hunk](#hunk-15))
- conformance-18 (T21) at `src/testing/transport.rs:729` ([hunk](#hunk-15))
- testing-infra-12 (T21) at `src/testing/transport.rs:816` ([hunk](#hunk-16))

### tests and prose

- tests-disruption-handshake-7 (T143) at `.config/nextest.toml:7` ([hunk](#hunk-2))
- tests-disruption-handshake-7 (T143) at `design/rumors-frame-fuzz.md:245` ([hunk](#hunk-3))
- tests-disruption-handshake-7 (T143) at `proptest-regressions/disruption.txt:7` ([hunk](#hunk-4))
- tests-observation-28 (T144) at `proptest-regressions/shadow_validity.txt:15` ([hunk](#hunk-5))
- conformance-18 (T21) at `src/conformance/link/tests.rs:10` ([hunk](#hunk-6))
- conformance-18 (T21) at `src/conformance/link/tests.rs:24` ([hunk](#hunk-7))
- conformance-18 (T21) at `src/conformance/link/tests.rs:41` ([hunk](#hunk-8))
- conformance-18 (T21) at `src/conformance/link/tests.rs:68` ([hunk](#hunk-9))
- conformance-18 (T21) at `src/conformance/link/tests.rs:80` ([hunk](#hunk-10))
- tests-disruption-handshake-7 (T143) at `tests/bookmark_causality.rs:1307` ([hunk](#hunk-18))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:1` ([hunk](#hunk-19))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:12` ([hunk](#hunk-20))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:25` ([hunk](#hunk-21))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:66` ([hunk](#hunk-22))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:71` ([hunk](#hunk-22))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:128` ([hunk](#hunk-23))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:141` ([hunk](#hunk-23))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:177` ([hunk](#hunk-24))
- tests-disruption-handshake-17 (T28) at `tests/common/fault.rs:189` ([hunk](#hunk-25))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:275` ([hunk](#hunk-25))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:291` ([hunk](#hunk-26))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:400` ([hunk](#hunk-27))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:414` ([hunk](#hunk-28))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:424` ([hunk](#hunk-29))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:437` ([hunk](#hunk-30))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:480` ([hunk](#hunk-31))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:494` ([hunk](#hunk-32))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:527` ([hunk](#hunk-33))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:559` ([hunk](#hunk-34))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:584` ([hunk](#hunk-35))
- tests-disruption-handshake-7 (T143) at `tests/common/fault.rs:604` ([hunk](#hunk-36))
- tests-common-12 (T144) at `tests/common/overlap.rs:23` ([hunk](#hunk-37))
- tests-common-12 (T144) at `tests/common/overlap.rs:184` ([hunk](#hunk-38))
- tests-observation-28 (T144) at `tests/common/overlap.rs:234` ([hunk](#hunk-39))
- tests-common-12 (T144) at `tests/common/overlap.rs:265` ([hunk](#hunk-40))
- tests-observation-28 (T144) at `tests/common/overlap.rs:321` ([hunk](#hunk-41))
- tests-observation-28 (T144) at `tests/common/overlap.rs:341` ([hunk](#hunk-42))
- tests-observation-28 (T144) at `tests/common/overlap.rs:372` ([hunk](#hunk-43))
- tests-observation-28 (T144) at `tests/common/overlap.rs:570` ([hunk](#hunk-44))
- tests-observation-28 (T144) at `tests/common/overlap.rs:638` ([hunk](#hunk-45))
- tests-observation-28 (T144) at `tests/common/overlap.rs:666` ([hunk](#hunk-46))
- tests-observation-28 (T144) at `tests/common/overlap.rs:699` ([hunk](#hunk-47))
- tests-observation-28 (T144) at `tests/common/overlap.rs:736` ([hunk](#hunk-48))
- tests-observation-28 (T144) at `tests/common/overlap.rs:744` ([hunk](#hunk-49))
- tests-observation-28 (T144) at `tests/common/overlap.rs:776` ([hunk](#hunk-50))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:21` ([hunk](#hunk-51))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:57` ([hunk](#hunk-52))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:77` ([hunk](#hunk-53))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:140` ([hunk](#hunk-54))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:150` ([hunk](#hunk-54))
- tests-disruption-handshake-3 (T143) at `tests/common/sim.rs:340` ([hunk](#hunk-56))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:370` ([hunk](#hunk-57))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:397` ([hunk](#hunk-58))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:432` ([hunk](#hunk-59))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:481` ([hunk](#hunk-60))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:503` ([hunk](#hunk-61))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:552` ([hunk](#hunk-62))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:570` ([hunk](#hunk-62))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:597` ([hunk](#hunk-62))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:653` ([hunk](#hunk-63))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:824` ([hunk](#hunk-64))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:853` ([hunk](#hunk-65))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:931` ([hunk](#hunk-66))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:985` ([hunk](#hunk-67))
- tests-disruption-handshake-7 (T143) at `tests/common/sim.rs:1110` ([hunk](#hunk-68))
- tests-disruption-handshake-7 (T143) at `tests/common/tcp.rs:1` ([hunk](#hunk-69))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:3` ([hunk](#hunk-70))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:40` ([hunk](#hunk-71))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:43` ([hunk](#hunk-71))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:78` ([hunk](#hunk-72))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:134` ([hunk](#hunk-73))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:189` ([hunk](#hunk-73))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:210` ([hunk](#hunk-73))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:277` ([hunk](#hunk-73))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:563` ([hunk](#hunk-74))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:605` ([hunk](#hunk-75))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:632` ([hunk](#hunk-76))
- tests-disruption-handshake-7 (T143) at `tests/disruption.rs:647` ([hunk](#hunk-77))
- tests-disruption-handshake-3 (T143) at `tests/disruption.rs:647` ([hunk](#hunk-77))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:48` ([hunk](#hunk-78))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:699` ([hunk](#hunk-79))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:717` ([hunk](#hunk-79))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:807` ([hunk](#hunk-79))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:864` ([hunk](#hunk-79))
- tests-disruption-handshake-17 (T143) at `tests/gossip_when.rs:884` ([hunk](#hunk-79))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:906` ([hunk](#hunk-79))
- tests-disruption-handshake-17 (T28) at `tests/gossip_when.rs:923` ([hunk](#hunk-80))
- tests-disruption-handshake-7 (T143) at `tests/gossip_when.rs:1115` ([hunk](#hunk-81))
- tests-lifecycle-15 (T26) at `tests/multi_peer.rs:15` ([hunk](#hunk-82))
- tests-lifecycle-15 (T26) at `tests/multi_peer.rs:206` ([hunk](#hunk-83))
- tests-lifecycle-15 (T26) at `tests/pairwise.rs:22` ([hunk](#hunk-84))
- tests-lifecycle-15 (T26) at `tests/pairwise.rs:242` ([hunk](#hunk-85))
- tests-observation-28 (T144) at `tests/shadow_validity.rs:35` ([hunk](#hunk-86))
- tests-observation-28 (T144) at `tests/shadow_validity.rs:45` ([hunk](#hunk-87))
- tests-disruption-handshake-7 (T143) at `tests/tcp_link.rs:1` ([hunk](#hunk-89))

### nits

- conformance-18 (T21) at `src/conformance/link/tests.rs:62` ([hunk](#hunk-9))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-harness-tests.tsv `@@ -0,0 +1,115 @@`

```diff
@@ -0,0 +1,115 @@
+# Lane p1-harness-tests annotations: path<TAB>line<TAB>entry id<TAB>ruling<TAB>note. Lines are in the final tree of triage/p1-harness-tests.
+.config/nextest.toml	7	tests-disruption-handshake-7	T143	The slow-timeout rationale no longer cites the deleted inter-process tests.
+design/rumors-frame-fuzz.md	245	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 4: the design doc's description of the disruption harness no longer names the inter-process family or real TCP; it names cuts and mid-stream vanishes.
+proptest-regressions/disruption.txt	7	tests-disruption-handshake-7	T143	Five seed rows deleted (the four `ProcPlan` counterexamples the reconstructed tests carried and the one persisted during this lane's hung-drain iteration): every one belongs to `inter_process_disruption_upholds_party_invariants`, which no longer exists, so the never-strip rule is honored, not bent. The two `Plan` rows (the intra-process family) stay; `tests/seed_liveness.rs` passes.
+proptest-regressions/shadow_validity.txt	15	tests-observation-28	T144	The stop's shrunk counterexample, committed at proptest's resolved path; it replays first and passes under the aligned model.
+src/conformance/link/tests.rs	10	conformance-18	T21	`pin` leaves the imports with the deleted Ready-only acceptor, whose accept pinned its inner future; nothing else here used it.
+src/conformance/link/tests.rs	24	conformance-18	T21	The direction the entry names: conformance tests already depend on `crate::testing`; the shared decorator is imported from there. `pin` left the import list with the deleted acceptor.
+src/conformance/link/tests.rs	41	conformance-18	T21	Deletion annotated at the line that follows it: `ReversingAcceptor` (the Ready-only drain) and `reversing` are gone; `with_acceptor` stays for the lossy fixture. `grep -rn ReversingAcceptor src` is empty.
+src/conformance/link/tests.rs	62	conformance-18	T21	Nit swept: the batch depth both reordering tests pass is named once instead of a bare 3 at each call, matching the proxy suite's constant of the same name.
+src/conformance/link/tests.rs	68	conformance-18	T21	fresh-eyes prose pass: shorter; `genuinely` twice and the pass-through clause tightened.
+src/conformance/link/tests.rs	80	conformance-18	T21	`reordered_accepts_conform` now decorates both memory ends with the shared `ReorderingAcceptor` (patient wait); its `reordered > 0` verdict holds (`PASS`), so per T21 the duplicate unifies rather than stays.
+src/conformance/link/tests.rs	851	conformance-18	T21	`reordering_acceptor_passes_independence` likewise; its `reordered > 0` verdict holds under the patient wait. The negative controls in this file do not use the reordering fixture and keep their verdicts (whole `conformance::` lib suite green).
+src/testing.rs	376	testing-infra-4	T28	deviation from the lane's hazard list (src/testing.rs is not among its two src/ files), isolated in its own commit so it can be dropped alone; the entry's Where and Resolution name this file, and the lane's Goal names the poll-budget verdict. fresh-eyes repair 4: the comment keeps the band (more than an order of magnitude above the deepest closed-world run, the link conformance suite at one-byte windows) and drops the one-off point counts; the sweep that produced them is described in commit d9bc1809's message.
+src/testing.rs	423	testing-infra-4	T28	fresh-eyes prose pass: test doc cut to its invariant; the motivation paragraph dropped.
+src/testing.rs	426	testing-infra-4	T28	The resolution's test verbatim in substance: a perpetually self-waking `poll_fn` returns `Err(Quiescence::PollBudget)`; a million polls take about 50 ms. negative control: the entry's construction (the guard returning `Stalled`) fails it with `left: Err(Stalled) / right: Err(PollBudget)`; output in the commit message.
+src/testing/transport.rs	666	testing-infra-12	T21	The decorator's doc now cites its own witness and states that the conformance suite runs under this same decorator; the sibling-duplicate paragraph is gone with the duplicate (conformance-18).
+src/testing/transport.rs	728	conformance-18	T21	`yield_once` doc restated without the `ReversingAcceptor` reference: the remaining reason for two copies is the feature seam (this crate-internal module must not depend on the public `conformance` feature), which still holds.
+src/testing/transport.rs	729	conformance-18	T21	fresh-eyes prose pass: the coined `seam` (T49) removed; the reason for the copy stated in one sentence.
+src/testing/transport.rs	816	testing-infra-12	T21	fresh-eyes prose pass: the witness doc states its invariant first and the mechanism below it.
+src/testing/transport.rs	823	testing-infra-12	T21	The resolution's (a): a `memory()` pair, one acceptor wrapped with batch 2, `join!` of a connector task and an acceptor task under `run_to_quiescence`; the connector yields four times between its two connects so the second arrival lands only after the acceptor has held the first and begun its patient wait. Asserts release order `21` and counter 1. negative control: a pass-through `accept` fails it (and both conformance reordering tests); `REORDER_PATIENCE = 0` (the Ready-only drain) fails only this test, so the witness pins the patient wait specifically. Outputs in the commit message.
+src/tree/mirror/streaming/remote/proxy/tests.rs	514	testing-infra-12	T21	deviation from the lane's hazard list (which opens only transport.rs and conformance/link/tests.rs under src/), isolated in its own commit so it can be dropped by itself. testing-infra-12's Resolution names this doc for rewriting, and once conformance-18 deletes `ReversingAcceptor` the sentence here is a ghost reference (a hard rule). Three doc lines, no code: the proof of the inversion firing is now the unit witness plus the conformance suite running under the same decorator. The `== 0` tripwire and the topology caveat stay as T21 rules.
+tests/bookmark_causality.rs	1307	tests-disruption-handshake-7	T143	`FaultPlan` gained a field; these literals say `vanish: None` and nothing else changes for this suite.
+tests/common/fault.rs	1	tests-disruption-handshake-7	T143	Orphaned mentions of the inter-process TCP link removed; the module doc says `simulation`, singular (the generic `faulty_link` it also qualified was later merged into `faulty` by the vanish commit).
+tests/common/fault.rs	12	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 7: em-dashes replaced in the touched paragraph. Item 5: `FaultPlan::is_clean` deleted (no consumer).
+tests/common/fault.rs	25	tests-disruption-handshake-7	T143	The vanish fault, per T143: the module doc states what a vanish is and what the survivor sees (end-of-stream on open halves, refused opens, accepts that wait forever), which is what the memory link's drop semantics give once the stream supply is held open.
+tests/common/fault.rs	66	tests-disruption-handshake-7	T143	Two points. `AtFirstConnect` is the ruling's named point (after the handshake, before the first data stream); `OnStream { index, offset }` is the generated family's point, a byte offset into a data stream the endpoint opened, mid-frame like a cut. I first implemented the ruling's `a byte offset like the cuts` over the endpoint's whole write stream; a probe showed every such offset landing just after the greeting (283 to 287 bytes) parks the survivor, i.e. reaches the open item, so the generated family draws the mid-stream point and the ruling's point stays deterministic; the `arb_fault_or_vanish` doc says so.
+tests/common/fault.rs	71	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 6: `no more than offset bytes` (a stream whose total equals the offset completes without tripping).
+tests/common/fault.rs	128	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: `Driven::vanished`, the tripped signal the fired floor sums through `SimOutcome::vanished`.
+tests/common/fault.rs	141	tests-disruption-handshake-7	T143	`drive` owns the vanish: it selects the session against the point, drops the session future and the link halves without shutdown at the point, and hands back the held stream supply in `Driven` so the survivor's accepts park like a dead peer's listener; `faulty` refuses a vanishing plan so no caller can run one without a driver. Without a vanish the path is the plain session (the arm's `fault disabled` claim).
+tests/common/fault.rs	177	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: `ByteMeter` carries the endpoint's stream ledger alongside its byte budgets.
+tests/common/fault.rs	189	tests-disruption-handshake-17	T28	`ByteMeter::read` exposes the read counter (the one a `read_cut` spends) so the asymmetric witness places its one-byte-short cut on the same accounting the cut uses, as the meter's doc promises.
+tests/common/fault.rs	275	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the per-endpoint stream ledger (opened streams and bytes on each), the source of both the vanish ordinal and `ByteMeter::streams_opened`/`widest_stream`; `VanishState` no longer counts opens itself.
+tests/common/fault.rs	291	tests-disruption-handshake-7	T143	Shared trip state: the tripping operation and every later one return `Pending` with no wake, and `vanished` resolves for the driver. Stream ordinals are counted at `connect` so `OnStream` names a stream the endpoint itself opened.
+tests/common/fault.rs	400	tests-disruption-handshake-7	T143	The connector holds the vanish state (so an open at the point parks) and, from round 2, the stream ledger it assigns ordinals from; its doc says it parks once the endpoint has vanished.
+tests/common/fault.rs	414	tests-disruption-handshake-7	T143	`Clone` carries the vanish state and the ledger; the memory connector is cloned per stream open inside the session.
+tests/common/fault.rs	424	tests-disruption-handshake-7	T143	An open at the `AtFirstConnect` point trips the vanish and parks forever, and every open after a trip parks: the driver drops the session on the trip.
+tests/common/fault.rs	437	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the ordinal a new stream gets comes from the ledger (pushed here), which the stream's writer then reports its bytes to; `VanishState` no longer counts opens itself, so the ledger is the one source of stream identity.
+tests/common/fault.rs	480	tests-disruption-handshake-7	T143	Accepted streams' readers carry the vanish state so they park after a trip like every other operation of the vanished endpoint.
+tests/common/fault.rs	494	tests-disruption-handshake-7	T143	`Fuse` carries the vanish state and, for a data stream, its ordinal with the ledger (`None` is the control half); `ordinal` is what the vanish point compares against.
+tests/common/fault.rs	527	tests-disruption-handshake-7	T143	The vanish check comes before the cut check: a write at the point returns `Pending` with no wake (the driver drops the session), a write before it is admitted only up to the point, and bytes written report to the ledger (round 2).
+tests/common/fault.rs	559	tests-disruption-handshake-7	T143	Flush and shutdown park after a trip too: a session may reach them in the same poll that tripped, and nothing the vanished endpoint owns may make progress again.
+tests/common/fault.rs	584	tests-disruption-handshake-7	T143	`Cut` carries the vanish state for the same reason as `Fuse`.
+tests/common/fault.rs	604	tests-disruption-handshake-7	T143	Reads park after a trip.
+tests/common/overlap.rs	23	tests-common-12	T144	Module doc: the model forks where the session does; the guard's skip is now drift, caught by the meta-test.
+tests/common/overlap.rs	184	tests-common-12	T144	`Open` doc states that nothing forks at `Open` and where each side does.
+tests/common/overlap.rs	234	tests-observation-28	T144	From the stop artifact: the executor split so the meta-test can compare the fleet as the schedule left it (no quiescence) with `resolved_versions`; `execute_overlap_and_quiesce` is the same run plus the quiesce.
+tests/common/overlap.rs	265	tests-common-12	T144	Guard comment restated as the model's premise (per T144 and T13's `guard stays a guard`): the Open-time consequence no longer follows, so the skip is drift, not design.
+tests/common/overlap.rs	321	tests-observation-28	T144	From the stop artifact: `execute_overlap` returns the run as the schedule left it (peers, oracle, `resolved_versions`); the quiesce lives in `execute_overlap_and_quiesce`, which the properties still call.
+tests/common/overlap.rs	341	tests-observation-28	T144	The model's premise, derived from the code and stated with its derivation: side `b` forks in the first polled round (it reads `a`'s preamble in the same round), side `a` in the second. T144 says `at its first poll`; the per-side reading is what the meta-test confirms exact: with both sides at the first round (`[1, 1]`) the meta-test fails on the recorded shape, so the distinction is load-bearing (calibration run, output in the commit message).
+tests/common/overlap.rs	372	tests-observation-28	T144	From the stop artifact: the `_with_shadow` twin of the overlap strategy, surfacing the final `Knowledge`.
+tests/common/overlap.rs	570	tests-observation-28	T144	An open session in the model: rounds received and each side's view at the round it forked; `Close` polls to completion so both views exist by then.
+tests/common/overlap.rs	638	tests-observation-28	T144	`merge_session` takes one view per side instead of one snapshot; the preamble round and `Gossip` events pass the same frozen state for both, which is the old behavior for serial sessions.
+tests/common/overlap.rs	666	tests-observation-28	T144	The builder's doc states the model (each side's view at the round it forks) and the builder returns the shadow's final state for the meta-test.
+tests/common/overlap.rs	699	tests-observation-28	T144	The preamble's serial sessions fork both sides at once, so both views are the same frozen state.
+tests/common/overlap.rs	736	tests-observation-28	T144	A `Gossip` choice is a serial session: the same frozen view for both sides.
+tests/common/overlap.rs	744	tests-observation-28	T144	`Open` records an unforked `OpenSession`; `Step` advances its rounds and forks any side whose round is reached at the current state; `Close` completes and delivers it. This is where the fork moved from `Open`.
+tests/common/overlap.rs	776	tests-observation-28	T144	The implicit closing tail closes leftover sessions through the same model, and the builder returns the schedule with the shadow.
+tests/common/sim.rs	21	tests-disruption-handshake-7	T143	Module doc: the chaos phase names endpoints vanishing mid-stream; round 2 item 7 replaced the em-dash in this paragraph.
+tests/common/sim.rs	57	tests-disruption-handshake-7	T143	Loss accounting names the vanished bootstrapper or retiree among the legitimate losses; round 2 item 7 replaced the em-dashes.
+tests/common/sim.rs	77	tests-disruption-handshake-7	T143	Imports for the session deadline (`Duration`, `Future`), `Gossiped` (the survivor assertion's type), and `Vanish`.
+tests/common/sim.rs	125	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 2: two seconds (headroom over scheduling only), with `MAX_SHRINK_TIME` bounding the proptests' shrink so a deadline failure lands inside nextest's budget with its seed persisted; control in the commit message. Also the round-2 deviation: a park after a *planned vanish* is counted and aborted rather than failed, see `SimOutcome::parked`; a park with no vanish planned is a deadlock and fails by name.
+tests/common/sim.rs	140	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the ordinal bound, measured (the envelope endpoint opens 2 data streams; band [2, 4]) and pinned two-sided by `vanish_draw_spans_the_envelope_session`.
+tests/common/sim.rs	150	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the offset bound, measured (the envelope's widest data stream carries 1093 bytes; band [1093, 2186]) and pinned with the ordinal bound.
+tests/common/sim.rs	318	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2: once vanishes actually fired, the proptest parked survivors after mid-stream vanishes too (three of three runs at the first tight draw), so the open item's reach is wider than the handshake window and the family could not stay green while failing parks by name. A park after a planned vanish is therefore aborted (the survivor's task, so its handle drops and the peer reclaims) and counted here; the count is the T145 lane's assertion target. Named deviation from T143's `fails by name`, for the owner.
+tests/common/sim.rs	340	tests-disruption-handshake-3	T143	`arb_fault_within` folded back into `arb_fault`: its only consumer was the deleted child family.
+tests/common/sim.rs	370	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the draw restated to what is drawn. Even inside the pinned bounds a uniform offset fired in 3 of 503 draws over 256 deterministic plans (typical streams are far shorter than the envelope's widest), so the draw is weighted toward the first stream and its first byte, which every endpoint that opens a stream reaches: 105 of 660 fired over the same sample, 10 within the first 32 plans. `arb_fault` stays cuts-only for `bookmark_causality`.
+tests/common/sim.rs	397	tests-disruption-handshake-7	T143	Sessions and retirements draw `arb_fault_or_vanish`, the engine's own strategy, so their endpoints may vanish.
+tests/common/sim.rs	432	tests-disruption-handshake-7	T143	Bootstrapping joiners draw `arb_fault_or_vanish` too.
+tests/common/sim.rs	481	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 5: narrowed to private with `probe_disjointness` (no consumer outside this module); item 7: em-dashes replaced in the two module-doc paragraphs the T143 commits touched.
+tests/common/sim.rs	503	tests-disruption-handshake-7	T143	`ConnectionRefused` leaves the honest set with its rationale: the only transport that produced it was the deleted TCP family; over the memory link a vanished peer's dropped acceptor refuses a connect as `BrokenPipe`.
+tests/common/sim.rs	552	tests-disruption-handshake-7	T143	The survivor of a vanish never certifies: the vanish trips at a write or an open, so the counterparty's completion marker (its last write) never lands. Applied in all three faulted phases.
+tests/common/sim.rs	570	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2: the deadline distinguishes a park after a planned vanish (returned as `Parked`, tasks aborted by the caller) from a deadlock (panics by name).
+tests/common/sim.rs	597	tests-disruption-handshake-7	T143	Sessions, bootstraps, and retirements run through `drive` under `bounded`; a vanished bootstrapper counts as a possible loss like a failed one, and a vanished retiree is a recorded loss (`Transfer::Lost`, slot empty): dropping a retire future destroys the consumed peer.
+tests/common/sim.rs	653	tests-disruption-handshake-7	T143	`run_boot` runs the joiner through `drive` under `bounded`; a vanished joiner is a possible loss like a failed one and its server must end with an honest error; round 2: a server parked on a vanished joiner is aborted with the joiner and counted, the loss unchanged.
+tests/common/sim.rs	824	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 5: `probe_disjointness` is private; its only caller is `run_plan`.
+tests/common/sim.rs	853	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2: the run's vanish and park counters, reported in `SimOutcome`.
+tests/common/sim.rs	931	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2: sessions and bootstraps fold their `Vanishes` into the counters as their tasks are joined.
+tests/common/sim.rs	985	tests-disruption-handshake-7	T143	The retirement runs its two sides as spawned tasks so a park can abort both (round 2); the retiree drives through `drive`; a vanished or parked retiree is `Transfer::Lost` and one possible loss, since dropping a retire future destroys the consumed peer, and the absorber then reclaims its own `Peer` as before.
+tests/common/sim.rs	1110	tests-disruption-handshake-7	T143	`SimOutcome` carries the vanish and park counts (round 2).
+tests/common/tcp.rs	1	tests-disruption-handshake-7	T143	`common::tcp` stays: `tests/tcp_link.rs` is its consumer (checked with `git grep`). Its module doc no longer names the deleted simulation as its purpose.
+tests/disruption.rs	3	tests-disruption-handshake-7	T143	Module doc for the one remaining simulation; the inter-process paragraph left with the family.
+tests/disruption.rs	40	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 4: the section header no longer says `intra-process` (one family); item 7: the em-dash in the property's doc replaced.
+tests/disruption.rs	43	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 2: shrink time bounded on the plan proptests.
+tests/disruption.rs	78	tests-disruption-handshake-7	T143	The property's doc names vanishing endpoints among the chaos and, from round 2, the counted park after a planned vanish versus the named deadlock.
+tests/disruption.rs	134	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the floor counts vanishes that fired (`SimOutcome::vanished` over 32 deterministic plans run through the engine), replacing the drawn-only pin.
+tests/disruption.rs	189	tests-disruption-handshake-7	T143	Under the closed-world poller: a mid-stream vanish severs the survivor with an honest error (the fixture the ignored test shares).
+tests/disruption.rs	210	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 3: the dormant ignored test is a positive pin of the current behavior (`Err(Quiescence::Stalled)`), doc naming the open item, ruling T145, and the `p2-vanish-liveness` lane that flips it; the ignore reason's text is in the doc.
+tests/disruption.rs	226	tests-disruption-handshake-7	T143	Deterministic pin that a mid-stream vanish fires in a retirement and is booked as one possible loss; the party check runs first so the accounting control fails there. negative control: booking the vanished retiree as `Transfer::Committed` with no loss fails `a loss-free run must reconstitute the seed's whole id-space`; output in the commit message.
+tests/disruption.rs	277	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: two-sided pins of both draw bounds against the envelope session's measured stream shape (`envelope_session_bytes` now reports streams opened and the widest stream).
+tests/disruption.rs	563	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the envelope meter returns an `EnvelopeExtent` so the byte pin and the vanish pins share one measured session.
+tests/disruption.rs	605	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the extent takes each figure from the wider endpoint's meter, as the byte figure always did.
+tests/disruption.rs	632	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 1: the byte pin reads the extent's `bytes`; its assertions are unchanged.
+tests/disruption.rs	647	tests-disruption-handshake-7	T143	Deletion, annotated at the file's last line: T143 dissolves the inter-process family. Gone from this file: the self re-executing children and their exit protocol, `ChildPlan`/`ProcPlan`/`arb_proc_plan`, `run_proc_plan_probed` with the stop signal, the grace drain (`SERVE_GRACE`) and its loss accounting, the fault-free loss pin, and the serving-task tripwire; the module doc describes one simulation. The lane's earlier commits that built those stay in history for the packet.
+tests/disruption.rs	647	tests-disruption-handshake-3	T143	Deletion, same site: the child-cycle envelope meter (`Envelope`, `child_cycle_bytes`, `metered_connection`, `send_complement`), the generator-bound constants, `MAX_CHILD_CUT` and its two-sided pin. The intra-process `MAX_CUT` pin stays.
+tests/gossip_when.rs	48	tests-disruption-handshake-17	T28	The `fault` module is imported for `metered`, which the fixture pin and the asymmetric witness use.
+tests/gossip_when.rs	699	tests-disruption-handshake-17	T28	The named bound replacing the literal 400. Measured clean fixture: A wrote 188, B wrote 191, B read 188; 256 sits in the band [191, 382]. The literal 400 fails the pin's upper side (more than twice the session), i.e. more than half the old family's cuts landed past the session's end.
+tests/gossip_when.rs	717	tests-disruption-handshake-17	T28	The proptest body split into `run_severed` (the fixture and drivers, faults per end) and `check_severed` (the contract's assertions, unchanged in substance) so the deterministic witness runs the same drivers and the same checks as the generated family.
+tests/gossip_when.rs	807	tests-disruption-handshake-17	T28	Clean metered run of the same fixture and drivers; the in-memory link and single-threaded driver make it byte-identical across runs (the lifecycle suite's marker witness rests on the same premise), which the witness relies on.
+tests/gossip_when.rs	847	tests-disruption-handshake-17	T28	Two-sided pin. negative control: the former literal 400 fails it (`MAX_SEVER_CUT (400) is more than twice the fixture session's 191 bytes`); output in the commit message.
+tests/gossip_when.rs	864	tests-disruption-handshake-17	T28	fresh-eyes prose pass: the witness doc shortened, and its claim that write-only cuts cannot reach the case (stale now that read cuts are in the family) replaced by the accurate one: a generated read cut reaches it only by landing exactly here.
+tests/gossip_when.rs	877	tests-disruption-handshake-17	T28	The acceptance's deterministic asymmetric case: B's read budget one byte short of a clean session, A `Ok`, B no `Ok` and terminal `Error::Epilogue`, then the full `check_severed` (certification holds). I first asserted only `B ends in Err`; a control with the budget exactly at the clean count still passed because B's remote-led driver errors on its next control read after a complete session, so the assertion now pins the class and the absence of any `Ok` at B. negative control: that exact-budget cut fails the tightened witness; output in the commit message.
+tests/gossip_when.rs	884	tests-disruption-handshake-17	T143	Same field addition in the severed-connection fixture's literals.
+tests/gossip_when.rs	906	tests-disruption-handshake-17	T28	fresh-eyes prose pass: em-dashes in the moved and edited paragraphs replaced with spaced double hyphens (T48); `MAX_SEVER_CUT` and `fixture_session_bytes` docs shortened.
+tests/gossip_when.rs	923	tests-disruption-handshake-17	T28	Read cuts join the family on either side (optional, so the always-present write cuts keep their coverage), and the doc claims only what the family produces, pointing at the deterministic witness for the asymmetric case.
+tests/gossip_when.rs	1115	tests-disruption-handshake-7	T143	`vanish: None` on the poisoned-link fixture's literal: `FaultPlan` gained the field, nothing else changes here.
+tests/multi_peer.rs	15	tests-lifecycle-15	T26	Imports for the deterministic runner and the event alphabet the pin inspects.
+tests/multi_peer.rs	206	tests-lifecycle-15	T26	fresh-eyes prose pass: one-sentence invariant; motivation dropped.
+tests/multi_peer.rs	210	tests-lifecycle-15	T26	Population pin for the schedule alphabet: 64 deterministic samples of `arb_schedule` at this suite's own `N_PEERS`/`MAX_EVENTS`, counting emitted `Redact` events (every emitted one is effectual by the shadow's construction, so no positional check is needed here). negative control: `RedactObservation` weight 0 in `arb_choice` fails only this test (multi_peer: 7 passed, 1 failed); output in the commit message.
+tests/pairwise.rs	22	tests-lifecycle-15	T26	Imports for the deterministic runner and the action alphabet the pin inspects.
+tests/pairwise.rs	242	tests-lifecycle-15	T26	fresh-eyes prose pass: one-sentence invariant; the `without this pin` motivation dropped (it changes nothing the reader does).
+tests/pairwise.rs	246	tests-lifecycle-15	T26	Population pin for the action alphabet, the shape of `membership_population_contains_churn`: 64 deterministic samples of `arb_local_actions`, counting a `Redact` only when an `Insert` precedes it, since `build_local` drops a `Redact` with nothing sent. Placed beside the strategy's primary consumer as the resolution offers. negative control: `Redact` weight 0 in `arb_actions` fails only this test (pairwise: 8 passed, 1 failed); output in the commit message.
+tests/shadow_validity.rs	35	tests-observation-28	T144	Imports of the overlap strategy's shadow twin and the pre-quiescence executor for the meta-test.
+tests/shadow_validity.rs	45	tests-observation-28	T144	The meta-test samples at the overlap suite's own fleet and event bounds, so it checks the population the properties run on.
+tests/shadow_validity.rs	165	tests-observation-28	T144	The T13 validity meta-test, from the stop artifact: per peer, the overlap shadow's `observed_log` and `live` sets match the executor's at the end of the schedule. negative control: with the fork moved back to `Open` (a reversible mutation of the `Open` arm taking both views there) it fails on the recorded shape, `peer 2 observation set disagrees with the overlap shadow`; output in the commit message.
+tests/tcp_link.rs	1	tests-disruption-handshake-7	T143	fresh-eyes repair, round 2, item 4: the module doc no longer says `tests/disruption.rs` trusts this link under process kills; it states the suite's purpose in the present tense.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### .config/nextest.toml `@@ -4,8 +4,8 @@`

```diff
@@ -4,8 +4,8 @@
 # that escapes that harness (a real-executor integration test, a proptest
 # whose case never completes) would otherwise hang the run forever.
 #
-# The slowest honest tests today are the proptest suites and the
-# inter-process disruption tests, all finishing well under 60 seconds
+# The slowest honest tests today are the proptest suites, all finishing
+# well under 60 seconds
 # (the deterministic fixture search in src/tree/arb.rs is around 6
 # seconds), so three 60-second periods —
 # termination at 180 seconds — is generous headroom, and everything past
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 7:
>
> The slow-timeout rationale no longer cites the deleted inter-process tests.

<a id="hunk-3"></a>
### design/rumors-frame-fuzz.md `@@ -242,8 +242,8 @@ grows organically thereafter.`

```diff
@@ -242,8 +242,8 @@ grows organically thereafter.
 ## 5. What this deliberately does not cover
 
 - **Fault injection on honest traffic.** `tests/disruption.rs` with
-  `tests/common/fault.rs` (byte-budgeted `Fuse`/`Cut` severing, intra-
-  and inter-process, real TCP included) and the `rumors::testing`
+  `tests/common/fault.rs` (byte-budgeted `Fuse`/`Cut` severing and
+  endpoints vanishing mid-stream) and the `rumors::testing`
   transport adversity (`IoPlan`/`IoFault`: chunking, delays,
   hold-until-flush, typed injected failures at every surface) already
   sweep sessions whose *content* is honest while the transport
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 245:
>
> fresh-eyes repair, round 2, item 4: the design doc's description of the disruption harness no longer names the inter-process family or real TCP; it names cuts and mid-stream vanishes.

<a id="hunk-4"></a>
### proptest-regressions/disruption.txt `@@ -4,9 +4,5 @@`

```diff
@@ -4,9 +4,5 @@
 #
 # It is recommended to check this file in to source control so that
 # everyone who runs the test benefits from these saved cases.
-cc 819e392f40e747b523333860d7a8ec83554283bba9917cee6c51705721fb1f3a # shrinks to plan = ProcPlan { n_parent_peers: 1, seed_messages: [], children: [ChildPlan { n_sends: 0, boot: FaultPlan { write_cut: None, read_cut: None }, sessions: [FaultPlan { write_cut: None, read_cut: None }], retire: FaultPlan { write_cut: Some(0), read_cut: None } }] }
-cc 77e3022b1bac70f21fe51a76afec5dd6387ff0fa27c10e186a2e81835977ed96 # shrinks to plan = ProcPlan { n_parent_peers: 1, seed_messages: [8910283091], children: [ChildPlan { n_sends: 2, boot: FaultPlan { write_cut: Some(1198), read_cut: None }, sessions: [FaultPlan { write_cut: Some(124), read_cut: None }, FaultPlan { write_cut: Some(1308), read_cut: Some(935) }], retire: FaultPlan { write_cut: None, read_cut: None } }] }
-cc 5b93a51b89ebae83ef4adf470c470a6042a892610fcad5a9134254910f89cf41 # shrinks to plan = ProcPlan { n_parent_peers: 1, seed_messages: [16893878652516216069, 17088246115921829969], children: [ChildPlan { n_sends: 1, boot: FaultPlan { write_cut: None, read_cut: Some(595) }, sessions: [FaultPlan { write_cut: None, read_cut: None }], retire: FaultPlan { write_cut: None, read_cut: Some(1243) } }, ChildPlan { n_sends: 2, boot: FaultPlan { write_cut: Some(1733), read_cut: Some(1980) }, sessions: [FaultPlan { write_cut: Some(151), read_cut: Some(348) }], retire: FaultPlan { write_cut: Some(1390), read_cut: None } }, ChildPlan { n_sends: 2, boot: FaultPlan { write_cut: Some(637), read_cut: Some(259) }, sessions: [FaultPlan { write_cut: None, read_cut: None }, FaultPlan { write_cut: Some(1206), read_cut: Some(1750) }, FaultPlan { write_cut: Some(954), read_cut: Some(25) }], retire: FaultPlan { write_cut: Some(98), read_cut: Some(1600) } }] }
-cc b7d223676a87f828738eae497654525b6a6e9c94894139b54b22011087233128 # shrinks to plan = ProcPlan { n_parent_peers: 1, seed_messages: [597761422003064892], children: [ChildPlan { n_sends: 4, boot: FaultPlan { write_cut: None, read_cut: Some(670) }, sessions: [FaultPlan { write_cut: Some(559), read_cut: None }, FaultPlan { write_cut: Some(2047), read_cut: None }], retire: FaultPlan { write_cut: Some(1947), read_cut: Some(1274) } }, ChildPlan { n_sends: 0, boot: FaultPlan { write_cut: None, read_cut: Some(1943) }, sessions: [FaultPlan { write_cut: None, read_cut: None }, FaultPlan { write_cut: None, read_cut: None }, FaultPlan { write_cut: Some(1489), read_cut: None }], retire: FaultPlan { write_cut: Some(695), read_cut: None } }, ChildPlan { n_sends: 5, boot: FaultPlan { write_cut: None, read_cut: None }, sessions: [FaultPlan { write_cut: None, read_cut: Some(1511) }, FaultPlan { write_cut: None, read_cut: None }, FaultPlan { write_cut: None, read_cut: Some(28) }], retire: FaultPlan { write_cut: Some(1124), read_cut: None } }] }
 cc 56668f71577e6e4d4df162330dc9ae28f35f4227f9b58968db160f18c6b92001 # shrinks to plan = Plan { n_peers: 5, seed_messages: [4258395568045126072, 74367208093149826, 4472911267720677566, 180972588182446959, 10798433443228348305, 2841452241123389561], faulty_boots: [FaultPlan { write_cut: None, read_cut: Some(2547) }], scripts: [[], [Redact(8), Redact(7), Send(8158681924227087528), Send(1551099043778874063), Redact(46)], [Send(11662032558820434527), Send(10531859297509817618), Redact(21), Send(13826850872425152894), Redact(58), Send(12856870337228783968), Redact(30)], [], [Send(7077125294277833408), Redact(38)]], sessions: [Session { a: 2, b: 4, fault_a: FaultPlan { write_cut: Some(1212), read_cut: None }, fault_b: FaultPlan { write_cut: Some(7), read_cut: Some(54) } }, Session { a: 3, b: 0, fault_a: FaultPlan { write_cut: Some(2076), read_cut: None }, fault_b: FaultPlan { write_cut: Some(708), read_cut: Some(3037) } }, Session { a: 2, b: 3, fault_a: FaultPlan { write_cut: Some(2077), read_cut: None }, fault_b: FaultPlan { write_cut: Some(1612), read_cut: None } }, Session { a: 0, b: 2, fault_a: FaultPlan { write_cut: Some(824), read_cut: None }, fault_b: FaultPlan { write_cut: Some(382), read_cut: Some(2392) } }, Session { a: 2, b: 1, fault_a: FaultPlan { write_cut: None, read_cut: None }, fault_b: FaultPlan { write_cut: Some(4), read_cut: None } }, Session { a: 3, b: 2, fault_a: FaultPlan { write_cut: None, read_cut: None }, fault_b: FaultPlan { write_cut: Some(904), read_cut: None } }, Session { a: 3, b: 0, fault_a: FaultPlan { write_cut: Some(2013), read_cut: Some(1725) }, fault_b: FaultPlan { write_cut: Some(2150), read_cut: None } }, Session { a: 3, b: 2, fault_a: FaultPlan { write_cut: None, read_cut: Some(2118) }, fault_b: FaultPlan { write_cut: None, read_cut: None } }, Session { a: 3, b: 2, fault_a: FaultPlan { write_cut: None, read_cut: Some(856) }, fault_b: FaultPlan { write_cut: None, read_cut: Some(809) } }, Session { a: 3, b: 1, fault_a: FaultPlan { write_cut: None, read_cut: Some(2420) }, fault_b: FaultPlan { write_cut: Some(643), read_cut: None } }, Session { a: 1, b: 3, fault_a: FaultPlan { write_cut: Some(2737), read_cut: Some(836) }, fault_b: FaultPlan { write_cut: Some(2315), read_cut: Some(2691) } }, Session { a: 3, b: 0, fault_a: FaultPlan { write_cut: Some(1252), read_cut: None }, fault_b: FaultPlan { write_cut: None, read_cut: None } }, Session { a: 0, b: 1, fault_a: FaultPlan { write_cut: None, read_cut: Some(532) }, fault_b: FaultPlan { write_cut: None, read_cut: Some(2727) } }], retires: [RetireOp { retiree: 3, absorber: 0, fault: FaultPlan { write_cut: None, read_cut: None } }, RetireOp { retiree: 2, absorber: 4, fault: FaultPlan { write_cut: None, read_cut: Some(1568) } }], windows: WindowAssignment([Budget(16777625), Budget(2097177), Budget(16780284), Floor, Default]) }
 cc f4118e1f4a87056230eecd9f58dfb5a87109cc99c687d5fc0a924704f9bec891 # shrinks to plan = Plan { n_peers: 2, seed_messages: [], faulty_boots: [], scripts: [[], []], sessions: [Session { a: 0, b: 1, fault_a: FaultPlan { write_cut: Some(0), read_cut: None }, fault_b: FaultPlan { write_cut: None, read_cut: None } }], retires: [], windows: WindowAssignment([Floor]) }
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 7:
>
> Five seed rows deleted (the four `ProcPlan` counterexamples the reconstructed tests carried and the one persisted during this lane's hung-drain iteration): every one belongs to `inter_process_disruption_upholds_party_invariants`, which no longer exists, so the never-strip rule is honored, not bent. The two `Plan` rows (the intra-process family) stay; `tests/seed_liveness.rs` passes.

<a id="hunk-5"></a>
### proptest-regressions/shadow_validity.txt `@@ -12,3 +12,4 @@ cc 40decade2ad8d7a6500bd2b280227516adf6747904929c0173343dff73428fff # shrinks to`

```diff
@@ -12,3 +12,4 @@ cc 40decade2ad8d7a6500bd2b280227516adf6747904929c0173343dff73428fff # shrinks to
 # ruling. Proptest reads only the hash before the first '#' on a cc line,
 # so this comment and the stale shrink note cost nothing at replay.
 cc 86723fa839f009875eafd473501eceafe68d5e6b00c28572965fb6fa4da8a381 # shrinks to (schedule, shadow_observed) = (Schedule { n_peers: 3, events: [Insert { peer: 0, value: 0 }, Gossip { a: 1, b: 0 }, Redact { peer: 0, target_event_idx: 0 }, Gossip { a: 2, b: 0 }, Gossip { a: 2, b: 1 }] }, [[0], [0], [0]])
+cc 3043e400b3fc6613642788bd0e9097db5cab996ee4eda7734dc3bab92ece4dc8 # shrinks to (schedule, shadow) = (OverlapSchedule { n_peers: 4, fork_parents: [0, 0, 0, 0], events: [Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Insert { peer: 0, value: 0 }, Gossip { a: 0, b: 1 }, Gossip { a: 0, b: 2 }, Gossip { a: 0, b: 3 }, Gossip { a: 1, b: 2 }, Gossip { a: 1, b: 3 }, Gossip { a: 2, b: 3 }, Open { slot: 2, a: 0, b: 1 }, Redact { peer: 3, target_event_idx: 8 }, Open { slot: 0, a: 2, b: 3 }, Insert { peer: 3, value: 4322033825067499434 }, Gossip { a: 1, b: 0 }, Step { slot: 2, polls: 9 }, Gossip { a: 0, b: 1 }, Insert { peer: 2, value: 150413318606988008 }, Open { slot: 1, a: 0, b: 1 }, Step { slot: 1, polls: 6 }, Gossip { a: 0, b: 2 }, Close { slot: 1 }, Redact { peer: 1, target_event_idx: 6 }, Redact { peer: 1, target_event_idx: 11 }, Insert { peer: 3, value: 3587337477721592521 }, Close { slot: 0 }] }, Knowledge { ever_known: [{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25}, {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11}, {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25}, {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 21, 32}], live: [{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25}, {0, 1, 2, 3, 4, 5, 7, 8, 9, 10}, {0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 25}, {0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 21, 32}], observed_log: [[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 25], [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 21, 32]] })
```

<!-- annotation -->
> **tests-observation-28** (T144), line 15:
>
> The stop's shrunk counterexample, committed at proptest's resolved path; it replays first and passes under the aligned model.

<a id="hunk-6"></a>
### src/conformance/link/tests.rs `@@ -7,7 +7,7 @@`

```diff
@@ -7,7 +7,7 @@
 
 use std::collections::{HashMap, VecDeque};
 use std::io;
-use std::pin::{Pin, pin};
+use std::pin::Pin;
 use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
 use std::sync::{Arc, Mutex};
 use std::task::{Context, Poll, Waker};
```

<!-- annotation -->
> **conformance-18** (T21), line 10:
>
> `pin` leaves the imports with the deleted Ready-only acceptor, whose accept pinned its inner future; nothing else here used it.

<a id="hunk-7"></a>
### src/conformance/link/tests.rs `@@ -21,7 +21,7 @@ use crate::link::{`

```diff
@@ -21,7 +21,7 @@ use crate::link::{
     Acceptor, Connector, Done, Link, LinkParts, MemoryAcceptor, MemoryConnector, MemoryLink,
     STREAM_COUNT, memory, memory_with_capacity,
 };
-use crate::testing::{Quiescence, run_to_quiescence};
+use crate::testing::{Quiescence, reorder_accepts, run_to_quiescence};
 
 /// The reference instantiation passes the whole suite under the
 /// deterministic closed-world driver.
```

<!-- annotation -->
> **conformance-18** (T21), line 24:
>
> The direction the entry names: conformance tests already depend on `crate::testing`; the shared decorator is imported from there. `pin` left the import list with the deleted acceptor.

<a id="hunk-8"></a>
### src/conformance/link/tests.rs `@@ -38,58 +38,6 @@ fn one_byte_windows_conform() {`

```diff
@@ -38,58 +38,6 @@ fn one_byte_windows_conform() {
         .expect("the suite stays live at one-byte windows");
 }
 
-/// An acceptor that delivers arrivals in batches of reversed order.
-///
-/// Legal under the contract — arrival order is the transport's own, and
-/// no cross-stream ordering may be assumed — so the protocol must
-/// tolerate it: the session's claim table pairs streams by label, not
-/// position. Each released batch of two or more is a genuine inversion,
-/// counted into the shared `reordered` counter; tests assert it is
-/// nonzero, so degeneration to pass-through (reordering nothing) fails
-/// loudly instead of silently.
-struct ReversingAcceptor<A: Acceptor> {
-    inner: A,
-    held: VecDeque<(A::Rx, Done<A::Rx>)>,
-    /// Arrivals buffered before each reversed release.
-    batch: usize,
-    /// Batches of two or more released: genuine inversions.
-    reordered: Arc<AtomicUsize>,
-}
-
-impl<A: Acceptor> Acceptor for ReversingAcceptor<A> {
-    type Rx = A::Rx;
-
-    async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
-        if let Some(held) = self.held.pop_front() {
-            return Ok(held);
-        }
-        // Await one arrival, then swallow whatever else is immediately
-        // ready — without blocking, so a lone stream still flows — and
-        // release the accumulated batch newest-first.
-        let first = self.inner.accept().await?;
-        self.held.push_front(first);
-        for _ in 1..self.batch {
-            let mut next = pin!(self.inner.accept());
-            let waker = futures::task::noop_waker();
-            let mut cx = Context::from_waker(&waker);
-            match next.as_mut().poll(&mut cx) {
-                Poll::Ready(Ok(rx)) => self.held.push_front(rx),
-                // Pending or errored: stop batching and release what is
-                // held. Swallowing an error here is sound for the wrapped
-                // `MemoryAcceptor`, whose errors are persistent (a closed
-                // channel errors on every later recv, so the next accept
-                // resurfaces it); the fixture is not built for acceptors
-                // with one-shot errors.
-                _ => break,
-            }
-        }
-        if self.held.len() > 1 {
-            self.reordered.fetch_add(1, Ordering::Relaxed);
-        }
-        Ok(self.held.pop_front().expect("at least one arrival is held"))
-    }
-}
-
 /// Decorate one memory end's acceptor, preserving every other part — the
 /// session state included, so the wrapped link stays in lockstep with its
 /// peer.
```

<!-- annotation -->
> **conformance-18** (T21), line 41:
>
> Deletion annotated at the line that follows it: `ReversingAcceptor` (the Ready-only drain) and `reversing` are gone; `with_acceptor` stays for the lossy fixture. `grep -rn ReversingAcceptor src` is empty.

<a id="hunk-9"></a>
### src/conformance/link/tests.rs `@@ -108,29 +56,20 @@ fn with_acceptor<A: Acceptor>(`

```diff
@@ -108,29 +56,20 @@ fn with_acceptor<A: Acceptor>(
     .into_link()
 }
 
-/// Reorder one memory end's arrivals in reversed batches, counting genuine
-/// inversions into `reordered`.
-fn reversing(
-    link: MemoryLink,
-    batch: usize,
-    reordered: Arc<AtomicUsize>,
-) -> Link<DuplexStream, DuplexStream, MemoryConnector, ReversingAcceptor<MemoryAcceptor>> {
-    with_acceptor(link, |inner| ReversingAcceptor {
-        inner,
-        held: VecDeque::new(),
-        batch,
-        reordered,
-    })
-}
+/// Arrivals the reordering acceptor holds before each newest-first
+/// release: deep enough to invert bursts, small enough never to starve a
+/// lone stream.
+const REORDER_BATCH: usize = 3;
 
 /// Worst-case accept reordering is adversarial but legal: streams are
 /// anonymous and arrival order is the transport's own, so the whole suite
 /// — the focused probes included — must stay live and convergent under it.
 ///
-/// The final assertion proves the adversity fired: at least one batch was
-/// genuinely released in inverted order somewhere across the suite, so a
-/// pass certifies tolerance of real reordering, not of a decorator that
-/// silently degenerated to pass-through.
+/// The adversity is the crate's `ReorderingAcceptor`, which holds each
+/// arrival and waits a bounded budget of yields for company before
+/// releasing the batch newest-first. The final assertion proves it fired
+/// somewhere across the suite, so a pass certifies tolerance of real
+/// reordering, not of a decorator degenerated to pass-through.
 #[test]
 fn reordered_accepts_conform() {
     let reordered = Arc::new(AtomicUsize::new(0));
```

<!-- annotation -->
> **conformance-18** (T21), line 62:
>
> Nit swept: the batch depth both reordering tests pass is named once instead of a bare 3 at each call, matching the proxy suite's constant of the same name.

<!-- annotation -->
> **conformance-18** (T21), line 68:
>
> fresh-eyes prose pass: shorter; `genuinely` twice and the pass-through clause tightened.

<a id="hunk-10"></a>
### src/conformance/link/tests.rs `@@ -138,8 +77,8 @@ fn reordered_accepts_conform() {`

```diff
@@ -138,8 +77,8 @@ fn reordered_accepts_conform() {
     run_to_quiescence(super::check(async || {
         let (a, b) = memory();
         (
-            reversing(a, 3, counter.clone()),
-            reversing(b, 3, counter.clone()),
+            reorder_accepts(a, REORDER_BATCH, counter.clone()),
+            reorder_accepts(b, REORDER_BATCH, counter.clone()),
         )
     }))
     .expect("the suite stays live under reordered accepts");
```

<!-- annotation -->
> **conformance-18** (T21), line 80:
>
> `reordered_accepts_conform` now decorates both memory ends with the shared `ReorderingAcceptor` (patient wait); its `reordered > 0` verdict holds (`PASS`), so per T21 the duplicate unifies rather than stays.

<a id="hunk-11"></a>
### src/conformance/link/tests.rs `@@ -909,8 +848,8 @@ fn reordering_acceptor_passes_independence() {`

```diff
@@ -909,8 +848,8 @@ fn reordering_acceptor_passes_independence() {
     let reordered = Arc::new(AtomicUsize::new(0));
     let (a, b) = memory();
     run_to_quiescence(super::check_independence(
-        reversing(a, 3, reordered.clone()),
-        reversing(b, 3, reordered.clone()),
+        reorder_accepts(a, REORDER_BATCH, reordered.clone()),
+        reorder_accepts(b, REORDER_BATCH, reordered.clone()),
     ))
     .expect("independence stays live under reordered accepts");
     assert!(
```

<!-- annotation -->
> **conformance-18** (T21), line 851:
>
> `reordering_acceptor_passes_independence` likewise; its `reordered > 0` verdict holds under the patient wait. The negative controls in this file do not use the reordering fixture and keep their verdicts (whole `conformance::` lib suite green).

<a id="hunk-12"></a>
### src/testing.rs `@@ -373,6 +373,10 @@ impl Wake for WakeFlag {`

```diff
@@ -373,6 +373,10 @@ impl Wake for WakeFlag {
 /// this detector from within a Tokio task cannot turn a scheduler yield into a
 /// false deadlock report.
 pub fn run_to_quiescence<F: Future>(future: F) -> Result<F::Output, Quiescence> {
+    // More than an order of magnitude above the deepest closed-world run
+    // the suites drive (the whole link conformance suite at one-byte
+    // windows), so a legitimate long session is never misreported as a
+    // runaway. Re-measure that run before lowering this.
     const MAX_POLLS: usize = 1_000_000;
 
     let wake = Arc::new(WakeFlag(AtomicBool::new(true)));
```

<!-- annotation -->
> **testing-infra-4** (T28), line 376:
>
> deviation from the lane's hazard list (src/testing.rs is not among its two src/ files), isolated in its own commit so it can be dropped alone; the entry's Where and Resolution name this file, and the lane's Goal names the poll-budget verdict. fresh-eyes repair 4: the comment keeps the band (more than an order of magnitude above the deepest closed-world run, the link conformance suite at one-byte windows) and drops the one-off point counts; the sweep that produced them is described in commit d9bc1809's message.

<a id="hunk-13"></a>
### src/testing.rs `@@ -416,6 +420,17 @@ mod tests {`

```diff
@@ -416,6 +420,17 @@ mod tests {
         );
     }
 
+    /// A future that self-wakes on every poll without ever completing
+    /// exhausts the poll budget and is reported as such, not as a stall.
+    #[test]
+    fn runaway_self_waking_exhausts_the_poll_budget() {
+        let runaway = std::future::poll_fn(|cx: &mut Context<'_>| {
+            cx.waker().wake_by_ref();
+            Poll::<()>::Pending
+        });
+        assert_eq!(run_to_quiescence(runaway), Err(Quiescence::PollBudget));
+    }
+
     /// An inherited Tokio task budget cannot masquerade as protocol quiescence.
     #[tokio::test(flavor = "current_thread")]
     async fn ignores_tokio_cooperative_yields() {
```

<!-- annotation -->
> **testing-infra-4** (T28), line 423:
>
> fresh-eyes prose pass: test doc cut to its invariant; the motivation paragraph dropped.

<!-- annotation -->
> **testing-infra-4** (T28), line 426:
>
> The resolution's test verbatim in substance: a perpetually self-waking `poll_fn` returns `Err(Quiescence::PollBudget)`; a million polls take about 50 ms. negative control: the entry's construction (the guard returning `Stalled`) fails it with `left: Err(Stalled) / right: Err(PollBudget)`; output in the commit message.

<a id="hunk-14"></a>
### src/testing/transport.rs `@@ -663,10 +663,12 @@ const REORDER_PATIENCE: u8 = 32;`

```diff
@@ -663,10 +663,12 @@ const REORDER_PATIENCE: u8 = 32;
 /// decorator that only drains arrivals already `Ready` never sees a second
 /// arrival under the deterministic scheduler and silently degenerates to
 /// pass-through — which is exactly what the asserted counter makes loud.
+/// The unit witness `reordering_acceptor_inverts_a_patient_batch` in this
+/// module's tests shows the wait forming a batch of two from an arrival
+/// that lands only after the first has been held.
 ///
-/// A sibling of the conformance suite's `ReversingAcceptor`
-/// (`src/conformance/link/tests.rs`), duplicated so this crate-internal seam
-/// does not depend on the public `conformance` feature.
+/// The link conformance suite decorates its memory ends with this
+/// acceptor too, so a conformance pass under reordering certifies it.
 pub struct ReorderingAcceptor<A: crate::link::Acceptor> {
     inner: A,
     held: VecDeque<(A::Rx, Done<A::Rx>)>,
```

<!-- annotation -->
> **testing-infra-12** (T21), line 666:
>
> The decorator's doc now cites its own witness and states that the conformance suite runs under this same decorator; the sibling-duplicate paragraph is gone with the duplicate (conformance-18).

<a id="hunk-15"></a>
### src/testing/transport.rs `@@ -724,9 +726,8 @@ impl<A: crate::link::Acceptor> crate::link::Acceptor for ReorderingAcceptor<A> {`

```diff
@@ -724,9 +726,8 @@ impl<A: crate::link::Acceptor> crate::link::Acceptor for ReorderingAcceptor<A> {
 /// self-wake.
 ///
 /// Runtime-agnostic (the deterministic driver is no runtime at all), unlike
-/// `tokio::task::yield_now`; a copy of `conformance`'s helper, on the same
-/// feature seam that keeps [`ReorderingAcceptor`] separate from its
-/// `ReversingAcceptor` sibling.
+/// `tokio::task::yield_now`. The `conformance` module keeps its own copy:
+/// this module must not depend on the public `conformance` feature.
 async fn yield_once() {
     let mut yielded = false;
     std::future::poll_fn(|cx| {
```

<!-- annotation -->
> **conformance-18** (T21), line 728:
>
> `yield_once` doc restated without the `ReversingAcceptor` reference: the remaining reason for two copies is the feature seam (this crate-internal module must not depend on the public `conformance` feature), which still holds.

<!-- annotation -->
> **conformance-18** (T21), line 729:
>
> fresh-eyes prose pass: the coined `seam` (T49) removed; the reason for the copy stated in one sentence.

<a id="hunk-16"></a>
### src/testing/transport.rs `@@ -800,12 +801,68 @@ fn suspend(`

```diff
@@ -800,12 +801,68 @@ fn suspend(
 
 #[cfg(test)]
 mod tests {
+    use std::sync::{
+        Arc,
+        atomic::{AtomicUsize, Ordering},
+    };
+
     use futures::{pin_mut, poll};
     use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex, split};
 
-    use super::{IoPlan, Side, wrap_io};
+    use super::{IoPlan, Side, reorder_accepts, wrap_io, yield_once};
+    use crate::link::{Acceptor, Connector, memory};
     use crate::testing::run_to_quiescence;
 
+    /// The patient wait forms a batch of two and releases it newest-first,
+    /// counting one inversion.
+    ///
+    /// The second stream is connected only after the acceptor has held the
+    /// first and begun yielding, so a drain of only-`Ready` arrivals would
+    /// release the first alone and count nothing.
+    #[test]
+    fn reordering_acceptor_inverts_a_patient_batch() {
+        let reordered = Arc::new(AtomicUsize::new(0));
+        let (a, b) = memory();
+        let mut acceptor = reorder_accepts(a, 2, reordered.clone())
+            .into_parts()
+            .acceptor;
+        let connector = b.into_parts().connector;
+        let released = run_to_quiescence(async {
+            let (_streams, released) = futures::join!(
+                async {
+                    let (mut first, _done) = connector.connect().await.unwrap();
+                    first.write_all(b"1").await.unwrap();
+                    // Let the acceptor take the first arrival and start
+                    // waiting before the second exists.
+                    for _ in 0..4 {
+                        yield_once().await;
+                    }
+                    let (mut second, _done) = connector.connect().await.unwrap();
+                    second.write_all(b"2").await.unwrap();
+                    (first, second)
+                },
+                async {
+                    let mut released = Vec::new();
+                    for _ in 0..2 {
+                        let (mut rx, _done) = acceptor.accept().await.unwrap();
+                        let mut tag = [0u8; 1];
+                        rx.read_exact(&mut tag).await.unwrap();
+                        released.push(tag[0]);
+                    }
+                    released
+                },
+            );
+            released
+        })
+        .expect("the batch releases and the harness stays live");
+        assert_eq!(released, b"21", "the batch of two releases newest-first");
+        assert_eq!(
+            reordered.load(Ordering::Relaxed),
+            1,
+            "one batch of two is one recorded inversion"
+        );
+    }
+
     /// Flush buffering keeps completed writes invisible to the peer until the
     /// corresponding flush is polled.
     #[test]
```

<!-- annotation -->
> **testing-infra-12** (T21), line 816:
>
> fresh-eyes prose pass: the witness doc states its invariant first and the mechanism below it.

<!-- annotation -->
> **testing-infra-12** (T21), line 823:
>
> The resolution's (a): a `memory()` pair, one acceptor wrapped with batch 2, `join!` of a connector task and an acceptor task under `run_to_quiescence`; the connector yields four times between its two connects so the second arrival lands only after the acceptor has held the first and begun its patient wait. Asserts release order `21` and counter 1. negative control: a pass-through `accept` fails it (and both conformance reordering tests); `REORDER_PATIENCE = 0` (the Ready-only drain) fails only this test, so the witness pins the patient wait specifically. Outputs in the commit message.

<a id="hunk-17"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -511,8 +511,10 @@ proptest! {`

```diff
@@ -511,8 +511,10 @@ proptest! {
 /// empirically: zero batches across the run, at any patience budget and at
 /// one-byte and 37-byte windows alike), so as exercised this property pins
 /// no more than [`reconcile_symmetric_accepts`]; the decorator's inversion
-/// genuinely firing is proven instead by the conformance suite's
-/// `ReversingAcceptor` tests, whose probes connect streams concurrently.
+/// genuinely firing is proven instead by its unit witness in
+/// `testing::transport` (a batch of two formed by the patient wait) and by
+/// the link conformance suite, which runs under the same decorator with
+/// its probes connecting streams concurrently.
 ///
 /// The final assertion is the tripwire keeping this caveat honest: if the
 /// topology ever admits a genuine inversion, it fails, and this doc's
```

<!-- annotation -->
> **testing-infra-12** (T21), line 514:
>
> deviation from the lane's hazard list (which opens only transport.rs and conformance/link/tests.rs under src/), isolated in its own commit so it can be dropped by itself. testing-infra-12's Resolution names this doc for rewriting, and once conformance-18 deletes `ReversingAcceptor` the sentence here is a ghost reference (a hard rule). Three doc lines, no code: the proof of the inversion firing is now the unit witness plus the conformance suite running under the same decorator. The `== 0` tripwire and the topology caveat stay as T21 rules.

<a id="hunk-18"></a>
### tests/bookmark_causality.rs `@@ -1304,10 +1304,12 @@ fn reconstructed_cut_gossip_then_retire_under_bookmark_faults() {`

```diff
@@ -1304,10 +1304,12 @@ fn reconstructed_cut_gossip_then_retire_under_bookmark_faults() {
                 FaultPlan {
                     write_cut: Some(1324),
                     read_cut: None,
+                    vanish: None,
                 },
                 FaultPlan {
                     write_cut: None,
                     read_cut: Some(134),
+                    vanish: None,
                 },
             ),
             Step::Retire(1, 2),
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 1307:
>
> `FaultPlan` gained a field; these literals say `vanish: None` and nothing else changes for this suite.

<a id="hunk-19"></a>
### tests/common/fault.rs `@@ -1,7 +1,7 @@`

```diff
@@ -1,7 +1,7 @@
-//! Wire-fault injection for the disruption simulations: deterministic,
+//! Wire-fault injection for the disruption simulation: deterministic,
 //! byte-budgeted severing of either direction of a gossip link.
 //!
-//! A "dropped connection" in the simulations is one or both directions of a
+//! A "dropped connection" in the simulation is one or both directions of a
 //! [`rumors::Link`] tripping at an arbitrary byte offset mid-session:
 //!
 //! - [`Fuse`] forwards writes until its budget is exhausted, then fails
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 1:
>
> Orphaned mentions of the inter-process TCP link removed; the module doc says `simulation`, singular (the generic `faulty_link` it also qualified was later merged into `faulty` by the vanish commit).

<a id="hunk-20"></a>
### tests/common/fault.rs `@@ -9,8 +9,8 @@`

```diff
@@ -9,8 +9,8 @@
 //! - [`Cut`] forwards reads until its budget is exhausted, then fails every
 //!   read with [`ConnectionReset`] — the connection died under our eyes.
 //!
-//! Each budget is shared across every stream of its direction — the control
-//! half and each data stream draw on one counter — so the cut lands at a
+//! Each budget is shared across every stream of its direction -- the control
+//! half and each data stream draw on one counter -- so the cut lands at a
 //! chosen offset in the endpoint's total traffic, wherever that byte
 //! happens to travel. A severed direction also refuses new streams: once
 //! its budget is exhausted, [`FaultConnector::connect`] fails alongside the
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 12:
>
> fresh-eyes repair, round 2, item 7: em-dashes replaced in the touched paragraph. Item 5: `FaultPlan::is_clean` deleted (no consumer).

<a id="hunk-21"></a>
### tests/common/fault.rs `@@ -20,13 +20,23 @@`

```diff
@@ -20,13 +20,23 @@
 //! counterparty observes it as end-of-stream (and a truncated frame) once
 //! the failing side's link drops, or as its own write error against the
 //! closed transport. Either way the session dies somewhere the protocol did
-//! not choose, which is exactly the disruption the simulations are after.
+//! not choose, which is exactly the disruption the simulation is after.
+//!
+//! A *vanish* is the other way a peer dies: not a wire error it can
+//! observe, but the peer itself gone mid-protocol, as a crashed process
+//! is. At its [`Vanish`] point the endpoint's session is dropped where it
+//! stands, its link halves with it and without any shutdown, and every
+//! stream it owes is never opened. Its counterparty sees exactly what a
+//! vanished socket gives: end-of-stream on what was open, a refused open
+//! toward the dead peer, and an accept that waits forever for a stream
+//! nobody will open. [`drive`] runs a session under such a plan.
 //!
 //! [`BrokenPipe`]: std::io::ErrorKind::BrokenPipe
 //! [`ConnectionReset`]: std::io::ErrorKind::ConnectionReset
 
 use std::io;
 use std::pin::Pin;
+use std::sync::atomic::{AtomicBool, Ordering};
 use std::sync::{Arc, Mutex};
 use std::task::{Context, Poll};
 
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 25:
>
> The vanish fault, per T143: the module doc states what a vanish is and what the survivor sees (end-of-stream on open halves, refused opens, accepts that wait forever), which is what the memory link's drop semantics give once the stream supply is held open.

<a id="hunk-22"></a>
### tests/common/fault.rs `@@ -34,29 +44,46 @@ use rumors::link::{`

```diff
@@ -34,29 +44,46 @@ use rumors::link::{
     Acceptor, Connector, Done, Link, LinkParts, MemoryAcceptor, MemoryConnector, MemoryLink,
 };
 use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};
+use tokio::sync::Notify;
 
 /// One endpoint's fault plan: byte budgets after which its write
-/// (respectively read) direction fails. `None` means that direction never
-/// fails.
+/// (respectively read) direction fails, and the point at which the
+/// endpoint vanishes. `None` means never.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub struct FaultPlan {
     /// Bytes this endpoint may write before its writers fail.
     pub write_cut: Option<usize>,
     /// Bytes this endpoint may read before its readers fail.
     pub read_cut: Option<usize>,
+    /// Where this endpoint vanishes, if it does.
+    pub vanish: Option<Vanish>,
+}
+
+/// The point at which an endpoint vanishes: its session is dropped there
+/// without any shutdown, its link halves with it, and any stream it owes
+/// is never opened.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Vanish {
+    /// While writing the `index`-th data stream it opened, `offset` bytes
+    /// into that stream.
+    ///
+    /// The next write there is where the endpoint dies, mid-frame like a
+    /// cut. An endpoint that opens fewer streams, or writes no more than
+    /// `offset` bytes on that one, never reaches the point.
+    OnStream { index: usize, offset: usize },
+    /// At its first outgoing stream open: after the handshake, which rides
+    /// the control half, and before its first data stream.
+    AtFirstConnect,
 }
 
 impl FaultPlan {
-    /// A clean endpoint: neither direction ever fails.
+    /// A clean endpoint: neither direction ever fails, and it never
+    /// vanishes.
     pub const NONE: Self = Self {
         write_cut: None,
         read_cut: None,
+        vanish: None,
     };
-
-    /// Whether this plan injects any fault at all.
-    pub fn is_clean(&self) -> bool {
-        *self == Self::NONE
-    }
 }
 
 /// The faulted shape of one in-memory link endpoint.
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 66:
>
> Two points. `AtFirstConnect` is the ruling's named point (after the handshake, before the first data stream); `OnStream { index, offset }` is the generated family's point, a byte offset into a data stream the endpoint opened, mid-frame like a cut. I first implemented the ruling's `a byte offset like the cuts` over the endpoint's whole write stream; a probe showed every such offset landing just after the greeting (283 to 287 bytes) parks the survivor, i.e. reaches the open item, so the generated family draws the mid-stream point and the ruling's point stays deterministic; the `arb_fault_or_vanish` doc says so.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 71:
>
> fresh-eyes repair, round 2, item 6: `no more than offset bytes` (a stream whose total equals the offset completes without tripping).

<a id="hunk-23"></a>
### tests/common/fault.rs `@@ -71,8 +98,69 @@ pub type FaultyLink = Link<`

```diff
@@ -71,8 +98,69 @@ pub type FaultyLink = Link<
 ///
 /// A clean plan still wraps (with effectively-infinite budgets), so every
 /// call site handles one pair of types regardless of whether it faults.
+///
+/// # Panics
+///
+/// If the plan vanishes: a vanishing endpoint's session must be driven by
+/// [`drive`], which owns dropping it at the point.
 pub fn faulty(link: MemoryLink, plan: FaultPlan) -> FaultyLink {
-    faulty_link(link, plan)
+    assert!(
+        plan.vanish.is_none(),
+        "a vanishing endpoint's session is driven by `drive`"
+    );
+    wrap(link, plan).0
+}
+
+/// What [`drive`] leaves behind.
+///
+/// Keep it alive until the counterparty's session has ended: after a
+/// vanish it holds the vanished endpoint's stream supply open, so the
+/// counterparty's accepts wait as they would on a dead peer's listener
+/// instead of erroring on a closed supply.
+pub struct Driven<Out> {
+    /// The session's outcome, or `None` if the endpoint vanished first.
+    pub outcome: Option<Out>,
+    _supply: Option<MemoryConnector>,
+}
+
+impl<Out> Driven<Out> {
+    /// Whether the endpoint reached its vanish point.
+    pub fn vanished(&self) -> bool {
+        self.outcome.is_none()
+    }
+}
+
+/// Run `session` over `link` under `plan`, vanishing at the plan's point.
+///
+/// Without a vanish this is `session` over [`faulty`]'s link. With one,
+/// the session is polled until the endpoint reaches its point, where its
+/// session future and its link halves are dropped with no shutdown: the
+/// counterparty then reads end-of-stream on the control half and on every
+/// open stream, its opens toward the vanished peer fail, and its accepts
+/// wait for streams that never come (see [`Driven`]).
+pub async fn drive<Out>(
+    link: MemoryLink,
+    plan: FaultPlan,
+    session: impl AsyncFnOnce(&mut FaultyLink) -> Out,
+) -> Driven<Out> {
+    let (mut link, vanish, supply) = wrap(link, plan);
+    let Some(vanish) = vanish else {
+        return Driven {
+            outcome: Some(session(&mut link).await),
+            _supply: None,
+        };
+    };
+    let outcome = tokio::select! {
+        biased;
+        outcome = session(&mut link) => Some(outcome),
+        () = vanish.vanished() => None,
+    };
+    let vanished = outcome.is_none();
+    drop(link);
+    Driven {
+        outcome,
+        _supply: vanished.then_some(supply),
+    }
 }
 
 /// Observer for the bytes one endpoint has moved through its fault
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 128:
>
> fresh-eyes repair, round 2, item 1: `Driven::vanished`, the tripped signal the fired floor sums through `SimOutcome::vanished`.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 141:
>
> `drive` owns the vanish: it selects the session against the point, drops the session future and the link halves without shutdown at the point, and hands back the held stream supply in `Driven` so the survivor's accepts park like a dead peer's listener; `faulty` refuses a vanishing plan so no caller can run one without a driver. Without a vanish the path is the plain session (the arm's `fault disabled` claim).

<a id="hunk-24"></a>
### tests/common/fault.rs `@@ -86,6 +174,7 @@ pub fn faulty(link: MemoryLink, plan: FaultPlan) -> FaultyLink {`

```diff
@@ -86,6 +174,7 @@ pub fn faulty(link: MemoryLink, plan: FaultPlan) -> FaultyLink {
 pub struct ByteMeter {
     write: Budget,
     read: Budget,
+    streams: Streams,
 }
 
 impl ByteMeter {
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 177:
>
> fresh-eyes repair, round 2, item 1: `ByteMeter` carries the endpoint's stream ledger alongside its byte budgets.

<a id="hunk-25"></a>
### tests/common/fault.rs `@@ -94,66 +183,97 @@ impl ByteMeter {`

```diff
@@ -94,66 +183,97 @@ impl ByteMeter {
     pub fn written(&self) -> usize {
         usize::MAX - *self.write.lock().expect("write budget lock")
     }
+
+    /// Total bytes the endpoint has read, across the control half and
+    /// every data stream: the counter a `read_cut` spends.
+    pub fn read(&self) -> usize {
+        usize::MAX - *self.read.lock().expect("read budget lock")
+    }
+
+    /// Data streams the endpoint has opened: the ordinals a
+    /// [`Vanish::OnStream`] can name.
+    pub fn streams_opened(&self) -> usize {
+        self.streams.lock().expect("stream ledger lock").len()
+    }
+
+    /// Most bytes the endpoint wrote on any one data stream: the offsets
+    /// a [`Vanish::OnStream`] can reach.
+    pub fn widest_stream(&self) -> usize {
+        self.streams
+            .lock()
+            .expect("stream ledger lock")
+            .iter()
+            .copied()
+            .max()
+            .unwrap_or(0)
+    }
 }
 
 /// Wrap one clean in-memory link endpoint with byte metering: no fault
 /// ever fires (the budgets are effectively infinite), and the returned
 /// [`ByteMeter`] reads out the endpoint's cumulative traffic.
 pub fn metered(link: MemoryLink) -> (FaultyLink, ByteMeter) {
-    let write = budget(None);
-    let read = budget(None);
-    let meter = ByteMeter {
-        write: write.clone(),
-        read: read.clone(),
-    };
-    let parts = link.into_parts();
-    let link = LinkParts {
-        control_read: Cut::new(parts.control_read, read.clone()),
-        control_write: Fuse::new(parts.control_write, write.clone()),
-        connector: FaultConnector {
-            inner: parts.connector,
-            budget: write,
-        },
-        acceptor: FaultAcceptor {
-            inner: parts.acceptor,
-            budget: read,
+    let (link, (write, read, streams), _) = wrap_with(link, budget(None), budget(None), None);
+    (
+        link,
+        ByteMeter {
+            write,
+            read,
+            streams,
         },
-        session: parts.session,
-    }
-    .into_link();
-    (link, meter)
+    )
 }
 
-/// [`faulty`] for any link shape, e.g. the inter-process TCP link.
-pub fn faulty_link<CR, CW, C, A>(
-    link: Link<CR, CW, C, A>,
+/// Wrap `link` under `plan`: the wrapped link, its vanish state if the plan
+/// vanishes, and a clone of the endpoint's stream supply.
+fn wrap(
+    link: MemoryLink,
     plan: FaultPlan,
-) -> Link<Cut<CR>, Fuse<CW>, FaultConnector<C>, FaultAcceptor<A>>
-where
-    CR: AsyncRead + Unpin + Send,
-    CW: AsyncWrite + Unpin + Send,
-    C: Connector,
-    A: Acceptor,
-{
-    let write_budget = budget(plan.write_cut);
-    let read_budget = budget(plan.read_cut);
+) -> (FaultyLink, Option<Arc<VanishState>>, MemoryConnector) {
+    let vanish = plan.vanish.map(VanishState::new);
+    let (link, _, supply) = wrap_with(
+        link,
+        budget(plan.write_cut),
+        budget(plan.read_cut),
+        vanish.clone(),
+    );
+    (link, vanish, supply)
+}
+
+fn wrap_with(
+    link: MemoryLink,
+    write: Budget,
+    read: Budget,
+    vanish: Option<Arc<VanishState>>,
+) -> (FaultyLink, (Budget, Budget, Streams), MemoryConnector) {
     let parts = link.into_parts();
-    LinkParts {
-        control_read: Cut::new(parts.control_read, read_budget.clone()),
-        control_write: Fuse::new(parts.control_write, write_budget.clone()),
+    let supply = parts.connector.clone();
+    let streams: Streams = Arc::new(Mutex::new(Vec::new()));
+    let link = LinkParts {
+        control_read: Cut::new(parts.control_read, read.clone(), vanish.clone()),
+        control_write: Fuse::new(parts.control_write, write.clone(), vanish.clone(), None),
         connector: FaultConnector {
             inner: parts.connector,
-            budget: write_budget,
+            budget: write.clone(),
+            vanish: vanish.clone(),
+            streams: streams.clone(),
         },
         acceptor: FaultAcceptor {
             inner: parts.acceptor,
-            budget: read_budget,
+            budget: read.clone(),
+            vanish,
         },
         session: parts.session,
     }
-    .into_link()
+    .into_link();
+    (link, (write, read, streams), supply)
 }
 
+/// One endpoint's data streams in open order, each with the bytes written
+/// on it: the ledger a [`Vanish::OnStream`] indexes and a [`ByteMeter`]
+/// reads out.
+type Streams = Arc<Mutex<Vec<usize>>>;
+
 /// A direction's shared byte budget.
 type Budget = Arc<Mutex<usize>>;
 
```

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 189:
>
> `ByteMeter::read` exposes the read counter (the one a `read_cut` spends) so the asymmetric witness places its one-byte-short cut on the same accounting the cut uses, as the meter's doc promises.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 275:
>
> fresh-eyes repair, round 2, item 1: the per-endpoint stream ledger (opened streams and bytes on each), the source of both the vanish ordinal and `ByteMeter::streams_opened`/`widest_stream`; `VanishState` no longer counts opens itself.

<a id="hunk-26"></a>
### tests/common/fault.rs `@@ -161,6 +281,103 @@ fn budget(cut: Option<usize>) -> Budget {`

```diff
@@ -161,6 +281,103 @@ fn budget(cut: Option<usize>) -> Budget {
     Arc::new(Mutex::new(cut.unwrap_or(usize::MAX)))
 }
 
+/// One endpoint's progress toward its [`Vanish`] point, shared by every
+/// wrapper of its link.
+///
+/// Reaching the point *trips* the state: the tripping operation returns
+/// `Pending` without arranging a wake, every later operation does the
+/// same, and [`vanished`](Self::vanished) resolves so the driver can drop
+/// the session. Nothing this endpoint owns makes progress again.
+struct VanishState {
+    point: Vanish,
+    /// Bytes still to write on the named stream before an
+    /// [`Vanish::OnStream`] trips.
+    remaining: Mutex<usize>,
+    tripped: AtomicBool,
+    notify: Notify,
+}
+
+impl VanishState {
+    fn new(point: Vanish) -> Arc<Self> {
+        Arc::new(Self {
+            point,
+            remaining: Mutex::new(match point {
+                Vanish::OnStream { offset, .. } => offset,
+                Vanish::AtFirstConnect => usize::MAX,
+            }),
+            tripped: AtomicBool::new(false),
+            notify: Notify::new(),
+        })
+    }
+
+    fn tripped(&self) -> bool {
+        self.tripped.load(Ordering::Acquire)
+    }
+
+    fn trip(&self) {
+        self.tripped.store(true, Ordering::Release);
+        self.notify.notify_one();
+    }
+
+    /// Resolves once the point is reached.
+    async fn vanished(&self) {
+        loop {
+            if self.tripped() {
+                return;
+            }
+            self.notify.notified().await;
+        }
+    }
+
+    /// Whether `stream` (a data stream's ordinal; `None` is the control
+    /// half) is the one the point names.
+    fn at_point(&self, stream: Option<usize>) -> bool {
+        matches!(self.point, Vanish::OnStream { index, .. } if stream == Some(index))
+    }
+
+    /// How many of `len` bytes a write on `stream` may admit, or `None`
+    /// once the endpoint has vanished (tripping it if this write is the
+    /// point).
+    fn admit(&self, stream: Option<usize>, len: usize) -> Option<usize> {
+        if self.tripped() {
+            return None;
+        }
+        if !self.at_point(stream) {
+            return Some(len);
+        }
+        let remaining = *self.remaining.lock().expect("vanish budget lock");
+        if remaining == 0 {
+            self.trip();
+            return None;
+        }
+        Some(len.min(remaining))
+    }
+
+    fn wrote(&self, stream: Option<usize>, bytes: usize) {
+        if self.at_point(stream) {
+            *self.remaining.lock().expect("vanish budget lock") -= bytes;
+        }
+    }
+
+    /// Whether a stream open may proceed (tripping if the open is the
+    /// point).
+    fn may_connect(&self) -> bool {
+        if self.tripped() {
+            return false;
+        }
+        if let Vanish::AtFirstConnect = self.point {
+            self.trip();
+            return false;
+        }
+        true
+    }
+}
+
+/// Whether the endpoint has vanished: its every operation then parks.
+fn vanished(vanish: &Option<Arc<VanishState>>) -> bool {
+    vanish.as_ref().is_some_and(|v| v.tripped())
+}
+
 /// The failure every write-direction surface reports once its budget is
 /// exhausted: writers and the connector fail identically.
 fn write_severed() -> io::Error {
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 291:
>
> Shared trip state: the tripping operation and every later one return `Pending` with no wake, and `vanished` resolves for the driver. Stream ordinals are counted at `connect` so `OnStream` names a stream the endpoint itself opened.

<a id="hunk-27"></a>
### tests/common/fault.rs `@@ -180,10 +397,13 @@ fn read_severed() -> io::Error {`

```diff
@@ -180,10 +397,13 @@ fn read_severed() -> io::Error {
 }
 
 /// A connector whose opened streams draw on the endpoint's write budget,
-/// and which itself fails once that budget is exhausted.
+/// and which itself fails once that budget is exhausted (or parks once
+/// the endpoint has vanished).
 pub struct FaultConnector<C> {
     inner: C,
     budget: Budget,
+    vanish: Option<Arc<VanishState>>,
+    streams: Streams,
 }
 
 impl<C: Clone> Clone for FaultConnector<C> {
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 400:
>
> The connector holds the vanish state (so an open at the point parks) and, from round 2, the stream ledger it assigns ordinals from; its doc says it parks once the endpoint has vanished.

<a id="hunk-28"></a>
### tests/common/fault.rs `@@ -191,6 +411,8 @@ impl<C: Clone> Clone for FaultConnector<C> {`

```diff
@@ -191,6 +411,8 @@ impl<C: Clone> Clone for FaultConnector<C> {
         Self {
             inner: self.inner.clone(),
             budget: self.budget.clone(),
+            vanish: self.vanish.clone(),
+            streams: self.streams.clone(),
         }
     }
 }
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 414:
>
> `Clone` carries the vanish state and the ledger; the memory connector is cloned per stream open inside the session.

<a id="hunk-29"></a>
### tests/common/fault.rs `@@ -199,6 +421,12 @@ impl<C: Connector> Connector for FaultConnector<C> {`

```diff
@@ -199,6 +421,12 @@ impl<C: Connector> Connector for FaultConnector<C> {
     type Tx = Fuse<C::Tx>;
 
     async fn connect(&self) -> io::Result<(Self::Tx, Done<Self::Tx>)> {
+        if let Some(vanish) = &self.vanish
+            && !vanish.may_connect()
+        {
+            std::future::pending::<()>().await;
+            unreachable!("a vanished endpoint never resumes");
+        }
         // A dead write direction cannot open new streams either; this is
         // what lets a cut exercise `SendError::Connect` deterministically
         // instead of only through real-transport races.
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 424:
>
> An open at the `AtFirstConnect` point trips the vanish and parks forever, and every open after a trip parks: the driver drops the session on the trip.

<a id="hunk-30"></a>
### tests/common/fault.rs `@@ -206,25 +434,40 @@ impl<C: Connector> Connector for FaultConnector<C> {`

```diff
@@ -206,25 +434,40 @@ impl<C: Connector> Connector for FaultConnector<C> {
             return Err(write_severed());
         }
         let (tx, done) = self.inner.connect().await?;
+        let ordinal = {
+            let mut streams = self.streams.lock().expect("stream ledger lock");
+            streams.push(0);
+            streams.len() - 1
+        };
         // Completion unwraps the fuse and passes the half through.
         Ok((
-            Fuse::new(tx, self.budget.clone()),
+            Fuse::new(
+                tx,
+                self.budget.clone(),
+                self.vanish.clone(),
+                Some((ordinal, self.streams.clone())),
+            ),
             Done::new(move |fuse: Fuse<C::Tx>| done.complete(fuse.inner)),
         ))
     }
 }
 
 /// An acceptor whose accepted streams draw on the endpoint's read budget,
-/// and which itself fails once that budget is exhausted.
+/// and which itself fails once that budget is exhausted (or parks once
+/// the endpoint has vanished).
 pub struct FaultAcceptor<A> {
     inner: A,
     budget: Budget,
+    vanish: Option<Arc<VanishState>>,
 }
 
 impl<A: Acceptor> Acceptor for FaultAcceptor<A> {
     type Rx = Cut<A::Rx>;
 
     async fn accept(&mut self) -> io::Result<(Self::Rx, Done<Self::Rx>)> {
+        if vanished(&self.vanish) {
+            std::future::pending::<()>().await;
+        }
         // A dead read direction cannot deliver new streams either; this
         // reaches the session's deferred supply-failure path (the parked
         // accept driver) deterministically rather than only via races.
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 437:
>
> fresh-eyes repair, round 2, item 1: the ordinal a new stream gets comes from the ledger (pushed here), which the stream's writer then reports its bytes to; `VanishState` no longer counts opens itself, so the ledger is the one source of stream identity.

<a id="hunk-31"></a>
### tests/common/fault.rs `@@ -234,7 +477,7 @@ impl<A: Acceptor> Acceptor for FaultAcceptor<A> {`

```diff
@@ -234,7 +477,7 @@ impl<A: Acceptor> Acceptor for FaultAcceptor<A> {
         let (rx, done) = self.inner.accept().await?;
         // Completion unwraps the cut and passes the half through.
         Ok((
-            Cut::new(rx, self.budget.clone()),
+            Cut::new(rx, self.budget.clone(), self.vanish.clone()),
             Done::new(move |cut: Cut<A::Rx>| done.complete(cut.inner)),
         ))
     }
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 480:
>
> Accepted streams' readers carry the vanish state so they park after a trip like every other operation of the vanished endpoint.

<a id="hunk-32"></a>
### tests/common/fault.rs `@@ -248,11 +491,29 @@ impl<A: Acceptor> Acceptor for FaultAcceptor<A> {`

```diff
@@ -248,11 +491,29 @@ impl<A: Acceptor> Acceptor for FaultAcceptor<A> {
 pub struct Fuse<W> {
     inner: W,
     remaining: Budget,
+    vanish: Option<Arc<VanishState>>,
+    /// Which data stream this writer is and the ledger it reports to;
+    /// `None` is the control half.
+    stream: Option<(usize, Streams)>,
 }
 
 impl<W> Fuse<W> {
-    fn new(inner: W, remaining: Budget) -> Self {
-        Self { inner, remaining }
+    fn new(
+        inner: W,
+        remaining: Budget,
+        vanish: Option<Arc<VanishState>>,
+        stream: Option<(usize, Streams)>,
+    ) -> Self {
+        Self {
+            inner,
+            remaining,
+            vanish,
+            stream,
+        }
+    }
+
+    fn ordinal(&self) -> Option<usize> {
+        self.stream.as_ref().map(|(ordinal, _)| *ordinal)
     }
 }
 
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 494:
>
> `Fuse` carries the vanish state and, for a data stream, its ordinal with the ledger (`None` is the control half); `ordinal` is what the vanish point compares against.

<a id="hunk-33"></a>
### tests/common/fault.rs `@@ -263,16 +524,31 @@ impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {`

```diff
@@ -263,16 +524,31 @@ impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
         buf: &[u8],
     ) -> Poll<io::Result<usize>> {
         let this = self.get_mut();
+        // A vanish point inside this write parks it (and every later
+        // operation) with no wake; the driver drops the session.
+        let before_vanish = match &this.vanish {
+            Some(vanish) => match vanish.admit(this.ordinal(), buf.len()) {
+                Some(admitted) => admitted,
+                None => return Poll::Pending,
+            },
+            None => buf.len(),
+        };
         let mut remaining = this.remaining.lock().expect("write budget lock");
         if *remaining == 0 {
             return Poll::Ready(Err(write_severed()));
         }
         // Admit at most the remaining budget; the writer's retry of the
         // unwritten tail then trips the exhausted fuse above.
-        let admitted = buf.len().min(*remaining);
+        let admitted = before_vanish.min(*remaining);
         match Pin::new(&mut this.inner).poll_write(cx, &buf[..admitted]) {
             Poll::Ready(Ok(n)) => {
                 *remaining -= n;
+                if let Some(vanish) = &this.vanish {
+                    vanish.wrote(this.ordinal(), n);
+                }
+                if let Some((ordinal, streams)) = &this.stream {
+                    streams.lock().expect("stream ledger lock")[*ordinal] += n;
+                }
                 Poll::Ready(Ok(n))
             }
             other => other,
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 527:
>
> The vanish check comes before the cut check: a write at the point returns `Pending` with no wake (the driver drops the session), a write before it is admitted only up to the point, and bytes written report to the ledger (round 2).

<a id="hunk-34"></a>
### tests/common/fault.rs `@@ -280,11 +556,19 @@ impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {`

```diff
@@ -280,11 +556,19 @@ impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
     }
 
     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
-        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
+        let this = self.get_mut();
+        if vanished(&this.vanish) {
+            return Poll::Pending;
+        }
+        Pin::new(&mut this.inner).poll_flush(cx)
     }
 
     fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
-        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
+        let this = self.get_mut();
+        if vanished(&this.vanish) {
+            return Poll::Pending;
+        }
+        Pin::new(&mut this.inner).poll_shutdown(cx)
     }
 }
 
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 559:
>
> Flush and shutdown park after a trip too: a session may reach them in the same poll that tripped, and nothing the vanished endpoint owns may make progress again.

<a id="hunk-35"></a>
### tests/common/fault.rs `@@ -297,11 +581,16 @@ impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {`

```diff
@@ -297,11 +581,16 @@ impl<W: AsyncWrite + Unpin> AsyncWrite for Fuse<W> {
 pub struct Cut<R> {
     inner: R,
     remaining: Budget,
+    vanish: Option<Arc<VanishState>>,
 }
 
 impl<R> Cut<R> {
-    fn new(inner: R, remaining: Budget) -> Self {
-        Self { inner, remaining }
+    fn new(inner: R, remaining: Budget, vanish: Option<Arc<VanishState>>) -> Self {
+        Self {
+            inner,
+            remaining,
+            vanish,
+        }
     }
 }
 
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 584:
>
> `Cut` carries the vanish state for the same reason as `Fuse`.

<a id="hunk-36"></a>
### tests/common/fault.rs `@@ -312,6 +601,9 @@ impl<R: AsyncRead + Unpin> AsyncRead for Cut<R> {`

```diff
@@ -312,6 +601,9 @@ impl<R: AsyncRead + Unpin> AsyncRead for Cut<R> {
         buf: &mut ReadBuf<'_>,
     ) -> Poll<io::Result<()>> {
         let this = self.get_mut();
+        if vanished(&this.vanish) {
+            return Poll::Pending;
+        }
         let mut remaining = this.remaining.lock().expect("read budget lock");
         if *remaining == 0 {
             return Poll::Ready(Err(read_severed()));
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 604:
>
> Reads park after a trip.

<a id="hunk-37"></a>
### tests/common/overlap.rs `@@ -15,14 +15,17 @@`

```diff
@@ -15,14 +15,17 @@
 //! sessions open, park, and close at generated points.
 //!
 //! The alphabet extends [`schedule::events::Event`] with three session
-//! events over a small set of *slots*: [`OverlapEvent::Open`] captures
-//! both endpoints' fork-time state and parks, [`OverlapEvent::Step`]
-//! polls the parked session a bounded number of times, and
-//! [`OverlapEvent::Close`] drives it to completion and installs. The
-//! generator keeps the schedule valid by construction the same way
-//! [`schedule::arb`] does — a shadow simulator tracks what every peer has
-//! observed, with open sessions modeled by their fork-time snapshots so a
-//! `Redact` is only ever emitted against a message its peer really holds.
+//! events over a small set of *slots*: [`OverlapEvent::Open`] builds a
+//! session and parks it unpolled, [`OverlapEvent::Step`] polls the parked
+//! session a bounded number of times, and [`OverlapEvent::Close`] drives
+//! it to completion and installs. As in [`schedule::arb`], a shadow
+//! simulator keeps the schedule valid by construction; it models an open
+//! session by each side's view at the moment the live side forks it,
+//! after that side's preamble exchange (see [`FORK_ROUNDS`]), so a
+//! `Redact` the shadow emits names a message its live peer holds. The
+//! executor still guards every `Redact` on the live observation log: a
+//! skip there is the model drifting from the protocol, which the
+//! shadow-validity meta-test catches.
 
 use std::collections::BTreeMap;
 use std::fmt::Debug;
```

<!-- annotation -->
> **tests-common-12** (T144), line 23:
>
> Module doc: the model forks where the session does; the guard's skip is now drift, caught by the meta-test.

<a id="hunk-38"></a>
### tests/common/overlap.rs `@@ -175,8 +178,12 @@ pub enum OverlapEvent<T> {`

```diff
@@ -175,8 +178,12 @@ pub enum OverlapEvent<T> {
     /// One whole (non-overlapped) session between `a` and `b`, as the
     /// serial executor runs them.
     Gossip { a: usize, b: usize },
-    /// Open a session between `a` and `b` in `slot`, forking both sides'
-    /// working state here; it installs only at its `Close`.
+    /// Open a session between `a` and `b` in `slot` without polling it;
+    /// it installs at its `Close`, or at a `Step` that completes it.
+    ///
+    /// Nothing forks here: each side forks its view under the polls that
+    /// complete its preamble exchange ([`FORK_ROUNDS`]), and the shadow
+    /// models exactly that.
     Open { slot: usize, a: usize, b: usize },
     /// Poll the session in `slot` at most `polls` times.
     Step { slot: usize, polls: usize },
```

<!-- annotation -->
> **tests-common-12** (T144), line 184:
>
> `Open` doc states that nothing forks at `Open` and where each side does.

<a id="hunk-39"></a>
### tests/common/overlap.rs `@@ -200,6 +207,31 @@ pub struct OverlapSchedule<T> {`

```diff
@@ -200,6 +207,31 @@ pub struct OverlapSchedule<T> {
 /// Returns the fleet and the spec-shaped oracle; the caller asserts the
 /// two agree.
 pub fn execute_overlap_and_quiesce<T>(schedule: &OverlapSchedule<T>) -> (Vec<Peer<T>>, Oracle<T>)
+where
+    T: Clone + Eq + Ord + Serialize + DeserializeOwned + Send + Sync + 'static,
+{
+    let OverlapRun {
+        mut peers, oracle, ..
+    } = execute_overlap(schedule);
+    quiesce(&mut peers);
+    (peers, oracle)
+}
+
+/// The fleet after an overlap schedule has run, before any quiescence.
+pub struct OverlapRun<T> {
+    pub peers: Vec<Peer<T>>,
+    pub oracle: Oracle<T>,
+    /// The [`Version`] each `Insert` event created, by event index.
+    pub resolved_versions: BTreeMap<EventIdx, Version>,
+}
+
+/// Run an overlap schedule against a fresh fleet and close every session
+/// still open (in ascending slot order), leaving the fleet as the schedule
+/// left it.
+///
+/// No quiescence: the observation logs and live sets are those the
+/// schedule alone produced.
+pub fn execute_overlap<T>(schedule: &OverlapSchedule<T>) -> OverlapRun<T>
 where
     T: Clone + Eq + Ord + Serialize + DeserializeOwned + Send + Sync + 'static,
 {
```

<!-- annotation -->
> **tests-observation-28** (T144), line 234:
>
> From the stop artifact: the executor split so the meta-test can compare the fleet as the schedule left it (no quiescence) with `resolved_versions`; `execute_overlap_and_quiesce` is the same run plus the quiesce.

<a id="hunk-40"></a>
### tests/common/overlap.rs `@@ -230,10 +262,12 @@ where`

```diff
@@ -230,10 +262,12 @@ where
                 target_event_idx,
             } => {
                 let version = &resolved_versions[target_event_idx];
-                // The generator's shadow makes this always-observed; the
-                // guard mirrors the serial executor's, so a shadow
-                // imprecision degrades to a skipped event on both sides
-                // of the comparison rather than an invalid `redact`.
+                // The shadow's premise is that each side forks where the
+                // live session does, so the target is observed here; if
+                // the model has drifted from the protocol, skip the event
+                // on both sides of the comparison rather than issue a
+                // `redact` the live peer could never have made. The
+                // shadow-validity meta-test is what makes such drift fail.
                 let observed = peers[*peer].observations.iter().any(|(v, _)| v == version);
                 if observed {
                     peers[*peer].redact_one(version);
```

<!-- annotation -->
> **tests-common-12** (T144), line 265:
>
> Guard comment restated as the model's premise (per T144 and T13's `guard stays a guard`): the Open-time consequence no longer follows, so the skip is drift, not design.

<a id="hunk-41"></a>
### tests/common/overlap.rs `@@ -284,8 +318,11 @@ where`

```diff
@@ -284,8 +318,11 @@ where
         peers[a].drain();
         peers[b].drain();
     }
-    quiesce(&mut peers);
-    (peers, oracle)
+    OverlapRun {
+        peers,
+        oracle,
+        resolved_versions,
+    }
 }
 
 /// How many session slots a generated schedule may hold open at once.
```

<!-- annotation -->
> **tests-observation-28** (T144), line 321:
>
> From the stop artifact: `execute_overlap` returns the run as the schedule left it (peers, oracle, `resolved_versions`); the quiesce lives in `execute_overlap_and_quiesce`, which the properties still call.

<a id="hunk-42"></a>
### tests/common/overlap.rs `@@ -293,6 +330,16 @@ where`

```diff
@@ -293,6 +330,16 @@ where
 /// lets overlaps themselves overlap.
 const SLOTS: usize = 3;
 
+/// Polling rounds after which each side of an open session has forked
+/// its working state, indexed like [`Session`]'s sides (`a`, then `b`).
+///
+/// A side forks once its preamble exchange completes. [`Session::step`]
+/// polls `a` then `b` each round: in the first round `a` writes its
+/// preamble and parks on the read, then `b` writes its own, reads `a`'s,
+/// and forks; `a` reads `b`'s preamble and forks in the second round. A
+/// `Close` polls to completion, so it forks whichever side has not.
+const FORK_ROUNDS: [usize; 2] = [2, 1];
+
 /// Strategy: overlap schedules that are valid by construction.
 ///
 /// Biased toward the shape that discovers install-time interleaving
```

<!-- annotation -->
> **tests-observation-28** (T144), line 341:
>
> The model's premise, derived from the code and stated with its derivation: side `b` forks in the first polled round (it reads `a`'s preamble in the same round), side `a` in the second. T144 says `at its first poll`; the per-side reading is what the meta-test confirms exact: with both sides at the first round (`[1, 1]`) the meta-test fails on the recorded shape, so the distinction is load-bearing (calibration run, output in the commit message).

<a id="hunk-43"></a>
### tests/common/overlap.rs `@@ -311,6 +358,22 @@ pub fn arb_overlap_schedule<T, S>(`

```diff
@@ -311,6 +358,22 @@ pub fn arb_overlap_schedule<T, S>(
     n_peers_range: RangeInclusive<usize>,
     max_events: usize,
 ) -> impl Strategy<Value = OverlapSchedule<T>>
+where
+    T: Clone + Debug + 'static,
+    S: Strategy<Value = T> + Clone + 'static,
+{
+    arb_overlap_schedule_with_shadow(value_strategy, n_peers_range, max_events)
+        .prop_map(|(schedule, _shadow)| schedule)
+}
+
+/// Variant of [`arb_overlap_schedule`] that also yields the shadow's
+/// final [`Knowledge`], for the shadow-validity meta-test that checks the
+/// generator's model against the live executor.
+pub fn arb_overlap_schedule_with_shadow<T, S>(
+    value_strategy: S,
+    n_peers_range: RangeInclusive<usize>,
+    max_events: usize,
+) -> impl Strategy<Value = (OverlapSchedule<T>, Knowledge)>
 where
     T: Clone + Debug + 'static,
     S: Strategy<Value = T> + Clone + 'static,
```

<!-- annotation -->
> **tests-observation-28** (T144), line 372:
>
> From the stop artifact: the `_with_shadow` twin of the overlap strategy, surfacing the final `Knowledge`.

<a id="hunk-44"></a>
### tests/common/overlap.rs `@@ -501,14 +564,54 @@ impl<T: Clone> Pincer<T> {`

```diff
@@ -501,14 +564,54 @@ impl<T: Clone> Pincer<T> {
     }
 }
 
+/// An open session in the model: its endpoints, the polling rounds it
+/// has received, and each side's view at the round it forked
+/// ([`FORK_ROUNDS`]), taken once that round is reached.
+struct OpenSession {
+    a: usize,
+    b: usize,
+    rounds: usize,
+    forks: [Option<Knowledge>; 2],
+}
+
+impl OpenSession {
+    /// Advance the session by `rounds` polling rounds, forking any side
+    /// whose round is reached at the current `sim`.
+    fn poll(&mut self, rounds: usize, sim: &Knowledge) {
+        self.rounds += rounds;
+        for (side, fork) in self.forks.iter_mut().enumerate() {
+            if fork.is_none() && self.rounds >= FORK_ROUNDS[side] {
+                *fork = Some(sim.clone());
+            }
+        }
+    }
+
+    /// Drive the session to completion (as a `Close` does), forking any
+    /// side that has not, and deliver it into `sim`.
+    fn close(mut self, sim: &mut Knowledge) {
+        self.poll(FORK_ROUNDS[0].max(FORK_ROUNDS[1]), sim);
+        let [fork_a, fork_b] = self.forks;
+        sim.merge_session(
+            &fork_a.expect("closed sessions have forked"),
+            &fork_b.expect("closed sessions have forked"),
+            self.a,
+            self.b,
+        );
+    }
+}
+
 /// Per-peer knowledge sets, as in `schedule::arb`'s shadow: everything
 /// the peer has ever held, the subset currently live, and the exact
 /// observation order.
-#[derive(Clone)]
-struct Knowledge {
-    ever_known: Vec<std::collections::BTreeSet<EventIdx>>,
-    live: Vec<std::collections::BTreeSet<EventIdx>>,
-    observed_log: Vec<Vec<EventIdx>>,
+#[derive(Clone, Debug)]
+pub struct Knowledge {
+    /// Per-peer set of `EventIdx`s whose message the peer has ever held.
+    pub ever_known: Vec<std::collections::BTreeSet<EventIdx>>,
+    /// Per-peer set of `EventIdx`s the model predicts the peer holds live.
+    pub live: Vec<std::collections::BTreeSet<EventIdx>>,
+    /// Per-peer sequence of `EventIdx`s the model predicts the peer's
+    /// observation log would have appended.
+    pub observed_log: Vec<Vec<EventIdx>>,
 }
 
 impl Knowledge {
```

<!-- annotation -->
> **tests-observation-28** (T144), line 570:
>
> An open session in the model: rounds received and each side's view at the round it forked; `Close` polls to completion so both views exist by then.

<a id="hunk-45"></a>
### tests/common/overlap.rs `@@ -520,27 +623,28 @@ impl Knowledge {`

```diff
@@ -520,27 +623,28 @@ impl Knowledge {
         }
     }
 
-    /// Merge what a session forked at `snapshot` delivers between `a`
-    /// and `b` into the *current* state.
+    /// Merge what a session delivers between `a` and `b` into the
+    /// *current* state, given each side's view at its fork (`fork_a` for
+    /// `a`, `fork_b` for `b`).
     ///
     /// The session carries each side's fork-time content only: messages
     /// one fork-time side held live propagate to a counterparty that has
     /// never known them; messages either fork-time side had redacted die
     /// on both current sides (deletion honoring, tombstone-free). A
-    /// message redacted *after* the fork stays dead locally —
-    /// `ever_known` guards resurrection — and its counterparty learns
+    /// message redacted *after* the fork stays dead locally --
+    /// `ever_known` guards resurrection -- and its counterparty learns
     /// that deletion only from a later session, exactly as the wire
     /// behaves.
-    fn merge_session(&mut self, snapshot: &Knowledge, a: usize, b: usize) {
-        let combined: std::collections::BTreeSet<EventIdx> = snapshot.ever_known[a]
-            .union(&snapshot.ever_known[b])
+    fn merge_session(&mut self, fork_a: &Knowledge, fork_b: &Knowledge, a: usize, b: usize) {
+        let combined: std::collections::BTreeSet<EventIdx> = fork_a.ever_known[a]
+            .union(&fork_b.ever_known[b])
             .copied()
             .collect();
         for k in combined {
-            let a_had = snapshot.ever_known[a].contains(&k);
-            let b_had = snapshot.ever_known[b].contains(&k);
-            let redacted_at_fork = (a_had && !snapshot.live[a].contains(&k))
-                || (b_had && !snapshot.live[b].contains(&k));
+            let a_had = fork_a.ever_known[a].contains(&k);
+            let b_had = fork_b.ever_known[b].contains(&k);
+            let redacted_at_fork =
+                (a_had && !fork_a.live[a].contains(&k)) || (b_had && !fork_b.live[b].contains(&k));
             if redacted_at_fork {
                 for p in [a, b] {
                     self.ever_known[p].insert(k);
```

<!-- annotation -->
> **tests-observation-28** (T144), line 638:
>
> `merge_session` takes one view per side instead of one snapshot; the preamble round and `Gossip` events pass the same frozen state for both, which is the old behavior for serial sessions.

<a id="hunk-46"></a>
### tests/common/overlap.rs `@@ -559,17 +663,21 @@ impl Knowledge {`

```diff
@@ -559,17 +663,21 @@ impl Knowledge {
     }
 }
 
-/// Build a valid overlap schedule by driving the shadow in lockstep with
-/// the emitted events, with each open session modeled by the fork-time
-/// snapshot its `Open` captured.
+/// Build an overlap schedule by driving the shadow in lockstep with the
+/// emitted events, each open session modeled by its sides' views at the
+/// rounds they fork, returning the schedule with the shadow's final
+/// state.
+///
+/// Every emitted `Redact` names a message its peer holds in the model,
+/// and the model forks where the live session does.
 fn build_overlap_schedule<T: Clone>(
     n_peers: usize,
     fork_parents: Vec<usize>,
     preamble: &[T],
     choices: Vec<Choice<T>>,
-) -> OverlapSchedule<T> {
+) -> (OverlapSchedule<T>, Knowledge) {
     let mut sim = Knowledge::new(n_peers);
-    let mut open: BTreeMap<usize, (usize, usize, Knowledge)> = BTreeMap::new();
+    let mut open: BTreeMap<usize, OpenSession> = BTreeMap::new();
     let mut events: Vec<OverlapEvent<T>> = Vec::new();
 
     // Converged preamble: populate the seed peer, then one sequential
```

<!-- annotation -->
> **tests-observation-28** (T144), line 666:
>
> The builder's doc states the model (each side's view at the round it forks) and the builder returns the shadow's final state for the meta-test.

<a id="hunk-47"></a>
### tests/common/overlap.rs `@@ -588,7 +696,7 @@ fn build_overlap_schedule<T: Clone>(`

```diff
@@ -588,7 +696,7 @@ fn build_overlap_schedule<T: Clone>(
     for a in 0..n_peers {
         for b in (a + 1)..n_peers {
             let frozen = sim.clone();
-            sim.merge_session(&frozen, a, b);
+            sim.merge_session(&frozen, &frozen, a, b);
             events.push(OverlapEvent::Gossip { a, b });
         }
     }
```

<!-- annotation -->
> **tests-observation-28** (T144), line 699:
>
> The preamble's serial sessions fork both sides at once, so both views are the same frozen state.

<a id="hunk-48"></a>
### tests/common/overlap.rs `@@ -625,7 +733,7 @@ fn build_overlap_schedule<T: Clone>(`

```diff
@@ -625,7 +733,7 @@ fn build_overlap_schedule<T: Clone>(
                     continue;
                 }
                 let frozen = sim.clone();
-                sim.merge_session(&frozen, a, b);
+                sim.merge_session(&frozen, &frozen, a, b);
                 events.push(OverlapEvent::Gossip { a, b });
             }
             Choice::Open { slot, a, b } => {
```

<!-- annotation -->
> **tests-observation-28** (T144), line 736:
>
> A `Gossip` choice is a serial session: the same frozen view for both sides.

<a id="hunk-49"></a>
### tests/common/overlap.rs `@@ -633,22 +741,31 @@ fn build_overlap_schedule<T: Clone>(`

```diff
@@ -633,22 +741,31 @@ fn build_overlap_schedule<T: Clone>(
                 if a == b || open.contains_key(&slot) {
                     continue;
                 }
-                open.insert(slot, (a, b, sim.clone()));
+                open.insert(
+                    slot,
+                    OpenSession {
+                        a,
+                        b,
+                        rounds: 0,
+                        forks: [None, None],
+                    },
+                );
                 events.push(OverlapEvent::Open { slot, a, b });
             }
             Choice::Step { slot, polls } => {
                 let slot = slot % SLOTS;
-                if !open.contains_key(&slot) {
+                let Some(session) = open.get_mut(&slot) else {
                     continue;
-                }
+                };
+                session.poll(polls, &sim);
                 events.push(OverlapEvent::Step { slot, polls });
             }
             Choice::Close { slot } => {
                 let slot = slot % SLOTS;
-                let Some((a, b, snapshot)) = open.remove(&slot) else {
+                let Some(session) = open.remove(&slot) else {
                     continue;
                 };
-                sim.merge_session(&snapshot, a, b);
+                session.close(&mut sim);
                 events.push(OverlapEvent::Close { slot });
             }
         }
```

<!-- annotation -->
> **tests-observation-28** (T144), line 744:
>
> `Open` records an unforked `OpenSession`; `Step` advances its rounds and forks any side whose round is reached at the current state; `Close` completes and delivers it. This is where the fork moved from `Open`.

<a id="hunk-50"></a>
### tests/common/overlap.rs `@@ -656,13 +773,16 @@ fn build_overlap_schedule<T: Clone>(`

```diff
@@ -656,13 +773,16 @@ fn build_overlap_schedule<T: Clone>(
 
     // The executor closes leftover sessions in ascending slot order;
     // mirror that so redact validity extends through the implicit tail.
-    for (_, (a, b, snapshot)) in open.into_iter() {
-        sim.merge_session(&snapshot, a, b);
+    for (_, session) in open.into_iter() {
+        session.close(&mut sim);
     }
 
-    OverlapSchedule {
-        n_peers,
-        fork_parents,
-        events,
-    }
+    (
+        OverlapSchedule {
+            n_peers,
+            fork_parents,
+            events,
+        },
+        sim,
+    )
 }
```

<!-- annotation -->
> **tests-observation-28** (T144), line 776:
>
> The implicit closing tail closes leftover sessions through the same model, and the builder returns the schedule with the shadow.

<a id="hunk-51"></a>
### tests/common/sim.rs `@@ -18,14 +18,15 @@`

```diff
@@ -18,14 +18,15 @@
 //! 1. **Fleet**: one seed and its clean bootstrap forks.
 //! 2. **Chaos**: every session, every activity script, and every extra
 //!    bootstrap attempt runs concurrently; channel cuts land at arbitrary
-//!    byte offsets via [`FaultPlan`]s. Serving a bootstrap mid-chaos puts
+//!    byte offsets, and endpoints vanish mid-stream, via [`FaultPlan`]s.
+//!    Serving a bootstrap mid-chaos puts
 //!    the snapshot-and-fork critical section under concurrent sends from
 //!    sibling handles (see [`run_boot`]); a failed attempt may orphan the
 //!    served fork's id-region — counted, see below. Each peer also carries
 //!    one observer of each kind
 //!    ([`UnorderedMessages`](rumors::UnorderedMessages) and
 //!    [`CausalMessages`](rumors::CausalMessages)), drained concurrently
-//!    with the chaos and asserting the delivery contracts inline — no
+//!    with the chaos and asserting the delivery contracts inline -- no
 //!    message twice, no causal inversion, and full coverage of the peer's live set
 //!    once the writers settle (see [`run_observers`]). This is the only
 //!    place the observers' watch-coalescing path runs against genuinely
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 21:
>
> Module doc: the chaos phase names endpoints vanishing mid-stream; round 2 item 7 replaced the em-dash in this paragraph.

<a id="hunk-52"></a>
### tests/common/sim.rs `@@ -53,13 +54,14 @@`

```diff
@@ -53,13 +54,14 @@
 //! # Loss accounting
 //!
 //! Party id-regions can leave the live universe *legitimately* when a wire
-//! drops mid-hand-off: a bootstrap fork lost in flight, or a retiree's
-//! [`Retire::Uncertain`] whose absorber also failed. The engine counts
+//! drops mid-hand-off: a bootstrap fork lost in flight, a retiree's
+//! [`Retire::Uncertain`] whose absorber also failed, or a peer that
+//! vanished while bootstrapping or retiring. The engine counts
 //! every such *possible* loss conservatively in
 //! [`SimOutcome::possible_losses`]. Disjointness must hold regardless;
-//! the sharper invariants — the surviving parties fold-join back to exactly
+//! the sharper invariants -- the surviving parties fold-join back to exactly
 //! [`Party::seed`], and the converged value multiset equals the ledger's
-//! inserts minus redactions — are asserted whenever the count is zero
+//! inserts minus redactions -- are asserted whenever the count is zero
 //! (which the plan generator arranges often, by disabling fault injection
 //! entirely in half its plans). Zero possible losses covers message
 //! content across faulted retires, not only id-regions: every retire arm
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 57:
>
> Loss accounting names the vanished bootstrapper or retiree among the legitimate losses; round 2 item 7 replaced the em-dashes.

<a id="hunk-53"></a>
### tests/common/sim.rs `@@ -72,17 +74,19 @@`

```diff
@@ -72,17 +74,19 @@
 
 use std::collections::{BTreeMap, BTreeSet};
 use std::convert::Infallible;
+use std::future::Future;
 use std::sync::Arc;
 use std::sync::atomic::{AtomicBool, Ordering};
+use std::time::Duration;
 
 use before::Party;
 use proptest::prelude::*;
 use rumors::error::{
     CodecDecodeErrorKind, CodecEncodeErrorKind, RemoteError, SendError, StreamError,
 };
-use rumors::{Error, MirrorError, Peer, Retire, Rumors, Version};
+use rumors::{Error, Gossiped, MirrorError, Peer, Retire, Rumors, Version};
 
-use crate::common::fault::{self, FaultPlan};
+use crate::common::fault::{self, FaultPlan, Vanish};
 use crate::common::oracle::{readout, readout_multiset, version_key};
 use crate::common::window::{WindowAssignment, WindowChoice, arb_window_choice};
 use crate::common::wire::wire_gossip_async;
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 77:
>
> Imports for the session deadline (`Duration`, `Future`), `Gossiped` (the survivor assertion's type), and `Vanish`.

<a id="hunk-54"></a>
### tests/common/sim.rs `@@ -106,6 +110,45 @@ pub const MAX_CUT: usize = 3072;`

```diff
@@ -106,6 +110,45 @@ pub const MAX_CUT: usize = 3072;
 /// Headroom on the heal loop, as in `peer::quiesce`.
 const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;
 
+/// Bound on one faulted session, bootstrap, or retirement.
+///
+/// Over in-memory wires these complete in milliseconds, so the bound is
+/// headroom over scheduling, not protocol work. A session still running
+/// at the deadline is parked: after a vanish, the survivor waiting on a
+/// stream its dead peer never opens (a wait the link contract leaves to
+/// the caller's timeout, which this is); otherwise a protocol deadlock.
+/// A deadlock fails by name instead of hanging the run; a park after a
+/// planned vanish is counted in [`SimOutcome::parked`] (see there).
+/// Single-digit seconds so that a deadlock's shrink, bounded by
+/// [`MAX_SHRINK_TIME`], finishes inside nextest's budget with its seed
+/// persisted.
+const SESSION_DEADLINE: Duration = Duration::from_secs(2);
+
+/// Shrink-time bound for the plan proptests, in milliseconds: with every
+/// shrink candidate that parks costing [`SESSION_DEADLINE`], this keeps a
+/// failing run under nextest's termination budget so the seed persists.
+pub const MAX_SHRINK_TIME: u32 = 60_000;
+
+/// Upper bound on the data-stream ordinal a generated vanish names.
+///
+/// Derived from measurement, like `MAX_CUT`: the envelope session opens
+/// fewer data streams per endpoint than this bound, and the bound stays
+/// within twice that count, so generated vanishes reach every stream an
+/// envelope endpoint opens and mostly name streams that exist. The
+/// two-sided pin is `vanish_draw_spans_the_envelope_session` in
+/// `tests/disruption.rs`.
+pub const MAX_VANISH_STREAM: usize = 2;
+
+/// Upper bound on the byte offset into a data stream at which a
+/// generated vanish lands.
+///
+/// Derived from measurement, like `MAX_CUT`: the envelope session's
+/// widest data stream carries fewer bytes than this bound, and the bound
+/// stays within twice that extent, so generated vanishes reach every
+/// byte of the widest stream and keep landing inside real ones. Pinned
+/// with [`MAX_VANISH_STREAM`].
+pub const MAX_VANISH_OFFSET: usize = 1280;
+
 /// One concurrently-executed simulation plan. See the module docs for how
 /// the pieces are scheduled.
 #[derive(Debug, Clone)]
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 125:
>
> fresh-eyes repair, round 2, item 2: two seconds (headroom over scheduling only), with `MAX_SHRINK_TIME` bounding the proptests' shrink so a deadline failure lands inside nextest's budget with its seed persisted; control in the commit message. Also the round-2 deviation: a park after a *planned vanish* is counted and aborted rather than failed, see `SimOutcome::parked`; a park with no vanish planned is a deadlock and fails by name.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 140:
>
> fresh-eyes repair, round 2, item 1: the ordinal bound, measured (the envelope endpoint opens 2 data streams; band [2, 4]) and pinned two-sided by `vanish_draw_spans_the_envelope_session`.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 150:
>
> fresh-eyes repair, round 2, item 1: the offset bound, measured (the envelope's widest data stream carries 1093 bytes; band [1093, 2186]) and pinned with the ordinal bound.

<a id="hunk-55"></a>
### tests/common/sim.rs `@@ -258,14 +301,28 @@ pub struct SimOutcome {`

```diff
@@ -258,14 +301,28 @@ pub struct SimOutcome {
     pub inserted: Vec<u64>,
     /// The execution-time redaction log; see [`Redaction`].
     pub redactions: Vec<Redaction>,
+    /// Endpoints that reached their vanish point, across sessions,
+    /// bootstraps, and retirements (a parked survivor's counterparty
+    /// counted among them).
+    pub vanished: usize,
+    /// Sessions, bootstraps, or retirements in which the survivor of a
+    /// planned vanish was still running at [`SESSION_DEADLINE`] and was
+    /// aborted.
+    ///
+    /// Such a survivor waits on a stream its dead peer never opens,
+    /// without consulting the control stream's end-of-stream: the open
+    /// item of ruling T143, pinned by
+    /// `survivor_parks_when_its_peer_vanishes_before_its_first_stream`
+    /// and ruled in T145; the lane that closes it asserts this count is
+    /// zero.
+    pub parked: usize,
 }
 
 // ---- strategies ------------------------------------------------------------
 
 /// Most peers a plan's founding fleet can hold. Every bound here is
-/// public so dominance arguments (the envelope session pin in
-/// `tests/disruption.rs`) derive from the generator instead of
-/// transcribing it.
+/// public so the envelope session pin in `tests/disruption.rs` derives
+/// from the generator instead of transcribing it.
 pub const MAX_PLAN_PEERS: usize = 5;
 
 /// Most messages a plan can insert at the seed before forking.
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 318:
>
> fresh-eyes repair, round 2: once vanishes actually fired, the proptest parked survivors after mid-stream vanishes too (three of three runs at the first tight draw), so the open item's reach is wider than the handshake window and the family could not stay green while failing parks by name. A park after a planned vanish is therefore aborted (the survivor's task, so its handle drops and the peer reclaims) and counted here; the count is the T145 lane's assertion target. Named deviation from T143's `fails by name`, for the owner.

<a id="hunk-56"></a>
### tests/common/sim.rs `@@ -274,11 +331,12 @@ pub const MAX_PLAN_SEED_MESSAGES: usize = 7;`

```diff
@@ -274,11 +331,12 @@ pub const MAX_PLAN_SEED_MESSAGES: usize = 7;
 /// Most operations one founder's activity script can carry.
 pub const MAX_PLAN_SCRIPT_OPS: usize = 7;
 
-/// Strategy for one endpoint's fault plan.
+/// Strategy for one endpoint's fault plan: cuts only.
 ///
 /// With `faults` disabled it is always clean, so a whole plan generated
 /// under `false` is loss-free by construction; enabled, each direction
-/// independently stays clean or cuts at an arbitrary offset.
+/// independently stays clean or cuts at an arbitrary offset. For harnesses
+/// that drive sessions through [`fault::faulty`], which admits no vanish.
 pub fn arb_fault(faults: bool) -> BoxedStrategy<FaultPlan> {
     if !faults {
         return Just(FaultPlan::NONE).boxed();
```

<!-- annotation -->
> **tests-disruption-handshake-3** (T143), line 340:
>
> `arb_fault_within` folded back into `arb_fault`: its only consumer was the deleted child family.

<a id="hunk-57"></a>
### tests/common/sim.rs `@@ -288,10 +346,44 @@ pub fn arb_fault(faults: bool) -> BoxedStrategy<FaultPlan> {`

```diff
@@ -288,10 +346,44 @@ pub fn arb_fault(faults: bool) -> BoxedStrategy<FaultPlan> {
         .prop_map(|(write_cut, read_cut)| FaultPlan {
             write_cut,
             read_cut,
+            vanish: None,
         })
         .boxed()
 }
 
+/// [`arb_fault`] for this engine's plans, whose endpoints may also vanish
+/// mid-stream.
+///
+/// A vanish lands on one of the endpoint's first [`MAX_VANISH_STREAM`]
+/// data streams, at an offset below [`MAX_VANISH_OFFSET`] into it,
+/// weighted toward the first stream and its first byte, which every
+/// endpoint that opens a stream at all reaches; the bounds keep every
+/// stream and byte of the envelope session reachable, and a point past
+/// what the endpoint writes never fires. Without a vanish an endpoint runs
+/// exactly as [`arb_fault`]'s would.
+///
+/// A vanish between the handshake and the first data stream is not drawn:
+/// a survivor of one parks on its first accept, the open item the ignored
+/// `survivor_notices_a_peer_vanished_before_its_first_stream` holds, and
+/// this family must stay green; that point joins the family when the
+/// item closes.
+fn arb_fault_or_vanish(faults: bool) -> BoxedStrategy<FaultPlan> {
+    if !faults {
+        return Just(FaultPlan::NONE).boxed();
+    }
+    let vanish = prop_oneof![
+        3 => Just(None),
+        1 => (
+            prop_oneof![3 => Just(0usize), 1 => 0..MAX_VANISH_STREAM],
+            prop_oneof![1 => Just(0usize), 2 => 0..MAX_VANISH_OFFSET],
+        )
+            .prop_map(|(index, offset)| Some(Vanish::OnStream { index, offset })),
+    ];
+    (arb_fault(faults), vanish)
+        .prop_map(|(cuts, vanish)| FaultPlan { vanish, ..cuts })
+        .boxed()
+}
+
 fn arb_activity() -> impl Strategy<Value = Activity> {
     prop_oneof![
         any::<u64>().prop_map(Activity::Send),
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 370:
>
> fresh-eyes repair, round 2, item 1: the draw restated to what is drawn. Even inside the pinned bounds a uniform offset fired in 3 of 503 draws over 256 deterministic plans (typical streams are far shorter than the envelope's widest), so the draw is weighted toward the first stream and its first byte, which every endpoint that opens a stream reaches: 105 of 660 fired over the same sample, 10 within the first 32 plans. `arb_fault` stays cuts-only for `bookmark_causality`.

<a id="hunk-58"></a>
### tests/common/sim.rs `@@ -302,18 +394,22 @@ fn arb_activity() -> impl Strategy<Value = Activity> {`

```diff
@@ -302,18 +394,22 @@ fn arb_activity() -> impl Strategy<Value = Activity> {
 /// `a` and `b` are kept distinct by construction (offset in `1..n`), so the
 /// shrinker can never collapse a session onto a single peer.
 fn arb_session(n: usize, faults: bool) -> impl Strategy<Value = Session> {
-    (0..n, 1..n, arb_fault(faults), arb_fault(faults)).prop_map(
-        move |(a, off, fault_a, fault_b)| Session {
+    (
+        0..n,
+        1..n,
+        arb_fault_or_vanish(faults),
+        arb_fault_or_vanish(faults),
+    )
+        .prop_map(move |(a, off, fault_a, fault_b)| Session {
             a,
             b: (a + off) % n,
             fault_a,
             fault_b,
-        },
-    )
+        })
 }
 
 fn arb_retire(n: usize, faults: bool) -> impl Strategy<Value = RetireOp> {
-    (0..n, 1..n, arb_fault(faults)).prop_map(move |(retiree, off, fault)| RetireOp {
+    (0..n, 1..n, arb_fault_or_vanish(faults)).prop_map(move |(retiree, off, fault)| RetireOp {
         retiree,
         absorber: (retiree + off) % n,
         fault,
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 397:
>
> Sessions and retirements draw `arb_fault_or_vanish`, the engine's own strategy, so their endpoints may vanish.

<a id="hunk-59"></a>
### tests/common/sim.rs `@@ -333,7 +429,7 @@ pub fn arb_plan() -> impl Strategy<Value = Plan> {`

```diff
@@ -333,7 +429,7 @@ pub fn arb_plan() -> impl Strategy<Value = Plan> {
     (any::<bool>(), 2usize..=MAX_PLAN_PEERS).prop_flat_map(|(faults, n)| {
         (
             prop::collection::vec(any::<u64>(), 0..=MAX_PLAN_SEED_MESSAGES),
-            prop::collection::vec(arb_fault(faults), 0..=3),
+            prop::collection::vec(arb_fault_or_vanish(faults), 0..=3),
             prop::collection::vec(
                 prop::collection::vec(arb_activity(), 0..=MAX_PLAN_SCRIPT_OPS),
                 n,
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 432:
>
> Bootstrapping joiners draw `arb_fault_or_vanish` too.

<a id="hunk-60"></a>
### tests/common/sim.rs `@@ -382,7 +478,7 @@ pub fn assert_honest_error(e: &Error) {`

```diff
@@ -382,7 +478,7 @@ pub fn assert_honest_error(e: &Error) {
 }
 
 /// Whether an error is exactly one of the disruption harness's wire cuts.
-pub fn is_honest_error(error: &Error) -> bool {
+fn is_honest_error(error: &Error) -> bool {
     match error {
         Error::Io(error) => honest_io(error),
         // A cut that lands inside the preamble surfaces as the typed
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 481:
>
> fresh-eyes repair, round 2, item 5: narrowed to private with `probe_disjointness` (no consumer outside this module); item 7: em-dashes replaced in the two module-doc paragraphs the T143 commits touched.

<a id="hunk-61"></a>
### tests/common/sim.rs `@@ -404,17 +500,11 @@ pub fn is_honest_error(error: &Error) -> bool {`

```diff
@@ -404,17 +500,11 @@ pub fn is_honest_error(error: &Error) -> bool {
 }
 
 /// Whether an I/O source is one of the fault harness's severed-wire outcomes.
-///
-/// `ConnectionRefused` belongs here because of how a dead peer manifests to
-/// a stream *open*: on a per-connection transport (the inter-process TCP
-/// link), a peer that died mid-session takes its stream listener with it,
-/// and the next `connect` is refused rather than reset.
 fn honest_io(error: &std::io::Error) -> bool {
     matches!(
         error.kind(),
         std::io::ErrorKind::BrokenPipe
             | std::io::ErrorKind::ConnectionReset
-            | std::io::ErrorKind::ConnectionRefused
             | std::io::ErrorKind::UnexpectedEof
     )
 }
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 503:
>
> `ConnectionRefused` leaves the honest set with its rationale: the only transport that produced it was the deleted TCP family; over the memory link a vanished peer's dropped acceptor refuses a connect as `BrokenPipe`.

<a id="hunk-62"></a>
### tests/common/sim.rs `@@ -450,36 +540,106 @@ fn honest_remote(error: &RemoteError<Infallible>) -> bool {`

```diff
@@ -450,36 +540,106 @@ fn honest_remote(error: &RemoteError<Infallible>) -> bool {
 }
 
 /// [`assert_honest_error`] over a session outcome.
-pub fn assert_honest_gossip(out: &Result<rumors::Gossiped, Error>) {
+pub fn assert_honest_gossip(out: &Result<Gossiped, Error>) {
     if let Err(e) = out {
         assert_honest_error(e);
     }
 }
 
+/// The survivor of a vanished counterparty ends with an honest error,
+/// never `Ok`: a vanish trips at a write or a stream open, so the
+/// counterparty's completion marker, its last write, never lands.
+pub fn assert_survivor(out: &Result<Gossiped, Error>) {
+    assert!(
+        out.is_err(),
+        "the survivor of a vanished peer certified the session: {out:?}"
+    );
+    assert_honest_gossip(out);
+}
+
+/// What awaiting under [`SESSION_DEADLINE`] produced.
+enum Bounded<T> {
+    Done(T),
+    /// The deadline passed with a vanish planned: the survivor parked on
+    /// its dead peer (see [`SimOutcome::parked`]).
+    Parked,
+}
+
+/// Await `work` under [`SESSION_DEADLINE`]: expiry with a vanish planned
+/// (`vanishing`) is a park, expiry without one a deadlock named `what`.
+async fn bounded<F: Future>(what: &str, vanishing: bool, work: F) -> Bounded<F::Output> {
+    match tokio::time::timeout(SESSION_DEADLINE, work).await {
+        Ok(output) => Bounded::Done(output),
+        Err(_) if vanishing => Bounded::Parked,
+        Err(_) => panic!(
+            "{what} parked past SESSION_DEADLINE ({SESSION_DEADLINE:?}): a protocol deadlock"
+        ),
+    }
+}
+
+/// One phase's vanish accounting: endpoints that vanished, and whether the
+/// survivor parked and was aborted.
+#[derive(Clone, Copy, Default)]
+struct Vanishes {
+    vanished: usize,
+    parked: bool,
+}
+
 // ---- the engine ------------------------------------------------------------
 
 /// Run one gossip session between two handles over a fault-injected
-/// in-memory wire.
+/// in-memory wire, returning its vanish accounting.
 ///
 /// Each side's halves are owned by its own task, so the failing side's
-/// drop surfaces as EOF to its counterparty instead of wedging the session.
-async fn run_session(a: Rumors<u64>, b: Rumors<u64>, fault_a: FaultPlan, fault_b: FaultPlan) {
+/// drop surfaces as EOF to its counterparty instead of wedging the
+/// session. A side that vanishes leaves its counterparty to end alone,
+/// with an honest error, or parked and aborted at the deadline.
+async fn run_session(
+    a: Rumors<u64>,
+    b: Rumors<u64>,
+    fault_a: FaultPlan,
+    fault_b: FaultPlan,
+) -> Vanishes {
     let (link_a, link_b) = rumors::link::memory();
-    let task_a = tokio::spawn(async move {
-        let mut link = fault::faulty(link_a, fault_a);
-        a.gossip(&mut link).await
-    });
-    let task_b = tokio::spawn(async move {
-        let mut link = fault::faulty(link_b, fault_b);
-        b.gossip(&mut link).await
-    });
-    assert_honest_gossip(&task_a.await.expect("session task A"));
-    assert_honest_gossip(&task_b.await.expect("session task B"));
+    let task_a = tokio::spawn(fault::drive(link_a, fault_a, async move |link| {
+        a.gossip(link).await
+    }));
+    let task_b = tokio::spawn(fault::drive(link_b, fault_b, async move |link| {
+        b.gossip(link).await
+    }));
+    let (abort_a, abort_b) = (task_a.abort_handle(), task_b.abort_handle());
+    let vanishing = fault_a.vanish.is_some() || fault_b.vanish.is_some();
+    let joined = bounded("a session", vanishing, async {
+        tokio::join!(task_a, task_b)
+    })
+    .await;
+    let Bounded::Done((driven_a, driven_b)) = joined else {
+        abort_a.abort();
+        abort_b.abort();
+        return Vanishes {
+            vanished: 1,
+            parked: true,
+        };
+    };
+    let driven_a = driven_a.expect("session task A");
+    let driven_b = driven_b.expect("session task B");
+    match (&driven_a.outcome, &driven_b.outcome) {
+        (Some(out_a), Some(out_b)) => {
+            assert_honest_gossip(out_a);
+            assert_honest_gossip(out_b);
+        }
+        (Some(survivor), None) | (None, Some(survivor)) => assert_survivor(survivor),
+        (None, None) => {}
+    }
+    Vanishes {
+        vanished: usize::from(driven_a.vanished()) + usize::from(driven_b.vanished()),
+        parked: false,
+    }
 }
 
 /// Serve one bootstrap from `server` mid-chaos, the joiner's endpoint
 /// faulted by `fault`. Returns the newcomer, or `None` for a possible
-/// in-flight loss of the donated fork.
+/// in-flight loss of the donated fork, with the vanish accounting.
 ///
 /// This is the most delicate intra-set race the engine exercises: serving
 /// a bootstrap must snapshot the tree and speculatively fork the party in
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 552:
>
> The survivor of a vanish never certifies: the vanish trips at a write or an open, so the counterparty's completion marker (its last write) never lands. Applied in all three faulted phases.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 570:
>
> fresh-eyes repair, round 2: the deadline distinguishes a park after a planned vanish (returned as `Parked`, tasks aborted by the caller) from a deadlock (panics by name).

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 597:
>
> Sessions, bootstraps, and retirements run through `drive` under `bounded`; a vanished bootstrapper counts as a possible loss like a failed one, and a vanished retiree is a recorded loss (`Transfer::Lost`, slot empty): dropping a retire future destroys the consumed peer.

<a id="hunk-63"></a>
### tests/common/sim.rs `@@ -490,32 +650,59 @@ async fn run_session(a: Rumors<u64>, b: Rumors<u64>, fault_a: FaultPlan, fault_b`

```diff
@@ -490,32 +650,59 @@ async fn run_session(a: Rumors<u64>, b: Rumors<u64>, fault_a: FaultPlan, fault_b
 ///
 /// The serving side stays clean; the joiner's fault plan covers both
 /// observable directions of a duplex (its read cut models the server's
-/// frames dying in flight). A joiner that fails may or may not have cost
-/// the server its donated fork, so it conservatively counts as a possible
-/// loss either way.
+/// frames dying in flight). A joiner that fails or vanishes may or may
+/// not have cost the server its donated fork, so it conservatively counts
+/// as a possible loss either way.
 async fn run_boot(
     server: Rumors<u64>,
     fault: FaultPlan,
     window: WindowChoice,
-) -> Option<Peer<u64>> {
+) -> (Option<Peer<u64>>, Vanishes) {
     let (boot_side, serve_side) = rumors::link::memory();
     let serve = tokio::spawn(async move {
         let mut link = fault::faulty(serve_side, FaultPlan::NONE);
         server.gossip(&mut link).await
     });
-    let boot = tokio::spawn(async move {
-        let mut link = fault::faulty(boot_side, fault);
-        Peer::<u64>::bootstrap().join(&mut link).await
-    });
-    assert_honest_gossip(&serve.await.expect("bootstrap serve task"));
-    match boot.await.expect("bootstrap join task") {
+    let boot = tokio::spawn(fault::drive(boot_side, fault, async move |link| {
+        Peer::<u64>::bootstrap().join(link).await
+    }));
+    let (abort_serve, abort_boot) = (serve.abort_handle(), boot.abort_handle());
+    let joined = bounded("a bootstrap", fault.vanish.is_some(), async {
+        tokio::join!(serve, boot)
+    })
+    .await;
+    let Bounded::Done((served, driven)) = joined else {
+        abort_serve.abort();
+        abort_boot.abort();
+        return (
+            None,
+            Vanishes {
+                vanished: 1,
+                parked: true,
+            },
+        );
+    };
+    let served = served.expect("bootstrap serve task");
+    let Some(joined) = driven.expect("bootstrap join task").outcome else {
+        assert_survivor(&served);
+        return (
+            None,
+            Vanishes {
+                vanished: 1,
+                parked: false,
+            },
+        );
+    };
+    assert_honest_gossip(&served);
+    let newcomer = match joined {
         Ok(Some(newcomer)) => Some(window.apply(newcomer)),
         Ok(None) => unreachable!("the serving peer is never itself bootstrapping"),
         Err(e) => {
             assert_honest_error(&e);
             None
         }
-    }
+    };
+    (newcomer, Vanishes::default())
 }
 
 /// Run one peer's activity script, yielding between operations so it
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 653:
>
> `run_boot` runs the joiner through `drive` under `bounded`; a vanished joiner is a possible loss like a failed one and its server must end with an honest error; round 2: a server parked on a vanished joiner is aborted with the joiner and counted, the loss unchanged.

<a id="hunk-64"></a>
### tests/common/sim.rs `@@ -634,7 +821,7 @@ async fn run_observers(handle: Rumors<u64>, done: Arc<AtomicBool>) {`

```diff
@@ -634,7 +821,7 @@ async fn run_observers(handle: Rumors<u64>, done: Arc<AtomicBool>) {
 /// *before* it rides the wire and joined into the recipient *after* it
 /// arrives, so no interleaving of these per-peer aliases can witness one
 /// region twice unless linearity is actually broken.
-pub async fn probe_disjointness(handles: Vec<Rumors<u64>>, done: Arc<AtomicBool>) {
+async fn probe_disjointness(handles: Vec<Rumors<u64>>, done: Arc<AtomicBool>) {
     loop {
         let finished = done.load(Ordering::Acquire);
         let parties: Vec<(usize, Party)> = handles
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 824:
>
> fresh-eyes repair, round 2, item 5: `probe_disjointness` is private; its only caller is `run_plan`.

<a id="hunk-65"></a>
### tests/common/sim.rs `@@ -663,6 +850,8 @@ pub async fn probe_disjointness(handles: Vec<Rumors<u64>>, done: Arc<AtomicBool>`

```diff
@@ -663,6 +850,8 @@ pub async fn probe_disjointness(handles: Vec<Rumors<u64>>, done: Arc<AtomicBool>
 /// violation observable mid-run (dishonest errors, transient overlap).
 pub async fn run_plan(plan: Plan) -> SimOutcome {
     let mut possible_losses = 0usize;
+    let mut vanished = 0usize;
+    let mut parked = 0usize;
 
     // The insert side of the value ledger, known from the plan alone:
     // sends are local operations and always execute.
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 853:
>
> fresh-eyes repair, round 2: the run's vanish and park counters, reported in `SimOutcome`.

<a id="hunk-66"></a>
### tests/common/sim.rs `@@ -739,11 +928,16 @@ pub async fn run_plan(plan: Plan) -> SimOutcome {`

```diff
@@ -739,11 +928,16 @@ pub async fn run_plan(plan: Plan) -> SimOutcome {
         redaction_logs.push(task.await.expect("activity task"));
     }
     for task in session_tasks {
-        task.await.expect("session task");
+        let vanishes = task.await.expect("session task");
+        vanished += vanishes.vanished;
+        parked += usize::from(vanishes.parked);
     }
     let mut newcomers = Vec::new();
     for task in boot_tasks {
-        match task.await.expect("bootstrap task") {
+        let (newcomer, vanishes) = task.await.expect("bootstrap task");
+        vanished += vanishes.vanished;
+        parked += usize::from(vanishes.parked);
+        match newcomer {
             Some(newcomer) => newcomers.push(newcomer),
             None => possible_losses += 1,
         }
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 931:
>
> fresh-eyes repair, round 2: sessions and bootstraps fold their `Vanishes` into the counters as their tasks are joined.

<a id="hunk-67"></a>
### tests/common/sim.rs `@@ -788,18 +982,60 @@ pub async fn run_plan(plan: Plan) -> SimOutcome {`

```diff
@@ -788,18 +982,60 @@ pub async fn run_plan(plan: Plan) -> SimOutcome {
         // on `Rumors`; it converts back the moment the session ends.
         let absorber = absorber.into_rumors();
         let (retiree_side, absorber_side) = rumors::link::memory();
-        let fault = op.fault;
-        let (outcome, absorbed) = tokio::join!(
+        let retiring = tokio::spawn(fault::drive(retiree_side, op.fault, async move |link| {
+            retiree.retire(link).await
+        }));
+        let absorbing = tokio::spawn({
+            let absorber = absorber.clone();
             async move {
-                let mut link = fault::faulty(retiree_side, fault);
-                retiree.retire(&mut link).await
-            },
-            async {
                 let mut link = fault::faulty(absorber_side, FaultPlan::NONE);
                 absorber.gossip(&mut link).await
-            },
-        );
-        assert_honest_gossip(&absorbed);
+            }
+        });
+        let (abort_retiring, abort_absorbing) = (retiring.abort_handle(), absorbing.abort_handle());
+        let joined = bounded("a retirement", op.fault.vanish.is_some(), async {
+            tokio::join!(retiring, absorbing)
+        })
+        .await;
+        // A retiree that vanished mid-retirement is gone with its party:
+        // dropping a retire future destroys the consumed peer, and the
+        // absorber, which cannot have committed, holds nothing of it. An
+        // absorber parked on the vanished retiree is aborted and the loss
+        // is the same.
+        let outcome = match joined {
+            Bounded::Parked => {
+                abort_retiring.abort();
+                abort_absorbing.abort();
+                parked += 1;
+                None
+            }
+            Bounded::Done((driven, absorbed)) => {
+                let driven = driven.expect("retire task");
+                let absorbed = absorbed.expect("absorb task");
+                match driven.outcome {
+                    None => {
+                        assert_survivor(&absorbed);
+                        None
+                    }
+                    Some(outcome) => {
+                        assert_honest_gossip(&absorbed);
+                        Some((outcome, absorbed))
+                    }
+                }
+            }
+        };
+        let Some((outcome, absorbed)) = outcome else {
+            vanished += 1;
+            possible_losses += 1;
+            transfers.push((op.retiree, op.absorber, Transfer::Lost));
+            slots[op.absorber] = Some(
+                absorber
+                    .try_into_peer()
+                    .await
+                    .expect("the absorber's sole handle reclaims the Peer"),
+            );
+            continue;
+        };
         match outcome {
             // The retiree believes its party was delivered; if the absorber
             // failed too, delivery is unconfirmed on both sides and the
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 985:
>
> The retirement runs its two sides as spawned tasks so a park can abort both (round 2); the retiree drives through `drive`; a vanished or parked retiree is `Transfer::Lost` and one possible loss, since dropping a retire future destroys the consumed peer, and the absorber then reclaims its own `Peer` as before.

<a id="hunk-68"></a>
### tests/common/sim.rs `@@ -871,6 +1107,8 @@ pub async fn run_plan(plan: Plan) -> SimOutcome {`

```diff
@@ -871,6 +1107,8 @@ pub async fn run_plan(plan: Plan) -> SimOutcome {
         possible_losses,
         inserted,
         redactions: by_version.into_values().collect(),
+        vanished,
+        parked,
     }
 }
 
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 1110:
>
> `SimOutcome` carries the vanish and park counts (round 2).

<a id="hunk-69"></a>
### tests/common/tcp.rs `@@ -1,7 +1,6 @@`

```diff
@@ -1,7 +1,6 @@
-//! A per-session TCP [`Link`] for the inter-process simulations.
+//! A per-session TCP [`Link`]: the one-connection-per-stream instantiation.
 //!
-//! This is the "one connection per stream" instantiation of the link
-//! contract at its simplest: every session gets its own dedicated listener
+//! The link contract at its simplest: every session gets its own dedicated listener
 //! on each side, so no routing header or connection table is needed — a
 //! connection arriving at a session's listener *is* one of that session's
 //! streams. The caller supplies the initial TCP connection; [`link`] turns
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 1:
>
> `common::tcp` stays: `tests/tcp_link.rs` is its consumer (checked with `git grep`). Its module doc no longer names the deleted simulation as its purpose.

<a id="hunk-70"></a>
### tests/disruption.rs `@@ -1,43 +1,32 @@`

```diff
@@ -1,43 +1,32 @@
 //! Party linearity and disjointness under arbitrary disruption.
 //!
-//! Two simulations, one engine of invariants (`common::sim`):
-//!
-//! - **Intra-process**: a fleet of peers on one multi-thread runtime,
-//!   every gossip session, bootstrap, send, and redact spawned at once,
-//!   over in-memory wires that may be severed at arbitrary byte offsets.
-//! - **Inter-process**: peers split across genuinely separate OS
-//!   processes — the test binary re-executes itself as each child — over a
-//!   real TCP link (one socket per stream; see `common::tcp`) with the same
-//!   fault injection on the child side, children retiring home at the end
-//!   so the id-space can be audited.
-//!
-//! Both assert the same global properties, stated on each test below. Task
-//! and process interleavings are nondeterministic, so a counterexample may
-//! not replay byte-for-byte; the invariants quantify over *all*
-//! interleavings, so any failure is a genuine one.
+//! One simulation (`common::sim`): a fleet of peers on one multi-thread
+//! runtime, every gossip session, bootstrap, send, and redact spawned at
+//! once, over in-memory wires that may be severed at arbitrary byte
+//! offsets. The global properties are stated on each test below. Task
+//! interleavings are nondeterministic, so a counterexample may not replay
+//! byte-for-byte; the invariants quantify over *all* interleavings, so any
+//! failure is a genuine one.
 
 mod common;
 
 use std::collections::BTreeSet;
-use std::process::Stdio;
-use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
-use std::sync::{Arc, Mutex};
-use std::time::Duration;
 
 use proptest::prelude::*;
-use rumors::{Peer, Retire, Rumors};
-use tokio::net::{TcpListener, TcpStream};
+use proptest::strategy::ValueTree;
+use proptest::test_runner::TestRunner;
+use rumors::testing::{Quiescence, run_to_quiescence};
+use rumors::{Peer, Rumors};
 
-use crate::common::fault::{self, FaultPlan};
+use crate::common::fault::{self, FaultPlan, Vanish};
 use crate::common::sim::{
-    Activity, MAX_PLAN_PEERS, MAX_PLAN_SCRIPT_OPS, MAX_PLAN_SEED_MESSAGES, Plan, Redaction,
-    RetireOp, Session, Transfer, arb_fault, arb_plan, assert_converged, assert_deletion_honored,
-    assert_honest_error, assert_honest_gossip, assert_party_invariants, assert_value_oracle,
-    is_honest_error, lost_custody, probe_disjointness, quiesce, run_plan, survivor_readouts,
+    Activity, MAX_PLAN_PEERS, MAX_PLAN_SCRIPT_OPS, MAX_PLAN_SEED_MESSAGES, MAX_SHRINK_TIME,
+    MAX_VANISH_OFFSET, MAX_VANISH_STREAM, Plan, Redaction, RetireOp, Session, Transfer, arb_plan,
+    assert_converged, assert_deletion_honored, assert_party_invariants, assert_survivor,
+    assert_value_oracle, lost_custody, quiesce, run_plan, survivor_readouts,
 };
-use crate::common::tcp;
 use crate::common::window::{WindowAssignment, WindowChoice};
-use crate::common::wire::bootstrap_fork_async;
+use crate::common::wire::bootstrap_fork;
 
 /// A fresh multi-thread runtime per simulation, so tasks interleave with
 /// real parallelism rather than cooperative scheduling alone.
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 3:
>
> Module doc for the one remaining simulation; the inter-process paragraph left with the family.

<a id="hunk-71"></a>
### tests/disruption.rs `@@ -48,18 +37,25 @@ fn mt_runtime() -> tokio::runtime::Runtime {`

```diff
@@ -48,18 +37,25 @@ fn mt_runtime() -> tokio::runtime::Runtime {
         .expect("build multi-thread runtime")
 }
 
-// ---- intra-process ----------------------------------------------------------
+// ---- the property -----------------------------------------------------------
 
 proptest! {
+    #![proptest_config(ProptestConfig {
+        max_shrink_time: MAX_SHRINK_TIME,
+        ..ProptestConfig::default()
+    })]
+
     /// Under arbitrary concurrent gossip over wires cut at arbitrary byte
-    /// offsets, the global party invariants hold:
+    /// offsets, with peers vanishing mid-stream, the global party
+    /// invariants hold:
     ///
-    /// 1. every session failure is an injected I/O fault, never
-    ///    `PartyOverlap` or a protocol violation;
+    /// 1. every session failure is an injected I/O fault or the honest
+    ///    severance a vanished peer leaves behind, never `PartyOverlap`
+    ///    or a protocol violation;
     /// 2. at every probed instant the live parties are pairwise disjoint;
     /// 3. after a clean heal, all survivors converge to identical content;
     /// 4. when no hand-off was lost in flight, the surviving parties
-    ///    fold-join back to exactly `Party::seed()` — the id-space is
+    ///    fold-join back to exactly `Party::seed()` -- the id-space is
     ///    conserved with no duplication and no leak;
     /// 5. no retained redaction's message is live at any survivor (deletion
     ///    honoring against the execution-time redaction log; every
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 40:
>
> fresh-eyes repair, round 2, item 4: the section header no longer says `intra-process` (one family); item 7: the em-dash in the property's doc replaced.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 43:
>
> fresh-eyes repair, round 2, item 2: shrink time bounded on the plan proptests.

<a id="hunk-72"></a>
### tests/disruption.rs `@@ -79,8 +75,13 @@ proptest! {`

```diff
@@ -79,8 +75,13 @@ proptest! {
     ///
     /// The chaos: overlapping sessions through
     /// cloned [`Rumors`] handles, concurrent sends and redactions,
-    /// bootstraps served mid-chaos against the same shared state, and
-    /// retirements.
+    /// bootstraps served mid-chaos against the same shared state,
+    /// retirements, and endpoints that vanish mid-protocol (their session
+    /// dropped with its link, promised streams never opened). A survivor
+    /// that parks on a vanished peer is aborted at the session deadline
+    /// and counted (`SimOutcome::parked`), the open item ruling T145
+    /// assigns; a session that parks with no vanish planned is a deadlock
+    /// and fails by name.
     #[test]
     fn disrupted_concurrent_gossip_upholds_party_invariants(plan in arb_plan()) {
         mt_runtime().block_on(check_plan(plan));
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 78:
>
> The property's doc names vanishing endpoints among the chaos and, from round 2, the counted park after a planned vanish versus the named deadlock.

<a id="hunk-73"></a>
### tests/disruption.rs `@@ -122,6 +123,191 @@ async fn check_plan(plan: Plan) {`

```diff
@@ -122,6 +123,191 @@ async fn check_plan(plan: Plan) {
     );
 }
 
+/// The vanish dimension is live in the generated plan population: across
+/// a deterministic sample of plans run through the engine, some endpoint
+/// reaches its vanish point.
+///
+/// A drawn vanish past the end of every stream its endpoint opens never
+/// trips, so the floor counts vanishes that fired, not plans that carry
+/// one.
+#[test]
+fn vanishes_fire_in_the_generated_population() {
+    let mut runner = TestRunner::deterministic();
+    let strategy = arb_plan();
+    let runtime = mt_runtime();
+    let mut fired = 0usize;
+    for _ in 0..32 {
+        let plan = strategy
+            .new_tree(&mut runner)
+            .expect("plan strategy always generates")
+            .current();
+        fired += runtime.block_on(run_plan(plan)).vanished;
+    }
+    assert!(
+        fired > 0,
+        "no endpoint of a sampled plan reached its vanish point: the vanish \
+         dimension has silently left the population"
+    );
+}
+
+/// Run one session under the closed-world poller in which `a`, holding
+/// the only new content, vanishes at `point`, and `b` survives with
+/// nothing to send: `b`'s outcome, or the poller's name for `b` parking.
+fn survive_a_vanish(
+    point: Vanish,
+) -> Result<Result<rumors::Gossiped, rumors::Error>, rumors::testing::Quiescence> {
+    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
+    a.send_all(0..8).unwrap();
+    let b = bootstrap_fork(&a);
+    a.send_all(8..16).unwrap();
+    let (a_link, b_link) = rumors::link::memory();
+    let vanishing = FaultPlan {
+        vanish: Some(point),
+        ..FaultPlan::NONE
+    };
+    run_to_quiescence(async {
+        let (driven, survivor) = futures::join!(
+            fault::drive(a_link, vanishing, async move |link| a.gossip(link).await),
+            async {
+                let mut link = fault::faulty(b_link, FaultPlan::NONE);
+                b.gossip(&mut link).await
+            },
+        );
+        assert!(
+            driven.outcome.is_none(),
+            "the vanishing peer never reached its point: {:?}",
+            driven.outcome
+        );
+        survivor
+    })
+}
+
+/// A peer whose counterparty vanishes mid-stream ends its session with
+/// an honest error: the vanish fires, and the survivor reads end-of-stream
+/// inside a frame.
+#[test]
+fn survivor_notices_a_peer_vanished_mid_stream() {
+    match survive_a_vanish(Vanish::OnStream {
+        index: 0,
+        offset: 8,
+    }) {
+        Ok(survivor) => assert_survivor(&survivor),
+        Err(quiescence) => {
+            panic!("the survivor parked after its peer vanished mid-stream: {quiescence:?}")
+        }
+    }
+}
+
+/// A peer whose counterparty vanishes after their handshake, before
+/// opening the first data stream, parks on its first accept without
+/// consulting the control stream's end-of-stream: the closed-world poller
+/// names it `Stalled`.
+///
+/// This pins the open item of ruling T143, ruled in T145: the
+/// `p2-vanish-liveness` lane makes the survivor end with an honest error,
+/// and flips this pin to `assert_survivor` on the outcome.
+#[test]
+fn survivor_parks_when_its_peer_vanishes_before_its_first_stream() {
+    let parked = survive_a_vanish(Vanish::AtFirstConnect);
+    assert!(
+        matches!(parked, Err(Quiescence::Stalled)),
+        "the survivor of a peer that vanished before opening a stream must park \
+         (Stalled) on this tree: {parked:?}"
+    );
+}
+
+/// A retiree that vanishes mid-retirement is a recorded loss: its party
+/// left with it, so the run counts one possible loss, its slot stays
+/// empty, and the relaxed party check holds over the survivor.
+///
+/// The retiree holds content the absorber lacks, so its retirement opens
+/// a stream and the mid-stream vanish fires.
+#[test]
+fn vanished_retiree_is_a_recorded_loss() {
+    mt_runtime().block_on(async {
+        let plan = Plan {
+            n_peers: 2,
+            seed_messages: vec![10],
+            faulty_boots: vec![],
+            scripts: vec![vec![], vec![Activity::Send(20), Activity::Send(30)]],
+            sessions: vec![],
+            retires: vec![RetireOp {
+                retiree: 1,
+                absorber: 0,
+                fault: FaultPlan {
+                    vanish: Some(Vanish::OnStream {
+                        index: 0,
+                        offset: 0,
+                    }),
+                    ..FaultPlan::NONE
+                },
+            }],
+            windows: WindowAssignment::floor(),
+        };
+        let outcome = run_plan(plan).await;
+        quiesce(&outcome.peers).await;
+        let readouts = survivor_readouts(&outcome.peers);
+        assert_converged(&outcome.peers, &readouts);
+        // The party check first: an accounting that missed the vanish
+        // would run it sharply and fail on the party that left.
+        assert_party_invariants(&outcome.peers, outcome.possible_losses);
+        assert_eq!(
+            outcome.possible_losses, 1,
+            "a vanished retiree is exactly one possible loss"
+        );
+        assert_eq!(
+            outcome.peers.len(),
+            1,
+            "the vanished retiree's slot is empty"
+        );
+    });
+}
+
+/// Pins the vanish draw's two bounds to the envelope session's measured
+/// stream shape, from both sides.
+///
+/// Every data stream an envelope endpoint opens is a reachable vanish
+/// ordinal (`streams <= MAX_VANISH_STREAM`) and the ordinal range is not
+/// vacuously wide (`MAX_VANISH_STREAM <= 2 * streams`); every byte of the
+/// widest stream is a reachable offset (`widest <= MAX_VANISH_OFFSET`)
+/// and the offset range is not vacuously wide
+/// (`MAX_VANISH_OFFSET <= 2 * widest`), so generated vanishes keep landing
+/// on streams that exist, inside them.
+#[test]
+fn vanish_draw_spans_the_envelope_session() {
+    let extent = mt_runtime().block_on(envelope_session_bytes());
+    println!(
+        "envelope session per endpoint: {} data streams, widest {} bytes",
+        extent.streams, extent.widest_stream
+    );
+    assert!(
+        extent.streams <= MAX_VANISH_STREAM,
+        "the envelope endpoint opens {} data streams, beyond MAX_VANISH_STREAM \
+         ({MAX_VANISH_STREAM}): later streams are unreachable vanish points",
+        extent.streams
+    );
+    assert!(
+        MAX_VANISH_STREAM <= 2 * extent.streams,
+        "MAX_VANISH_STREAM ({MAX_VANISH_STREAM}) is more than twice the envelope \
+         endpoint's {} data streams: most generated vanishes would name a stream \
+         that never opens",
+        extent.streams
+    );
+    assert!(
+        extent.widest_stream <= MAX_VANISH_OFFSET,
+        "the envelope's widest data stream carries {} bytes, beyond \
+         MAX_VANISH_OFFSET ({MAX_VANISH_OFFSET}): deep offsets are unreachable",
+        extent.widest_stream
+    );
+    assert!(
+        MAX_VANISH_OFFSET <= 2 * extent.widest_stream,
+        "MAX_VANISH_OFFSET ({MAX_VANISH_OFFSET}) is more than twice the envelope's \
+         widest stream ({} bytes): most generated vanishes would land past the \
+         end of every stream",
+        extent.widest_stream
+    );
+}
+
 // ---- value-oracle adequacy tripwires -----------------------------------------
 
 /// Whether `f` panics, with the unwind caught so the test can assert on it.
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 134:
>
> fresh-eyes repair, round 2, item 1: the floor counts vanishes that fired (`SimOutcome::vanished` over 32 deterministic plans run through the engine), replacing the drawn-only pin.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 189:
>
> Under the closed-world poller: a mid-stream vanish severs the survivor with an honest error (the fixture the ignored test shares).

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 210:
>
> fresh-eyes repair, round 2, item 3: the dormant ignored test is a positive pin of the current behavior (`Err(Quiescence::Stalled)`), doc naming the open item, ruling T145, and the `p2-vanish-liveness` lane that flips it; the ignore reason's text is in the doc.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 226:
>
> Deterministic pin that a mid-stream vanish fires in a retirement and is booked as one possible loss; the party check runs first so the accounting control fails there. negative control: booking the vanished retiree as `Transfer::Committed` with no loss fails `a loss-free run must reconstitute the seed's whole id-space`; output in the commit message.

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 277:
>
> fresh-eyes repair, round 2, item 1: two-sided pins of both draw bounds against the envelope session's measured stream shape (`envelope_session_bytes` now reports streams opened and the widest stream).

<a id="hunk-74"></a>
### tests/disruption.rs `@@ -374,7 +560,7 @@ const ENVELOPE_VALUES_PER_SIDE: u64 =`

```diff
@@ -374,7 +560,7 @@ const ENVELOPE_VALUES_PER_SIDE: u64 =
 /// [`max_cut_spans_the_envelope_session`] is what keeps the constant
 /// tracking reality. Metered with the same counters the fault cuts
 /// spend, so the result is directly comparable to cut offsets.
-async fn envelope_session_bytes() -> usize {
+async fn envelope_session_bytes() -> EnvelopeExtent {
     let seed = WindowChoice::Default
         .apply(Peer::<u64>::seed())
         .into_rumors();
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 563:
>
> fresh-eyes repair, round 2, item 1: the envelope meter returns an `EnvelopeExtent` so the byte pin and the vanish pins share one measured session.

<a id="hunk-75"></a>
### tests/disruption.rs `@@ -416,7 +602,20 @@ async fn envelope_session_bytes() -> usize {`

```diff
@@ -416,7 +602,20 @@ async fn envelope_session_bytes() -> usize {
     let (out_a, out_b) = tokio::join!(a.gossip(&mut link_a), b.gossip(&mut link_b));
     out_a.expect("envelope session A");
     out_b.expect("envelope session B");
-    meter_a.written().max(meter_b.written())
+    EnvelopeExtent {
+        bytes: meter_a.written().max(meter_b.written()),
+        streams: meter_a.streams_opened().max(meter_b.streams_opened()),
+        widest_stream: meter_a.widest_stream().max(meter_b.widest_stream()),
+    }
+}
+
+/// The envelope session's extent per endpoint, each the wider endpoint's:
+/// total bytes written, data streams opened, and bytes on the widest data
+/// stream.
+struct EnvelopeExtent {
+    bytes: usize,
+    streams: usize,
+    widest_stream: usize,
 }
 
 /// Pins `MAX_CUT` to the envelope session's measured byte extent, from
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 605:
>
> fresh-eyes repair, round 2, item 1: the extent takes each figure from the wider endpoint's meter, as the byte figure always did.

<a id="hunk-76"></a>
### tests/disruption.rs `@@ -430,7 +629,7 @@ async fn envelope_session_bytes() -> usize {`

```diff
@@ -430,7 +629,7 @@ async fn envelope_session_bytes() -> usize {
 /// stated at [`envelope_session_bytes`].
 #[test]
 fn max_cut_spans_the_envelope_session() {
-    let measured = mt_runtime().block_on(envelope_session_bytes());
+    let measured = mt_runtime().block_on(envelope_session_bytes()).bytes;
     println!("envelope session bytes per endpoint: {measured}");
     assert!(
         measured <= crate::common::sim::MAX_CUT,
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 632:
>
> fresh-eyes repair, round 2, item 1: the byte pin reads the extent's `bytes`; its assertions are unchanged.

<a id="hunk-77"></a>
### tests/disruption.rs `@@ -446,564 +645,3 @@ fn max_cut_spans_the_envelope_session() {`

```diff
@@ -446,564 +645,3 @@ fn max_cut_spans_the_envelope_session() {
         crate::common::sim::MAX_CUT,
     );
 }
-
-// ---- inter-process ----------------------------------------------------------
-
-/// Environment protocol between the parent test and its child processes.
-/// The presence of `CHILD_ADDR` is what turns the re-executed test binary
-/// into a child peer.
-const CHILD_ADDR: &str = "RUMORS_SIM_CHILD_ADDR";
-const CHILD_INDEX: &str = "RUMORS_SIM_CHILD_INDEX";
-const CHILD_SENDS: &str = "RUMORS_SIM_CHILD_SENDS";
-const CHILD_BOOT: &str = "RUMORS_SIM_CHILD_BOOT";
-const CHILD_SESSIONS: &str = "RUMORS_SIM_CHILD_SESSIONS";
-const CHILD_RETIRE: &str = "RUMORS_SIM_CHILD_RETIRE";
-
-/// Child exit codes: the loss-accounting back-channel. Anything else
-/// (including a panic's 101) fails the parent test.
-const EXIT_CLEAN: i32 = 0;
-/// Retired cleanly, but an earlier faulty bootstrap attempt failed: the
-/// fork served for it may be orphaned (possible loss).
-const EXIT_BOOT_LOSS: i32 = 2;
-/// The final retirement ended [`Retire::Uncertain`]: the party may be in
-/// limbo (possible loss).
-const EXIT_UNCERTAIN: i32 = 3;
-/// A state the protocol promises is unreachable for this topology.
-const EXIT_ANOMALY: i32 = 4;
-
-/// Wall-clock bound on each child process.
-const CHILD_DEADLINE: Duration = Duration::from_secs(60);
-
-/// The value of child `index`'s `s`-th send: distinct per child and per
-/// send, so the parent can assert that a cleanly-retired child's content
-/// all made it home.
-fn child_value(index: usize, s: usize) -> u64 {
-    (index as u64 + 1) * 1_000_000 + s as u64
-}
-
-fn encode_cut(cut: Option<usize>) -> String {
-    cut.map_or_else(|| "-".to_owned(), |n| n.to_string())
-}
-
-fn decode_cut(s: &str) -> Option<usize> {
-    (s != "-").then(|| s.parse().expect("malformed cut budget"))
-}
-
-fn encode_fault(fault: &FaultPlan) -> String {
-    format!(
-        "{}:{}",
-        encode_cut(fault.write_cut),
-        encode_cut(fault.read_cut)
-    )
-}
-
-fn decode_fault(s: &str) -> FaultPlan {
-    let (write, read) = s.split_once(':').expect("malformed fault plan");
-    FaultPlan {
-        write_cut: decode_cut(write),
-        read_cut: decode_cut(read),
-    }
-}
-
-/// One child process's script: how many sends, the fault plan for an
-/// initial (deliberately lossy) bootstrap attempt, per-session fault
-/// plans, and the fault plan for its final retirement.
-#[derive(Debug, Clone)]
-struct ChildPlan {
-    n_sends: usize,
-    boot: FaultPlan,
-    sessions: Vec<FaultPlan>,
-    retire: FaultPlan,
-}
-
-#[derive(Debug, Clone)]
-struct ProcPlan {
-    n_parent_peers: usize,
-    seed_messages: Vec<u64>,
-    children: Vec<ChildPlan>,
-}
-
-fn arb_child_plan(faults: bool) -> impl Strategy<Value = ChildPlan> {
-    (
-        0usize..6,
-        arb_fault(faults),
-        prop::collection::vec(arb_fault(faults), 1..4),
-        arb_fault(faults),
-    )
-        .prop_map(|(n_sends, boot, sessions, retire)| ChildPlan {
-            n_sends,
-            boot,
-            sessions,
-            retire,
-        })
-}
-
-/// As in `arb_plan`, the leading `bool` turns fault injection off for half
-/// of all plans, so the sharp seed-reconstitution check runs often.
-fn arb_proc_plan() -> impl Strategy<Value = ProcPlan> {
-    any::<bool>().prop_flat_map(|faults| {
-        (
-            1usize..=2,
-            prop::collection::vec(any::<u64>(), 0..4),
-            prop::collection::vec(arb_child_plan(faults), 1..=3),
-        )
-            .prop_map(|(n_parent_peers, seed_messages, children)| ProcPlan {
-                n_parent_peers,
-                seed_messages,
-                children,
-            })
-    })
-}
-
-/// Kill (and reap) a child process if the parent unwinds before it exits.
-struct KillOnDrop(std::process::Child);
-
-impl Drop for KillOnDrop {
-    fn drop(&mut self) {
-        let _ = self.0.kill();
-        let _ = self.0.wait();
-    }
-}
-
-proptest! {
-    #![proptest_config(ProptestConfig::with_cases(8))]
-
-    /// The same four invariants as the intra-process simulation, with the
-    /// fleet split across OS processes gossiping over real TCP sockets
-    /// severed at arbitrary byte offsets.
-    ///
-    /// Child processes bootstrap from
-    /// the parent, gossip concurrently with it (and with its own
-    /// in-process sessions), then retire home. Cleanly-retired children
-    /// must additionally leave every one of their sends in the parent's
-    /// converged content, and a loss-free run must fold the parent's
-    /// surviving parties back to exactly `Party::seed()`.
-    #[test]
-    fn inter_process_disruption_upholds_party_invariants(plan in arb_proc_plan()) {
-        mt_runtime().block_on(run_proc_plan(plan));
-    }
-}
-
-// ---- reconstructed inter-process counterexamples -----------------------------
-//
-// Historical shrunk counterexamples, preserved as explicit constructions:
-// their committed seeds regenerate through the fault strategy's cut range,
-// so a range change re-maps the offsets and the seed no longer replays the
-// case it pinned. Each test runs the exact plan its seed's shrink recorded,
-// under the same invariants as the proptest above. The seed files stay
-// committed; these constructions carry the counterexamples themselves.
-
-/// Shorthand for one endpoint's fault plan in a reconstructed counterexample.
-fn fp(write_cut: Option<usize>, read_cut: Option<usize>) -> FaultPlan {
-    FaultPlan {
-        write_cut,
-        read_cut,
-    }
-}
-
-/// Reconstructed counterexample: a single clean child whose final retirement
-/// wire dies at the very first written byte, forcing the
-/// recovered-then-retry path against a live parent.
-#[test]
-fn reconstructed_child_retire_cut_at_first_byte() {
-    mt_runtime().block_on(run_proc_plan(ProcPlan {
-        n_parent_peers: 1,
-        seed_messages: vec![],
-        children: vec![ChildPlan {
-            n_sends: 0,
-            boot: FaultPlan::NONE,
-            sessions: vec![FaultPlan::NONE],
-            retire: fp(Some(0), None),
-        }],
-    }));
-}
-
-/// Reconstructed counterexample: one child whose deliberately lossy first
-/// bootstrap dies mid-transfer and whose gossip sessions are cut in both
-/// directions, retiring cleanly afterward.
-#[test]
-fn reconstructed_child_faulted_boot_and_sessions() {
-    mt_runtime().block_on(run_proc_plan(ProcPlan {
-        n_parent_peers: 1,
-        seed_messages: vec![8910283091],
-        children: vec![ChildPlan {
-            n_sends: 2,
-            boot: fp(Some(1198), None),
-            sessions: vec![fp(Some(124), None), fp(Some(1308), Some(935))],
-            retire: FaultPlan::NONE,
-        }],
-    }));
-}
-
-/// Reconstructed counterexample: three children with cuts across every phase
-/// (bootstrap, sessions, retirement), overlapping at the parent.
-#[test]
-fn reconstructed_three_children_cut_across_phases() {
-    mt_runtime().block_on(run_proc_plan(ProcPlan {
-        n_parent_peers: 1,
-        seed_messages: vec![16893878652516216069, 17088246115921829969],
-        children: vec![
-            ChildPlan {
-                n_sends: 1,
-                boot: fp(None, Some(595)),
-                sessions: vec![FaultPlan::NONE],
-                retire: fp(None, Some(1243)),
-            },
-            ChildPlan {
-                n_sends: 2,
-                boot: fp(Some(1733), Some(1980)),
-                sessions: vec![fp(Some(151), Some(348))],
-                retire: fp(Some(1390), None),
-            },
-            ChildPlan {
-                n_sends: 2,
-                boot: fp(Some(637), Some(259)),
-                sessions: vec![
-                    FaultPlan::NONE,
-                    fp(Some(1206), Some(1750)),
-                    fp(Some(954), Some(25)),
-                ],
-                retire: fp(Some(98), Some(1600)),
-            },
-        ],
-    }));
-}
-
-/// Reconstructed counterexample: three children whose cuts sit near the deep
-/// end of the fault range the case was found under, severing sessions and
-/// retirements late in their byte streams.
-#[test]
-fn reconstructed_three_children_deep_cuts() {
-    mt_runtime().block_on(run_proc_plan(ProcPlan {
-        n_parent_peers: 1,
-        seed_messages: vec![597761422003064892],
-        children: vec![
-            ChildPlan {
-                n_sends: 4,
-                boot: fp(None, Some(670)),
-                sessions: vec![fp(Some(559), None), fp(Some(2047), None)],
-                retire: fp(Some(1947), Some(1274)),
-            },
-            ChildPlan {
-                n_sends: 0,
-                boot: fp(None, Some(1943)),
-                sessions: vec![FaultPlan::NONE, FaultPlan::NONE, fp(Some(1489), None)],
-                retire: fp(Some(695), None),
-            },
-            ChildPlan {
-                n_sends: 5,
-                boot: FaultPlan::NONE,
-                sessions: vec![fp(None, Some(1511)), FaultPlan::NONE, fp(None, Some(28))],
-                retire: fp(Some(1124), None),
-            },
-        ],
-    }));
-}
-
-async fn run_proc_plan(plan: ProcPlan) {
-    // Parent fleet: the seed and its clean forks, as shared-state handles
-    // so inbound sessions can overlap arbitrarily.
-    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
-    {
-        seed.send_all(plan.seed_messages.iter().copied()).unwrap();
-    }
-    let mut casts: Vec<Rumors<u64>> = vec![seed];
-    for _ in 1..plan.n_parent_peers {
-        let fork = bootstrap_fork_async(&casts[0]).await;
-        casts.push(fork);
-    }
-
-    let listener = TcpListener::bind("127.0.0.1:0")
-        .await
-        .expect("bind simulation listener");
-    let addr = listener.local_addr().expect("listener address");
-
-    // Serve every inbound connection with plain gossip, which transparently
-    // handles a bootstrapping or retiring counterparty. Errors are expected
-    // (the children sever wires); *dishonest* errors are recorded for the
-    // final assertion rather than panicking inside a detached task, and
-    // every error conservatively counts as a possible in-flight loss (a
-    // dying session may have been a bootstrap holding a donated fork).
-    let serve_errors = Arc::new(AtomicUsize::new(0));
-    let dishonest = Arc::new(Mutex::new(Vec::<String>::new()));
-    let accept = {
-        let casts = casts.clone();
-        let serve_errors = Arc::clone(&serve_errors);
-        let dishonest = Arc::clone(&dishonest);
-        tokio::spawn(async move {
-            let mut sessions = tokio::task::JoinSet::new();
-            let mut next = 0usize;
-            loop {
-                let Ok((socket, _)) = listener.accept().await else {
-                    break;
-                };
-                let handle = casts[next % casts.len()].clone();
-                next += 1;
-                let serve_errors = Arc::clone(&serve_errors);
-                let dishonest = Arc::clone(&dishonest);
-                sessions.spawn(async move {
-                    // A child that dies before the listener-port swap is the
-                    // same honest severance as one that dies mid-session.
-                    let mut link = match tcp::link(socket).await {
-                        Ok(link) => link,
-                        Err(_) => {
-                            serve_errors.fetch_add(1, Ordering::Relaxed);
-                            return;
-                        }
-                    };
-                    if let Err(e) = handle.gossip(&mut link).await {
-                        serve_errors.fetch_add(1, Ordering::Relaxed);
-                        if !is_honest_error(&e) {
-                            dishonest
-                                .lock()
-                                .expect("dishonest log")
-                                .push(format!("{e:?}"));
-                        }
-                    }
-                });
-            }
-        })
-    };
-
-    // Probe parent-side party disjointness while the children hammer it.
-    let done = Arc::new(AtomicBool::new(false));
-    let prober = tokio::spawn(probe_disjointness(casts.clone(), Arc::clone(&done)));
-
-    // Spawn the children: this same test binary, re-executed straight into
-    // `sim_child` with its script in the environment.
-    let exe = std::env::current_exe().expect("current test binary");
-    let mut children = Vec::new();
-    for (index, child) in plan.children.iter().enumerate() {
-        let sessions: Vec<String> = child.sessions.iter().map(encode_fault).collect();
-        let process = std::process::Command::new(&exe)
-            .args(["--exact", "sim_child", "--ignored"])
-            .env(CHILD_ADDR, addr.to_string())
-            .env(CHILD_INDEX, index.to_string())
-            .env(CHILD_SENDS, child.n_sends.to_string())
-            .env(CHILD_BOOT, encode_fault(&child.boot))
-            .env(CHILD_SESSIONS, sessions.join(","))
-            .env(CHILD_RETIRE, encode_fault(&child.retire))
-            .stdout(Stdio::null())
-            .stderr(Stdio::inherit())
-            .spawn()
-            .expect("spawn child process");
-        children.push(KillOnDrop(process));
-    }
-
-    // Reap the children, folding their exit codes into the loss accounting.
-    let deadline = tokio::time::Instant::now() + CHILD_DEADLINE;
-    let mut possible_losses = 0usize;
-    let mut clean_children = vec![false; plan.children.len()];
-    for (index, child) in children.iter_mut().enumerate() {
-        let status = loop {
-            if let Some(status) = child.0.try_wait().expect("poll child") {
-                break status;
-            }
-            assert!(
-                tokio::time::Instant::now() < deadline,
-                "child {index} did not finish within {CHILD_DEADLINE:?}"
-            );
-            tokio::time::sleep(Duration::from_millis(25)).await;
-        };
-        match status.code() {
-            Some(EXIT_CLEAN) => clean_children[index] = true,
-            Some(EXIT_BOOT_LOSS) | Some(EXIT_UNCERTAIN) => possible_losses += 1,
-            other => panic!(
-                "child {index} exited abnormally ({other:?}): an invariant \
-                 violation or panic in the child process"
-            ),
-        }
-    }
-    possible_losses += serve_errors.load(Ordering::Relaxed);
-
-    // Wind down: stop the prober and the accept loop (dropping its
-    // `JoinSet` aborts any straggling serve task), then reclaim the
-    // parent's `Peer`s — `try_into_peer` resolves once every serving clone
-    // is gone, so this is the synchronization point proving quiescence.
-    // The heal phase below runs on the data plane, so each reclaimed
-    // `Peer` converts straight back out.
-    done.store(true, Ordering::Release);
-    prober.await.expect("prober task");
-    accept.abort();
-    let _ = accept.await;
-    let mut survivors = Vec::new();
-    for cast in casts {
-        survivors.push(
-            cast.try_into_peer()
-                .await
-                .expect("all serving clones dropped")
-                .into_rumors(),
-        );
-    }
-
-    assert!(
-        dishonest.lock().expect("dishonest log").is_empty(),
-        "serving sessions surfaced non-fault errors: {:?}",
-        dishonest.lock().expect("dishonest log")
-    );
-
-    quiesce(&survivors).await;
-    let readouts = survivor_readouts(&survivors);
-    assert_converged(&survivors, &readouts);
-
-    // Every cleanly-retired child's sends must have survived into the
-    // parent's converged content: its final retirement reconciled before
-    // the party hand-off, so nothing it published may be lost.
-    let live: BTreeSet<u64> = readouts[0].values().copied().collect();
-    for (index, child) in plan.children.iter().enumerate() {
-        if clean_children[index] {
-            for s in 0..child.n_sends {
-                assert!(
-                    live.contains(&child_value(index, s)),
-                    "send {s} of cleanly-retired child {index} was lost"
-                );
-            }
-        }
-    }
-
-    assert_party_invariants(&survivors, possible_losses);
-}
-
-// ---- the child process ------------------------------------------------------
-
-/// Child-process entry point for `inter_process_disruption_*`: **not a
-/// test**.
-///
-/// The parent re-executes this binary with `--exact sim_child
-/// --ignored` and the script in the environment; without `CHILD_ADDR` set
-/// (e.g. under `--run-ignored`) it is a no-op. Outcomes travel back as
-/// exit codes (`EXIT_*`); invariant violations panic, which the parent
-/// sees as an abnormal exit.
-#[test]
-#[ignore = "child-process entry point for the inter-process simulation, not a test"]
-fn sim_child() {
-    let Ok(addr) = std::env::var(CHILD_ADDR) else {
-        return;
-    };
-    let code = mt_runtime().block_on(child_main(addr));
-    if code != EXIT_CLEAN {
-        std::process::exit(code);
-    }
-}
-
-async fn child_main(addr: String) -> i32 {
-    let env = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));
-    let index: usize = env(CHILD_INDEX).parse().expect("child index");
-    let n_sends: usize = env(CHILD_SENDS).parse().expect("send count");
-    let boot = decode_fault(&env(CHILD_BOOT));
-    let sessions: Vec<FaultPlan> = {
-        let raw = env(CHILD_SESSIONS);
-        raw.split(',')
-            .filter(|s| !s.is_empty())
-            .map(decode_fault)
-            .collect()
-    };
-    let retire_fault = decode_fault(&env(CHILD_RETIRE));
-
-    // Join the universe. A faulty plan first attempts a bootstrap over a
-    // cut wire: on failure the fork served for it may be orphaned, which
-    // the parent must hear about (the exit code), and the child joins for
-    // real over a clean connection.
-    let mut had_boot_loss = false;
-    let mut known: Option<Peer<u64>> = None;
-    if !boot.is_clean() {
-        let socket = TcpStream::connect(&addr)
-            .await
-            .expect("connect for faulty bootstrap");
-        let link = tcp::link(socket).await.expect("swap listener ports");
-        let mut link = fault::faulty_link(link, boot);
-        match Peer::<u64>::bootstrap().join(&mut link).await {
-            Ok(Some(k)) => known = Some(k.sync_window_floor()),
-            Ok(None) => panic!("the parent never bootstraps"),
-            Err(e) => {
-                assert_honest_error(&e);
-                had_boot_loss = true;
-            }
-        }
-    }
-    let known = match known {
-        Some(k) => k,
-        None => {
-            let socket = TcpStream::connect(&addr)
-                .await
-                .expect("connect for bootstrap");
-            let mut link = tcp::link(socket).await.expect("swap listener ports");
-            Peer::<u64>::bootstrap()
-                .join(&mut link)
-                .await
-                .expect("clean bootstrap")
-                .expect("the parent serves every bootstrap")
-                .sync_window_floor()
-        }
-    };
-
-    // Chaos: local sends concurrent with possibly-severed gossip sessions
-    // back to the parent.
-    let cast = known.into_rumors();
-    let sender = {
-        let handle = cast.clone();
-        tokio::spawn(async move {
-            for s in 0..n_sends {
-                handle.send(child_value(index, s)).unwrap();
-                tokio::task::yield_now().await;
-            }
-        })
-    };
-    for fault in sessions {
-        let socket = TcpStream::connect(&addr)
-            .await
-            .expect("connect for session");
-        let link = tcp::link(socket).await.expect("swap listener ports");
-        let mut link = fault::faulty_link(link, fault);
-        let handle = cast.clone();
-        assert_honest_gossip(&handle.gossip(&mut link).await);
-    }
-    sender.await.expect("sender task");
-
-    // One clean session so everything this child published is home even
-    // before the retirement reconciles.
-    {
-        let socket = TcpStream::connect(&addr)
-            .await
-            .expect("connect for final gossip");
-        let mut link = tcp::link(socket).await.expect("swap listener ports");
-        cast.gossip(&mut link).await.expect("clean final gossip");
-    }
-
-    // Retire home, possibly through a cut wire; a recovered retiree gets
-    // one clean retry. Outcomes map to the exit-code protocol.
-    let mut known = cast
-        .try_into_peer()
-        .await
-        .expect("sender finished; sole handle");
-    let mut fault = retire_fault;
-    for _attempt in 0..2 {
-        let socket = TcpStream::connect(&addr).await.expect("connect for retire");
-        let link = tcp::link(socket).await.expect("swap listener ports");
-        let mut link = fault::faulty_link(link, fault);
-        match known.retire(&mut link).await {
-            Retire::Retired => {
-                return if had_boot_loss {
-                    EXIT_BOOT_LOSS
-                } else {
-                    EXIT_CLEAN
-                };
-            }
-            Retire::Recovered {
-                peer: recovered,
-                error,
-            } => {
-                assert_honest_error(&error);
-                known = recovered;
-                fault = FaultPlan::NONE;
-            }
-            Retire::Uncertain { error } => {
-                assert_honest_error(&error);
-                return EXIT_UNCERTAIN;
-            }
-            Retire::Declined { .. } => return EXIT_ANOMALY,
-        }
-    }
-    // A clean retry can only end `Retired`; reaching here is an anomaly.
-    EXIT_ANOMALY
-}
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 647:
>
> Deletion, annotated at the file's last line: T143 dissolves the inter-process family. Gone from this file: the self re-executing children and their exit protocol, `ChildPlan`/`ProcPlan`/`arb_proc_plan`, `run_proc_plan_probed` with the stop signal, the grace drain (`SERVE_GRACE`) and its loss accounting, the fault-free loss pin, and the serving-task tripwire; the module doc describes one simulation. The lane's earlier commits that built those stay in history for the packet.

<!-- annotation -->
> **tests-disruption-handshake-3** (T143), line 647:
>
> Deletion, same site: the child-cycle envelope meter (`Envelope`, `child_cycle_bytes`, `metered_connection`, `send_complement`), the generator-bound constants, `MAX_CHILD_CUT` and its two-sided pin. The intra-process `MAX_CUT` pin stays.

<a id="hunk-78"></a>
### tests/gossip_when.rs `@@ -45,7 +45,7 @@ use tokio::io::AsyncWriteExt;`

```diff
@@ -45,7 +45,7 @@ use tokio::io::AsyncWriteExt;
 use tokio::time::timeout;
 
 use crate::common::action::created_version;
-use crate::common::fault::{FaultPlan, faulty};
+use crate::common::fault::{self, FaultPlan, faulty};
 use crate::common::wire::{bootstrap_fork_async, tokio_block_on as block_on, wire_gossip_async};
 
 /// Generous wall-clock bound: everything here is in-memory and finishes in
```

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 48:
>
> The `fault` module is imported for `metered`, which the fixture pin and the asymmetric witness use.

<a id="hunk-79"></a>
### tests/gossip_when.rs `@@ -687,10 +687,225 @@ async fn a_dropped_driver_does_not_block_peer_reclaim() {`

```diff
@@ -687,10 +687,225 @@ async fn a_dropped_driver_does_not_block_peer_reclaim() {
     assert!(rumors.try_into_peer().await.is_some());
 }
 
+/// Upper bound on the byte offset at which a severed connection's cut
+/// can land.
+///
+/// Derived from measurement: a clean run of the severed-connection
+/// fixture ([`run_severed`]) moves fewer bytes per endpoint than this
+/// bound, and the bound stays within twice that, so generated cuts reach
+/// every byte of the session and keep landing inside it. The two-sided
+/// pin is [`max_sever_cut_spans_the_fixture_session`]; re-measure there
+/// before touching this number.
+const MAX_SEVER_CUT: usize = 256;
+
+/// What one severed-connection run of the fixture leaves behind: each
+/// driver's items in order, and the two replicas for the checks.
+struct Severed {
+    a_items: Vec<Result<Gossiped, Error>>,
+    b_items: Vec<Result<Gossiped, Error>>,
+    a: Rumors<u64>,
+    b: Rumors<u64>,
+}
+
+/// Run the severed-connection fixture: the [`pair`] with one send on each
+/// side, A driven by a single tick and B by remote-led serving, over one
+/// in-memory connection with `a_fault` and `b_fault` applied to the two
+/// ends.
+///
+/// Each side's link is owned by its future, so the failing side's drop
+/// surfaces as EOF to the other rather than deadlocking the join.
+async fn run_severed(a_fault: FaultPlan, b_fault: FaultPlan) -> Severed {
+    let (a, b) = pair().await;
+    a.send(1).unwrap();
+    b.send(2).unwrap();
+
+    let (a_side, b_side) = links();
+    let a_link = faulty(a_side, a_fault);
+    let b_link = faulty(b_side, b_fault);
+    let a_task = async {
+        let mut a_link = a_link;
+        let once = stream::once(std::future::ready(()));
+        a.gossip_when(once, &mut a_link).collect::<Vec<_>>().await
+    };
+    let b_task = async {
+        let mut b_link = b_link;
+        b.gossip_when(stream::pending::<()>(), &mut b_link)
+            .collect::<Vec<_>>()
+            .await
+    };
+    let (a_items, b_items) = timeout(DEADLINE, futures::future::join(a_task, b_task))
+        .await
+        .expect("a severed connection wedged a driver");
+    Severed {
+        a_items,
+        b_items,
+        a,
+        b,
+    }
+}
+
+/// The severed-connection contract, checked on one run's outcome:
+/// terminal shape, atomicity, the epilogue's certification, and recovery
+/// over a fresh clean connection.
+async fn check_severed(run: &Severed) {
+    let Severed {
+        a_items,
+        b_items,
+        a,
+        b,
+    } = run;
+
+    // Terminal shape: zero or more Ok items, then at most one Err.
+    for items in [a_items, b_items] {
+        if let Some(err_at) = items.iter().position(|i| i.is_err()) {
+            assert_eq!(err_at, items.len() - 1, "Err must be terminal");
+        }
+    }
+
+    // Atomicity: whatever happened, each side holds its own send,
+    // nothing beyond the union, and never a torn intermediate.
+    let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
+    assert!(a_snapshot.iter().any(|(_, m)| *m == 1));
+    assert!(b_snapshot.iter().any(|(_, m)| *m == 2));
+    assert!(a_snapshot.len() <= 2);
+    assert!(b_snapshot.len() <= 2);
+
+    // Certification: the epilogue's central promise, under a cut at
+    // an arbitrary offset. A driver yields `Ok` only after reading
+    // the peer's completion marker, which the peer writes only
+    // after its own commit -- so any `Ok` on either side means both
+    // replicas already hold the session's full union, here, before
+    // the recovery gossip has run.
+    if a_items.iter().any(|i| i.is_ok()) || b_items.iter().any(|i| i.is_ok()) {
+        assert_eq!(
+            a_snapshot.hash(),
+            b_snapshot.hash(),
+            "a session Ok certifies the peer committed: the pair must already be converged",
+        );
+        assert_eq!(
+            a_snapshot.len(),
+            2,
+            "the certified session moved both sends"
+        );
+    }
+
+    // Recovery: a fresh, clean connection converges the pair.
+    wire_gossip_async(a, b).await;
+    let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
+    assert_eq!(a_snapshot.hash(), b_snapshot.hash());
+    assert_eq!(a_snapshot.len(), 2);
+}
+
+/// Byte extent of the severed-connection fixture on a clean run, per
+/// endpoint: `(a_written, b_written, b_read)`.
+///
+/// The same pair, sends, drivers, and link capacity as [`run_severed`],
+/// metered with the counters the fault cuts spend. The in-memory link and
+/// the single-threaded driver make the run byte-identical across
+/// invocations, which lets [`a_lost_marker_certifies_one_side`] place a
+/// cut one byte short of a clean session.
+async fn fixture_session_bytes() -> (usize, usize, usize) {
+    let (a, b) = pair().await;
+    a.send(1).unwrap();
+    b.send(2).unwrap();
+
+    let (a_side, b_side) = links();
+    let (a_link, a_meter) = fault::metered(a_side);
+    let (b_link, b_meter) = fault::metered(b_side);
+    let a_task = async {
+        let mut a_link = a_link;
+        let once = stream::once(std::future::ready(()));
+        a.gossip_when(once, &mut a_link).collect::<Vec<_>>().await
+    };
+    let b_task = async {
+        let mut b_link = b_link;
+        b.gossip_when(stream::pending::<()>(), &mut b_link)
+            .collect::<Vec<_>>()
+            .await
+    };
+    let (a_items, b_items) = timeout(DEADLINE, futures::future::join(a_task, b_task))
+        .await
+        .expect("the clean fixture session wedged a driver");
+    for items in [&a_items, &b_items] {
+        assert!(
+            items.iter().all(|i| i.is_ok()),
+            "the clean fixture session must end without error: {items:?}"
+        );
+    }
+    assert_eq!(a.snapshot().hash(), b.snapshot().hash());
+    (a_meter.written(), b_meter.written(), b_meter.read())
+}
+
+/// Pins `MAX_SEVER_CUT` to the fixture session's measured byte extent,
+/// from both sides.
+///
+/// Every byte of the clean session is a reachable cut offset
+/// (`measured <= MAX_SEVER_CUT`), and the cut range is not vacuously wide
+/// (`MAX_SEVER_CUT <= 2 * measured`), so generated cuts keep landing
+/// inside the session rather than past its end.
+#[test]
+fn max_sever_cut_spans_the_fixture_session() {
+    let (a_written, b_written, b_read) = block_on(fixture_session_bytes());
+    println!("fixture session bytes: A wrote {a_written}, B wrote {b_written}, B read {b_read}");
+    let measured = a_written.max(b_written);
+    assert!(
+        measured <= MAX_SEVER_CUT,
+        "the fixture session moves {measured} bytes per endpoint, beyond \
+         MAX_SEVER_CUT ({MAX_SEVER_CUT}): deep cut offsets are unreachable"
+    );
+    assert!(
+        MAX_SEVER_CUT <= 2 * measured,
+        "MAX_SEVER_CUT ({MAX_SEVER_CUT}) is more than twice the fixture \
+         session's {measured} bytes: most generated cuts would land past the \
+         end of the session and never fire"
+    );
+}
+
+/// The certification's asymmetric case: A `Ok`, B `Err`, both replicas
+/// converged.
+///
+/// B's read budget is one byte short of a clean session (measured from a
+/// byte-identical metered run), so the byte B never reads is A's
+/// completion marker: A yields `Ok`; B yields no `Ok` and fails with the
+/// post-commit [`Error::Epilogue`] residue. The class places the cut: one
+/// byte more and B completes the session, failing only on its next
+/// control read after an `Ok`; any earlier and B fails pre-commit. A
+/// generated read cut reaches this case only by landing exactly here, so
+/// this deterministic witness holds the certification assertions to the
+/// case they exist for.
+#[test]
+fn a_lost_marker_certifies_one_side() {
+    let (_, _, b_read) = block_on(fixture_session_bytes());
+    let run = block_on(run_severed(
+        FaultPlan::NONE,
+        FaultPlan {
+            write_cut: None,
+            read_cut: Some(b_read - 1),
+            vanish: None,
+        },
+    ));
+    assert!(
+        run.a_items.iter().any(|i| i.is_ok()),
+        "A read B's marker and certifies the session: {:?}",
+        run.a_items
+    );
+    assert!(
+        run.b_items.iter().all(|i| i.is_err()),
+        "B never read A's marker, so it certifies nothing: {:?}",
+        run.b_items
+    );
+    assert!(
+        matches!(run.b_items.last(), Some(Err(Error::Epilogue(_)))),
+        "B's lost marker must surface as the post-commit Epilogue: {:?}",
+        run.b_items
+    );
+    block_on(check_severed(&run));
+}
+
 proptest! {
-    /// A connection severed at an arbitrary byte offset — either side's
-    /// write direction, any budget, including zero — fails loudly and
-    /// recoverably.
+    /// A connection severed at arbitrary byte offsets -- both sides' write
+    /// directions, any budget including zero, and either side's read
+    /// direction as well -- fails loudly and recoverably.
     ///
     /// That is: no hang, each driver ends after at most one terminal
     /// `Err`, each replica still holds its own sends and nothing beyond
```

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 699:
>
> The named bound replacing the literal 400. Measured clean fixture: A wrote 188, B wrote 191, B read 188; 256 sits in the band [191, 382]. The literal 400 fails the pin's upper side (more than twice the session), i.e. more than half the old family's cuts landed past the session's end.

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 717:
>
> The proptest body split into `run_severed` (the fixture and drivers, faults per end) and `check_severed` (the contract's assertions, unchanged in substance) so the deterministic witness runs the same drivers and the same checks as the generated family.

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 807:
>
> Clean metered run of the same fixture and drivers; the in-memory link and single-threaded driver make it byte-identical across runs (the lifecycle suite's marker witness rests on the same premise), which the witness relies on.

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 847:
>
> Two-sided pin. negative control: the former literal 400 fails it (`MAX_SEVER_CUT (400) is more than twice the fixture session's 191 bytes`); output in the commit message.

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 864:
>
> fresh-eyes prose pass: the witness doc shortened, and its claim that write-only cuts cannot reach the case (stale now that read cuts are in the family) replaced by the accurate one: a generated read cut reaches it only by landing exactly here.

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 877:
>
> The acceptance's deterministic asymmetric case: B's read budget one byte short of a clean session, A `Ok`, B no `Ok` and terminal `Error::Epilogue`, then the full `check_severed` (certification holds). I first asserted only `B ends in Err`; a control with the budget exactly at the clean count still passed because B's remote-led driver errors on its next control read after a complete session, so the assertion now pins the class and the absence of any `Ok` at B. negative control: that exact-budget cut fails the tightened witness; output in the commit message.

<!-- annotation -->
> **tests-disruption-handshake-17** (T143), line 884:
>
> Same field addition in the severed-connection fixture's literals.

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 906:
>
> fresh-eyes prose pass: em-dashes in the moved and edited paragraphs replaced with spaced double hyphens (T48); `MAX_SEVER_CUT` and `fixture_session_bytes` docs shortened.

<a id="hunk-80"></a>
### tests/gossip_when.rs `@@ -699,81 +914,30 @@ proptest! {`

```diff
@@ -699,81 +914,30 @@ proptest! {
     /// already converged before any recovery (the epilogue's certification,
     /// held under arbitrary cut geometry rather than only at pinned byte
     /// boundaries), and a fresh clean connection converges the pair fully.
+    /// The one-`Ok`-one-`Err` case that certification exists for has its
+    /// deterministic witness in [`a_lost_marker_certifies_one_side`].
     #[test]
     fn severed_connections_fail_loudly_and_recover(
-        a_write_cut in 0usize..400,
-        b_write_cut in 0usize..400,
+        a_write_cut in 0..MAX_SEVER_CUT,
+        b_write_cut in 0..MAX_SEVER_CUT,
+        a_read_cut in prop::option::of(0..MAX_SEVER_CUT),
+        b_read_cut in prop::option::of(0..MAX_SEVER_CUT),
     ) {
         block_on(async {
-            let (a, b) = pair().await;
-            a.send(1).unwrap();
-            b.send(2).unwrap();
-
-            let (a_side, b_side) = rumors::link::memory_with_capacity(LINK_BUF);
-            let a_link = faulty(a_side, FaultPlan {
-                write_cut: Some(a_write_cut),
-                read_cut: None,
-            });
-            let b_link = faulty(b_side, FaultPlan {
-                write_cut: Some(b_write_cut),
-                read_cut: None,
-            });
-
-            // Each side's link is owned by its future, so the failing
-            // side's drop surfaces as EOF to the other rather than
-            // deadlocking the join.
-            let a_task = async {
-                let mut a_link = a_link;
-                let once = stream::once(std::future::ready(()));
-                a.gossip_when(once, &mut a_link)
-                    .collect::<Vec<_>>()
-                    .await
-            };
-            let b_task = async {
-                let mut b_link = b_link;
-                b.gossip_when(stream::pending::<()>(), &mut b_link)
-                    .collect::<Vec<_>>()
-                    .await
-            };
-            let (a_items, b_items) = timeout(DEADLINE, futures::future::join(a_task, b_task))
-                .await
-                .expect("a severed connection wedged a driver");
-
-            // Terminal shape: zero or more Ok items, then at most one Err.
-            for items in [&a_items, &b_items] {
-                if let Some(err_at) = items.iter().position(|i| i.is_err()) {
-                    assert_eq!(err_at, items.len() - 1, "Err must be terminal");
-                }
-            }
-
-            // Atomicity: whatever happened, each side holds its own send,
-            // nothing beyond the union, and never a torn intermediate.
-            let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
-            assert!(a_snapshot.iter().any(|(_, m)| *m == 1));
-            assert!(b_snapshot.iter().any(|(_, m)| *m == 2));
-            assert!(a_snapshot.len() <= 2);
-            assert!(b_snapshot.len() <= 2);
-
-            // Certification: the epilogue's central promise, under a cut at
-            // an arbitrary offset. A driver yields `Ok` only after reading
-            // the peer's completion marker, which the peer writes only
-            // after its own commit — so any `Ok` on either side means both
-            // replicas already hold the session's full union, here, before
-            // the recovery gossip has run.
-            if a_items.iter().any(|i| i.is_ok()) || b_items.iter().any(|i| i.is_ok()) {
-                assert_eq!(
-                    a_snapshot.hash(),
-                    b_snapshot.hash(),
-                    "a session Ok certifies the peer committed: the pair must already be converged",
-                );
-                assert_eq!(a_snapshot.len(), 2, "the certified session moved both sends");
-            }
-
-            // Recovery: a fresh, clean connection converges the pair.
-            wire_gossip_async(&a, &b).await;
-            let (a_snapshot, b_snapshot) = (a.snapshot(), b.snapshot());
-            assert_eq!(a_snapshot.hash(), b_snapshot.hash());
-            assert_eq!(a_snapshot.len(), 2);
+            let run = run_severed(
+                FaultPlan {
+                    write_cut: Some(a_write_cut),
+                    read_cut: a_read_cut,
+                    vanish: None,
+                },
+                FaultPlan {
+                    write_cut: Some(b_write_cut),
+                    read_cut: b_read_cut,
+                    vanish: None,
+                },
+            )
+            .await;
+            check_severed(&run).await;
         });
     }
 }
```

<!-- annotation -->
> **tests-disruption-handshake-17** (T28), line 923:
>
> Read cuts join the family on either side (optional, so the always-present write cuts keep their coverage), and the doc claims only what the family produces, pointing at the deterministic witness for the asymmetric case.

<a id="hunk-81"></a>
### tests/gossip_when.rs `@@ -948,6 +1112,7 @@ async fn a_control_read_error_on_the_idle_boundary_poisons_the_link() {`

```diff
@@ -948,6 +1112,7 @@ async fn a_control_read_error_on_the_idle_boundary_poisons_the_link() {
         FaultPlan {
             write_cut: None,
             read_cut: Some(0),
+            vanish: None,
         },
     );
 
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 1115:
>
> `vanish: None` on the poisoned-link fixture's literal: `FaultPlan` gained the field, nothing else changes here.

<a id="hunk-82"></a>
### tests/multi_peer.rs `@@ -12,9 +12,12 @@ mod common;`

```diff
@@ -12,9 +12,12 @@ mod common;
 use std::collections::BTreeMap;
 
 use proptest::prelude::*;
+use proptest::strategy::ValueTree;
+use proptest::test_runner::TestRunner;
 
 use crate::common::oracle::{readout, readout_multiset, version_key};
 use crate::common::peer::gossip_step;
+use crate::common::schedule::events::Event;
 use crate::common::schedule::{Schedule, arb_schedule, execute_and_quiesce};
 use crate::common::window::{WindowAssignment, arb_window_assignment};
 
```

<!-- annotation -->
> **tests-lifecycle-15** (T26), line 15:
>
> Imports for the deterministic runner and the event alphabet the pin inspects.

<a id="hunk-83"></a>
### tests/multi_peer.rs `@@ -199,3 +202,29 @@ proptest! {`

```diff
@@ -199,3 +202,29 @@ proptest! {
         }
     }
 }
+
+/// Among 64 deterministic samples, `arb_schedule` emits a `Redact` event
+/// (always against a message its peer has observed, so every one is
+/// effectual), so the redaction dimension is live in the population.
+#[test]
+fn schedule_population_contains_redactions() {
+    let mut runner = TestRunner::deterministic();
+    let strategy = schedule_u64();
+    let mut redactions = 0usize;
+    for _ in 0..64 {
+        let schedule = strategy
+            .new_tree(&mut runner)
+            .expect("schedule strategy always generates")
+            .current();
+        redactions += schedule
+            .events
+            .iter()
+            .filter(|event| matches!(event, Event::Redact { .. }))
+            .count();
+    }
+    assert!(
+        redactions > 0,
+        "no sampled schedule redacts a message: the redaction dimension \
+         has silently left the population"
+    );
+}
```

<!-- annotation -->
> **tests-lifecycle-15** (T26), line 206:
>
> fresh-eyes prose pass: one-sentence invariant; motivation dropped.

<!-- annotation -->
> **tests-lifecycle-15** (T26), line 210:
>
> Population pin for the schedule alphabet: 64 deterministic samples of `arb_schedule` at this suite's own `N_PEERS`/`MAX_EVENTS`, counting emitted `Redact` events (every emitted one is effectual by the shadow's construction, so no positional check is needed here). negative control: `RedactObservation` weight 0 in `arb_choice` fails only this test (multi_peer: 7 passed, 1 failed); output in the commit message.

<a id="hunk-84"></a>
### tests/pairwise.rs `@@ -19,9 +19,11 @@`

```diff
@@ -19,9 +19,11 @@
 mod common;
 
 use proptest::prelude::*;
+use proptest::strategy::ValueTree;
+use proptest::test_runner::TestRunner;
 use rumors::{Rumors, Version, causally};
 
-use crate::common::action::{arb_local_actions, build_local};
+use crate::common::action::{LocalAction, arb_local_actions, build_local};
 use crate::common::oracle::readout;
 use crate::common::wire::{bootstrap_fork, wire_gossip};
 
```

<!-- annotation -->
> **tests-lifecycle-15** (T26), line 22:
>
> Imports for the deterministic runner and the action alphabet the pin inspects.

<a id="hunk-85"></a>
### tests/pairwise.rs `@@ -236,3 +238,32 @@ proptest! {`

```diff
@@ -236,3 +238,32 @@ proptest! {
         prop_assert_eq!(readout(&b.snapshot()), expected);
     }
 }
+
+/// Among 64 deterministic samples, `arb_local_actions` emits a `Redact`
+/// that follows at least one `Insert` (the position where `build_local`
+/// applies it), so the redaction dimension is live in the population.
+#[test]
+fn action_population_contains_effectual_redactions() {
+    let mut runner = TestRunner::deterministic();
+    let strategy = arb_local_actions();
+    let mut effectual = 0usize;
+    for _ in 0..64 {
+        let actions = strategy
+            .new_tree(&mut runner)
+            .expect("action strategy always generates")
+            .current();
+        let mut inserted = false;
+        for action in &actions {
+            match action {
+                LocalAction::Insert(_) => inserted = true,
+                LocalAction::Redact(_) if inserted => effectual += 1,
+                LocalAction::Redact(_) => {}
+            }
+        }
+    }
+    assert!(
+        effectual > 0,
+        "no sampled action sequence redacts after an insert: the redaction \
+         dimension has silently left the population"
+    );
+}
```

<!-- annotation -->
> **tests-lifecycle-15** (T26), line 242:
>
> fresh-eyes prose pass: one-sentence invariant; the `without this pin` motivation dropped (it changes nothing the reader does).

<!-- annotation -->
> **tests-lifecycle-15** (T26), line 246:
>
> Population pin for the action alphabet, the shape of `membership_population_contains_churn`: 64 deterministic samples of `arb_local_actions`, counting a `Redact` only when an `Insert` precedes it, since `build_local` drops a `Redact` with nothing sent. Placed beside the strategy's primary consumer as the resolution offers. negative control: `Redact` weight 0 in `arb_actions` fails only this test (pairwise: 8 passed, 1 failed); output in the commit message.

<a id="hunk-86"></a>
### tests/shadow_validity.rs `@@ -32,6 +32,7 @@ use std::collections::{BTreeMap, BTreeSet};`

```diff
@@ -32,6 +32,7 @@ use std::collections::{BTreeMap, BTreeSet};
 use proptest::prelude::*;
 
 use crate::common::oracle::{readout, version_key};
+use crate::common::overlap::{arb_overlap_schedule_with_shadow, execute_overlap};
 use crate::common::schedule::{
     EventIdx, arb_membership_schedule_with_shadow, arb_schedule_with_shadow, execute_membership,
     execute_with,
```

<!-- annotation -->
> **tests-observation-28** (T144), line 35:
>
> Imports of the overlap strategy's shadow twin and the pre-quiescence executor for the meta-test.

<a id="hunk-87"></a>
### tests/shadow_validity.rs `@@ -41,6 +42,11 @@ use crate::common::window::arb_window_assignment;`

```diff
@@ -41,6 +42,11 @@ use crate::common::window::arb_window_assignment;
 const N_PEERS: std::ops::RangeInclusive<usize> = 2..=8;
 const MAX_EVENTS: usize = 50;
 
+/// The overlap suite's own fleet and event bounds (`tests/session_overlap.rs`),
+/// so the meta-test samples the population the properties run on.
+const OVERLAP_N_PEERS: std::ops::RangeInclusive<usize> = 2..=4;
+const OVERLAP_MAX_EVENTS: usize = 24;
+
 proptest! {
     /// For every peer, the shadow simulator's `observed_log` and
     /// `live` sets (as `BTreeSet<EventIdx>`) match the live
```

<!-- annotation -->
> **tests-observation-28** (T144), line 45:
>
> The meta-test samples at the overlap suite's own fleet and event bounds, so it checks the population the properties run on.

<a id="hunk-88"></a>
### tests/shadow_validity.rs `@@ -147,4 +153,50 @@ proptest! {`

```diff
@@ -147,4 +153,50 @@ proptest! {
             }
         }
     }
+
+    /// The overlap-alphabet twin of `shadow_predicts_live_state`.
+    ///
+    /// For every peer, the overlap generator's `Knowledge` shadow
+    /// predicts the `observed_log` and `live` sets the overlap executor
+    /// produces at the end of the schedule (every leftover session
+    /// closed, no quiescence), translated through `resolved_versions`
+    /// back to event indices.
+    #[test]
+    fn overlap_shadow_predicts_live_state(
+        (schedule, shadow) in arb_overlap_schedule_with_shadow(
+            any::<u64>(),
+            OVERLAP_N_PEERS,
+            OVERLAP_MAX_EVENTS,
+        ),
+    ) {
+        let run = execute_overlap(&schedule);
+        let version_to_event_idx: BTreeMap<Vec<u8>, EventIdx> = run
+            .resolved_versions
+            .iter()
+            .map(|(eid, v)| (version_key(v), *eid))
+            .collect();
+
+        for (p, peer) in run.peers.iter().enumerate() {
+            let live_observed: BTreeSet<EventIdx> = peer
+                .observations
+                .iter()
+                .map(|(v, _)| version_to_event_idx[v.as_bytes()])
+                .collect();
+            let predicted_observed: BTreeSet<EventIdx> =
+                shadow.observed_log[p].iter().copied().collect();
+            prop_assert_eq!(
+                live_observed, predicted_observed,
+                "peer {} observation set disagrees with the overlap shadow", p,
+            );
+
+            let live_held: BTreeSet<EventIdx> = readout(&peer.local.snapshot())
+                .into_keys()
+                .map(|k| version_to_event_idx[&k])
+                .collect();
+            prop_assert_eq!(
+                live_held, shadow.live[p].clone(),
+                "peer {} live set disagrees with the overlap shadow", p,
+            );
+        }
+    }
 }
```

<!-- annotation -->
> **tests-observation-28** (T144), line 165:
>
> The T13 validity meta-test, from the stop artifact: per peer, the overlap shadow's `observed_log` and `live` sets match the executor's at the end of the schedule. negative control: with the fork moved back to `Open` (a reversible mutation of the `Open` arm taking both views there) it fails on the recorded shape, `peer 2 observation set disagrees with the overlap shadow`; output in the commit message.

<a id="hunk-89"></a>
### tests/tcp_link.rs `@@ -1,10 +1,9 @@`

```diff
@@ -1,10 +1,9 @@
-//! The simulations' TCP link satisfies the link contract.
+//! The per-session TCP link satisfies the link contract.
 //!
 //! `common::tcp` is the workspace's one [`Link`](rumors::link::Link)
-//! instantiation over real sockets, and `tests/disruption.rs` trusts it
-//! under process kills; these tests run it through the public
-//! [`rumors::conformance::link`] suite so a disruption failure indicts the
-//! protocol, never an accidentally nonconforming transport. Real sockets
+//! instantiation over real sockets; these tests run it through the public
+//! [`rumors::conformance::link`] suite so the contract is known to hold on
+//! a real transport, not only in memory. Real sockets
 //! need real time: a paused clock's auto-advance would fire the harness
 //! timeout while socket I/O is genuinely pending. Every check runs under an
 //! explicit timeout because the contract's liveness clauses fail as hangs,
```

<!-- annotation -->
> **tests-disruption-handshake-7** (T143), line 1:
>
> fresh-eyes repair, round 2, item 4: the module doc no longer says `tests/disruption.rs` trusts this link under process kills; it states the suite's purpose in the present tense.

