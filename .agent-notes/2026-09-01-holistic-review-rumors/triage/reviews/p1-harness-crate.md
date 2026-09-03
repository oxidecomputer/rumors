<!-- CAVEAT LECTOR: review packet for lane p1-harness-crate, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-harness-crate

## Goal

The in-crate test harness for the streaming mirror (the suites under
`src/tree/mirror/streaming/`, the remote proxy harness, and the codec
decode tests) is the instrument every P2 and later protocol lane is
judged by. The review found probes whose "stalled" and "completed"
verdicts could both be reported for a session that did neither, a
session constructor duplicated at dozens of sites with divergent
defaults, a forgotten-sibling verdict no test held to an oracle, a
proxy harness with an asymmetric twin and position-named errors, and a
decode error classified in two places. This lane gives the harness one
session builder, a probe classifier that names violations and budget
exhaustion, oracle-held verdicts, one proxy drive over a topology
enum, one classifier for decode failures with the sync decoder as
oracle, and the tests the rulings asked for, each with a committed
control that fails by name.

## Rulings landed

- T19: two of the three cancellation and wide-window instruments land:
  a walk-tier proptest cancelling the mirror future at a drawn poll
  count and asserting what must survive cancellation, and a wide-window
  arm in the join-oracle property.
- T22: the proxy harness runs production's arrangement by default; the
  hand-rolled two-proxy topologies consolidate onto one `drive`; the
  proxy's `Connect` impls stay as the alternative arrangement, with a
  doc.
- T24: the sync decoder stays as the codec differential's oracle,
  documented as such, with the fragments duplicated between it and the
  async reader made one definition each.
- T26: the P1 entries this lane carries (the probe verdicts, the
  forgotten-sibling oracle test, the trace coverage floor) land per
  their stated resolutions and acceptance.
- T127: the `LocalSession` builder and its three-way outcome land in
  this lane, consolidating the session construction sites while the
  probes are rewritten.
- T141: the prose standard.

## Stack position

- Parent: `triage/p2-codec` (both edit the codec decode tests; this
  lane rebased onto its tip and merges after it).
- Base for the diff: the lane's six commits over `e4c74cfe~6`; earlier
  commits on the branch are replays of `main`'s notes and p2-codec's
  commits, dropped by patch-id at the merge rebase.
- Children: `p1-collision-mode` (sweeps the tests this lane adds).

## Acceptance table

Every row is one the coordinator's verification runner ran against the
lane at `e4c74cfe` (first pass) or a detached scratch worktree at
`20672a66` (second pass) on the illumos box, with whole logs kept.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| all | `e4c74cfe` | `cargo nextest run -p rumors --all-features --locked --no-fail-fast -E 'test(/tree::mirror::streaming::/) \| test(/tree::mirror::framing::/)'` (box) | `285 tests run: 285 passed` |
| `streaming-tests-3` (T127) | `cd50fd12` | `git grep -c 'Handshaking::start(Local' -- src/tree/mirror/streaming` | `tests.rs:3` (the builder and `floor_start`) and `remote/proxy/tests.rs:2` (two sites the lane's report did not count; see round 1) |
| `streaming-tests-11` (T26) | `cd50fd12` | every probed server wrapped in `Faulting(UnexpectedQuery)` at `LocalSession::run`'s drive, `-E 'test(capacity::)'` | six capacity tests FAIL with `the probed session died with a violation, neither completing nor stalling: Client(Violation(UnexpectedQuery))`; `probe_classifier_rejects_a_violation` passes as `should_panic` |
| `materialized-27` (T26) | `bbeafc1f` | the leaf-height verdict inverted in `materialized/unknown.rs`, `-E 'test(forgotten_sibling) \| test(leaf_height_verdicts)'` | exactly the three new tests FAIL; `0 passed, 3 failed` |
| `remote-proxy-19` (T26) | `7f4d0934` | the initiator's opening records and `next_scope` deleted in `proxy/work/pump.rs`, the three consumers | all three FAIL: `a divergent session records exactly one greeting-seeded opening reply, found 0` |
| `remote-proxy-tests-24`, `-5` (T22) | `7f4d0934` | `git grep -n 'RemoteHandshaking::start' -- src`; `git grep -c 'PayloadCodec::new' -- src/tree/mirror/streaming/remote/proxy` | two lines, both in `harness.rs`'s `drive`; `harness.rs:1` plus `work/tests.rs:1` (a second site outside the harness; see round 1) |
| `remote-codec-8`, `-10` (T24) | `4f5ce126` | `git grep 'RECORD_TAG_LEN + 1'`; `git grep -c 'UnexpectedEof => DecodeErrorKind::Truncated'`; the `listing_issue` line reverted to the parent's text, the three listing tests | only the constant's definition; `decode.rs:1`; `non_canonical_listing_heads_are_rejected` FAILS: `the two decoders classify the failure differently` (`"listing head is not canonical"` against `"indefinite-length head"`) |
| decision 56 (T19) | `cd50fd12` | the wide arm handed `WindowConfig::FLOOR`, `-E 'test(streaming_matches_join_oracle)'` | FAIL at the width assertion: `left: 1, right: 48588` |
| all | `e4c74cfe` | `git diff --stat main...HEAD` over the codec and wire snapshot directories | 0 files: no snapshot moved |
| all | `e4c74cfe` | `just doclint` (Mac) | self-test ok; exit 0 |
| all | `e4c74cfe` | `just gate` on the box (lane log `gate3-e4c74cfe.log`) | seven streams ok (`workspace`: `1843 tests run: 1843 passed`); `fuzz` failed on the accepted illumos leg |
| repairs (`e01c2fd1`, `9683c837`, `170d0cda`) | `20672a66` | `just doclint` at the tip and at each of `dcf62248`, `6f55d4be` extracted alone | exit 0 at all three: every commit passes on its own after the fold |
| (same) | `20672a66` | the cancellation pin, the oracle, the EOF test, the opener-failure test, the whole proxy suite (box) | `40 tests run: 40 passed` |
| (negative control, cancellation) | | a `Drop` on `Resolver` leaking one held child only when reactions are still owed | oracle PASS; pin FAIL: `left: 40, right: 39: a cancelled session must release every node handle it built` |
| (negative control, EOF) | | `Arrived::short` reverted to `Read { part, source }` for any sourced failure | `body_eof_failures_are_truncations_in_both_decoders` FAILS at the `Truncated` assertion: `Initiator, QueryChildren: Read { part: QueryChildren, source: Kind(UnexpectedEof) }` |
| (pin's floor) | `e01c2fd1` | `arb_cancellation` draws `polls in 1..length` from a measured `session_length`; each case `prop_assert_eq!(cancelled, polls < length)`; the run asserts `cancelled_after_building > 0` | present at `tests.rs:393-450` |
| all | `20672a66` | `awk 'length > 100' src/tree/arb.rs` | 0 lines |
| all | `20672a66` | `just gate` on the box (lane log `gate4-20672a66.log`) | seven streams ok (`workspace`: `1844 tests run: 1844 passed`); `fuzz` the accepted illumos leg |
| round-2 docs | `f6f3a0b6` | `just doclint`; `cargo fmt --check` (Mac) | both clean; docs only |

## Fresh-eyes rounds

**Round 1** (surface correctness with operational validity), by sha:
no defect in the code. The reviewer traced the probe classifier's four
arms (a violation or a budget exhaustion panics by its own message,
never reads as completed or stalled), every `LocalSession` builder site
against its predecessor's defaults, both orientations of the
forgotten-sibling oracle, the trace floor's premise (protocol-derived:
role election is antisymmetric, so exactly one proxy runs the initiator
and one the opening), the wide arm's liveness check, and the codec's
one-definition fragments. One medium finding, repaired: the
cancellation pin's negative control leaked unconditionally, so it did
not discriminate cancellation from completion, and nothing showed a
mid-walk cancellation ever occurred; the pin now measures each pair's
session length, draws the poll count inside it, asserts per case that
cancellation happened exactly when expected, and asserts over the run
that some case cancelled after building a node; a `Drop`-only leak on
the resolver fails it while the oracle passes. Also repaired: the
`Arrived::short` reclassification of an `UnexpectedEof`-kind failure
(deliberate harmonization with the sync oracle, now named and tested);
the doclint fix folded into the commits it repairs; four prose items.
Coverage accepted by ruling: the proxy harness's production default
drops adversity coverage of the connecting-proxy arrangement (T22: a
path production never takes).

**Round 2** (operational validity and tree interaction), against
`20672a66`: the pin's measurement uses the same polling as the pin,
the high-water mark reset cannot be defeated from construction, the
run-level count is a genuine floor, seed persistence survives the
manual runner; the EOF test reaches both async body routes and the
oracle's classification is `Truncated`; the fold distributed exactly
the doclint fix; a rebase probe onto the merged codec replays clean.
Three doc items landed as the last commit (a stale contract sentence
on `Arrived::short`, the fixture's doc naming its failure kind, one
overstated strategy doc). The rounds stopped here.

Reviewer notes not acted on: the wide arm exercises the no-backpressure
regime only; `stalls_under_any_schedule`'s short-circuit is inherent to
the probe; the census is process-global, so the pin relies on nextest's
one-test-per-process, as every census assertion already does; the
`Connect` impl docs use T22's phrase "wire-path arrangement" where the
harness names `Topology::RightProxyConnects`, since non-test rustdoc
cannot link a `cfg(test)` item.

## Stops

None reported by the lane. Two things for your eye, not stops: the
probe classifier is an enum with one owner of messages rather than the
resolution's `stalls`/`completes` pair (same predicates); and the
intermediate commits `bbeafc1f` and `4f5ce126` fail doclint on their
own (three over-long doc summaries, repaired at the tip), so the gate
of record covers the tip; the merge rebase folds the doclint fix into
the commits it repairs. <!-- STOPS -->

## Reading order

### new tests and negative controls

- materialized-27 (T26) at `src/tree/mirror/streaming/materialized/unknown/tests.rs:115` ([hunk](#hunk-7))
- remote-codec-10 (T24) at `src/tree/mirror/streaming/remote/codec/decode.rs:318` ([hunk](#hunk-13))
- remote-codec-10 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:600` ([hunk](#hunk-22))
- remote-proxy-19 (T26) at `src/tree/mirror/streaming/remote/proxy/work/progress/trace/tests.rs:70` ([hunk](#hunk-77))
- Decision 56 (T19) at `src/tree/mirror/streaming/tests.rs:420` ([hunk](#hunk-81))
- Decision 56 (T19) at `src/tree/mirror/streaming/tests.rs:540` ([hunk](#hunk-83))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:10` ([hunk](#hunk-84))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:230` ([hunk](#hunk-87))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:270` ([hunk](#hunk-88))
- materialized-27 (T26) at `src/tree/mirror/streaming/tests/stats.rs:250` ([hunk](#hunk-104))

### production edits

- materialized-27 (T26) at `src/tree/arb.rs:509` ([hunk](#hunk-3))
- materialized-27 (T26) at `src/tree/arb.rs:690` ([hunk](#hunk-4))
- materialized-27 (T26) at `src/tree/arb.rs:730` ([hunk](#hunk-4))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/budget.rs:43` ([hunk](#hunk-8))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/budget.rs:168` ([hunk](#hunk-9))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode.rs:55` ([hunk](#hunk-10))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode.rs:156` ([hunk](#hunk-11))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode.rs:210` ([hunk](#hunk-12))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode.rs:335` ([hunk](#hunk-14))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode.rs:348` ([hunk](#hunk-15))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:16` ([hunk](#hunk-16))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:212` ([hunk](#hunk-17))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:209` ([hunk](#hunk-17))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:463` ([hunk](#hunk-18))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:553` ([hunk](#hunk-19))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/frame.rs:36` ([hunk](#hunk-29))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/start.rs:112` ([hunk](#hunk-30))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/start.rs:136` ([hunk](#hunk-31))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/start.rs:163` ([hunk](#hunk-32))
- remote-proxy-19 (T26) at `src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:60` ([hunk](#hunk-76))

### tests and prose

- materialized-27 (T26) at `.agent-notes/2026-08-21-unknown-pruning-survivor/README.md:130` ([hunk](#hunk-1))
- materialized-27 (T26) at `src/tree/mirror/streaming/materialized/unknown/tests.rs:13` ([hunk](#hunk-5))
- materialized-27 (T26) at `src/tree/mirror/streaming/materialized/unknown/tests.rs:60` ([hunk](#hunk-6))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:14` ([hunk](#hunk-20))
- remote-codec-10 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:582` ([hunk](#hunk-21))
- remote-codec-10 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:660` ([hunk](#hunk-23))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:870` ([hunk](#hunk-24))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:885` ([hunk](#hunk-25))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:912` ([hunk](#hunk-26))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:1010` ([hunk](#hunk-27))
- remote-codec-8 (T24) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:1268` ([hunk](#hunk-28))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:3` ([hunk](#hunk-33))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:30` ([hunk](#hunk-34))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:60` ([hunk](#hunk-35))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:95` ([hunk](#hunk-36))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:121` ([hunk](#hunk-37))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:150` ([hunk](#hunk-38))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:175` ([hunk](#hunk-39))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:200` ([hunk](#hunk-40))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:230` ([hunk](#hunk-41))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:263` ([hunk](#hunk-42))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:278` ([hunk](#hunk-43))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:292` ([hunk](#hunk-44))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:321` ([hunk](#hunk-45))
- remote-proxy-19 (T26) at `src/tree/mirror/streaming/remote/proxy/tests.rs:340` ([hunk](#hunk-46))
- remote-proxy-19 (T26) at `src/tree/mirror/streaming/remote/proxy/tests.rs:370` ([hunk](#hunk-47))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:386` ([hunk](#hunk-48))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:425` ([hunk](#hunk-49))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:482` ([hunk](#hunk-50))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:525` ([hunk](#hunk-51))
- remote-proxy-19 (T26) at `src/tree/mirror/streaming/remote/proxy/tests.rs:540` ([hunk](#hunk-52))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests.rs:562` ([hunk](#hunk-53))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/containment.rs:20` ([hunk](#hunk-54))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/containment.rs:45` ([hunk](#hunk-55))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/containment.rs:62` ([hunk](#hunk-56))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/containment.rs:80` ([hunk](#hunk-57))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:40` ([hunk](#hunk-58))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:125` ([hunk](#hunk-59))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:188` ([hunk](#hunk-60))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:235` ([hunk](#hunk-61))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:291` ([hunk](#hunk-62))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/failures.rs:20` ([hunk](#hunk-63))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/failures.rs:135` ([hunk](#hunk-64))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/failures.rs:348` ([hunk](#hunk-65))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/failures.rs:375` ([hunk](#hunk-66))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:20` ([hunk](#hunk-67))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:60` ([hunk](#hunk-68))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:555` ([hunk](#hunk-69))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:585` ([hunk](#hunk-70))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:655` ([hunk](#hunk-71))
- remote-proxy-tests-5 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:700` ([hunk](#hunk-72))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:740` ([hunk](#hunk-73))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:3` ([hunk](#hunk-74))
- remote-proxy-tests-24 (T22) at `src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:45` ([hunk](#hunk-75))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests.rs:10` ([hunk](#hunk-78))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests.rs:80` ([hunk](#hunk-79))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests.rs:200` ([hunk](#hunk-79))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests.rs:250` ([hunk](#hunk-79))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests.rs:300` ([hunk](#hunk-80))
- Decision 56 (T19) at `src/tree/mirror/streaming/tests.rs:340` ([hunk](#hunk-81))
- Decision 56 (T19) at `src/tree/mirror/streaming/tests.rs:360` ([hunk](#hunk-81))
- Decision 56 (T19) at `src/tree/mirror/streaming/tests.rs:390` ([hunk](#hunk-81))
- Decision 56 (T19) at `src/tree/mirror/streaming/tests.rs:392` ([hunk](#hunk-81))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests.rs:515` ([hunk](#hunk-82))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:30` ([hunk](#hunk-84))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:60` ([hunk](#hunk-84))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:158` ([hunk](#hunk-85))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:216` ([hunk](#hunk-86))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:316` ([hunk](#hunk-89))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:326` ([hunk](#hunk-90))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:360` ([hunk](#hunk-91))
- streaming-tests-11 (T26) at `src/tree/mirror/streaming/tests/capacity.rs:375` ([hunk](#hunk-92))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:6` ([hunk](#hunk-93))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:20` ([hunk](#hunk-94))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:77` ([hunk](#hunk-95))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:93` ([hunk](#hunk-96))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:104` ([hunk](#hunk-97))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:145` ([hunk](#hunk-98))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:219` ([hunk](#hunk-99))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:271` ([hunk](#hunk-100))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/faults.rs:284` ([hunk](#hunk-101))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/stats.rs:22` ([hunk](#hunk-102))
- streaming-tests-3 (T127) at `src/tree/mirror/streaming/tests/stats.rs:40` ([hunk](#hunk-103))
- materialized-27 (T26) at `src/tree/mirror/streaming/tests/stats.rs:210` ([hunk](#hunk-104))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-08-21-unknown-pruning-survivor/README.md `@@ -127,3 +127,36 @@ convergence while H1 is true for redaction.`

```diff
@@ -127,3 +127,36 @@ convergence while H1 is true for redaction.
 Related but separate: campaign survivor #49 (`recv_msg_with`'s EOF arm
 in `remote.rs`, diagnostic-prose-only) is analyzed in the campaign's
 triage record and is not part of this handoff.
+
+## Disposition (2026-09-02, triage entry materialized-27, ruling T26)
+
+H3 held, and H1 with it: the suites never built a mixed-knowledge
+leaf-parent that only one side occupies, and under the inversion the
+redacted leaf is re-supplied and absorbed. Reading the reachability
+explains the shadow: a leaf reached from any height above is a
+path-compressed spine whose span is a point, so `knowledge()` answers
+`Before` or `After` and the recursion never descends to height 0. The arm
+runs only when a real height-1 branch (two or more leaves sharing 31 path
+bytes) classifies `Between` and the counterparty lacks the parent; every
+committed fixture avoided that shape.
+
+Committed kills, each failing with the `!` removed and passing on HEAD:
+
+- `materialized::unknown::tests::leaf_height_verdicts_agree_with_materialized_oracle`:
+  sibling leaves under one 31-byte prefix, each on its own party, with
+  per-leaf known flags, held to the materialized `Unknown` oracle.
+- `streaming::tests::stats::forgotten_sibling_is_judged_at_leaf_height`
+  and `forgotten_siblings_match_the_join_oracle`: a holder of concurrent
+  sibling leaves against a peer that forgot a subset and never held the
+  rest (`arb::forgotten_sibling_pair`, `arb::arb_forgotten_siblings`),
+  held to `Tree::join` in both orientations, with the holder shedding
+  exactly the forgotten leaves and the peer gaining exactly the rest.
+
+The survival is now measured, not agent-reported: with the `!` removed,
+the whole workspace suite (`cargo nextest run --workspace --all-features`,
+1838 tests, on ox-east-1) fails exactly those three tests and nothing
+else. That is the expected result and the reason the tests exist: the
+filter's contribution is behavioral (the redaction contract), and every
+other observable the suites pin is blind to it.
+
+This note is closed; the tests above are the instrument of record.
```

<!-- annotation -->
> **materialized-27** (T26), line 130:
>
> The handoff note's disposition, appended rather than deleting the note as the brief directs: which hypothesis held, the committed kills, and the measured whole-suite result under the inversion (1838 tests, exactly the three new ones fail).

<a id="hunk-2"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-harness-crate.tsv `@@ -0,0 +1,117 @@`

```diff
@@ -0,0 +1,117 @@
+# Lane p1-harness-crate: the in-crate streaming harnesses. Rulings T26, T127, T22, T24, T19.
+# Line numbers are new-side lines of `git diff 70acb940...HEAD`; a deletion is annotated at the line that follows it.
+src/tree/mirror/streaming/tests.rs	10	streaming-tests-3	T127	Imports follow the builder: Quiescence and node_census from testing, the channel instruments, Start for floor_start's return type, the window types for the wide arm. The rewrite of this file's helper ladder is wholesale, so every hunk below is one region of it.
+src/tree/mirror/streaming/tests.rs	80	streaming-tests-3	T127	The entry claimed the two-Local session was spelled nine times behind a six-function ladder. LocalSession is the one construction: schedules, a per-kind capacity, a window, and the three instruments are independent settings, and run nests the with_* closures only for the settings asked for. Verdict keeps completion, violation, and quiescence apart; the same type resolves streaming-tests-11.
+src/tree/mirror/streaming/tests.rs	200	streaming-tests-3	T127	Outcome owns the two expects (sides, converged) and names a stall and a poll-budget overrun separately; the instrument accessors panic when the session was not asked to record them, so a test cannot read an instrument it never attached. Fields stay plain so transcribed_mirror_sides can take the trace and transcript out by value.
+src/tree/mirror/streaming/tests.rs	250	streaming-tests-3	T127	floor_start is the one Local start outside the builder, for the sites that hold the undriven session: the fault- and failure-wrapped endpoints and the cancellation pin's own polling (fresh-eyes repair: the doc names that class). streaming_mirror_sides, streaming_mirror, and fully_scheduled_streaming_mirror keep their signatures and validate the trace as before; streaming_mirror_sides_with_schedule and scheduled_streaming_mirror are gone. transcribed_mirror_sides's doc lost its false second paragraph (it was never generic over the payload type); streaming-tests-5's other site at converges_on_leaf_parent_dispute is untouched and reported as a finding.
+src/tree/mirror/streaming/tests.rs	300	streaming-tests-3	T127	fully_scheduled_streaming_mirror stays as the scheduled convergence wrapper (three callers); the builder makes it three lines.
+src/tree/mirror/streaming/tests.rs	340	Decision 56	T19	wide_window solves the budget at a million-message corpus and asserts widest() > 1: the width is asserted, not assumed, at construction, and the session's granted width is checked again from its stats. DEFAULT_SYNC_MEMORY_BUDGET rather than usize::MAX so the arm runs a real budget. Premise for the corpus size: the generators never exceed 16 leaves, so any corpus far above them widens the top stages.
+src/tree/mirror/streaming/tests.rs	360	Decision 56	T19	cancel_after polls a fixed number of times under a noop waker and drops the future: the cancellation point is the drawn poll count. Kept private to the streaming suites rather than added to crate::testing, which is public surface.
+src/tree/mirror/streaming/tests.rs	515	streaming-tests-3	T127	uncontained_supply_is_rejected_by_streaming's inline construction goes through the builder; the verdict's outer expect still names a stall as the failure.
+src/tree/mirror/streaming/tests.rs	540	Decision 56	T19	The wide arm: a drawn bool picks floor or wide, and when the greetings differ (the only case a window is derived) both sides' window_granted must equal the chosen width. Negative control: handing the wide arm WindowConfig::FLOOR fails with left 1, right 48588 (the commit message quotes it). Fresh-eyes repair: the doc now says why the differential has value (traverse::unknown and materialized::unknown are two implementations of one contract) and that a divergence is a bug in one of them.
+src/tree/mirror/streaming/tests.rs	420	Decision 56	T19	Fresh-eyes repair (round 1, item 1). The pin draws its poll count strictly below the session's measured length (arb_cancellation), checks the measurement reproduces per case, tallies cases cancelled after the walk raised the census high-water mark, and asserts at least one over the run; the two live clauses (census back to baseline; retry at the join oracle) are the doc. The inputs-untouched clause is gone: nothing at this tier can falsify it. Negative control, cancellation-specific: a Drop impl on Resolver leaking one held child only when dropped with reactions owed fails the pin (left 49, right 48) while the oracle test stays green (commit message).
+src/tree/mirror/streaming/tests/capacity.rs	10	streaming-tests-11	T26	Imports: the builder, Verdict, floor_start; Fault, Faulting, Violation for the committed negative control; the direct Handshaking, StreamingRoot, and schedule imports leave with the hand-rolled probes.
+src/tree/mirror/streaming/tests/capacity.rs	30	streaming-tests-11	T26	The entry claimed the boolean probes read a violation or a poll-budget overrun as "did not stall". Probe has the two outcomes the capacity laws speak of; classify holds a completion to the join oracle on both sides and panics by name on the other two. I chose an enum over the resolution's stalls/completes pair so one classifier owns every message and the any/all sites read as predicates over it.
+src/tree/mirror/streaming/tests/capacity.rs	60	streaming-tests-11	T26	probe replaces both shape_stalls and underbuffered_mirror_stalls (they differed only in schedules); it computes the oracle itself so every completion is compared.
+src/tree/mirror/streaming/tests/capacity.rs	158	streaming-tests-11	T26	The witness's two probes become assert_eq against Probe::Stalled and Probe::Completed; the negative site now compares roots to the oracle.
+src/tree/mirror/streaming/tests/capacity.rs	216	streaming-tests-11	T26	stalls_under_any_schedule is any(Stalled) over the probe.
+src/tree/mirror/streaming/tests/capacity.rs	230	streaming-tests-11	T26	completes_under_every_schedule is all(Completed); internal_fan is hoisted from a closure so the negative control can build the same shape.
+src/tree/mirror/streaming/tests/capacity.rs	270	streaming-tests-11	T26	Negative control: the witness construction (server wrapped in Faulting(UnexpectedQuery) at capacity 1 on internal_fan(3)) fed to the classifier, should_panic on "died with a violation". The acceptance's other form, the fault injected into every probed server, was run as a reversible mutation: parent_delay_single_parent_boundary fails with that message (commit message).
+src/tree/mirror/streaming/tests/capacity.rs	316	streaming-tests-11	T26	Every !stalls site is a completes_under_every_schedule site; the assertion messages keep their law statements.
+src/tree/mirror/streaming/tests/capacity.rs	326	streaming-tests-11	T26	As above.
+src/tree/mirror/streaming/tests/capacity.rs	360	streaming-tests-11	T26	As above.
+src/tree/mirror/streaming/tests/capacity.rs	375	streaming-tests-11	T26	As above (the width sweep).
+src/tree/mirror/streaming/tests/faults.rs	6	streaming-tests-3	T127	floor_start imported from the parent.
+src/tree/mirror/streaming/tests/faults.rs	20	streaming-tests-3	T127	failing_start folds failing_root into a floor-window Failing<Local> start; with floor_start it removes every line over 100 columns that the unformatted proptest bodies carried.
+src/tree/mirror/streaming/tests/faults.rs	77	streaming-tests-3	T127	Fault-wrapped sites keep their own construction, through floor_start.
+src/tree/mirror/streaming/tests/faults.rs	93	streaming-tests-3	T127	As above.
+src/tree/mirror/streaming/tests/faults.rs	104	streaming-tests-3	T127	A prop_assert line wrapped by hand (rustfmt does not enter proptest! bodies); no change in meaning.
+src/tree/mirror/streaming/tests/faults.rs	145	streaming-tests-3	T127	As at 77.
+src/tree/mirror/streaming/tests/faults.rs	219	streaming-tests-3	T127	Failing-backend sites through failing_start.
+src/tree/mirror/streaming/tests/faults.rs	271	streaming-tests-3	T127	As above.
+src/tree/mirror/streaming/tests/faults.rs	284	streaming-tests-3	T127	As above.
+src/tree/mirror/streaming/tests/stats.rs	22	streaming-tests-3	T127	Imports: the builder and join_oracle from the parent, the new arb fixtures; the direct session construction imports leave.
+src/tree/mirror/streaming/tests/stats.rs	40	streaming-tests-3	T127	mirror_with_stats through the builder's stats instrument; SessionStats is Copy, so the pair is dereferenced, not cloned (the first box gate caught the clone as clippy's clone_on_copy).
+src/tree/mirror/streaming/tests/stats.rs	210	materialized-27	T26	The cross-peer pin the entry asked for: forgotten_sibling_pair in both orientations, both endpoints at the join oracle, holder sheds one and gains none, forgetter gains one and sheds none. Placed here rather than tests.rs because the shed and gain counts need the stats harness.
+src/tree/mirror/streaming/tests/stats.rs	250	materialized-27	T26	The generalization: any forgotten subset, Tree::join as oracle, plus the exact shed and gain counts (sheds equal the forgotten count, gains the rest), which cost nothing and also pin the whole-parent shed above one. Negative control: with the leaf verdict inverted both tests fail (commit message).
+src/tree/arb.rs	509	materialized-27	T26	leaf_sibling_path is crate-visible so unknown/tests.rs can build siblings under one 31-byte prefix.
+src/tree/arb.rs	690	materialized-27	T26	forgotten_sibling_pair: the entry's construction (0x00 on party 0, 0x01 on party 1, b empty with ceiling v00 | tick(party 2)). Its doc states why the leaf-height verdict decides here: b holds nothing under the parent, so no dispute answers for the leaves.
+src/tree/arb.rs	730	materialized-27	T26	arb_forgotten_siblings: up to eight siblings on distinct parties, a drawn forgotten subset, the empty and full subsets included so the Before and After fast paths sit beside the leaf verdicts.
+src/tree/mirror/streaming/materialized/unknown/tests.rs	13	materialized-27	T26	Import of leaf_sibling_path.
+src/tree/mirror/streaming/materialized/unknown/tests.rs	60	materialized-27	T26	sibling_tree_and_known: the leaf_sibling_path variant of tree_and_known the entry named, one party per leaf so the leaves are mutually concurrent and the parent classifies mixed whenever both flag values appear.
+src/tree/mirror/streaming/materialized/unknown/tests.rs	115	materialized-27	T26	The cheapest kill: the sibling tree against the materialized Unknown oracle. Negative control: fails with the ! removed (root hashes differ; commit message).
+.agent-notes/2026-08-21-unknown-pruning-survivor/README.md	130	materialized-27	T26	The handoff note's disposition, appended rather than deleting the note as the brief directs: which hypothesis held, the committed kills, and the measured whole-suite result under the inversion (1838 tests, exactly the three new ones fail).
+src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs	60	remote-proxy-19	T26	The floor. Premise deriving each count: a divergent session elects one initiator and one responder; the initiator-side proxy seeds exactly one under-root reply from the greeting (Work::initiator) and the responder-side proxy publishes exactly one under-root question after its opening wire reply (encode::opening), and nothing else records at that height, so the counts are exactly one each and the wire-reply floor is at least one. No observed value enters.
+src/tree/mirror/streaming/remote/proxy/work/progress/trace/tests.rs	70	remote-proxy-19	T26	Negative control: the entry's construction (with_trace(|| ()) passes both ordering assertions) fails the floor by name. The second test pins the least trace that meets it, so the floor cannot drift above the mechanism's minimum.
+src/tree/mirror/streaming/remote/proxy/tests.rs	3	remote-proxy-tests-5	T22	Imports follow the harness: the codec incantation, RemoteHandshaking, MirrorError, and FailingNode leave this file.
+src/tree/mirror/streaming/remote/proxy/tests.rs	30	remote-proxy-tests-5	T22	The harness's drive, Topology, Backends, codec, and EndpointError are this file's vocabulary; LeftFailure/RightFailure become one EndpointFailure.
+src/tree/mirror/streaming/remote/proxy/tests.rs	60	remote-proxy-tests-24	T22	reconcile (the asymmetric twin) is gone: its two pollster callers and instrumented_reconcile run the production arrangement. reconcile_symmetric_accepts drops T (only () was ever instantiated) and is a wrapper choosing links and topology.
+src/tree/mirror/streaming/remote/proxy/tests.rs	95	remote-proxy-tests-5	T22	The reordered variant wraps links and drops T likewise.
+src/tree/mirror/streaming/remote/proxy/tests.rs	121	remote-proxy-tests-5	T22	reconcile_after_preamble keeps T (its u64 caller exists) and passes codec::<T>().
+src/tree/mirror/streaming/remote/proxy/tests.rs	150	remote-proxy-tests-5	T22	As above: the preamble helper's drive call.
+src/tree/mirror/streaming/remote/proxy/tests.rs	175	remote-proxy-tests-5	T22	reconcile_with_failing_proxy returns roots; failing_root is gone (TreeBackend for Failing<Local> lifts and lowers).
+src/tree/mirror/streaming/remote/proxy/tests.rs	200	remote-proxy-tests-5	T22	reconcile_with_stacked_failures returns roots, as remote-proxy-tests-7 needs; that entry's oracle comparison is not landed here (it belongs to its own lane) and is noted in the report.
+src/tree/mirror/streaming/remote/proxy/tests.rs	230	remote-proxy-tests-5	T22	The four backends are named per participant; the materialized ones stay never-failing as before.
+src/tree/mirror/streaming/remote/proxy/tests.rs	263	remote-proxy-tests-24	T22	equal_versions_return_both_roots runs production's arrangement.
+src/tree/mirror/streaming/remote/proxy/tests.rs	278	remote-proxy-tests-24	T22	divergent_leaves_converge likewise.
+src/tree/mirror/streaming/remote/proxy/tests.rs	292	remote-proxy-tests-5	T22	Call site without the turbofish.
+src/tree/mirror/streaming/remote/proxy/tests.rs	321	remote-proxy-tests-5	T22	As above.
+src/tree/mirror/streaming/remote/proxy/tests.rs	340	remote-proxy-19	T26	wire_reconciliation_matches_local calls the floor when the greetings differ; an equal pair ends at the greeting and records no proxy work, so the floor would be false there. divergent is computed before the roots move.
+src/tree/mirror/streaming/remote/proxy/tests.rs	370	remote-proxy-19	T26	context_registration_is_causal likewise.
+src/tree/mirror/streaming/remote/proxy/tests.rs	386	remote-proxy-tests-5	T22	Call site without the turbofish.
+src/tree/mirror/streaming/remote/proxy/tests.rs	425	remote-proxy-tests-24	T22	proxy_backend_failures_are_fail_fast under production: both proxies are servers, so the two position arms collapse to one EndpointError::Proxy match on the faulted side. This is how far the topology forces the projection (remote-proxy-tests-16's helper is not built here).
+src/tree/mirror/streaming/remote/proxy/tests.rs	482	remote-proxy-tests-5	T22	Call site without the turbofish.
+src/tree/mirror/streaming/remote/proxy/tests.rs	525	remote-proxy-tests-5	T22	As above.
+src/tree/mirror/streaming/remote/proxy/tests.rs	540	remote-proxy-19	T26	instrumented_channels_cover_every_proxy_edge calls the floor beside the channel floor; its fixture is divergent by construction.
+src/tree/mirror/streaming/remote/proxy/tests.rs	562	remote-proxy-tests-24	T22	instrumented_reconcile runs production's arrangement; the trace and channel instruments are topology-independent.
+src/tree/mirror/streaming/remote/proxy/tests/harness.rs	20	remote-proxy-tests-5	T22	Imports for the generic drive: serde bounds for codec::<T>, Backend/Leaf/Failing/FailingNode for TreeBackend, Z for the backend bound.
+src/tree/mirror/streaming/remote/proxy/tests/harness.rs	60	remote-proxy-tests-24	T22	EndpointError names the participant, not the protocol position, because position is the topology's choice: this is the collapse of the Server/Client projection, as far as the topology forces it and no further (position stays visible through Topology). Topology's two variants; TreeBackend lifts and lowers tree::Root for Local and Failing<Local>; Backends names the four participants; codec::<T>() is the partition's one PayloadCodec::new.
+src/tree/mirror/streaming/remote/proxy/tests/harness.rs	555	remote-proxy-tests-24	T22	reconcile (the transport and failure suites' entry) drives production by default.
+src/tree/mirror/streaming/remote/proxy/tests/harness.rs	585	remote-proxy-tests-24	T22	reconcile_rewritten_greetings likewise, still under the default window.
+src/tree/mirror/streaming/remote/proxy/tests/harness.rs	655	remote-proxy-tests-24	T22	reconcile_scripted likewise.
+src/tree/mirror/streaming/remote/proxy/tests/harness.rs	700	remote-proxy-tests-5	T22	drive: the one construction of the four participants and the two RemoteHandshaking::start lines (one site, two proxies). The left endpoint's arrangement is the same in both topologies; only the right endpoint's mirror argument order changes. Rejected alternative: a type-level topology trait keeping MirrorError raw; it needed the same two projections and left the seven Client/Server sites in place. Fresh-eyes repair: the too_many_arguments rationale sits above the attribute, as the codec's encoder does it.
+src/tree/mirror/streaming/remote/proxy/tests/harness.rs	740	remote-proxy-tests-24	T22	local_client and local_server are the only two places MirrorError's positions are read in the partition; each names which participant held which position.
+src/tree/mirror/streaming/remote/proxy/tests/containment.rs	20	remote-proxy-tests-5	T22	reconcile_results is the harness's drive with Topology::RightProxyConnects: the one asymmetric caller, kept per T22.
+src/tree/mirror/streaming/remote/proxy/tests/containment.rs	45	remote-proxy-tests-24	T22	The check site now says the topology puts the right materialized participant in the server position, since the endpoint error no longer carries the position.
+src/tree/mirror/streaming/remote/proxy/tests/containment.rs	62	remote-proxy-tests-24	T22	EndpointError::Local in the client position.
+src/tree/mirror/streaming/remote/proxy/tests/containment.rs	80	remote-proxy-tests-24	T22	EndpointError::Local in the server position; the position is the topology's, stated in the doc.
+src/tree/mirror/streaming/remote/proxy/tests/declarations.rs	40	remote-proxy-tests-24	T22	The four inline projections were identical once both proxies are servers; receiver_error is their one copy, with the lie named in its message. Only as far as the topology forces: the receiver_left flag keeps its meaning (the reporting side).
+src/tree/mirror/streaming/remote/proxy/tests/declarations.rs	125	remote-proxy-tests-24	T22	Site through receiver_error.
+src/tree/mirror/streaming/remote/proxy/tests/declarations.rs	188	remote-proxy-tests-24	T22	As above.
+src/tree/mirror/streaming/remote/proxy/tests/declarations.rs	235	remote-proxy-tests-24	T22	As above.
+src/tree/mirror/streaming/remote/proxy/tests/declarations.rs	291	remote-proxy-tests-24	T22	As above.
+src/tree/mirror/streaming/remote/proxy/tests/failures.rs	20	remote-proxy-tests-24	T22	Imports: EndpointError replaces MirrorError here.
+src/tree/mirror/streaming/remote/proxy/tests/failures.rs	135	remote-proxy-tests-24	T22	endpoint_error: one EndpointError::Proxy arm over the selected side; the fail_left flag keeps its meaning.
+src/tree/mirror/streaming/remote/proxy/tests/failures.rs	348	remote-proxy-tests-24	T22	Stacked-failure matches on the proxy error; the results now carry roots and the test keeps discarding them at its own sites.
+src/tree/mirror/streaming/remote/proxy/tests/failures.rs	375	remote-proxy-tests-24	T22	As above.
+src/tree/mirror/streaming/remote/proxy/tests/malformed.rs	3	remote-proxy-tests-24	T22	Imports: EndpointError and EndpointFailure replace the two position-typed aliases.
+src/tree/mirror/streaming/remote/proxy/tests/malformed.rs	45	remote-proxy-tests-24	T22	receiving_error: one EndpointError::Proxy arm over the receiving side; corrupt_left keeps its polarity (remote-proxy-tests-16 decides whether the two flags unify).
+src/tree/mirror/streaming/remote/proxy/start.rs	112	remote-proxy-tests-24	T22	Connecting's doc says it is reached only through the client-position impl the harness runs. Fresh-eyes repair: same trim.
+src/tree/mirror/streaming/remote/proxy/start.rs	136	remote-proxy-tests-24	T22	The doc T22 orders at the Connect impl. Fresh-eyes repair: trimmed to T22's sentence (harness wire-path arrangement, no production caller); "right endpoint" was a harness-local name.
+src/tree/mirror/streaming/remote/proxy/start.rs	163	remote-proxy-tests-24	T22	The same at CompleteConnect. Fresh-eyes repair: trimmed to T22's sentence.
+src/tree/mirror/streaming/remote/codec/budget.rs	43	remote-codec-8	T24	Import of DecodeErrorKind; error.rs does not import budget.rs, so no cycle.
+src/tree/mirror/streaming/remote/codec/budget.rs	168	remote-codec-8	T24	RunBudget::overbatched is the one construction of OverbatchedRun (the error.rs line is the variant's declaration).
+src/tree/mirror/streaming/remote/codec/frame.rs	36	remote-codec-8	T24	MIN_RECORD_HEADS_LEN names the bare + 1 at both decoders.
+src/tree/mirror/streaming/remote/codec/decode.rs	55	remote-codec-8	T24	The oracle role T24 orders at the sync decoder's struct doc: what it is for and the read shape it contributes.
+src/tree/mirror/streaming/remote/codec/decode.rs	156	remote-codec-8	T24	The sync over-budget gate calls overbatched and MIN_RECORD_HEADS_LEN.
+src/tree/mirror/streaming/remote/codec/decode.rs	210	remote-codec-8	T24	The sync read_exact classifies through classify.
+src/tree/mirror/streaming/remote/codec/decode.rs	318	remote-codec-10	T24	The one-line fix: a listing head defect names the head's own defect, so both decoders spell it identically. Negative control: reverting this line makes non_canonical_listing_heads_are_rejected panic inside decode_both with "the two decoders classify the failure differently" (commit message).
+src/tree/mirror/streaming/remote/codec/decode.rs	335	remote-codec-8	T24	head_error's Io arm routes through classify.
+src/tree/mirror/streaming/remote/codec/decode.rs	348	remote-codec-8	T24	classify hoisted here as pub(super): the one UnexpectedEof to Truncated mapping.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	16	remote-codec-8	T24	classify imported from decode.rs.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	212	remote-codec-8	T24	Arrived::short classifies a close as a fresh UnexpectedEof through classify, as the resolution suggested.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	463	remote-codec-8	T24	The async over-budget gate calls overbatched and MIN_RECORD_HEADS_LEN.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	553	remote-codec-8	T24	The local classify is deleted (annotated at the line after it).
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	14	remote-codec-8	T24	MIN_RECORD_HEADS_LEN imported for the corner loop.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	582	remote-codec-10	T24	unordered_query_is_rejected runs through decode_both.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	600	remote-codec-10	T24	The listing-head spelling proptest over the four defects the resolution named, through decode_both, asserting the named detail. Negative control: see decode.rs:318.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	660	remote-codec-10	T24	arb_listing_head_defect builds each entry with a full digest behind the defect, so only the head is wrong; empty_query_listing_is_rejected runs through decode_both.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	1268	remote-codec-8	T24	The corner loop's bound is the named constant.
+
+src/tree/mirror/streaming/tests.rs	390	Decision 56	T19	Fresh-eyes repair: session_length measures a pair's poll count under the same polling cancel_after applies, and arb_cancellation draws the cancellation point strictly below it, so every case with a session longer than one poll cancels by construction rather than by luck.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	885	remote-codec-8	T24	Fresh-eyes repair (round 1, item 2): FailAfter carries the kind its failing read reports; failing_with is the constructor, new keeps Other.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	912	remote-codec-8	T24	Fresh-eyes repair: the failing read reports the fixture's kind.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	1010	remote-codec-8	T24	Fresh-eyes repair: names and tests the classification the classify routing changed, an UnexpectedEof-kind transport failure inside a supply body and inside a listing entry is Truncated at the interrupted part in both decoders (it was Read on the async side); the commit message says so.
+src/tree/mirror/streaming/remote/codec/decode/async_io.rs	209	remote-codec-8	T24	Fresh-eyes repair, round 2: Arrived::short's doc says what classify does with an UnexpectedEof-kind failure (a truncation), which body_eof_failures_are_truncations_in_both_decoders pins.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	870	remote-codec-8	T24	Fresh-eyes repair, round 2: FailAfter's doc names the kind it carries and which constructor sets it.
+src/tree/mirror/streaming/tests.rs	392	Decision 56	T19	Fresh-eyes repair, round 2: arb_cancellation's doc uses the accurate form (a length-1 session completes; the pin asserts that), and the pin doc keeps the clauses, the drawn range, and the run assertion in separate paragraphs.
```

*(review record; no annotation expected)*

<a id="hunk-3"></a>
### src/tree/arb.rs `@@ -506,7 +506,7 @@ pub fn uncontained_supply_pair() -> (crate::tree::Root, crate::tree::Root, Path,`

```diff
@@ -506,7 +506,7 @@ pub fn uncontained_supply_pair() -> (crate::tree::Root, crate::tree::Root, Path,
 /// Real leaves are version-addressed, so two distinct messages share a
 /// 31-byte prefix only under a hash-prefix collision; these hand-picked
 /// paths let a test construct that shape deliberately.
-fn leaf_sibling_path(last: u8) -> Path {
+pub fn leaf_sibling_path(last: u8) -> Path {
     let mut bytes = [0u8; 32];
     bytes[31] = last;
     Path::from(bytes)
```

<!-- annotation -->
> **materialized-27** (T26), line 509:
>
> leaf_sibling_path is crate-visible so unknown/tests.rs can build siblings under one 31-byte prefix.

<a id="hunk-4"></a>
### src/tree/arb.rs `@@ -669,6 +669,92 @@ pub fn leaf_parent_redaction_pair() -> (crate::tree::Root, crate::tree::Root, cr`

```diff
@@ -669,6 +669,92 @@ pub fn leaf_parent_redaction_pair() -> (crate::tree::Root, crate::tree::Root, cr
     )
 }
 
+/// A pair where `a` holds two concurrent leaves under one leaf-parent
+/// (`S<Z>`) prefix and `b` has forgotten one of them and never held the
+/// other, plus the tree both sides must converge to.
+///
+/// `b` holds nothing under the parent, so the parent is never disputed:
+/// the streaming filter judges `a`'s leaves one at a time, at leaf height,
+/// against `b`'s ceiling, which dominates the forgotten leaf's version and
+/// is concurrent with the other's. The survivor is the concurrent leaf
+/// alone.
+pub fn forgotten_sibling_pair() -> (crate::tree::Root, crate::tree::Root, crate::tree::Root) {
+    let mut forgotten_version = Version::new();
+    forgotten_version.tick(&nth_party(0));
+    let mut survivor_version = Version::new();
+    survivor_version.tick(&nth_party(1));
+    let survivor = (
+        leaf_sibling_path(0x01),
+        survivor_version.clone(),
+        Action::Insert(Message::new(())),
+    );
+    let a_node = act(
+        None,
+        vec![
+            (
+                leaf_sibling_path(0x00),
+                forgotten_version.clone(),
+                Action::Insert(Message::new(())),
+            ),
+            survivor.clone(),
+        ],
+        &mut |_| (),
+    );
+
+    // b remembers the forgotten leaf only through its ceiling: a forget
+    // tick on its own party, joined with the leaf's version.
+    let mut forget_version = Version::new();
+    forget_version.tick(&nth_party(2));
+    let b_ceiling = forgotten_version.clone() | forget_version;
+
+    let a_ceiling = forgotten_version | survivor_version;
+    let expected = root_with_ceiling(
+        act(None, vec![survivor], &mut |_| ()),
+        a_ceiling.clone() | b_ceiling.clone(),
+    );
+    (
+        root_with_ceiling(a_node, a_ceiling),
+        root_with_ceiling(None, b_ceiling),
+        expected,
+    )
+}
+
+/// Generate a pair where `a` holds concurrent sibling leaves under one
+/// leaf-parent prefix and `b` has forgotten a drawn subset of them and
+/// never held the rest, plus the count of forgotten leaves.
+///
+/// Each leaf sits on its own party. The general form of
+/// [`forgotten_sibling_pair`]: every subset, the empty one (nothing to
+/// shed) and the full one (the whole parent sheds without a leaf-height
+/// verdict) included.
+pub fn arb_forgotten_siblings() -> BoxedStrategy<(crate::tree::Root, crate::tree::Root, usize)> {
+    vec(any::<bool>(), 1..=8)
+        .prop_map(|forgotten| {
+            let mut leaves = Vec::new();
+            let mut a_ceiling = Version::new();
+            let mut b_ceiling = Version::new();
+            for (index, &forget) in forgotten.iter().enumerate() {
+                let mut version = Version::new();
+                version.tick(&nth_party(index));
+                a_ceiling |= version.clone();
+                if forget {
+                    b_ceiling |= version.clone();
+                }
+                leaves.push((
+                    leaf_sibling_path(index as u8),
+                    version,
+                    Action::Insert(Message::new(())),
+                ));
+            }
+            let mut forget_version = Version::new();
+            forget_version.tick(&nth_party(forgotten.len()));
+            let a = root_with_ceiling(act(None, leaves, &mut |_| ()), a_ceiling);
+            let b = root_with_ceiling(None, b_ceiling | forget_version);
+            (a, b, forgotten.iter().filter(|&&forget| forget).count())
+        })
+        .boxed()
+}
+
 #[cfg(test)]
 mod test {
     use super::nth_party;
```

<!-- annotation -->
> **materialized-27** (T26), line 690:
>
> forgotten_sibling_pair: the entry's construction (0x00 on party 0, 0x01 on party 1, b empty with ceiling v00 | tick(party 2)). Its doc states why the leaf-height verdict decides here: b holds nothing under the parent, so no dispute answers for the leaves.

<!-- annotation -->
> **materialized-27** (T26), line 730:
>
> arb_forgotten_siblings: up to eight siblings on distinct parties, a drawn forgotten subset, the empty and full subsets included so the Before and After fast paths sit beside the leaf verdicts.

<a id="hunk-5"></a>
### src/tree/mirror/streaming/materialized/unknown/tests.rs `@@ -10,7 +10,7 @@ use crate::{`

```diff
@@ -10,7 +10,7 @@ use crate::{
     Version,
     message::Message,
     tree::{
-        arb::nth_party,
+        arb::{leaf_sibling_path, nth_party},
         mirror::streaming::{Backend, Local, materialized::unknown::unknown},
         traverse::{Action, act, unknown::Unknown},
         typed::{self, Path, Prefix, height::Root},
```

<!-- annotation -->
> **materialized-27** (T26), line 13:
>
> Import of leaf_sibling_path.

<a id="hunk-6"></a>
### src/tree/mirror/streaming/materialized/unknown/tests.rs `@@ -47,6 +47,32 @@ fn tree_and_known(flags_a: &[bool], flags_b: &[bool]) -> (Option<typed::node::Ro`

```diff
@@ -47,6 +47,32 @@ fn tree_and_known(flags_a: &[bool], flags_b: &[bool]) -> (Option<typed::node::Ro
     (act(None, actions, &mut |_| ()), known)
 }
 
+/// Build a root of `flags.len()` sibling leaves under one 31-byte prefix,
+/// each on its own party, plus a `known` version that is the join of the
+/// leaf versions flagged `true`.
+///
+/// The leaves are mutually concurrent, so with both flag values present
+/// the parent's span classifies as mixed and the prune descends to judge
+/// each leaf on its own: the one shape in which the leaf-height verdict
+/// decides.
+fn sibling_tree_and_known(flags: &[bool]) -> (Option<typed::node::Root>, Version) {
+    let mut actions: Vec<(Path, Version, Action)> = Vec::new();
+    let mut known = Version::new();
+    for (index, &flagged) in flags.iter().enumerate() {
+        let mut version = Version::new();
+        version.tick(&nth_party(index));
+        actions.push((
+            leaf_sibling_path(index as u8),
+            version.clone(),
+            Action::Insert(Message::new(())),
+        ));
+        if flagged {
+            known |= version;
+        }
+    }
+    (act(None, actions, &mut |_| ()), known)
+}
+
 /// Prune an optional root through the single-node streaming filter, driving
 /// the future to completion with a trivial executor.
 fn stream_prune(root: Option<typed::node::Root>, known: &Version) -> Option<typed::node::Root> {
```

<!-- annotation -->
> **materialized-27** (T26), line 60:
>
> sibling_tree_and_known: the leaf_sibling_path variant of tree_and_known the entry named, one party per leaf so the leaves are mutually concurrent and the parent classifies mixed whenever both flag values appear.

<a id="hunk-7"></a>
### src/tree/mirror/streaming/materialized/unknown/tests.rs `@@ -81,4 +107,22 @@ proptest! {`

```diff
@@ -81,4 +107,22 @@ proptest! {
             typed::Node::root_hash(&streamed),
         );
     }
+
+    /// Sibling leaves under one leaf-parent prefix are judged one at a
+    /// time, and each leaf-height verdict agrees with the materialized
+    /// prune: the flagged leaves drop and the concurrent ones survive.
+    #[test]
+    fn leaf_height_verdicts_agree_with_materialized_oracle(
+        flags in vec(any::<bool>(), 1..=8),
+    ) {
+        let (root, known) = sibling_tree_and_known(&flags);
+
+        let oracle = Unknown::unknown(root.clone(), &known);
+        let streamed = stream_prune(root, &known);
+
+        prop_assert_eq!(
+            typed::Node::root_hash(&oracle),
+            typed::Node::root_hash(&streamed),
+        );
+    }
 }
```

<!-- annotation -->
> **materialized-27** (T26), line 115:
>
> The cheapest kill: the sibling tree against the materialized Unknown oracle. Negative control: fails with the ! removed (root hashes differ; commit message).

<a id="hunk-8"></a>
### src/tree/mirror/streaming/remote/codec/budget.rs `@@ -40,6 +40,7 @@`

```diff
@@ -40,6 +40,7 @@
 use crate::tree::mirror::cbor;
 use crate::tree::mirror::streaming::window::FAN;
 
+use super::error::DecodeErrorKind;
 use super::frame::{MAX_QUERY_CHILDREN, listing_entry_len};
 use super::signal::WireSignal;
 
```

<!-- annotation -->
> **remote-codec-8** (T24), line 43:
>
> Import of DecodeErrorKind; error.rs does not import budget.rs, so no cycle.

<a id="hunk-9"></a>
### src/tree/mirror/streaming/remote/codec/budget.rs `@@ -160,6 +161,16 @@ impl RunBudget {`

```diff
@@ -160,6 +161,16 @@ impl RunBudget {
     pub fn covers(self, body: usize) -> bool {
         SUPPLY_FRAME_OVERHEAD.saturating_add(body) <= self.bytes
     }
+
+    /// The rejection of a supply frame whose `body` run bytes this budget
+    /// does not cover: the frame's charged wire size beside the budget it
+    /// broke.
+    pub(super) fn overbatched(self, body: usize) -> DecodeErrorKind {
+        DecodeErrorKind::OverbatchedRun {
+            declared: SUPPLY_FRAME_OVERHEAD.saturating_add(body),
+            budget: self.bytes,
+        }
+    }
 }
 
 impl Default for RunBudget {
```

<!-- annotation -->
> **remote-codec-8** (T24), line 168:
>
> RunBudget::overbatched is the one construction of OverbatchedRun (the error.rs line is the variant's declaration).

<a id="hunk-10"></a>
### src/tree/mirror/streaming/remote/codec/decode.rs `@@ -51,7 +51,13 @@ pub fn decode_exact(`

```diff
@@ -51,7 +51,13 @@ pub fn decode_exact(
     }
 }
 
-/// Frame reader that adds protocol context as soon as the signal reveals it.
+/// The synchronous oracle of the codec differential: a frame reader over
+/// `std::io::Read` that adds protocol context as soon as the signal
+/// reveals it.
+///
+/// It reads each body whole and one head at a time, the simplest shape
+/// the grammar admits, so the tests can hold the async reader's chunked
+/// reads to the same classification of every frame and every defect.
 #[cfg(test)]
 struct FrameDecoder<'a, R> {
     speaker: Speaker,
```

<!-- annotation -->
> **remote-codec-8** (T24), line 55:
>
> The oracle role T24 orders at the sync decoder's struct doc: what it is for and the read shape it contributes.

<a id="hunk-11"></a>
### src/tree/mirror/streaming/remote/codec/decode.rs `@@ -146,13 +152,10 @@ impl<'a, R: Read> FrameDecoder<'a, R> {`

```diff
@@ -146,13 +152,10 @@ impl<'a, R: Read> FrameDecoder<'a, R> {
         // record's heads alone.
         if !self.budget.covers(len) {
             let budget = self.budget;
-            let overbatched = move || DecodeErrorKind::OverbatchedRun {
-                declared: super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
-                budget: budget.bytes(),
-            };
+            let overbatched = move || budget.overbatched(len);
             // A body too short to hold a record's heads cannot be a lone
             // record: rejected on the declared length alone.
-            if len < super::frame::RECORD_TAG_LEN + 1 {
+            if len < super::frame::MIN_RECORD_HEADS_LEN {
                 return Err(overbatched());
             }
             let Some((prefix, record)) = self.record_prefix()? else {
```

<!-- annotation -->
> **remote-codec-8** (T24), line 156:
>
> The sync over-budget gate calls overbatched and MIN_RECORD_HEADS_LEN.

<a id="hunk-12"></a>
### src/tree/mirror/streaming/remote/codec/decode.rs `@@ -204,13 +207,7 @@ impl<'a, R: Read> FrameDecoder<'a, R> {`

```diff
@@ -204,13 +207,7 @@ impl<'a, R: Read> FrameDecoder<'a, R> {
     fn read_exact(&mut self, bytes: &mut [u8], part: FramePart) -> Result<(), DecodeErrorKind> {
         self.read
             .read_exact(bytes)
-            .map_err(|source| match source.kind() {
-                ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
-                    missing: part,
-                    source,
-                },
-                _ => DecodeErrorKind::Read { part, source },
-            })
+            .map_err(|source| classify(part, source))
     }
 }
 
```

<!-- annotation -->
> **remote-codec-8** (T24), line 210:
>
> The sync read_exact classifies through classify.

<a id="hunk-13"></a>
### src/tree/mirror/streaming/remote/codec/decode.rs `@@ -317,9 +314,9 @@ pub(super) fn run_head(tag: cbor::Head, body: cbor::Head) -> Result<usize, Decod`

```diff
@@ -317,9 +314,9 @@ pub(super) fn run_head(tag: cbor::Head, body: cbor::Head) -> Result<usize, Decod
 pub(super) fn listing_issue(issue: ListingIssue) -> DecodeErrorKind {
     match issue {
         ListingIssue::Order(order) => DecodeErrorKind::QueryOutOfOrder(order),
-        ListingIssue::Head(_) => DecodeErrorKind::Malformed {
+        ListingIssue::Head(head) => DecodeErrorKind::Malformed {
             part: FramePart::QueryChildren,
-            detail: "listing head is not canonical",
+            detail: head_detail(head),
         },
         ListingIssue::Shape(detail) => DecodeErrorKind::Malformed {
             part: FramePart::QueryChildren,
```

<!-- annotation -->
> **remote-codec-10** (T24), line 318:
>
> The one-line fix: a listing head defect names the head's own defect, so both decoders spell it identically. Negative control: reverting this line makes non_canonical_listing_heads_are_rejected panic inside decode_both with "the two decoders classify the failure differently" (commit message).

<a id="hunk-14"></a>
### src/tree/mirror/streaming/remote/codec/decode.rs `@@ -335,13 +332,7 @@ pub(super) fn listing_issue(issue: ListingIssue) -> DecodeErrorKind {`

```diff
@@ -335,13 +332,7 @@ pub(super) fn listing_issue(issue: ListingIssue) -> DecodeErrorKind {
 /// Type a head-read failure by the frame part it interrupted.
 pub(super) fn head_error(part: FramePart, error: HeadReadError) -> DecodeErrorKind {
     match error {
-        HeadReadError::Io(source) => match source.kind() {
-            std::io::ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
-                missing: part,
-                source,
-            },
-            _ => DecodeErrorKind::Read { part, source },
-        },
+        HeadReadError::Io(source) => classify(part, source),
         HeadReadError::Malformed(head) => DecodeErrorKind::Malformed {
             part,
             detail: head_detail(head),
```

<!-- annotation -->
> **remote-codec-8** (T24), line 335:
>
> head_error's Io arm routes through classify.

<a id="hunk-15"></a>
### src/tree/mirror/streaming/remote/codec/decode.rs `@@ -349,6 +340,18 @@ pub(super) fn head_error(part: FramePart, error: HeadReadError) -> DecodeErrorKi`

```diff
@@ -349,6 +340,18 @@ pub(super) fn head_error(part: FramePart, error: HeadReadError) -> DecodeErrorKi
     }
 }
 
+/// Type an I/O failure by the frame part it interrupted: end-of-stream is
+/// a contextual truncation, anything else a plain read failure.
+pub(super) fn classify(part: FramePart, source: std::io::Error) -> DecodeErrorKind {
+    match source.kind() {
+        std::io::ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
+            missing: part,
+            source,
+        },
+        _ => DecodeErrorKind::Read { part, source },
+    }
+}
+
 /// Name a deterministic-contract violation for the error taxonomy.
 fn head_detail(error: cbor::HeadError) -> &'static str {
     match error {
```

<!-- annotation -->
> **remote-codec-8** (T24), line 348:
>
> classify hoisted here as pub(super): the one UnexpectedEof to Truncated mapping.

<a id="hunk-16"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -13,8 +13,8 @@ use super::super::{`

```diff
@@ -13,8 +13,8 @@ use super::super::{
     signal::{Signal, Speaker, WireSignal},
 };
 use super::{
-    OpenerItem, check_arity, decode_signal, frame_arity, head_error, listing_issue, opener_item,
-    query_listing, run_head,
+    OpenerItem, check_arity, classify, decode_signal, frame_arity, head_error, listing_issue,
+    opener_item, query_listing, run_head,
 };
 use crate::tree::{
     mirror::cbor::{self, HeadError, HeadReadError},
```

<!-- annotation -->
> **remote-codec-8** (T24), line 16:
>
> classify imported from decode.rs.

<a id="hunk-17"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -205,16 +205,15 @@ struct Arrived {`

```diff
@@ -205,16 +205,15 @@ struct Arrived {
 }
 
 impl Arrived {
-    /// Type a short delivery by the part left incomplete: a close is a
-    /// truncation, a failure a read error.
+    /// Type a short delivery by the part left incomplete: a close, or a
+    /// failure of kind `UnexpectedEof`, is a truncation; any other failure
+    /// is a read error.
     fn short(self, part: FramePart) -> DecodeErrorKind {
-        match self.failure {
-            Some(source) => DecodeErrorKind::Read { part, source },
-            None => DecodeErrorKind::Truncated {
-                missing: part,
-                source: ErrorKind::UnexpectedEof.into(),
-            },
-        }
+        classify(
+            part,
+            self.failure
+                .unwrap_or_else(|| ErrorKind::UnexpectedEof.into()),
+        )
     }
 }
 
```

<!-- annotation -->
> **remote-codec-8** (T24), line 212:
>
> Arrived::short classifies a close as a fresh UnexpectedEof through classify, as the resolution suggested.

<!-- annotation -->
> **remote-codec-8** (T24), line 209:
>
> Fresh-eyes repair, round 2: Arrived::short's doc says what classify does with an UnexpectedEof-kind failure (a truncation), which body_eof_failures_are_truncations_in_both_decoders pins.

<a id="hunk-18"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -462,11 +461,8 @@ impl<'a, R: AsyncRead + Unpin> AsyncFrameDecoder<'a, R> {`

```diff
@@ -462,11 +461,8 @@ impl<'a, R: AsyncRead + Unpin> AsyncFrameDecoder<'a, R> {
             // short to hold a record's heads cannot be a lone record and
             // is rejected on the declared length alone.
             let budget = self.budget;
-            let overbatched = move || DecodeErrorKind::OverbatchedRun {
-                declared: super::super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
-                budget: budget.bytes(),
-            };
-            if len < super::super::frame::RECORD_TAG_LEN + 1 {
+            let overbatched = move || budget.overbatched(len);
+            if len < super::super::frame::MIN_RECORD_HEADS_LEN {
                 return Err(overbatched());
             }
             let Some((prefix, record)) = self.record_prefix().await? else {
```

<!-- annotation -->
> **remote-codec-8** (T24), line 463:
>
> The async over-budget gate calls overbatched and MIN_RECORD_HEADS_LEN.

<a id="hunk-19"></a>
### src/tree/mirror/streaming/remote/codec/decode/async_io.rs `@@ -557,15 +553,3 @@ fn partial_head(input: &mut &[u8]) -> Result<Option<cbor::Head>, DecodeErrorKind`

```diff
@@ -557,15 +553,3 @@ fn partial_head(input: &mut &[u8]) -> Result<Option<cbor::Head>, DecodeErrorKind
         ))),
     }
 }
-
-/// Type an I/O failure by the frame part it interrupted: end-of-stream is a
-/// contextual truncation, anything else a plain read failure.
-fn classify(part: FramePart, source: std::io::Error) -> DecodeErrorKind {
-    match source.kind() {
-        ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated {
-            missing: part,
-            source,
-        },
-        _ => DecodeErrorKind::Read { part, source },
-    }
-}
```

<!-- annotation -->
> **remote-codec-8** (T24), line 553:
>
> The local classify is deleted (annotated at the line after it).

<a id="hunk-20"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -14,7 +14,7 @@ use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};`

```diff
@@ -14,7 +14,7 @@ use crate::tree::typed::{Hash, hash::MERKLE_HASH_LEN};
 
 use super::super::{
     error::{DecodeLeafError, Origin, QueryOrderError},
-    frame::{LeafRunError, MAX_QUERY_CHILDREN, RECORD_TAG_LEN},
+    frame::{LeafRunError, MAX_QUERY_CHILDREN, MIN_RECORD_HEADS_LEN, RECORD_TAG_LEN},
     signal::{DecodeSignalError, End, Flow, Speaker, Stream, StreamError},
 };
 
```

<!-- annotation -->
> **remote-codec-8** (T24), line 14:
>
> MIN_RECORD_HEADS_LEN imported for the corner loop.

<a id="hunk-21"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -579,7 +579,8 @@ proptest! {`

```diff
@@ -579,7 +579,8 @@ proptest! {
         let stream = stream(index);
         let children = vec![(previous, Hash::default()), (radix, Hash::default())];
         let encoded = query(stream, Flow::Continue, &children);
-        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
+        let error = decode_both(speaker, RunBudget::default(), &encoded)
+            .expect_err("a non-ascending listing cannot decode");
         prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
         let correct = matches!(
             error.kind,
```

<!-- annotation -->
> **remote-codec-10** (T24), line 582:
>
> unordered_query_is_rejected runs through decode_both.

<a id="hunk-22"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -591,6 +592,32 @@ proptest! {`

```diff
@@ -591,6 +592,32 @@ proptest! {
         prop_assert!(correct);
     }
 
+    /// Every non-canonical spelling of a listing entry head is rejected by
+    /// both decoders, naming the head's defect and the query listing it
+    /// sits in.
+    #[test]
+    fn non_canonical_listing_heads_are_rejected(
+        index in 1_u8..Stream::MAX,
+        speaker in arb_speaker(),
+        (entry, detail) in arb_listing_head_defect(),
+    ) {
+        let stream = stream(index);
+        let mut encoded = frame_head(3, stream, Signal::Query(Flow::Continue));
+        cbor::write_head(&mut encoded, MAJOR_MAP, 1);
+        encoded.extend_from_slice(&entry);
+        let error = decode_both(speaker, RunBudget::default(), &encoded)
+            .expect_err("a listing with a non-canonical head cannot decode");
+        prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
+        let named = matches!(
+            error.kind,
+            DecodeErrorKind::Malformed {
+                part: FramePart::QueryChildren,
+                detail: actual,
+            } if actual == detail
+        );
+        prop_assert!(named, "expected a {detail} defect, got {:?}", error.kind);
+    }
+
     /// An arbitrary canonical query round-trips through the decoder.
     #[test]
     fn canonical_queries_decode(
```

<!-- annotation -->
> **remote-codec-10** (T24), line 600:
>
> The listing-head spelling proptest over the four defects the resolution named, through decode_both, asserting the named detail. Negative control: see decode.rs:318.

<a id="hunk-23"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -612,16 +639,56 @@ proptest! {`

```diff
@@ -612,16 +639,56 @@ proptest! {
     }
 }
 
-/// A query body whose listing map is empty is rejected: an empty query
-/// travels as its own signal, so the map spelling requires at least one
-/// child (the upper bound is pinned by
+/// One listing entry with a non-canonical head, and the defect both
+/// decoders must name.
+///
+/// The spellings: a widened key (a radix below 24 spelled with a one-byte
+/// argument), a widened value head (the digest's length spelled with a
+/// two-byte argument), an indefinite-length value head, or a reserved key
+/// head. Each entry carries a full digest behind the defect, so nothing
+/// but the head is wrong.
+fn arb_listing_head_defect() -> impl Strategy<Value = (Vec<u8>, &'static str)> {
+    let digest = [0u8; MERKLE_HASH_LEN];
+    let canonical_value = move |entry: &mut Vec<u8>| {
+        cbor::write_head(entry, MAJOR_BSTR, MERKLE_HASH_LEN as u64);
+        entry.extend_from_slice(&digest);
+    };
+    prop_oneof![
+        (0_u8..24).prop_map(move |radix| {
+            let mut entry = vec![0x18, radix];
+            canonical_value(&mut entry);
+            (entry, "head not in shortest form")
+        }),
+        (0_u8..24).prop_map(move |radix| {
+            let mut entry = vec![radix, 0x59, 0x00, MERKLE_HASH_LEN as u8];
+            entry.extend_from_slice(&digest);
+            (entry, "head not in shortest form")
+        }),
+        (0_u8..24).prop_map(move |radix| {
+            let mut entry = vec![radix, 0x5f];
+            entry.extend_from_slice(&digest);
+            (entry, "indefinite-length head")
+        }),
+        Just({
+            let mut entry = vec![0x1c];
+            canonical_value(&mut entry);
+            (entry, "reserved head")
+        }),
+    ]
+}
+
+/// A query body whose listing map is empty is rejected by both decoders.
+///
+/// An empty query travels as its own signal, so the map spelling requires
+/// at least one child (the upper bound is pinned by
 /// `oversized_query_listing_is_rejected`).
 #[test]
 fn empty_query_listing_is_rejected() {
     let stream = stream(5);
     let encoded = query(stream, Flow::Continue, &[]);
     for speaker in SPEAKERS {
-        let error = decode_exact(speaker, RunBudget::default(), &encoded).unwrap_err();
+        let error = decode_both(speaker, RunBudget::default(), &encoded)
+            .expect_err("an empty listing cannot decode");
         assert!(matches!(
             error.kind,
             DecodeErrorKind::Malformed {
```

<!-- annotation -->
> **remote-codec-10** (T24), line 660:
>
> arb_listing_head_defect builds each entry with a full digest behind the defect, so only the head is wrong; empty_query_listing_is_rejected runs through decode_both.

<a id="hunk-24"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -800,7 +867,8 @@ fn reader_errors_are_contextual() {`

```diff
@@ -800,7 +867,8 @@ fn reader_errors_are_contextual() {
 }
 
 /// A transport that delivers the first `remaining` bytes of `bytes`,
-/// fails with `Other`, and then either closes cleanly or keeps failing.
+/// fails with its `kind` (`Other` from `new`, chosen by `failing_with`),
+/// and then either closes cleanly or keeps failing.
 ///
 /// One fixture serves both decoders: it reads synchronously for the
 /// oracle and asynchronously for `FrameRead`.
```

<!-- annotation -->
> **remote-codec-8** (T24), line 870:
>
> Fresh-eyes repair, round 2: FailAfter's doc names the kind it carries and which constructor sets it.

<a id="hunk-25"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -810,16 +878,29 @@ struct FailAfter {`

```diff
@@ -810,16 +878,29 @@ struct FailAfter {
     remaining: usize,
     then_eof: bool,
     failed: bool,
+    /// The kind the failing read reports.
+    kind: std::io::ErrorKind,
 }
 
 impl FailAfter {
     fn new(bytes: &[u8], remaining: usize, then_eof: bool) -> Self {
+        Self::failing_with(bytes, remaining, then_eof, std::io::ErrorKind::Other)
+    }
+
+    /// Like [`new`](Self::new), with the failing read reporting `kind`.
+    fn failing_with(
+        bytes: &[u8],
+        remaining: usize,
+        then_eof: bool,
+        kind: std::io::ErrorKind,
+    ) -> Self {
         Self {
             bytes: bytes.to_vec(),
             position: 0,
             remaining,
             then_eof,
             failed: false,
+            kind,
         }
     }
 
```

<!-- annotation -->
> **remote-codec-8** (T24), line 885:
>
> Fresh-eyes repair (round 1, item 2): FailAfter carries the kind its failing read reports; failing_with is the constructor, new keeps Other.

<a id="hunk-26"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -830,7 +911,7 @@ impl FailAfter {`

```diff
@@ -830,7 +911,7 @@ impl FailAfter {
                 return Ok(&[]);
             }
             self.failed = true;
-            return Err(std::io::ErrorKind::Other.into());
+            return Err(self.kind.into());
         }
         let available = self.bytes.len() - self.position;
         let served = self.remaining.min(available).min(want);
```

<!-- annotation -->
> **remote-codec-8** (T24), line 912:
>
> Fresh-eyes repair: the failing read reports the fixture's kind.

<a id="hunk-27"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -922,6 +1003,60 @@ fn opener_read_failures_are_reported_in_wire_order() {`

```diff
@@ -922,6 +1003,60 @@ fn opener_read_failures_are_reported_in_wire_order() {
     }
 }
 
+/// A transport failure of kind `UnexpectedEof` inside a body is a
+/// truncation of that body in both decoders, not a read error.
+///
+/// The kind alone decides, whether a bulk read met it after delivering
+/// some bytes or a whole-body read met it outright.
+#[test]
+fn body_eof_failures_are_truncations_in_both_decoders() {
+    let stream = stream(6);
+    let body = record(&Version::new(), &Message::new(1u64));
+    let supply_frame = supply(stream, Flow::Continue, &body);
+    let listing_frame = query(stream, Flow::Continue, &[(3, Hash::default())]);
+    let cases = [
+        // Two body bytes short of the run.
+        (supply_frame.len() - 2, supply_frame, FramePart::SupplyRun),
+        // Inside the listing's one entry.
+        (
+            listing_frame.len() - 8,
+            listing_frame,
+            FramePart::QueryChildren,
+        ),
+    ];
+    for speaker in SPEAKERS {
+        for (remaining, encoded, missing) in &cases {
+            let budget = RunBudget::default();
+            let eof = std::io::ErrorKind::UnexpectedEof;
+            let from_sync = decode(
+                speaker,
+                budget,
+                &mut FailAfter::failing_with(encoded, *remaining, false, eof),
+            )
+            .expect_err("a body cut by a failing read cannot decode");
+            let mut reader = FrameRead::new(
+                speaker,
+                budget,
+                FailAfter::failing_with(encoded, *remaining, false, eof),
+            );
+            let from_async = pollster::block_on(reader.frame())
+                .expect_err("a body cut by a failing read cannot decode");
+            assert_eq!(from_async.origin, from_sync.origin);
+            for error in [&from_sync, &from_async] {
+                assert!(
+                    matches!(
+                        &error.kind,
+                        DecodeErrorKind::Truncated { missing: actual, source }
+                            if actual == missing && source.kind() == eof
+                    ),
+                    "{speaker:?}, {missing:?}: {:?}",
+                    error.kind
+                );
+            }
+        }
+    }
+}
+
 /// Supply-body truncation cuts at every seeded offset all classify as a
 /// truncated `SupplyRun` with an `UnexpectedEof` source.
 ///
```

<!-- annotation -->
> **remote-codec-8** (T24), line 1010:
>
> Fresh-eyes repair: names and tests the classification the classify routing changed, an UnexpectedEof-kind transport failure inside a supply body and inside a listing entry is Truncated at the interrupted part in both decoders (it was Read on the async side); the commit message says so.

<a id="hunk-28"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -1131,7 +1266,7 @@ fn overbatched_corners_classify_exactly() {`

```diff
@@ -1131,7 +1266,7 @@ fn overbatched_corners_classify_exactly() {
     let zero = RunBudget::from_bytes(0);
     for speaker in SPEAKERS {
         // Declared bodies too short for a record's heads, none delivered.
-        for declared in 0..RECORD_TAG_LEN + 1 {
+        for declared in 0..MIN_RECORD_HEADS_LEN {
             let encoded = supply_declaring(stream, Flow::End, declared, &[]);
             let error = decode_both(speaker, zero, &encoded)
                 .expect_err("a headless over-budget body cannot decode");
```

<!-- annotation -->
> **remote-codec-8** (T24), line 1268:
>
> The corner loop's bound is the named constant.

<a id="hunk-29"></a>
### src/tree/mirror/streaming/remote/codec/frame.rs `@@ -32,6 +32,10 @@ pub const fn listing_entry_len(radix: u8) -> usize {`

```diff
@@ -32,6 +32,10 @@ pub const fn listing_entry_len(radix: u8) -> usize {
 /// run and every record within one.
 pub(super) const RECORD_TAG_LEN: usize = cbor::head_len(TAG_CBOR_SEQUENCE);
 
+/// The fewest bytes a record's heads can occupy: the tag head plus a
+/// one-byte byte-string head.
+pub(super) const MIN_RECORD_HEADS_LEN: usize = RECORD_TAG_LEN + 1;
+
 /// Head bytes of the version-atom tag ahead of a record's version.
 const VERSION_TAG_LEN: usize = cbor::head_len(crate::tags::VERSION_TAG);
 
```

<!-- annotation -->
> **remote-codec-8** (T24), line 36:
>
> MIN_RECORD_HEADS_LEN names the bare + 1 at both decoders.

<a id="hunk-30"></a>
### src/tree/mirror/streaming/remote/proxy/start.rs `@@ -109,6 +109,9 @@ where`

```diff
@@ -109,6 +109,9 @@ where
 pub struct Start;
 
 /// The peer greeting received before the local server produces its response.
+///
+/// Reached only through the [`Connect`] impl below, which the test
+/// harness's wire-path arrangement runs and production never does.
 pub struct Connecting {
     remote: Greeting,
 }
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 112:
>
> Connecting's doc says it is reached only through the client-position impl the harness runs. Fresh-eyes repair: same trim.

<a id="hunk-31"></a>
### src/tree/mirror/streaming/remote/proxy/start.rs `@@ -127,6 +130,9 @@ where`

```diff
@@ -127,6 +130,9 @@ where
     type Output = (R, W);
 }
 
+/// The wire participant in the protocol's client position: this impl and
+/// its [`CompleteConnect`] continuation exist for the test harness's
+/// wire-path arrangement and have no production caller.
 impl<B, R, W, C, A> Connect<B> for Handshaking<B, R, W, C, A>
 where
     B: Backend<Node<Z>: Leaf>,
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 136:
>
> The doc T22 orders at the Connect impl. Fresh-eyes repair: trimmed to T22's sentence (harness wire-path arrangement, no production caller); "right endpoint" was a harness-local name.

<a id="hunk-32"></a>
### src/tree/mirror/streaming/remote/proxy/start.rs `@@ -154,6 +160,8 @@ where`

```diff
@@ -154,6 +160,8 @@ where
     }
 }
 
+/// The client-position continuation of [`Connect`]: for the test
+/// harness's wire-path arrangement, with no production caller.
 impl<B, R, W, C, A> CompleteConnect<B> for Handshaking<B, R, W, C, A, Connecting>
 where
     B: Backend<Node<Z>: Leaf>,
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 163:
>
> The same at CompleteConnect. Fresh-eyes repair: trimmed to T22's sentence.

<a id="hunk-33"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -1,6 +1,5 @@`

```diff
@@ -1,6 +1,5 @@
 //! End-to-end sessions between materialized peers and protocol-start proxies.
 
-use crate::message::{PayloadCodec, PayloadDepthLimit};
 use serde::Serialize;
 use serde::de::DeserializeOwned;
 use std::convert::Infallible;
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 3:
>
> Imports follow the harness: the codec incantation, RemoteHandshaking, MirrorError, and FailingNode leave this file.

<a id="hunk-34"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -25,22 +24,22 @@ use crate::tree::{`

```diff
@@ -25,22 +24,22 @@ use crate::tree::{
     Action, Root as TreeRoot, Tree,
     arb::{arb_divergent_pair, arb_wide_divergent_pair, early_first_child_dispute_pair, nth_party},
     mirror::streaming::{
-        Failing, FailingNode, Failure, Local, Operation, Root,
-        materialized::{Error as MaterializedError, Handshaking},
+        Failing, Failure, Local, Operation, Root,
+        materialized::Handshaking,
         mirror,
         remote::{
-            Error as RemoteError, Handshaking as RemoteHandshaking,
+            Error as RemoteError,
             proxy::work::progress::{Trace, with_trace},
         },
     },
 };
-use crate::{Version, message::Message, tree::mirror::Error as MirrorError};
+use crate::{Version, message::Message};
+
+use harness::{Backends, EndpointError, Topology, codec, drive};
 
 type BackendFailure = Failure<Infallible>;
-type LocalFailure = MaterializedError<BackendFailure>;
 type ProxyFailure = RemoteError<BackendFailure>;
-type LeftFailure = MirrorError<LocalFailure, ProxyFailure>;
-type RightFailure = MirrorError<ProxyFailure, LocalFailure>;
+type EndpointFailure = EndpointError<BackendFailure>;
 
 mod containment;
 mod declarations;
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 30:
>
> The harness's drive, Topology, Backends, codec, and EndpointError are this file's vocabulary; LeftFailure/RightFailure become one EndpointFailure.

<a id="hunk-35"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -53,61 +52,29 @@ mod transport;`

```diff
@@ -53,61 +52,29 @@ mod transport;
 /// Bytes buffered by each per-stream pipe before backpressure applies.
 const TRANSPORT_CAPACITY: usize = 37;
 
-/// Drive two local starts, each paired directly with its remote protocol start.
-async fn reconcile(a: TreeRoot, b: TreeRoot) -> (TreeRoot, TreeRoot) {
-    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
-    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
-
-    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
-    let remote_b = RemoteHandshaking::start(
-        Local,
-        a_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-    let remote_a = RemoteHandshaking::start(
-        Local,
-        b_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-
-    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
-    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
-    let (_control, b) = b.expect("endpoint B should reconcile through its proxy");
-    (a.into(), b.into())
-}
-
 /// Drive the production topology: each materialized local is the client of
 /// its own proxy, so both physical endpoints execute `Accept` concurrently.
-async fn reconcile_symmetric_accepts<T>(
+async fn reconcile_symmetric_accepts(
     a: TreeRoot,
     b: TreeRoot,
     transport_capacity: usize,
-) -> (TreeRoot, TreeRoot)
-where
-    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
-{
-    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
-    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
+) -> (TreeRoot, TreeRoot) {
     let (a_link, b_link) = memory_with_capacity(transport_capacity);
-    let remote_b = RemoteHandshaking::start(
-        Local,
+    let (a, b) = drive(
+        Topology::Production,
+        Backends::local(),
+        a,
+        b,
         a_link,
-        PayloadCodec::new::<T>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-    let remote_a = RemoteHandshaking::start(
-        Local,
         b_link,
-        PayloadCodec::new::<T>(PayloadDepthLimit::default()),
+        codec::<()>(),
+        WindowConfig::FLOOR,
+    )
+    .await;
+    (
+        a.expect("endpoint A should reconcile through its proxy"),
+        b.expect("endpoint B should reconcile through its proxy"),
     )
-    .window(WindowConfig::FLOOR);
-
-    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(b, remote_a)),);
-    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
-    let (b, _control) = b.expect("endpoint B should reconcile through its proxy");
-    (a.into(), b.into())
 }
 
 /// Arrivals held and released newest-first by the reordering acceptor: deep
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 60:
>
> reconcile (the asymmetric twin) is gone: its two pollster callers and instrumented_reconcile run the production arrangement. reconcile_symmetric_accepts drops T (only () was ever instantiated) and is a wrapper choosing links and topology.

<a id="hunk-36"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -120,37 +87,30 @@ const REORDER_BATCH: usize = 3;`

```diff
@@ -120,37 +87,30 @@ const REORDER_BATCH: usize = 3;
 ///
 /// `reordered` counts the genuine inversions both ends release; the caller
 /// asserts its disposition across the run.
-async fn reconcile_symmetric_accepts_reordered<T>(
+async fn reconcile_symmetric_accepts_reordered(
     a: TreeRoot,
     b: TreeRoot,
     transport_capacity: usize,
     reordered: Arc<AtomicUsize>,
-) -> (TreeRoot, TreeRoot)
-where
-    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
-{
-    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
-    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
+) -> (TreeRoot, TreeRoot) {
     let (a_link, b_link) = memory_with_capacity(transport_capacity);
     let a_link = reorder_accepts(a_link, REORDER_BATCH, reordered.clone());
     let b_link = reorder_accepts(b_link, REORDER_BATCH, reordered);
-    let remote_b = RemoteHandshaking::start(
-        Local,
+    let (a, b) = drive(
+        Topology::Production,
+        Backends::local(),
+        a,
+        b,
         a_link,
-        PayloadCodec::new::<T>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-    let remote_a = RemoteHandshaking::start(
-        Local,
         b_link,
-        PayloadCodec::new::<T>(PayloadDepthLimit::default()),
+        codec::<()>(),
+        WindowConfig::FLOOR,
+    )
+    .await;
+    (
+        a.expect("endpoint A should reconcile through its proxy"),
+        b.expect("endpoint B should reconcile through its proxy"),
     )
-    .window(WindowConfig::FLOOR);
-
-    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(b, remote_a)),);
-    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
-    let (b, _control) = b.expect("endpoint B should reconcile through its proxy");
-    (a.into(), b.into())
 }
 
 /// Drive the production proxy topology after the shared preamble on the same
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 95:
>
> The reordered variant wraps links and drops T likewise.

<a id="hunk-37"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -159,8 +119,6 @@ async fn reconcile_after_preamble<T>(a: TreeRoot, b: TreeRoot) -> (TreeRoot, Tre`

```diff
@@ -159,8 +119,6 @@ async fn reconcile_after_preamble<T>(a: TreeRoot, b: TreeRoot) -> (TreeRoot, Tre
 where
     T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
 {
-    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
-    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
     let (mut a_link, mut b_link) = memory_with_capacity(64 * 1024);
     let network = crate::Network::from_bytes([1; 16]);
     let mut a_staged = handshake::Staged::new();
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 121:
>
> reconcile_after_preamble keeps T (its u64 caller exists) and passes codec::<T>().

<a id="hunk-38"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -187,22 +145,21 @@ where`

```diff
@@ -187,22 +145,21 @@ where
     seen_a.expect("A preamble");
     seen_b.expect("B preamble");
 
-    let remote_b = RemoteHandshaking::start(
-        Local,
+    let (a, b) = drive(
+        Topology::Production,
+        Backends::local(),
+        a,
+        b,
         a_link,
-        PayloadCodec::new::<T>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-    let remote_a = RemoteHandshaking::start(
-        Local,
         b_link,
-        PayloadCodec::new::<T>(PayloadDepthLimit::default()),
+        codec::<T>(),
+        WindowConfig::FLOOR,
+    )
+    .await;
+    (
+        a.expect("endpoint A should reconcile through its proxy"),
+        b.expect("endpoint B should reconcile through its proxy"),
     )
-    .window(WindowConfig::FLOOR);
-    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(b, remote_a)),);
-    let (a, _control) = a.expect("endpoint A should reconcile through its proxy");
-    let (b, _control) = b.expect("endpoint B should reconcile through its proxy");
-    (a.into(), b.into())
 }
 
 /// Reconcile the same pair entirely in process as the behavioral oracle.
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 150:
>
> As above: the preamble helper's drive call.

<a id="hunk-39"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -215,21 +172,16 @@ async fn reconcile_locally(a: TreeRoot, b: TreeRoot) -> (TreeRoot, TreeRoot) {`

```diff
@@ -215,21 +172,16 @@ async fn reconcile_locally(a: TreeRoot, b: TreeRoot) -> (TreeRoot, TreeRoot) {
     (a.into(), b.into())
 }
 
-/// Translate a local root into the composable failing backend's node type.
-fn failing_root(root: TreeRoot) -> Root<Failing<Local>> {
-    Root {
-        ceiling: root.ceiling,
-        root: root.root.map(FailingNode::new),
-    }
-}
-
 /// Reconcile with exactly one proxy using the supplied failing backend.
 async fn reconcile_with_failing_proxy(
     a: TreeRoot,
     b: TreeRoot,
     failing: Failing<Local>,
     fail_left: bool,
-) -> (Result<(), LeftFailure>, Result<(), RightFailure>) {
+) -> (
+    Result<TreeRoot, EndpointFailure>,
+    Result<TreeRoot, EndpointFailure>,
+) {
     reconcile_with_stacked_failures(a, b, failing, fail_left, IoPlan::default())
         .await
         .0
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 175:
>
> reconcile_with_failing_proxy returns roots; failing_root is gone (TreeBackend for Failing<Local> lifts and lowers).

<a id="hunk-40"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -243,14 +195,12 @@ async fn reconcile_with_stacked_failures(`

```diff
@@ -243,14 +195,12 @@ async fn reconcile_with_stacked_failures(
     fail_left: bool,
     io_plan: IoPlan,
 ) -> (
-    (Result<(), LeftFailure>, Result<(), RightFailure>),
+    (
+        Result<TreeRoot, EndpointFailure>,
+        Result<TreeRoot, EndpointFailure>,
+    ),
     IoReportHandle,
 ) {
-    let a = Handshaking::start(Failing::after(Local, usize::MAX), failing_root(a))
-        .window(WindowConfig::FLOOR);
-    let b = Handshaking::start(Failing::after(Local, usize::MAX), failing_root(b))
-        .window(WindowConfig::FLOOR);
-
     let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
     let (a_link, a_io) = wrap_link(
         IoSide::Left,
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 200:
>
> reconcile_with_stacked_failures returns roots, as remote-proxy-tests-7 needs; that entry's oracle comparison is not landed here (it belongs to its own lane) and is noted in the report.

<a id="hunk-41"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -270,34 +220,25 @@ async fn reconcile_with_stacked_failures(`

```diff
@@ -270,34 +220,25 @@ async fn reconcile_with_stacked_failures(
         },
         b_link,
     );
-    let left_backend = if fail_left {
-        failing.clone()
-    } else {
-        Failing::after(Local, usize::MAX)
+    let sound = || Failing::after(Local, usize::MAX);
+    let backends = Backends {
+        left: sound(),
+        left_proxy: if fail_left { failing.clone() } else { sound() },
+        right: sound(),
+        right_proxy: if fail_left { sound() } else { failing },
     };
-    let right_backend = if fail_left {
-        Failing::after(Local, usize::MAX)
-    } else {
-        failing
-    };
-    let remote_b = RemoteHandshaking::start(
-        left_backend,
+    let results = drive(
+        Topology::Production,
+        backends,
+        a,
+        b,
         a_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-    let remote_a = RemoteHandshaking::start(
-        right_backend,
         b_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-
-    let (left, right) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
-    (
-        (left.map(|_| ()), right.map(|_| ())),
-        if fail_left { a_io } else { b_io },
+        codec::<()>(),
+        WindowConfig::FLOOR,
     )
+    .await;
+    (results, if fail_left { a_io } else { b_io })
 }
 
 /// Extract the injected backend operation from a proxy conversion failure.
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 230:
>
> The four backends are named per participant; the materialized ones stay never-failing as before.

<a id="hunk-42"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -320,7 +261,7 @@ async fn equal_versions_return_both_roots() {`

```diff
@@ -320,7 +261,7 @@ async fn equal_versions_return_both_roots() {
         ceiling: Version::new(),
         root: None,
     };
-    let (a, b) = reconcile(root.clone(), root.clone()).await;
+    let (a, b) = reconcile_symmetric_accepts(root.clone(), root.clone(), TRANSPORT_CAPACITY).await;
     assert_eq!(a, root);
     assert_eq!(b, root);
 }
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 263:
>
> equal_versions_return_both_roots runs production's arrangement.

<a id="hunk-43"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -335,7 +276,7 @@ async fn divergent_leaves_converge() {`

```diff
@@ -335,7 +276,7 @@ async fn divergent_leaves_converge() {
     let mut expected = a.clone();
     expected.join(b.clone());
 
-    let (a, b) = reconcile(a.root, b.root).await;
+    let (a, b) = reconcile_symmetric_accepts(a.root, b.root, TRANSPORT_CAPACITY).await;
     assert_eq!(a, expected.root);
     assert_eq!(b, expected.root);
 }
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 278:
>
> divergent_leaves_converge likewise.

<a id="hunk-44"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -349,7 +290,7 @@ fn symmetric_accept_handshakes_are_live() {`

```diff
@@ -349,7 +290,7 @@ fn symmetric_accept_handshakes_are_live() {
     let mut b = Tree::<()>::new();
     b.act(&nth_party(1), [Action::Insert(Message::new(()))]);
 
-    let (a, b) = run_to_quiescence(reconcile_symmetric_accepts::<()>(a.root, b.root, 1))
+    let (a, b) = run_to_quiescence(reconcile_symmetric_accepts(a.root, b.root, 1))
         .expect("the production proxy topology became quiescent");
     assert_eq!(a, b);
 }
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 292:
>
> Call site without the turbofish.

<a id="hunk-45"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -378,7 +319,7 @@ proptest! {`

```diff
@@ -378,7 +319,7 @@ proptest! {
     fn symmetric_accepts_match_local((a, b) in arb_divergent_pair()) {
         let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
             .expect("local reconciliation should remain live");
-        let actual = run_to_quiescence(reconcile_symmetric_accepts::<()>(a, b, TRANSPORT_CAPACITY))
+        let actual = run_to_quiescence(reconcile_symmetric_accepts(a, b, TRANSPORT_CAPACITY))
             .map_err(|stopped| TestCaseError::fail(format!(
                 "symmetric proxy reconciliation became quiescent: {stopped:?}",
             )))?;
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 321:
>
> As above.

<a id="hunk-46"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -394,12 +335,16 @@ proptest! {`

```diff
@@ -394,12 +335,16 @@ proptest! {
     ) {
         let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
         .expect("local reconciliation should remain live");
+        let divergent = a.ceiling != b.ceiling;
         let (actual, channels, trace) = instrumented_reconcile(a, b, schedule);
         let actual = actual
             .map_err(|stopped| TestCaseError::fail(format!(
                 "wire reconciliation became quiescent: {stopped:?}",
             )))?;
         trace.assert_valid();
+        if divergent {
+            trace.assert_covers_divergent_session();
+        }
         assert_proxy_channels_are_bounded(&channels);
         prop_assert_eq!(actual, expected);
     }
```

<!-- annotation -->
> **remote-proxy-19** (T26), line 340:
>
> wire_reconciliation_matches_local calls the floor when the greetings differ; an equal pair ends at the greeting and records no proxy work, so the floor would be false there. divergent is computed before the roots move.

<a id="hunk-47"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -419,10 +364,14 @@ proptest! {`

```diff
@@ -419,10 +364,14 @@ proptest! {
         (a, b) in arb_divergent_pair(),
         schedule in vec(0_u8..=2, 0..128),
     ) {
+        let divergent = a.ceiling != b.ceiling;
         let (result, _channels, trace) = instrumented_reconcile(a, b, schedule);
         result.map_err(|stopped| TestCaseError::fail(format!(
             "wire reconciliation became quiescent: {stopped:?}",
         )))?;
+        if divergent {
+            trace.assert_covers_divergent_session();
+        }
         trace.assert_registration_causality();
     }
 
```

<!-- annotation -->
> **remote-proxy-19** (T26), line 370:
>
> context_registration_is_causal likewise.

<a id="hunk-48"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -435,7 +384,7 @@ proptest! {`

```diff
@@ -435,7 +384,7 @@ proptest! {
     fn wide_symmetric_accepts_match_local((a, b) in arb_wide_divergent_pair()) {
         let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
             .expect("local reconciliation should remain live");
-        let actual = run_to_quiescence(reconcile_symmetric_accepts::<()>(a, b, 1))
+        let actual = run_to_quiescence(reconcile_symmetric_accepts(a, b, 1))
             .map_err(|stopped| TestCaseError::fail(format!(
                 "wide symmetric proxy reconciliation became quiescent: {stopped:?}",
             )))?;
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 386:
>
> Call site without the turbofish.

<a id="hunk-49"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -466,26 +415,18 @@ proptest! {`

```diff
@@ -466,26 +415,18 @@ proptest! {
         let history = failing.history();
 
         if let Some(expected) = history.get(operations).copied() {
-            let actual = if fail_left {
-                match &result.0 {
-                    Err(MirrorError::Server(error)) => injected_operation(error),
-                    other => return Err(TestCaseError::fail(format!(
-                        "left proxy failure was masked: {other:?}",
-                    ))),
-                }
+            let (side, faulted) = if fail_left {
+                ("left", &result.0)
             } else {
-                match &result.1 {
-                    Err(MirrorError::Client(error)) => injected_operation(error),
-                    other => return Err(TestCaseError::fail(format!(
-                        "right proxy failure was masked: {other:?}",
-                    ))),
-                }
+                ("right", &result.1)
             };
-            let observed = if fail_left {
-                format!("{:?}", result.0)
-            } else {
-                format!("{:?}", result.1)
+            let actual = match faulted {
+                Err(EndpointError::Proxy(error)) => injected_operation(error),
+                other => return Err(TestCaseError::fail(format!(
+                    "{side} proxy failure was masked: {other:?}",
+                ))),
             };
+            let observed = format!("{faulted:?}");
             prop_assert_eq!(
                 actual,
                 Some(expected),
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 425:
>
> proxy_backend_failures_are_fail_fast under production: both proxies are servers, so the two position arms collapse to one EndpointError::Proxy match on the faulted side. This is how far the topology forces the projection (remote-proxy-tests-16's helper is not built here).

<a id="hunk-50"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -539,7 +480,7 @@ fn wide_symmetric_accepts_reordered_match_local() {`

```diff
@@ -539,7 +480,7 @@ fn wide_symmetric_accepts_reordered_match_local() {
     let cases = runner.run(&arb_wide_divergent_pair(), move |(a, b)| {
         let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
             .expect("local reconciliation should remain live");
-        let actual = run_to_quiescence(reconcile_symmetric_accepts_reordered::<()>(
+        let actual = run_to_quiescence(reconcile_symmetric_accepts_reordered(
             a,
             b,
             1,
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 482:
>
> Call site without the turbofish.

<a id="hunk-51"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -581,9 +522,8 @@ fn early_first_child_dispute_is_live() {`

```diff
@@ -581,9 +522,8 @@ fn early_first_child_dispute_is_live() {
     let (a, b) = early_first_child_dispute_pair();
     let expected = run_to_quiescence(reconcile_locally(a.clone(), b.clone()))
         .expect("local reconciliation of the trigger geometry remains live");
-    let (left, right) =
-        run_to_quiescence(reconcile_symmetric_accepts::<()>(a, b, TRANSPORT_CAPACITY))
-            .expect("the trigger geometry must reconcile over the wire");
+    let (left, right) = run_to_quiescence(reconcile_symmetric_accepts(a, b, TRANSPORT_CAPACITY))
+        .expect("the trigger geometry must reconcile over the wire");
     assert_eq!((left, right), expected);
 }
 
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 525:
>
> As above.

<a id="hunk-52"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -597,6 +537,7 @@ fn instrumented_channels_cover_every_proxy_edge() {`

```diff
@@ -597,6 +537,7 @@ fn instrumented_channels_cover_every_proxy_edge() {
     let (result, report, trace) = instrumented_reconcile(a.root, b.root, Vec::new());
     result.expect("the instrumented wire session should remain live");
     trace.assert_valid();
+    trace.assert_covers_divergent_session();
     assert_proxy_channels_are_bounded(&report);
     for kind in QueueKind::PROXY {
         assert!(
```

<!-- annotation -->
> **remote-proxy-19** (T26), line 540:
>
> instrumented_channels_cover_every_proxy_edge calls the floor beside the channel floor; its fixture is divergent by construction.

<a id="hunk-53"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -617,7 +558,11 @@ fn instrumented_reconcile(`

```diff
@@ -617,7 +558,11 @@ fn instrumented_reconcile(
     Trace,
 ) {
     let ((result, channels), trace) = with_trace(|| {
-        with_observation(|| with_schedule(schedule, || run_to_quiescence(reconcile(a, b))))
+        with_observation(|| {
+            with_schedule(schedule, || {
+                run_to_quiescence(reconcile_symmetric_accepts(a, b, TRANSPORT_CAPACITY))
+            })
+        })
     });
     (result, channels, trace)
 }
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 562:
>
> instrumented_reconcile runs production's arrangement; the trace and channel instruments are topology-independent.

<a id="hunk-54"></a>
### src/tree/mirror/streaming/remote/proxy/tests/containment.rs `@@ -1,56 +1,39 @@`

```diff
@@ -1,56 +1,39 @@
 //! Version-containment enforcement over the full wire stack.
 
-use crate::message::{PayloadCodec, PayloadDepthLimit};
-use futures::join;
-
 use crate::link::memory_with_capacity;
 use crate::testing::run_to_quiescence;
 use crate::tree::arb::uncontained_supply_pair;
 use crate::tree::mirror::streaming::window::WindowConfig;
 use crate::tree::{
     Root as TreeRoot,
-    mirror::{
-        Error as MirrorError,
-        streaming::{
-            Local, Root,
-            materialized::{Error as MaterializedError, Handshaking, Violation},
-            mirror,
-            remote::Handshaking as RemoteHandshaking,
-        },
-    },
+    mirror::streaming::materialized::{Error as MaterializedError, Violation},
 };
 
 use super::TRANSPORT_CAPACITY;
-use super::harness::{LeftError, RightError};
+use super::harness::{Backends, EndpointError, EndpointFailure, Topology, codec, drive};
 
-/// Drive the two-proxy topology, returning each endpoint's result instead
-/// of asserting success.
+/// Drive the two-proxy topology in which the right endpoint's materialized
+/// participant is the protocol server, returning each endpoint's result
+/// instead of asserting success.
 async fn reconcile_results(
     a: TreeRoot,
     b: TreeRoot,
-) -> (Result<TreeRoot, LeftError>, Result<TreeRoot, RightError>) {
-    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
-    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
-
+) -> (
+    Result<TreeRoot, EndpointFailure>,
+    Result<TreeRoot, EndpointFailure>,
+) {
     let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
-    let remote_b = RemoteHandshaking::start(
-        Local,
+    drive(
+        Topology::RightProxyConnects,
+        Backends::local(),
+        a,
+        b,
         a_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-    let remote_a = RemoteHandshaking::start(
-        Local,
         b_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(WindowConfig::FLOOR);
-
-    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
-    (
-        a.map(|(root, _control)| root.into()),
-        b.map(|(_control, root)| root.into()),
+        codec::<()>(),
+        WindowConfig::FLOOR,
     )
+    .await
 }
 
 /// A supplied leaf whose version escapes the sender's declared greeting
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 20:
>
> reconcile_results is the harness's drive with Topology::RightProxyConnects: the one asymmetric caller, kept per T22.

<a id="hunk-55"></a>
### src/tree/mirror/streaming/remote/proxy/tests/containment.rs `@@ -58,10 +41,11 @@ async fn reconcile_results(`

```diff
@@ -58,10 +41,11 @@ async fn reconcile_results(
 /// link.
 ///
 /// The enforcement holds through the frame codec and supply decoder, not
-/// only in process. The receiving endpoint reports the violation from its own materialized
-/// participant in either endpoint position; the sender's endpoint is left
-/// to whatever its aborted transport surfaces, which is not this
-/// tripwire's concern. The in-process twin is
+/// only in process. The receiving endpoint reports the violation from its
+/// own materialized participant in either protocol position (the topology
+/// makes the left one the client and the right one the server); the
+/// sender's endpoint is left to whatever its aborted transport surfaces,
+/// which is not this tripwire's concern. The in-process twin is
 /// `uncontained_supply_is_rejected_by_streaming`.
 #[test]
 fn uncontained_supply_is_rejected_at_the_wire() {
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 45:
>
> The check site now says the topology puts the right materialized participant in the server position, since the endpoint error no longer carries the position.

<a id="hunk-56"></a>
### src/tree/mirror/streaming/remote/proxy/tests/containment.rs `@@ -75,7 +59,7 @@ fn uncontained_supply_is_rejected_at_the_wire() {`

```diff
@@ -75,7 +59,7 @@ fn uncontained_supply_is_rejected_at_the_wire() {
         assert!(
             matches!(
                 receiver_out,
-                Err(MirrorError::Client(MaterializedError::Violation(
+                Err(EndpointError::Local(MaterializedError::Violation(
                     Violation::UncontainedSupply
                 ))),
             ),
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 62:
>
> EndpointError::Local in the client position.

<a id="hunk-57"></a>
### src/tree/mirror/streaming/remote/proxy/tests/containment.rs `@@ -93,7 +77,7 @@ fn uncontained_supply_is_rejected_at_the_wire() {`

```diff
@@ -93,7 +77,7 @@ fn uncontained_supply_is_rejected_at_the_wire() {
         assert!(
             matches!(
                 receiver_out,
-                Err(MirrorError::Server(MaterializedError::Violation(
+                Err(EndpointError::Local(MaterializedError::Violation(
                     Violation::UncontainedSupply
                 ))),
             ),
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 80:
>
> EndpointError::Local in the server position; the position is the topology's, stated in the doc.

<a id="hunk-58"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -15,20 +15,37 @@ use crate::testing::run_to_quiescence;`

```diff
@@ -15,20 +15,37 @@ use crate::testing::run_to_quiescence;
 use crate::tree::{
     Action, Tree,
     arb::{early_first_child_dispute_pair, nth_party},
-    mirror::{
-        Error as MirrorError,
-        streaming::{
-            remote::{
-                CodecDecodeError, CodecDecodeErrorKind, Error as RemoteError, ReplyDecodeError,
-                StreamError,
-            },
-            window::FAN,
+    mirror::streaming::{
+        remote::{
+            CodecDecodeError, CodecDecodeErrorKind, Error as RemoteError, ReplyDecodeError,
+            StreamError,
         },
+        window::FAN,
     },
     typed::hash::MERKLE_HASH_LEN,
 };
 
-use super::harness::{self, GreetingRewrite};
+use super::harness::{self, EndpointError, EndpointFailure, GreetingRewrite};
+
+/// Borrow the proxy error the receiving side reported.
+fn receiver_error<'a>(
+    receiver_left: bool,
+    left: &'a Result<crate::tree::Root, EndpointFailure>,
+    right: &'a Result<crate::tree::Root, EndpointFailure>,
+    lie: &str,
+) -> &'a RemoteError<std::convert::Infallible> {
+    let (side, receiving) = if receiver_left {
+        ("left", left)
+    } else {
+        ("right", right)
+    };
+    match receiving {
+        Err(EndpointError::Proxy(error)) => error,
+        other => {
+            panic!("undetected {lie} lie: the {side} proxy did not report the violation: {other:?}")
+        }
+    }
+}
 
 /// The observable root hash of a reconciled `tree::Root`.
 fn hash_of(root: &crate::tree::Root) -> [u8; MERKLE_HASH_LEN] {
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 40:
>
> The four inline projections were identical once both proxies are servers; receiver_error is their one copy, with the lie named in its message. Only as far as the topology forces: the receiver_left flag keeps its meaning (the reporting side).

<a id="hunk-59"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -106,23 +123,7 @@ fn understated_target_message_size_fails_the_session() {`

```diff
@@ -106,23 +123,7 @@ fn understated_target_message_size_fails_the_session() {
             left, right, hears.0, hears.1,
         ))
         .expect("an overbatched supply run must terminate both sessions, not stall them");
-        let receiver_error = if receiver_left {
-            match &left {
-                Err(MirrorError::Server(error)) => error,
-                other => panic!(
-                    "undetected target_message_size lie: the left proxy did not \
-                     report the violation: {other:?}"
-                ),
-            }
-        } else {
-            match &right {
-                Err(MirrorError::Client(error)) => error,
-                other => panic!(
-                    "undetected target_message_size lie: the right proxy did not \
-                     report the violation: {other:?}"
-                ),
-            }
-        };
+        let receiver_error = receiver_error(receiver_left, &left, &right, "target_message_size");
         assert!(
             matches!(
                 receiver_error,
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 125:
>
> Site through receiver_error.

<a id="hunk-60"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -185,17 +186,7 @@ fn understated_version_bytes_fail_the_session() {`

```diff
@@ -185,17 +186,7 @@ fn understated_version_bytes_fail_the_session() {
             (!receiver_left).then_some(rewrite),
         ))
         .expect("an oversized supplied version must terminate both sessions");
-        let receiver_error = if receiver_left {
-            match &left {
-                Err(MirrorError::Server(error)) => error,
-                other => panic!("the left proxy did not report the violation: {other:?}"),
-            }
-        } else {
-            match &right {
-                Err(MirrorError::Client(error)) => error,
-                other => panic!("the right proxy did not report the violation: {other:?}"),
-            }
-        };
+        let receiver_error = receiver_error(receiver_left, &left, &right, "max_version_bytes");
         assert!(matches!(
             receiver_error,
             RemoteError::Decode(ReplyDecodeError::OversizedVersion { declared: 0, .. })
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 188:
>
> As above.

<a id="hunk-61"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -242,23 +233,7 @@ fn understated_set_len_fails_the_session() {`

```diff
@@ -242,23 +233,7 @@ fn understated_set_len_fails_the_session() {
             left, right, hears.0, hears.1,
         ))
         .expect("an overdrawn supply stream must terminate both sessions");
-        let receiver_error = if receiver_left {
-            match &left {
-                Err(MirrorError::Server(error)) => error,
-                other => panic!(
-                    "undetected set_len lie: the left proxy did not report \
-                     the violation: {other:?}"
-                ),
-            }
-        } else {
-            match &right {
-                Err(MirrorError::Client(error)) => error,
-                other => panic!(
-                    "undetected set_len lie: the right proxy did not report \
-                     the violation: {other:?}"
-                ),
-            }
-        };
+        let receiver_error = receiver_error(receiver_left, &left, &right, "set_len");
         assert!(
             matches!(
                 receiver_error,
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 235:
>
> As above.

<a id="hunk-62"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -314,23 +289,8 @@ fn set_len_overrun_within_one_reply_fails_at_ingress() {`

```diff
@@ -314,23 +289,8 @@ fn set_len_overrun_within_one_reply_fails_at_ingress() {
             left, right, hears.0, hears.1,
         ))
         .expect("a mid-reply overdrawn supply must terminate both sessions, not stall them");
-        let receiver_error = if receiver_left {
-            match &left {
-                Err(MirrorError::Server(error)) => error,
-                other => panic!(
-                    "undetected within-one-reply set_len lie: the left proxy \
-                     did not report the violation: {other:?}"
-                ),
-            }
-        } else {
-            match &right {
-                Err(MirrorError::Client(error)) => error,
-                other => panic!(
-                    "undetected within-one-reply set_len lie: the right proxy \
-                     did not report the violation: {other:?}"
-                ),
-            }
-        };
+        let receiver_error =
+            receiver_error(receiver_left, &left, &right, "within-one-reply set_len");
         assert!(
             matches!(
                 receiver_error,
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 291:
>
> As above.

<a id="hunk-63"></a>
### src/tree/mirror/streaming/remote/proxy/tests/failures.rs `@@ -13,18 +13,17 @@ use crate::tree::{`

```diff
@@ -13,18 +13,17 @@ use crate::tree::{
     Action, Tree,
     arb::arb_divergent_pair,
     arb::nth_party,
-    mirror::{
-        Error as MirrorError,
-        streaming::{
-            Failing, Local,
-            remote::{
-                CodecDecodeErrorKind, CodecEncodeErrorKind, Error as RemoteError, SendError,
-                StreamError,
-            },
+    mirror::streaming::{
+        Failing, Local,
+        remote::{
+            CodecDecodeErrorKind, CodecEncodeErrorKind, Error as RemoteError, SendError,
+            StreamError,
         },
     },
 };
 
+use super::harness::EndpointError;
+
 /// Find the typed injected source retained anywhere below a remote failure.
 fn injected<E>(error: &RemoteError<E>) -> Option<InjectedIo> {
     let source = match error {
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 20:
>
> Imports: EndpointError replaces MirrorError here.

<a id="hunk-64"></a>
### src/tree/mirror/streaming/remote/proxy/tests/failures.rs `@@ -131,20 +130,16 @@ fn endpoint_error(`

```diff
@@ -131,20 +130,16 @@ fn endpoint_error(
     outcome: &harness::Outcome,
     fail_left: bool,
 ) -> Result<&RemoteError<Infallible>, TestCaseError> {
-    if fail_left {
-        match &outcome.left {
-            Err(MirrorError::Server(error)) => Ok(error),
-            other => Err(TestCaseError::fail(format!(
-                "left transport fault was masked: {other:?}",
-            ))),
-        }
+    let (side, faulted) = if fail_left {
+        ("left", &outcome.left)
     } else {
-        match &outcome.right {
-            Err(MirrorError::Client(error)) => Ok(error),
-            other => Err(TestCaseError::fail(format!(
-                "right transport fault was masked: {other:?}",
-            ))),
-        }
+        ("right", &outcome.right)
+    };
+    match faulted {
+        Err(EndpointError::Proxy(error)) => Ok(error),
+        other => Err(TestCaseError::fail(format!(
+            "{side} transport fault was masked: {other:?}",
+        ))),
     }
 }
 
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 135:
>
> endpoint_error: one EndpointError::Proxy arm over the selected side; the fail_left flag keeps its meaning.

<a id="hunk-65"></a>
### src/tree/mirror/streaming/remote/proxy/tests/failures.rs `@@ -351,7 +346,7 @@ fn stacked_backend_and_transport_failures_remain_distinct() {`

```diff
@@ -351,7 +346,7 @@ fn stacked_backend_and_transport_failures_remain_distinct() {
     ))
     .expect("backend-first stacked failure should terminate");
     let backend_error = match &left_result {
-        Err(MirrorError::Server(error)) => error,
+        Err(EndpointError::Proxy(error)) => error,
         other => panic!("backend error was masked: {other:?}"),
     };
     assert_eq!(
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 348:
>
> Stacked-failure matches on the proxy error; the results now carry roots and the test keeps discarding them at its own sites.

<a id="hunk-66"></a>
### src/tree/mirror/streaming/remote/proxy/tests/failures.rs `@@ -378,7 +373,7 @@ fn stacked_backend_and_transport_failures_remain_distinct() {`

```diff
@@ -378,7 +373,7 @@ fn stacked_backend_and_transport_failures_remain_distinct() {
     ))
     .expect("transport-first stacked failure should terminate");
     let transport_error = match &left_result {
-        Err(MirrorError::Server(error)) => error,
+        Err(EndpointError::Proxy(error)) => error,
         other => panic!("transport error was masked: {other:?}"),
     };
     assert_eq!(injected(transport_error), io.snapshot().injected);
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 375:
>
> As above.

<a id="hunk-67"></a>
### src/tree/mirror/streaming/remote/proxy/tests/harness.rs `@@ -15,16 +15,19 @@ use tokio::io::{AsyncRead, AsyncWrite};`

```diff
@@ -15,16 +15,19 @@ use tokio::io::{AsyncRead, AsyncWrite};
 
 use tokio::io::ReadBuf;
 
+use serde::{Serialize, de::DeserializeOwned};
+
 use crate::link::{Acceptor, Connector, Done, Link, MemoryLink, memory_with_capacity};
 use crate::testing::{IoPlan, IoReportHandle, IoSide, wrap_link};
 use crate::tree::mirror::cbor;
 use crate::tree::mirror::streaming::window::WindowConfig;
+use crate::tree::typed::height::Z;
 use crate::tree::{
     Root as TreeRoot,
     mirror::{
         Error as MirrorError,
         streaming::{
-            Local, Root,
+            Backend, Failing, FailingNode, Leaf, Local, Root,
             materialized::{Error as MaterializedError, Handshaking},
             mirror,
             remote::{Error as RemoteError, Handshaking as RemoteHandshaking},
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 20:
>
> Imports for the generic drive: serde bounds for codec::<T>, Backend/Leaf/Failing/FailingNode for TreeBackend, Z for the backend bound.

<a id="hunk-68"></a>
### src/tree/mirror/streaming/remote/proxy/tests/harness.rs `@@ -41,18 +44,108 @@ const QUERY_STATES: RangeInclusive<u8> = 4..=5;`

```diff
@@ -41,18 +44,108 @@ const QUERY_STATES: RangeInclusive<u8> = 4..=5;
 /// Dense states below this boundary carry reactions rather than bare ends.
 const REACTION_STATE_COUNT: u8 = 8;
 
-/// Failure returned by the materialized-left/proxy-right driver.
-pub type LeftError = MirrorError<MaterializedError<Infallible>, RemoteError<Infallible>>;
+/// One endpoint's session failure, named by the participant that raised
+/// it.
+///
+/// `MirrorError` names protocol positions; which position a participant
+/// holds is the [`Topology`]'s choice, so the harness reports by
+/// participant and the topology alone says who was client and who was
+/// server.
+#[derive(Debug)]
+pub enum EndpointError<E> {
+    /// The endpoint's materialized participant failed.
+    Local(MaterializedError<E>),
+    /// The endpoint's proxy failed.
+    Proxy(RemoteError<E>),
+}
+
+/// An endpoint failure over the infallible `Local` backend.
+pub type EndpointFailure = EndpointError<Infallible>;
+
+/// How each endpoint pairs its materialized participant with its proxy.
+#[derive(Clone, Copy)]
+pub enum Topology {
+    /// What `Peer` runs: each materialized participant is the client of
+    /// its own proxy, so both proxies accept and exchange greetings
+    /// concurrently.
+    Production,
+    /// The right endpoint's proxy connects and its materialized
+    /// participant accepts: the arrangement that pins the materialized
+    /// participant's behavior in the server position.
+    RightProxyConnects,
+}
+
+/// A backend whose roots convert to and from `tree::Root`: `Local`, and
+/// the failing wrapper over it.
+pub trait TreeBackend: Backend<Node<Z>: Leaf> + Clone + Send + Sync + 'static {
+    /// Wrap a tree root in this backend's node type.
+    fn lift(root: TreeRoot) -> Root<Self>;
+    /// Unwrap a reconciled root back to the tree's.
+    fn lower(root: Root<Self>) -> TreeRoot;
+}
+
+impl TreeBackend for Local {
+    fn lift(root: TreeRoot) -> Root<Self> {
+        root.into()
+    }
+
+    fn lower(root: Root<Self>) -> TreeRoot {
+        root.into()
+    }
+}
+
+impl TreeBackend for Failing<Local> {
+    fn lift(root: TreeRoot) -> Root<Self> {
+        Root {
+            ceiling: root.ceiling,
+            root: root.root.map(FailingNode::new),
+        }
+    }
+
+    fn lower(root: Root<Self>) -> TreeRoot {
+        TreeRoot {
+            ceiling: root.ceiling,
+            root: root.root.map(FailingNode::into_inner),
+        }
+    }
+}
+
+/// The four participants' backends: each endpoint's materialized
+/// participant and the proxy beside it.
+pub struct Backends<B> {
+    pub left: B,
+    pub left_proxy: B,
+    pub right: B,
+    pub right_proxy: B,
+}
+
+impl Backends<Local> {
+    /// Every participant on the infallible in-memory backend.
+    pub fn local() -> Self {
+        Self {
+            left: Local,
+            left_proxy: Local,
+            right: Local,
+            right_proxy: Local,
+        }
+    }
+}
 
-/// Failure returned by the proxy-left/materialized-right driver.
-pub type RightError = MirrorError<RemoteError<Infallible>, MaterializedError<Infallible>>;
+/// The payload codec for `T` at the default depth limit: the one every
+/// proxy in the partition decodes through.
+pub fn codec<T>() -> PayloadCodec
+where
+    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
+{
+    PayloadCodec::new::<T>(PayloadDepthLimit::default())
+}
 
 /// Both endpoint results and their physical-I/O observations.
 pub struct Outcome {
     /// The first materialized tree, or its session failure.
-    pub left: Result<TreeRoot, LeftError>,
+    pub left: Result<TreeRoot, EndpointFailure>,
     /// The second materialized tree, or its session failure.
-    pub right: Result<TreeRoot, RightError>,
+    pub right: Result<TreeRoot, EndpointFailure>,
     /// I/O performed by the first proxy endpoint.
     pub left_io: IoReportHandle,
     /// I/O performed by the second proxy endpoint.
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 60:
>
> EndpointError names the participant, not the protocol position, because position is the topology's choice: this is the collapse of the Server/Client projection, as far as the topology forces it and no further (position stays visible through Topology). Topology's two variants; TreeBackend lifts and lowers tree::Root for Local and Failing<Local>; Backends names the four participants; codec::<T>() is the partition's one PayloadCodec::new.

<a id="hunk-69"></a>
### src/tree/mirror/streaming/remote/proxy/tests/harness.rs `@@ -457,7 +550,17 @@ pub async fn reconcile(`

```diff
@@ -457,7 +550,17 @@ pub async fn reconcile(
     let (left_link, left_io) = wrap_link(IoSide::Left, left_plan, left_link);
     let (right_link, right_io) = wrap_link(IoSide::Right, right_plan, right_link);
 
-    let (left, right) = drive(left, right, left_link, right_link, WindowConfig::FLOOR).await;
+    let (left, right) = drive(
+        Topology::Production,
+        Backends::local(),
+        left,
+        right,
+        left_link,
+        right_link,
+        codec::<()>(),
+        WindowConfig::FLOOR,
+    )
+    .await;
 
     Outcome {
         left,
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 555:
>
> reconcile (the transport and failure suites' entry) drives production by default.

<a id="hunk-70"></a>
### src/tree/mirror/streaming/remote/proxy/tests/harness.rs `@@ -476,13 +579,19 @@ pub async fn reconcile_rewritten_greetings(`

```diff
@@ -476,13 +579,19 @@ pub async fn reconcile_rewritten_greetings(
     right: TreeRoot,
     left_hears: Option<GreetingRewrite>,
     right_hears: Option<GreetingRewrite>,
-) -> (Result<TreeRoot, LeftError>, Result<TreeRoot, RightError>) {
+) -> (
+    Result<TreeRoot, EndpointFailure>,
+    Result<TreeRoot, EndpointFailure>,
+) {
     let (left_link, right_link) = memory_with_capacity(TRANSPORT_CAPACITY);
     drive(
+        Topology::Production,
+        Backends::local(),
         left,
         right,
         rewritten(left_link, left_hears),
         rewritten(right_link, right_hears),
+        codec::<()>(),
         WindowConfig::default(),
     )
     .await
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 585:
>
> reconcile_rewritten_greetings likewise, still under the default window.

<a id="hunk-71"></a>
### src/tree/mirror/streaming/remote/proxy/tests/harness.rs `@@ -539,13 +648,19 @@ pub async fn reconcile_scripted(`

```diff
@@ -539,13 +648,19 @@ pub async fn reconcile_scripted(
     right: TreeRoot,
     left_script: Option<Script>,
     right_script: Option<Script>,
-) -> (Result<TreeRoot, LeftError>, Result<TreeRoot, RightError>) {
+) -> (
+    Result<TreeRoot, EndpointFailure>,
+    Result<TreeRoot, EndpointFailure>,
+) {
     let (left_link, right_link) = memory_with_capacity(TRANSPORT_CAPACITY);
     drive(
+        Topology::Production,
+        Backends::local(),
         left,
         right,
         scripted(left_link, left_script),
         scripted(right_link, right_script),
+        codec::<()>(),
         WindowConfig::FLOOR,
     )
     .await
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 655:
>
> reconcile_scripted likewise.

<a id="hunk-72"></a>
### src/tree/mirror/streaming/remote/proxy/tests/harness.rs `@@ -575,15 +690,31 @@ fn scripted(`

```diff
@@ -575,15 +690,31 @@ fn scripted(
     .into_link()
 }
 
-/// Drive the shared two-mirror topology over already-wrapped links.
-async fn drive<LR, LW, LC, LA, RR, RW, RC, RA>(
+/// Drive one two-proxy session over already-wrapped links: the one place
+/// the four participants are constructed.
+///
+/// `left_link` carries the left endpoint, whose proxy represents the right
+/// peer; `right_link` the reverse. Every result is the endpoint's
+/// materialized tree or the failure of whichever of its two participants
+/// raised one.
+// One premise per argument: the topology, the four backends, the two
+// roots, the two links, the codec, and the window.
+#[allow(clippy::too_many_arguments)]
+pub async fn drive<B, LR, LW, LC, LA, RR, RW, RC, RA>(
+    topology: Topology,
+    backends: Backends<B>,
     left: TreeRoot,
     right: TreeRoot,
     left_link: Link<LR, LW, LC, LA>,
     right_link: Link<RR, RW, RC, RA>,
+    codec: PayloadCodec,
     window: WindowConfig,
-) -> (Result<TreeRoot, LeftError>, Result<TreeRoot, RightError>)
+) -> (
+    Result<TreeRoot, EndpointError<B::Error>>,
+    Result<TreeRoot, EndpointError<B::Error>>,
+)
 where
+    B: TreeBackend,
     LR: AsyncRead + Unpin + Send,
     LW: AsyncWrite + Unpin + Send,
     LC: Connector,
```

<!-- annotation -->
> **remote-proxy-tests-5** (T22), line 700:
>
> drive: the one construction of the four participants and the two RemoteHandshaking::start lines (one site, two proxies). The left endpoint's arrangement is the same in both topologies; only the right endpoint's mirror argument order changes. Rejected alternative: a type-level topology trait keeping MirrorError raw; it needed the same two projections and left the seven Client/Server sites in place. Fresh-eyes repair: the too_many_arguments rationale sits above the attribute, as the codec's encoder does it.

<a id="hunk-73"></a>
### src/tree/mirror/streaming/remote/proxy/tests/harness.rs `@@ -593,26 +724,50 @@ where`

```diff
@@ -593,26 +724,50 @@ where
     RC: Connector,
     RA: Acceptor,
 {
-    let left = Handshaking::start(Local, Root::<Local>::from(left)).window(window);
-    let right = Handshaking::start(Local, Root::<Local>::from(right)).window(window);
-    let remote_right = RemoteHandshaking::start(
-        Local,
-        left_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(window);
-    let remote_left = RemoteHandshaking::start(
-        Local,
-        right_link,
-        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
-    )
-    .window(window);
-    let (left, right) = join!(
-        Box::pin(mirror(left, remote_right)),
-        Box::pin(mirror(remote_left, right)),
-    );
-    (
-        left.map(|(root, _control)| root.into()),
-        right.map(|(_control, root)| root.into()),
-    )
+    let Backends {
+        left: left_backend,
+        left_proxy,
+        right: right_backend,
+        right_proxy,
+    } = backends;
+    let left = Handshaking::start(left_backend, B::lift(left)).window(window);
+    let right = Handshaking::start(right_backend, B::lift(right)).window(window);
+    let remote_right = RemoteHandshaking::start(left_proxy, left_link, codec).window(window);
+    let remote_left = RemoteHandshaking::start(right_proxy, right_link, codec).window(window);
+
+    // The left endpoint's materialized participant is the client of its
+    // proxy in both arrangements; the topology decides the right one.
+    let left = mirror(left, remote_right);
+    match topology {
+        Topology::Production => {
+            let (left, right) = join!(Box::pin(left), Box::pin(mirror(right, remote_left)));
+            (local_client(left), local_client(right))
+        }
+        Topology::RightProxyConnects => {
+            let (left, right) = join!(Box::pin(left), Box::pin(mirror(remote_left, right)));
+            (local_client(left), local_server(right))
+        }
+    }
+}
+
+/// Name an endpoint result whose materialized participant was the client.
+fn local_client<B: TreeBackend, W>(
+    result: Result<(Root<B>, W), MirrorError<MaterializedError<B::Error>, RemoteError<B::Error>>>,
+) -> Result<TreeRoot, EndpointError<B::Error>> {
+    match result {
+        Ok((root, _control)) => Ok(B::lower(root)),
+        Err(MirrorError::Client(error)) => Err(EndpointError::Local(error)),
+        Err(MirrorError::Server(error)) => Err(EndpointError::Proxy(error)),
+    }
+}
+
+/// Name an endpoint result whose materialized participant was the server.
+fn local_server<B: TreeBackend, W>(
+    result: Result<(W, Root<B>), MirrorError<RemoteError<B::Error>, MaterializedError<B::Error>>>,
+) -> Result<TreeRoot, EndpointError<B::Error>> {
+    match result {
+        Ok((_control, root)) => Ok(B::lower(root)),
+        Err(MirrorError::Client(error)) => Err(EndpointError::Proxy(error)),
+        Err(MirrorError::Server(error)) => Err(EndpointError::Local(error)),
+    }
 }
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 740:
>
> local_client and local_server are the only two places MirrorError's positions are read in the partition; each names which participant held which position.

<a id="hunk-74"></a>
### src/tree/mirror/streaming/remote/proxy/tests/malformed.rs `@@ -1,14 +1,11 @@`

```diff
@@ -1,14 +1,11 @@
 //! Full-stack rejection of peer-controlled malformed frames.
 
-use super::harness::{self, FrameMutation, FrameSelector, Script};
+use super::harness::{self, EndpointError, EndpointFailure, FrameMutation, FrameSelector, Script};
 use crate::testing::run_to_quiescence;
 use crate::tree::{
     arb::early_first_child_dispute_pair,
-    mirror::{
-        Error as MirrorError,
-        streaming::remote::{
-            CodecDecodeErrorKind, DecodeSignalError, Error as RemoteError, StreamError,
-        },
+    mirror::streaming::remote::{
+        CodecDecodeErrorKind, DecodeSignalError, Error as RemoteError, StreamError,
     },
 };
 
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 3:
>
> Imports: EndpointError and EndpointFailure replace the two position-typed aliases.

<a id="hunk-75"></a>
### src/tree/mirror/streaming/remote/proxy/tests/malformed.rs `@@ -42,19 +39,17 @@ fn reserved_state(error: &RemoteError<std::convert::Infallible>) -> Option<u64>`

```diff
@@ -42,19 +39,17 @@ fn reserved_state(error: &RemoteError<std::convert::Infallible>) -> Option<u64>
 /// Borrow the remote error detected opposite the corrupt writer.
 fn receiving_error<'a>(
     corrupt_left: bool,
-    left: &'a Result<crate::tree::Root, harness::LeftError>,
-    right: &'a Result<crate::tree::Root, harness::RightError>,
+    left: &'a Result<crate::tree::Root, EndpointFailure>,
+    right: &'a Result<crate::tree::Root, EndpointFailure>,
 ) -> &'a RemoteError<std::convert::Infallible> {
-    if corrupt_left {
-        match right {
-            Err(MirrorError::Client(error)) => error,
-            other => panic!("receiving right proxy did not report the fault: {other:?}"),
-        }
+    let (side, receiving) = if corrupt_left {
+        ("right", right)
     } else {
-        match left {
-            Err(MirrorError::Server(error)) => error,
-            other => panic!("receiving left proxy did not report the fault: {other:?}"),
-        }
+        ("left", left)
+    };
+    match receiving {
+        Err(EndpointError::Proxy(error)) => error,
+        other => panic!("receiving {side} proxy did not report the fault: {other:?}"),
     }
 }
 
```

<!-- annotation -->
> **remote-proxy-tests-24** (T22), line 45:
>
> receiving_error: one EndpointError::Proxy arm over the receiving side; corrupt_left keeps its polarity (remote-proxy-tests-16 decides whether the two flags unify).

<a id="hunk-76"></a>
### src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs `@@ -54,6 +54,42 @@ impl Trace {`

```diff
@@ -54,6 +54,42 @@ impl Trace {
         );
     }
 
+    /// Assert the floor every divergent two-proxy session meets: exactly one
+    /// greeting-seeded opening reply and exactly one opening question at the
+    /// under-root height, and at least one wire reply.
+    ///
+    /// A divergent session elects one initiator and one responder. The
+    /// initiator-side proxy seeds exactly one under-root reply from the
+    /// greeting (`Work::initiator`), and the responder-side proxy publishes
+    /// exactly one under-root question after flushing its opening wire
+    /// reply (`encode::opening`); nothing else records at that height. The
+    /// ordering assertions quantify over whatever was recorded and pass on
+    /// an empty trace; this floor is what makes them bite.
+    pub fn assert_covers_divergent_session(&self) {
+        let at_under_root = |kind: fn(Kind) -> bool| {
+            self.0
+                .iter()
+                .filter(|event| event.height == UnderRoot::HEIGHT && kind(event.kind))
+                .count()
+        };
+        let openings = at_under_root(|kind| matches!(kind, Kind::DecodedReply { .. }));
+        assert_eq!(
+            openings, 1,
+            "a divergent session records exactly one greeting-seeded opening reply, found {openings}",
+        );
+        let questions = at_under_root(|kind| matches!(kind, Kind::LocalQuestion));
+        assert_eq!(
+            questions, 1,
+            "a divergent session records exactly one opening question, found {questions}",
+        );
+        assert!(
+            self.0
+                .iter()
+                .any(|event| matches!(event.kind, Kind::WireReply { .. })),
+            "a divergent session records at least one wire reply, found none",
+        );
+    }
+
     /// Assert context-registration causality: no decoded reply overtakes
     /// the flushed local question whose scope interprets it.
     ///
```

<!-- annotation -->
> **remote-proxy-19** (T26), line 60:
>
> The floor. Premise deriving each count: a divergent session elects one initiator and one responder; the initiator-side proxy seeds exactly one under-root reply from the greeting (Work::initiator) and the responder-side proxy publishes exactly one under-root question after its opening wire reply (encode::opening), and nothing else records at that height, so the counts are exactly one each and the wire-reply floor is at least one. No observed value enters.

<a id="hunk-77"></a>
### src/tree/mirror/streaming/remote/proxy/work/progress/trace/tests.rs `@@ -65,6 +65,33 @@ fn rejects_next_decoded_reply_before_scopes() {`

```diff
@@ -65,6 +65,33 @@ fn rejects_next_decoded_reply_before_scopes() {
     trace.assert_valid();
 }
 
+/// An empty trace, which both ordering assertions accept, fails the
+/// divergent-session floor by name.
+#[test]
+#[should_panic(expected = "exactly one greeting-seeded opening reply, found 0")]
+fn empty_trace_fails_the_divergent_session_floor() {
+    let (_, trace) = with_trace(|| ());
+    trace.assert_valid();
+    trace.assert_registration_causality();
+    trace.assert_covers_divergent_session();
+}
+
+/// The least a divergent session records meets the floor: the responder
+/// proxy's opening wire reply and question, and the initiator proxy's
+/// greeting-seeded opening reply.
+#[test]
+fn minimal_divergent_session_meets_the_floor() {
+    let (_, trace) = with_trace(|| {
+        record(0, Kind::WireReply { questions: 1 }, UnderRoot::HEIGHT);
+        record(0, Kind::LocalQuestion, UnderRoot::HEIGHT);
+        record(1, Kind::DecodedReply { scopes: 1 }, UnderRoot::HEIGHT);
+        record(1, Kind::NextScope, UnderRoot::HEIGHT);
+    });
+    trace.assert_valid();
+    trace.assert_registration_causality();
+    trace.assert_covers_divergent_session();
+}
+
 /// A decode following its flushed question satisfies registration causality.
 #[test]
 fn accepts_decode_after_flushed_question() {
```

<!-- annotation -->
> **remote-proxy-19** (T26), line 70:
>
> Negative control: the entry's construction (with_trace(|| ()) passes both ordering assertions) fails the floor by name. The second test pins the least trace that meets it, so the floor cannot drift above the mechanism's minimum.

<a id="hunk-78"></a>
### src/tree/mirror/streaming/tests.rs `@@ -4,24 +4,32 @@`

```diff
@@ -4,24 +4,32 @@
 //! lifecycle checks in [`faults`], and deterministic tree builders in
 //! [`fixtures`].
 
-use std::{convert::Infallible, future};
+use std::{
+    cell::Cell,
+    convert::Infallible,
+    future::{self, Future},
+    pin::pin,
+    task::{Context, Poll, Waker},
+};
 
 use proptest::prelude::*;
 
 use super::driver::try_join_mapped;
-use crate::testing::run_to_quiescence;
+use crate::testing::{Quiescence, node_census, node_census_reset, run_to_quiescence};
 use crate::tree::arb::{
     arb_divergent_pair, arb_tree_root, leaf_parent_dispute_pair, leaf_parent_redaction_pair,
     uncontained_supply_pair,
 };
 use crate::tree::mirror::streaming::backend::with_local_schedule;
-use crate::tree::mirror::streaming::materialized::channel::with_schedule;
+use crate::tree::mirror::streaming::materialized::channel::{
+    QueueKind, with_kind_capacity, with_schedule,
+};
 use crate::tree::mirror::streaming::materialized::progress::{Trace, with_trace};
 use crate::tree::mirror::streaming::materialized::transcript::{Transcript, with_transcript};
-use crate::tree::mirror::streaming::window::WindowConfig;
-use crate::tree::mirror::streaming::{
-    Local, Root as StreamingRoot, materialized::Handshaking, mirror as drive_streaming,
-};
+use crate::tree::mirror::streaming::materialized::{Error as MaterializedError, Start};
+use crate::tree::mirror::streaming::stats::{Recorder, SessionStats};
+use crate::tree::mirror::streaming::window::{DEFAULT_SYNC_MEMORY_BUDGET, Window, WindowConfig};
+use crate::tree::mirror::streaming::{Local, materialized::Handshaking, mirror as drive_streaming};
 use crate::tree::{Root, Tree, mirror::Error as MirrorError};
 
 mod announced;
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 10:
>
> Imports follow the builder: Quiescence and node_census from testing, the channel instruments, Start for floor_start's return type, the window types for the wide arm. The rewrite of this file's helper ladder is wholesale, so every hunk below is one region of it.

<a id="hunk-79"></a>
### src/tree/mirror/streaming/tests.rs `@@ -59,60 +67,223 @@ fn terminal_errors_preempt_parked_peers() {`

```diff
@@ -59,60 +67,223 @@ fn terminal_errors_preempt_parked_peers() {
     ));
 }
 
-/// Reconcile `a` and `b` through the streaming local backend, returning both
-/// sides' reconciled roots in argument order, with no convergence assertion.
-fn streaming_mirror_sides(a: Root, b: Root) -> (Root, Root) {
-    streaming_mirror_sides_with_schedule(a, b, Vec::new())
-}
+/// The failure of a session between two `Local` endpoints: the backend is
+/// infallible, so only protocol violations are inhabited.
+type LocalSessionError = MirrorError<MaterializedError<Infallible>, MaterializedError<Infallible>>;
 
-/// Reconcile under an explicit, shrinkable channel-poll schedule.
-fn streaming_mirror_sides_with_schedule(a: Root, b: Root, schedule: Vec<u8>) -> (Root, Root) {
-    streaming_mirror_sides_with_schedules(a, b, schedule, Vec::new())
-}
+/// How a local session ended: both reconciled roots in argument order, a
+/// violation, or quiescence before either.
+type Verdict = Result<Result<(Root, Root), LocalSessionError>, Quiescence>;
 
-/// Reconcile under independent channel and Local-backend poll schedules.
-fn streaming_mirror_sides_with_schedules(
-    a: Root,
-    b: Root,
+/// One session between two `Local` endpoints under closed-world polling,
+/// with the instruments a test asks for attached.
+///
+/// The floor window is the default; every schedule, capacity, and
+/// instrument setting is optional and independent of the others.
+struct LocalSession {
+    client: Root,
+    server: Root,
+    window: WindowConfig,
     channel_schedule: Vec<u8>,
     backend_schedule: Vec<u8>,
-) -> (Root, Root) {
-    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
-    let client = Handshaking::start(Local, a.clone()).window(WindowConfig::FLOOR);
-    let server = Handshaking::start(Local, b.clone()).window(WindowConfig::FLOOR);
-    let (result, trace) = with_trace(|| {
-        with_schedule(channel_schedule, || {
-            with_local_schedule(backend_schedule, || {
-                run_to_quiescence(drive_streaming(client, server))
+    kind_capacity: Option<(QueueKind, usize)>,
+    stats: bool,
+    trace: bool,
+    transcript: bool,
+}
+
+impl LocalSession {
+    /// A session in which `client` connects to `server`, at the floor
+    /// window, with no instrument attached.
+    fn new(client: Root, server: Root) -> Self {
+        Self {
+            client,
+            server,
+            window: WindowConfig::FLOOR,
+            channel_schedule: Vec::new(),
+            backend_schedule: Vec::new(),
+            kind_capacity: None,
+            stats: false,
+            trace: false,
+            transcript: false,
+        }
+    }
+
+    /// Run at `window` instead of the floor.
+    fn window(mut self, window: WindowConfig) -> Self {
+        self.window = window;
+        self
+    }
+
+    /// Delay channel polls by `schedule`, one entry per poll boundary.
+    fn channel_schedule(mut self, schedule: Vec<u8>) -> Self {
+        self.channel_schedule = schedule;
+        self
+    }
+
+    /// Delay `Local` backend polls by `schedule`, one entry per poll
+    /// boundary.
+    fn backend_schedule(mut self, schedule: Vec<u8>) -> Self {
+        self.backend_schedule = schedule;
+        self
+    }
+
+    /// Cap every height of one queue kind at `limit` slots.
+    fn kind_capacity(mut self, kind: QueueKind, limit: usize) -> Self {
+        self.kind_capacity = Some((kind, limit));
+        self
+    }
+
+    /// Record both sides' session stats.
+    fn stats(mut self) -> Self {
+        self.stats = true;
+        self
+    }
+
+    /// Record the materialized publication trace.
+    fn trace(mut self) -> Self {
+        self.trace = true;
+        self
+    }
+
+    /// Record the payload-erased wire transcript.
+    fn transcript(mut self) -> Self {
+        self.transcript = true;
+        self
+    }
+
+    /// Drive the session until it completes or becomes quiescent.
+    fn run(self) -> Outcome {
+        let Self {
+            client,
+            server,
+            window,
+            channel_schedule,
+            backend_schedule,
+            kind_capacity,
+            stats,
+            trace,
+            transcript,
+        } = self;
+        let recorders = stats.then(|| (Recorder::default(), Recorder::default()));
+        let mut client = Handshaking::start(Local, client.into()).window(window);
+        let mut server = Handshaking::start(Local, server.into()).window(window);
+        if let Some((client_recorder, server_recorder)) = &recorders {
+            client = client.stats(client_recorder.clone());
+            server = server.stats(server_recorder.clone());
+        }
+
+        let run = move || run_to_quiescence(drive_streaming(client, server));
+        let run = move || {
+            with_schedule(channel_schedule, move || {
+                with_local_schedule(backend_schedule, run)
             })
-        })
-    });
-    let (ours, theirs) = result
-        .expect("streaming mirror became quiescent before completion")
-        // `Local` is infallible, so the session's only inhabited errors are
-        // violations — which two honest local endpoints must never speak.
-        .expect("local mirror speaks no violations");
-    trace.assert_valid();
-    (ours.into(), theirs.into())
+        };
+        let run = move || match kind_capacity {
+            Some((kind, limit)) => with_kind_capacity(kind, limit, run),
+            None => run(),
+        };
+        let run = move || {
+            if trace {
+                let (verdict, trace) = with_trace(run);
+                (verdict, Some(trace))
+            } else {
+                (run(), None)
+            }
+        };
+        let ((verdict, trace), transcript) = if transcript {
+            let (traced, transcript) = with_transcript(run);
+            (traced, Some(transcript))
+        } else {
+            (run(), None)
+        };
+
+        Outcome {
+            verdict: verdict
+                .map(|session| session.map(|(ours, theirs)| (ours.into(), theirs.into()))),
+            stats: recorders.map(|(client, server)| (client.snapshot(), server.snapshot())),
+            trace,
+            transcript,
+        }
+    }
+}
+
+/// What a [`LocalSession`] run produced: its verdict and the instruments it
+/// was asked to attach.
+struct Outcome {
+    verdict: Verdict,
+    stats: Option<(SessionStats, SessionStats)>,
+    trace: Option<Trace>,
+    transcript: Option<Transcript>,
+}
+
+impl Outcome {
+    /// Both reconciled roots in argument order; a violation, a stall, and
+    /// an exhausted poll budget each fail by name.
+    fn sides(self) -> (Root, Root) {
+        match self.verdict {
+            Ok(Ok(sides)) => sides,
+            Ok(Err(error)) => panic!("local mirror speaks no violations: {error:?}"),
+            Err(Quiescence::Stalled) => panic!("streaming mirror stalled before completion"),
+            Err(Quiescence::PollBudget) => {
+                panic!("streaming mirror exhausted its poll budget before completion")
+            }
+        }
+    }
+
+    /// The one root both sides converged to.
+    fn converged(self) -> Root {
+        let (ours, theirs) = self.sides();
+        assert_eq!(ours, theirs, "streaming endpoints should converge");
+        ours
+    }
+
+    /// Both sides' stats in argument order, which the session was asked
+    /// to record.
+    fn stats(&self) -> &(SessionStats, SessionStats) {
+        self.stats
+            .as_ref()
+            .expect("the session was asked to record its stats")
+    }
+
+    /// The publication trace, which the session was asked to record.
+    fn trace(&self) -> &Trace {
+        self.trace
+            .as_ref()
+            .expect("the session was asked to record its trace")
+    }
+}
+
+/// A `Local` endpoint at the floor window, for the sites that hold the
+/// undriven session: the ones that wrap it in a fault or failure decorator,
+/// and the ones that poll it themselves.
+fn floor_start(root: Root) -> Handshaking<Local, Start> {
+    Handshaking::start(Local, root.into()).window(WindowConfig::FLOOR)
+}
+
+/// Reconcile `a` and `b` through the streaming local backend, returning both
+/// sides' reconciled roots in argument order, with no convergence assertion.
+fn streaming_mirror_sides(a: Root, b: Root) -> (Root, Root) {
+    let outcome = LocalSession::new(a, b).trace().run();
+    outcome.trace().assert_valid();
+    outcome.sides()
 }
 
 /// Reconcile through the local backend, returning both roots, the validated
-/// progress trace, and the payload-erased wire transcript.
-///
-/// The skeleton-bridge harness ([`skeleton`]): generic over the payload type
-/// so payload-perturbation twins (same paths, different contents) can run
-/// through the identical machinery.
+/// publication trace, and the payload-erased wire transcript.
 fn transcribed_mirror_sides(a: Root, b: Root) -> (Root, Root, Trace, Transcript) {
-    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
-    let client = Handshaking::start(Local, a).window(WindowConfig::FLOOR);
-    let server = Handshaking::start(Local, b).window(WindowConfig::FLOOR);
-    let ((result, trace), transcript) =
-        with_transcript(|| with_trace(|| run_to_quiescence(drive_streaming(client, server))));
-    let (ours, theirs) = result
-        .expect("streaming mirror became quiescent before completion")
-        .expect("local mirror speaks no violations");
+    let mut outcome = LocalSession::new(a, b).trace().transcript().run();
+    let trace = outcome
+        .trace
+        .take()
+        .expect("the session was asked to record its trace");
+    let transcript = outcome
+        .transcript
+        .take()
+        .expect("the session was asked to record its transcript");
     trace.assert_valid();
-    (ours.into(), theirs.into(), trace, transcript)
+    let (ours, theirs) = outcome.sides();
+    (ours, theirs, trace, transcript)
 }
 
 /// Reconcile `a` and `b` through the streaming local backend, asserting the
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 80:
>
> The entry claimed the two-Local session was spelled nine times behind a six-function ladder. LocalSession is the one construction: schedules, a per-kind capacity, a window, and the three instruments are independent settings, and run nests the with_* closures only for the settings asked for. Verdict keeps completion, violation, and quiescence apart; the same type resolves streaming-tests-11.

<!-- annotation -->
> **streaming-tests-3** (T127), line 200:
>
> Outcome owns the two expects (sides, converged) and names a stall and a poll-budget overrun separately; the instrument accessors panic when the session was not asked to record them, so a test cannot read an instrument it never attached. Fields stay plain so transcribed_mirror_sides can take the trace and transcript out by value.

<!-- annotation -->
> **streaming-tests-3** (T127), line 250:
>
> floor_start is the one Local start outside the builder, for the sites that hold the undriven session: the fault- and failure-wrapped endpoints and the cancellation pin's own polling (fresh-eyes repair: the doc names that class). streaming_mirror_sides, streaming_mirror, and fully_scheduled_streaming_mirror keep their signatures and validate the trace as before; streaming_mirror_sides_with_schedule and scheduled_streaming_mirror are gone. transcribed_mirror_sides's doc lost its false second paragraph (it was never generic over the payload type); streaming-tests-5's other site at converges_on_leaf_parent_dispute is untouched and reported as a finding.

<a id="hunk-80"></a>
### src/tree/mirror/streaming/tests.rs `@@ -123,30 +294,21 @@ fn streaming_mirror(a: Root, b: Root) -> Root {`

```diff
@@ -123,30 +294,21 @@ fn streaming_mirror(a: Root, b: Root) -> Root {
     ours
 }
 
-/// Reconcile under an explicit channel-poll schedule, asserting convergence.
-fn scheduled_streaming_mirror(a: Root, b: Root, schedule: Vec<u8>) -> Root {
-    let (ours, theirs) = streaming_mirror_sides_with_schedule(a, b, schedule);
-    assert_eq!(
-        ours, theirs,
-        "scheduled streaming endpoints should converge"
-    );
-    ours
-}
-
-/// Reconcile under independent channel and Local-backend poll schedules.
+/// Reconcile under independent channel and Local-backend poll schedules,
+/// with the publication trace validated, asserting convergence.
 fn fully_scheduled_streaming_mirror(
     a: Root,
     b: Root,
     channel_schedule: Vec<u8>,
     backend_schedule: Vec<u8>,
 ) -> Root {
-    let (ours, theirs) =
-        streaming_mirror_sides_with_schedules(a, b, channel_schedule, backend_schedule);
-    assert_eq!(
-        ours, theirs,
-        "fully scheduled streaming endpoints should converge"
-    );
-    ours
+    let outcome = LocalSession::new(a, b)
+        .channel_schedule(channel_schedule)
+        .backend_schedule(backend_schedule)
+        .trace()
+        .run();
+    outcome.trace().assert_valid();
+    outcome.converged()
 }
 
 /// Merge `a` and `b` through `Tree::join`: the in-memory oracle.
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 300:
>
> fully_scheduled_streaming_mirror stays as the scheduled convergence wrapper (three callers); the builder makes it three lines.

<a id="hunk-81"></a>
### src/tree/mirror/streaming/tests.rs `@@ -173,6 +335,125 @@ fn arb_oracle_pair() -> impl Strategy<Value = (Root, Root)> {`

```diff
@@ -173,6 +335,125 @@ fn arb_oracle_pair() -> impl Strategy<Value = (Root, Root)> {
     ]
 }
 
+/// The corpus size the wide window is solved for: far past what the
+/// generators produce, so the top stages get capacities above the floor.
+const WIDE_CORPUS: u64 = 1_000_000;
+
+/// The window the wide oracle arm runs under and its widest capacity: the
+/// budget solve at [`WIDE_CORPUS`], asserted wider than the floor rather
+/// than assumed so.
+fn wide_window() -> (WindowConfig, u64) {
+    let window = Window::from_budget(
+        WIDE_CORPUS,
+        WIDE_CORPUS,
+        0,
+        0,
+        DEFAULT_SYNC_MEMORY_BUDGET,
+        Local::node_bytes,
+    );
+    let widest = window.widest();
+    assert!(
+        widest > 1,
+        "the wide arm's window solved to the floor: it exercises nothing the floor arm does not"
+    );
+    (WindowConfig::Fixed(window), widest)
+}
+
+/// Poll `future` at most `polls` times, then drop it: a session cancelled
+/// at a drawn point. Returns the output when it completed within the
+/// budget.
+fn cancel_after<F: Future>(future: F, polls: usize) -> Option<F::Output> {
+    let mut future = pin!(tokio::task::coop::unconstrained(future));
+    let mut cx = Context::from_waker(Waker::noop());
+    for _ in 0..polls {
+        if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
+            return Some(output);
+        }
+    }
+    None
+}
+
+/// The polls a session between `a` and `b` takes to complete under the
+/// same polling `cancel_after` applies: the length every drawn
+/// cancellation point stays below.
+fn session_length(a: Root, b: Root) -> usize {
+    const MAX_POLLS: usize = 1_000_000;
+    let mut session = pin!(tokio::task::coop::unconstrained(drive_streaming(
+        floor_start(a),
+        floor_start(b),
+    )));
+    let mut cx = Context::from_waker(Waker::noop());
+    (1..=MAX_POLLS)
+        .find(|_| session.as_mut().poll(&mut cx).is_ready())
+        .expect("a local session completes within the poll budget")
+}
+
+/// A generated pair with its measured session length and a cancellation
+/// point drawn below it, so every case with a session longer than one
+/// poll cancels mid-session.
+fn arb_cancellation() -> impl Strategy<Value = ((Root, Root), usize, usize)> {
+    arb_oracle_pair().prop_flat_map(|(a, b)| {
+        let length = session_length(a.clone(), b.clone());
+        (Just((a, b)), Just(length), 1..length.max(2))
+    })
+}
+
+/// Cancelling a session at a drawn poll before its measured completion
+/// leaves nothing behind.
+///
+/// Every node handle the session built is released (the census returns
+/// to its baseline), and a fresh session over the same inputs reaches the
+/// join oracle.
+/// The poll count is drawn in `1..length`, `length` being the session's
+/// measured poll count, so every case with a session longer than one poll
+/// cancels.
+///
+/// The run asserts that some case cancelled after the walk had built node
+/// handles, which is where a cancellation could leak.
+#[test]
+fn cancelled_session_leaves_no_residue() {
+    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
+        source_file: Some(file!()),
+        ..ProptestConfig::default()
+    });
+    let cancelled_after_building = Cell::new(0usize);
+    let cases = runner.run(&arb_cancellation(), |((a, b), length, polls)| {
+        let expected = join_oracle(a.clone(), b.clone());
+        let baseline = node_census().live;
+        let session = drive_streaming(floor_start(a.clone()), floor_start(b.clone()));
+        node_census_reset();
+        let before_polling = node_census().live;
+        let completed = cancel_after(session, polls);
+        let cancelled = completed.is_none();
+        drop(completed);
+        let census = node_census();
+        prop_assert_eq!(
+            cancelled,
+            polls < length,
+            "the measured session length is not reproducible"
+        );
+        prop_assert_eq!(
+            census.live,
+            baseline,
+            "a cancelled session must release every node handle it built"
+        );
+        if cancelled && census.peak > before_polling {
+            cancelled_after_building.set(cancelled_after_building.get() + 1);
+        }
+        let (ours, theirs) = streaming_mirror_sides(a, b);
+        prop_assert_eq!(&ours, &expected);
+        prop_assert_eq!(&theirs, &expected);
+        Ok(())
+    });
+    if let Err(failure) = cases {
+        panic!("{failure}\n{runner}");
+    }
+    assert!(
+        cancelled_after_building.get() > 0,
+        "no case cancelled a session after it had built node handles: the pin exercised nothing"
+    );
+}
+
 /// A dispute that survives to leaf-parent height — both sides hold the same
 /// `S<Z>` prefix with different leaf sets — converges to the union.
 ///
```

<!-- annotation -->
> **Decision 56** (T19), line 340:
>
> wide_window solves the budget at a million-message corpus and asserts widest() > 1: the width is asserted, not assumed, at construction, and the session's granted width is checked again from its stats. DEFAULT_SYNC_MEMORY_BUDGET rather than usize::MAX so the arm runs a real budget. Premise for the corpus size: the generators never exceed 16 leaves, so any corpus far above them widens the top stages.

<!-- annotation -->
> **Decision 56** (T19), line 360:
>
> cancel_after polls a fixed number of times under a noop waker and drops the future: the cancellation point is the drawn poll count. Kept private to the streaming suites rather than added to crate::testing, which is public surface.

<!-- annotation -->
> **Decision 56** (T19), line 420:
>
> Fresh-eyes repair (round 1, item 1). The pin draws its poll count strictly below the session's measured length (arb_cancellation), checks the measurement reproduces per case, tallies cases cancelled after the walk raised the census high-water mark, and asserts at least one over the run; the two live clauses (census back to baseline; retry at the join oracle) are the doc. The inputs-untouched clause is gone: nothing at this tier can falsify it. Negative control, cancellation-specific: a Drop impl on Resolver leaking one held child only when dropped with reactions owed fails the pin (left 49, right 48) while the oracle test stays green (commit message).

<!-- annotation -->
> **Decision 56** (T19), line 390:
>
> Fresh-eyes repair: session_length measures a pair's poll count under the same polling cancel_after applies, and arb_cancellation draws the cancellation point strictly below it, so every case with a session longer than one poll cancels by construction rather than by luck.

<!-- annotation -->
> **Decision 56** (T19), line 392:
>
> Fresh-eyes repair, round 2: arb_cancellation's doc uses the accurate form (a length-1 session completes; the pin asserts that), and the pin doc keeps the clauses, the drawn range, and the run assertion in separate paragraphs.

<a id="hunk-82"></a>
### src/tree/mirror/streaming/tests.rs `@@ -232,11 +513,9 @@ fn uncontained_supply_is_rejected_by_streaming() {`

```diff
@@ -232,11 +513,9 @@ fn uncontained_supply_is_rejected_by_streaming() {
     use crate::tree::mirror::streaming::materialized::{Error, Violation};
 
     let (receiver, poisoned, _, _) = uncontained_supply_pair();
-    let (receiver, poisoned): (StreamingRoot<Local>, StreamingRoot<Local>) =
-        (receiver.into(), poisoned.into());
-    let client = Handshaking::start(Local, receiver).window(WindowConfig::FLOOR);
-    let server = Handshaking::start(Local, poisoned).window(WindowConfig::FLOOR);
-    let result = run_to_quiescence(drive_streaming(client, server))
+    let result = LocalSession::new(receiver, poisoned)
+        .run()
+        .verdict
         .expect("the rejecting session becomes quiescent");
     assert!(
         matches!(
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 515:
>
> uncontained_supply_is_rejected_by_streaming's inline construction goes through the builder; the verdict's outer expect still names a stall as the failure.

<a id="hunk-83"></a>
### src/tree/mirror/streaming/tests.rs `@@ -250,20 +529,37 @@ fn uncontained_supply_is_rejected_by_streaming() {`

```diff
@@ -250,20 +529,37 @@ fn uncontained_supply_is_rejected_by_streaming() {
 }
 
 proptest! {
-    /// Streaming and the in-memory join oracle agree in both orientations.
+    /// Streaming and the in-memory join oracle agree in both orientations,
+    /// at the floor window and at a wide one.
     ///
     /// Across every generated causal relationship, both wire endpoints
     /// converge to exactly `Tree::join` of the two inputs. This ties the
     /// wire reconciliation to the in-memory merge the convergence suites
-    /// are stated over: join and mirror delegate deletion honoring to the
-    /// same filter, so any divergence here is a protocol bug.
+    /// are stated over: join prunes through `traverse::unknown` and the
+    /// session through `materialized::unknown`, two implementations of one
+    /// deletion-honoring contract, so a divergence here is a bug in one of
+    /// them. The wide arm runs the pipeline window's real behavior, and
+    /// each session reports the width it was granted so the arm cannot
+    /// silently run at the floor.
     #[test]
-    fn streaming_matches_join_oracle((a, b) in arb_oracle_pair()) {
+    fn streaming_matches_join_oracle((a, b) in arb_oracle_pair(), wide in any::<bool>()) {
         let expected = join_oracle(a.clone(), b.clone());
+        let (window, width) = if wide { wide_window() } else { (WindowConfig::FLOOR, 1) };
+        // A window is derived only once the greetings disagree; an equal
+        // pair ends at the greeting and grants nothing.
+        let derives_window = a.ceiling != b.ceiling;
         for (left, right) in [(a.clone(), b.clone()), (b, a)] {
-            let actual = streaming_mirror_sides(left, right);
-            prop_assert_eq!(&actual.0, &expected);
-            prop_assert_eq!(&actual.1, &expected);
+            let outcome = LocalSession::new(left, right).window(window).trace().stats().run();
+            outcome.trace().assert_valid();
+            let (ours_stats, theirs_stats) = *outcome.stats();
+            let (ours, theirs) = outcome.sides();
+            prop_assert_eq!(&ours, &expected);
+            prop_assert_eq!(&theirs, &expected);
+            if derives_window {
+                prop_assert_eq!(ours_stats.window_granted, width);
+                prop_assert_eq!(theirs_stats.window_granted, width);
+            }
         }
     }
+
 }
```

<!-- annotation -->
> **Decision 56** (T19), line 540:
>
> The wide arm: a drawn bool picks floor or wide, and when the greetings differ (the only case a window is derived) both sides' window_granted must equal the chosen width. Negative control: handing the wide arm WindowConfig::FLOOR fails with left 1, right 48588 (the commit message quotes it). Fresh-eyes repair: the doc now says why the differential has value (traverse::unknown and materialized::unknown are two implementations of one contract) and that a divergence is a bug in one of them.

<a id="hunk-84"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -5,34 +5,74 @@ use proptest::prelude::*;`

```diff
@@ -5,34 +5,74 @@ use proptest::prelude::*;
 use super::fixtures::{
     LeafOrder, divergent_cells_pair, full_depth_comb_pair, one_sided_pair, pyramid_pair,
 };
-use super::{fully_scheduled_streaming_mirror, join_oracle, scheduled_streaming_mirror};
+use super::{LocalSession, Verdict, floor_start, fully_scheduled_streaming_mirror, join_oracle};
 use crate::testing::{Quiescence, run_to_quiescence};
-use crate::tree::mirror::streaming::window::WindowConfig;
 use crate::tree::{
     Root,
     arb::leaf_parent_dispute_pair,
     mirror::streaming::{
-        Local, Root as StreamingRoot,
-        backend::with_local_schedule,
+        Fault, Faulting,
         materialized::{
-            Handshaking,
-            channel::{QueueKind, with_kind_capacity, with_observation, with_schedule},
+            Violation,
+            channel::{QueueKind, with_kind_capacity, with_observation},
         },
         mirror as drive_streaming,
     },
 };
 
-/// Whether the session stalls at a selected capacity for the fan return queue.
-fn underbuffered_mirror_stalls(a: Root, b: Root, capacity: usize) -> bool {
-    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
-    let client = Handshaking::start(Local, a).window(WindowConfig::FLOOR);
-    let server = Handshaking::start(Local, b).window(WindowConfig::FLOOR);
-    with_kind_capacity(QueueKind::AssemblyLevelReturns, capacity, || {
-        matches!(
-            run_to_quiescence(drive_streaming(client, server)),
-            Err(Quiescence::Stalled)
-        )
-    })
+/// How one probed session ended: the two outcomes the capacity laws are
+/// stated over.
+#[derive(Debug, PartialEq, Eq)]
+enum Probe {
+    /// Both sides completed holding the join oracle's tree.
+    Completed,
+    /// The session parked with no wake arranged.
+    Stalled,
+}
+
+/// Classify a probed session's verdict.
+///
+/// A completion is held to the join oracle. A violation and an exhausted
+/// poll budget are neither outcome the laws speak of, so each fails by
+/// name instead of being read as "did not stall".
+fn classify(verdict: Verdict, expected: &Root) -> Probe {
+    match verdict {
+        Ok(Ok((ours, theirs))) => {
+            assert_eq!(
+                &ours, expected,
+                "a completed probe's initiating side must hold the join oracle's tree"
+            );
+            assert_eq!(
+                &theirs, expected,
+                "a completed probe's responding side must hold the join oracle's tree"
+            );
+            Probe::Completed
+        }
+        Ok(Err(error)) => panic!(
+            "the probed session died with a violation, neither completing nor stalling: {error:?}"
+        ),
+        Err(Quiescence::Stalled) => Probe::Stalled,
+        Err(Quiescence::PollBudget) => {
+            panic!("the probed session exhausted its poll budget, neither completing nor stalling")
+        }
+    }
+}
+
+/// Probe one shape at a capacity for the fan return queue under explicit
+/// channel and Local-backend poll schedules.
+fn probe(
+    pair: &(Root, Root),
+    capacity: usize,
+    channel_schedule: Vec<u8>,
+    backend_schedule: Vec<u8>,
+) -> Probe {
+    let expected = join_oracle(pair.0.clone(), pair.1.clone());
+    let outcome = LocalSession::new(pair.0.clone(), pair.1.clone())
+        .kind_capacity(QueueKind::AssemblyLevelReturns, capacity)
+        .channel_schedule(channel_schedule)
+        .backend_schedule(backend_schedule)
+        .run();
+    classify(outcome.verdict, &expected)
 }
 
 /// Check one structural stress case under endpoint and poll-order variations.
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 10:
>
> Imports: the builder, Verdict, floor_start; Fault, Faulting, Violation for the committed negative control; the direct Handshaking, StreamingRoot, and schedule imports leave with the hand-rolled probes.

<!-- annotation -->
> **streaming-tests-11** (T26), line 30:
>
> The entry claimed the boolean probes read a violation or a poll-budget overrun as "did not stall". Probe has the two outcomes the capacity laws speak of; classify holds a completion to the join oracle on both sides and panics by name on the other two. I chose an enum over the resolution's stalls/completes pair so one classifier owns every message and the any/all sites read as predicates over it.

<!-- annotation -->
> **streaming-tests-11** (T26), line 60:
>
> probe replaces both shape_stalls and underbuffered_mirror_stalls (they differed only in schedules); it computes the oracle itself so every completion is compared.

<a id="hunk-85"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -115,9 +155,9 @@ fn capacity_stress_matrix() {`

```diff
@@ -115,9 +155,9 @@ fn capacity_stress_matrix() {
 fn capacity_stress_covers_every_queue_role() {
     let (pair, report) = with_observation(|| {
         let (a, b) = pyramid_pair(&[4, 4, 2], 2, LeafOrder::Interleaved);
-        let pair = scheduled_streaming_mirror(a, b, vec![2; 16_384]);
+        let pair = fully_scheduled_streaming_mirror(a, b, vec![2; 16_384], Vec::new());
         let (a, b, _) = leaf_parent_dispute_pair();
-        scheduled_streaming_mirror(a, b, vec![2; 16_384]);
+        fully_scheduled_streaming_mirror(a, b, vec![2; 16_384], Vec::new());
         pair
     });
     drop(pair);
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 158:
>
> The witness's two probes become assert_eq against Probe::Stalled and Probe::Completed; the negative site now compares roots to the oracle.

<a id="hunk-86"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -173,8 +213,9 @@ fn capacity_stress_covers_every_queue_role() {`

```diff
@@ -173,8 +213,9 @@ fn capacity_stress_covers_every_queue_role() {
 fn capacity_stress_witness_requires_inter_level_fan() {
     let (a, b) = pyramid_pair(&[32, 256], 1, LeafOrder::Reversed);
     let expected = join_oracle(a.clone(), b.clone());
-    let (actual, report) =
-        with_observation(|| scheduled_streaming_mirror(a.clone(), b.clone(), vec![2; 16_384]));
+    let (actual, report) = with_observation(|| {
+        fully_scheduled_streaming_mirror(a.clone(), b.clone(), vec![2; 16_384], Vec::new())
+    });
     assert_eq!(
         actual, expected,
         "the full-fan witness must complete at the documented capacities",
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 216:
>
> stalls_under_any_schedule is any(Stalled) over the probe.

<a id="hunk-87"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -184,39 +225,19 @@ fn capacity_stress_witness_requires_inter_level_fan() {`

```diff
@@ -184,39 +225,19 @@ fn capacity_stress_witness_requires_inter_level_fan() {
         "the witness did not create its expected near-fan return backlog: {:?}",
         report.kind(QueueKind::AssemblyLevelReturns),
     );
-    assert!(
-        underbuffered_mirror_stalls(a.clone(), b.clone(), 253),
+    let pair = (a, b);
+    assert_eq!(
+        probe(&pair, 253, Vec::new(), Vec::new()),
+        Probe::Stalled,
         "the stress witness must stall just below its required return capacity",
     );
-    assert!(
-        !underbuffered_mirror_stalls(a, b, 254),
+    assert_eq!(
+        probe(&pair, 254, Vec::new(), Vec::new()),
+        Probe::Completed,
         "the stress witness should complete once its near-fan return backlog fits",
     );
 }
 
-/// Whether one shape stalls at a return capacity under explicit poll schedules.
-fn shape_stalls(
-    pair: &(Root, Root),
-    capacity: usize,
-    channel_schedule: Vec<u8>,
-    backend_schedule: Vec<u8>,
-) -> bool {
-    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) =
-        (pair.0.clone().into(), pair.1.clone().into());
-    let client = Handshaking::start(Local, a).window(WindowConfig::FLOOR);
-    let server = Handshaking::start(Local, b).window(WindowConfig::FLOOR);
-    with_kind_capacity(QueueKind::AssemblyLevelReturns, capacity, || {
-        with_schedule(channel_schedule, || {
-            with_local_schedule(backend_schedule, || {
-                matches!(
-                    run_to_quiescence(drive_streaming(client, server)),
-                    Err(Quiescence::Stalled)
-                )
-            })
-        })
-    })
-}
-
 /// The poll-order variations each parent-delay probe shape runs under.
 fn probe_schedules() -> [(Vec<u8>, Vec<u8>); 5] {
     [
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 230:
>
> completes_under_every_schedule is all(Completed); internal_fan is hoisted from a closure so the negative control can build the same shape.

<a id="hunk-88"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -238,7 +259,46 @@ fn probe_schedules() -> [(Vec<u8>, Vec<u8>); 5] {`

```diff
@@ -238,7 +259,46 @@ fn probe_schedules() -> [(Vec<u8>, Vec<u8>); 5] {
 fn stalls_under_any_schedule(pair: &(Root, Root), capacity: usize) -> bool {
     probe_schedules()
         .into_iter()
-        .any(|(chan, back)| shape_stalls(pair, capacity, chan, back))
+        .any(|(chan, back)| probe(pair, capacity, chan, back) == Probe::Stalled)
+}
+
+/// Whether a shape completes, holding the join oracle's tree, under every
+/// probe schedule at the given capacity.
+fn completes_under_every_schedule(pair: &(Root, Root), capacity: usize) -> bool {
+    probe_schedules()
+        .into_iter()
+        .all(|(chan, back)| probe(pair, capacity, chan, back) == Probe::Completed)
+}
+
+/// An internal scope with `n` disputed children, each carrying leaf-level
+/// grandchildren, so dependent queries follow the final child resolution
+/// while the enclosing parent resolution waits in the scope epilogue.
+fn internal_fan(n: u8) -> (Root, Root) {
+    let cells: Vec<Vec<u8>> = (0..n).map(|radix| vec![0, radix]).collect();
+    divergent_cells_pair(&cells, 1, LeafOrder::Outside)
+}
+
+/// A probed session that dies with a protocol violation is neither a
+/// completion nor a stall: the classifier fails by name rather than
+/// reading the violation as "did not stall".
+#[test]
+#[should_panic(expected = "died with a violation")]
+fn probe_classifier_rejects_a_violation() {
+    let (a, b) = internal_fan(3);
+    let expected = join_oracle(a.clone(), b.clone());
+    let client = floor_start(a);
+    let server = Faulting::new(
+        floor_start(b),
+        0,
+        Some(Fault::Reply(Violation::UnexpectedQuery)),
+    );
+    let verdict = with_kind_capacity(QueueKind::AssemblyLevelReturns, 1, || {
+        run_to_quiescence(drive_streaming(client, server))
+    });
+    classify(
+        verdict.map(|session| session.map(|(ours, theirs)| (ours.into(), theirs.into()))),
+        &expected,
+    );
 }
 
 /// Probe (model finding #7): a lone parent scope stalls the real encoder
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 270:
>
> Negative control: the witness construction (server wrapped in Faulting(UnexpectedQuery) at capacity 1 on internal_fan(3)) fed to the classifier, should_panic on "died with a violation". The acceptance's other form, the fault injected into every probed server, was run as a reversible mutation: parent_delay_single_parent_boundary fails with that message (commit message).

<a id="hunk-89"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -254,16 +314,8 @@ fn stalls_under_any_schedule(pair: &(Root, Root), capacity: usize) -> bool {`

```diff
@@ -254,16 +314,8 @@ fn stalls_under_any_schedule(pair: &(Root, Root), capacity: usize) -> bool {
 /// pinned here from both sides at cap 1.
 #[test]
 fn parent_delay_single_parent_boundary() {
-    // An internal scope with n disputed children, each carrying leaf-level
-    // grandchildren, so dependent queries follow the final child resolution
-    // while the enclosing parent resolution waits in the scope epilogue.
-    let internal_fan = |n: u8| {
-        let cells: Vec<Vec<u8>> = (0..n).map(|radix| vec![0, radix]).collect();
-        divergent_cells_pair(&cells, 1, LeafOrder::Outside)
-    };
-
     assert!(
-        !stalls_under_any_schedule(&internal_fan(3), 1),
+        completes_under_every_schedule(&internal_fan(3), 1),
         "fan = cap + 2 must complete: the model's tighter pdelay boundary \
          is not realizable in the sequential encoder"
     );
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 316:
>
> Every !stalls site is a completes_under_every_schedule site; the assertion messages keep their law statements.

<a id="hunk-90"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -272,7 +324,7 @@ fn parent_delay_single_parent_boundary() {`

```diff
@@ -272,7 +324,7 @@ fn parent_delay_single_parent_boundary() {
         "fan = cap + 3 must stall: the tightness law's boundary has moved"
     );
     assert!(
-        !stalls_under_any_schedule(&internal_fan(4), 2),
+        completes_under_every_schedule(&internal_fan(4), 2),
         "fan = cap + 2 must complete at the wider capacity too"
     );
 }
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 326:
>
> As above.

<a id="hunk-91"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -306,7 +358,7 @@ fn parent_delay_no_cross_parent_backlog() {`

```diff
@@ -306,7 +358,7 @@ fn parent_delay_no_cross_parent_backlog() {
          per-scope tightness law"
     );
     assert!(
-        !stalls_under_any_schedule(&parents_of_three(4), 2),
+        completes_under_every_schedule(&parents_of_three(4), 2),
         "a fan-4 root over fan-3 parents must complete at cap 2 per the \
          per-scope tightness law"
     );
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 360:
>
> As above.

<a id="hunk-92"></a>
### src/tree/mirror/streaming/tests/capacity.rs `@@ -321,7 +373,7 @@ fn parent_delay_no_cross_parent_backlog() {`

```diff
@@ -321,7 +373,7 @@ fn parent_delay_no_cross_parent_backlog() {
         let widths = vec![3usize; depth];
         let pair = pyramid_pair(&widths, 1, LeafOrder::Outside);
         assert!(
-            !stalls_under_any_schedule(&pair, 1),
+            completes_under_every_schedule(&pair, 1),
             "a width-{} stage of fan-3 scopes must complete at cap 1: \
              return backlog must not span sibling parents",
             3usize.pow(depth as u32),
```

<!-- annotation -->
> **streaming-tests-11** (T26), line 375:
>
> As above (the width sweep).

<a id="hunk-93"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -4,7 +4,7 @@ use proptest::prelude::*;`

```diff
@@ -4,7 +4,7 @@ use proptest::prelude::*;
 
 use super::{
     fixtures::{LeafOrder, full_depth_comb_pair, one_sided_pair},
-    streaming_mirror_sides,
+    floor_start, streaming_mirror_sides,
 };
 use crate::testing::run_to_quiescence;
 use crate::tree::arb::arb_divergent_pair;
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 6:
>
> floor_start imported from the parent.

<a id="hunk-94"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -14,18 +14,24 @@ use crate::tree::mirror::{`

```diff
@@ -14,18 +14,24 @@ use crate::tree::mirror::{
     streaming::{
         Failing, FailingNode, Failure, Fault, Faulting, GreetingLie, Local, Root as StreamingRoot,
         materialized::{
-            Error as MaterializedError, Handshaking, Violation,
+            Error as MaterializedError, Handshaking, Start, Violation,
             channel::{with_observation, with_schedule},
         },
         mirror as drive_streaming,
     },
 };
 
-fn failing_root(root: crate::tree::Root) -> StreamingRoot<Failing<Local>> {
-    StreamingRoot {
+/// A `Failing<Local>` endpoint at the floor window over `root`, its nodes
+/// wrapped for the failing backend.
+fn failing_start(
+    backend: Failing<Local>,
+    root: crate::tree::Root,
+) -> Handshaking<Failing<Local>, Start> {
+    let root = StreamingRoot {
         ceiling: root.ceiling,
         root: root.root.map(FailingNode::new),
-    }
+    };
+    Handshaking::start(backend, root).window(WindowConfig::FLOOR)
 }
 
 /// The connected abort suite's injected faults: one structural shape
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 20:
>
> failing_start folds failing_root into a floor-window Failing<Local> start; with floor_start it removes every line over 100 columns that the unformatted proptest bodies carried.

<a id="hunk-95"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -69,9 +75,10 @@ proptest! {`

```diff
@@ -69,9 +75,10 @@ proptest! {
         let (client_root, server_root) =
             full_depth_comb_pair(2, LeafOrder::Interleaved);
         let before = (client_root.clone(), server_root.clone());
-        let local = Handshaking::start(Local, StreamingRoot::from(client_root.clone())).window(WindowConfig::FLOOR);
-        let honest_server = Handshaking::start(Local, StreamingRoot::from(server_root.clone())).window(WindowConfig::FLOOR);
-        let faulting_server = Faulting::new(honest_server, server_steps, Some(Fault::Reply(violation)));
+        let local = floor_start(client_root.clone());
+        let honest_server = floor_start(server_root.clone());
+        let faulting_server =
+            Faulting::new(honest_server, server_steps, Some(Fault::Reply(violation)));
         let result = run_to_quiescence(drive_streaming(local, faulting_server))
             .expect("the connected driver must surface the fault, not stall");
         match result {
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 77:
>
> Fault-wrapped sites keep their own construction, through floor_start.

<a id="hunk-96"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -84,9 +91,10 @@ proptest! {`

```diff
@@ -84,9 +91,10 @@ proptest! {
 
         // Reversing the handshake sides also reverses initiator order: the
         // driver's frame-relative error is flipped back to the original client.
-        let honest_client = Handshaking::start(Local, StreamingRoot::from(client_root.clone())).window(WindowConfig::FLOOR);
-        let faulting_client = Faulting::new(honest_client, client_steps, Some(Fault::Reply(violation)));
-        let local = Handshaking::start(Local, StreamingRoot::from(server_root.clone())).window(WindowConfig::FLOOR);
+        let honest_client = floor_start(client_root.clone());
+        let faulting_client =
+            Faulting::new(honest_client, client_steps, Some(Fault::Reply(violation)));
+        let local = floor_start(server_root.clone());
         let result = run_to_quiescence(drive_streaming(faulting_client, local))
             .expect("the reversed connected driver must surface the fault, not stall");
         match result {
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 93:
>
> As above.

<a id="hunk-97"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -94,7 +102,10 @@ proptest! {`

```diff
@@ -94,7 +102,10 @@ proptest! {
                 prop_assert_eq!(actual, violation);
             }
             Err(other) => prop_assert!(false, "unexpected reversed driver error: {other:?}"),
-            Ok(_) => prop_assert!(false, "the reversed faulting counterparty unexpectedly completed"),
+            Ok(_) => prop_assert!(
+                false,
+                "the reversed faulting counterparty unexpectedly completed"
+            ),
         }
 
         prop_assert_eq!((client_root, server_root), before);
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 104:
>
> A prop_assert line wrapped by hand (rustfmt does not enter proptest! bodies); no change in meaning.

<a id="hunk-98"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -132,10 +143,8 @@ proptest! {`

```diff
@@ -132,10 +143,8 @@ proptest! {
             GreetingLie::InflatedSetLen | GreetingLie::InflatedVersion => None,
         };
 
-        let client = Handshaking::start(Local, StreamingRoot::from(client_root.clone()))
-            .window(WindowConfig::FLOOR);
-        let server = Handshaking::start(Local, StreamingRoot::from(server_root.clone()))
-            .window(WindowConfig::FLOOR);
+        let client = floor_start(client_root.clone());
+        let server = floor_start(server_root.clone());
         let result = if fault_client {
             let faulting = Faulting::new(client, 0, Some(Fault::Greeting(lie)));
             run_to_quiescence(drive_streaming(faulting, server))
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 145:
>
> As at 77.

<a id="hunk-99"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -208,8 +217,8 @@ proptest! {`

```diff
@@ -208,8 +217,8 @@ proptest! {
         } else {
             failing.clone()
         };
-        let client = Handshaking::start(client_backend, failing_root(client_root)).window(WindowConfig::FLOOR);
-        let server = Handshaking::start(server_backend, failing_root(server_root)).window(WindowConfig::FLOOR);
+        let client = failing_start(client_backend, client_root);
+        let server = failing_start(server_backend, server_root);
         let result = with_schedule(schedule, || {
             run_to_quiescence(drive_streaming(client, server))
         })
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 219:
>
> Failing-backend sites through failing_start.

<a id="hunk-100"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -260,9 +269,8 @@ fn equal_versions_return_outputs_without_descent() {`

```diff
@@ -260,9 +269,8 @@ fn equal_versions_return_outputs_without_descent() {
 fn semantic_and_backend_failure_layers_compose() {
     let (client_root, server_root) = one_sided_pair(&[(0x20, 1, 1)]);
     let backend = Failing::after(Local, usize::MAX);
-    let client =
-        Handshaking::start(backend.clone(), failing_root(client_root)).window(WindowConfig::FLOOR);
-    let server = Handshaking::start(backend, failing_root(server_root)).window(WindowConfig::FLOOR);
+    let client = failing_start(backend.clone(), client_root);
+    let server = failing_start(backend, server_root);
     let server = Faulting::new(server, 0, Some(Fault::Reply(Violation::UnexpectedQuery)));
     let error = run_to_quiescence(drive_streaming(client, server))
         .expect("the stacked session must terminate")
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 271:
>
> As above.

<a id="hunk-101"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -274,9 +282,8 @@ fn semantic_and_backend_failure_layers_compose() {`

```diff
@@ -274,9 +282,8 @@ fn semantic_and_backend_failure_layers_compose() {
 
     let (client_root, server_root) = one_sided_pair(&[(0x20, 1, 1)]);
     let backend = Failing::after(Local, 0);
-    let client =
-        Handshaking::start(backend.clone(), failing_root(client_root)).window(WindowConfig::FLOOR);
-    let server = Handshaking::start(backend, failing_root(server_root)).window(WindowConfig::FLOOR);
+    let client = failing_start(backend.clone(), client_root);
+    let server = failing_start(backend, server_root);
     let server = Faulting::new(server, 0, None);
     let error = run_to_quiescence(drive_streaming(client, server))
         .expect("the stacked session must terminate")
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 284:
>
> As above.

<a id="hunk-102"></a>
### src/tree/mirror/streaming/tests/stats.rs `@@ -19,18 +19,15 @@ use std::collections::BTreeSet;`

```diff
@@ -19,18 +19,15 @@ use std::collections::BTreeSet;
 
 use proptest::prelude::*;
 
-use super::fixtures::{divergent_cells_pair, grown, path_at, rooted};
-use crate::testing::run_to_quiescence;
+use super::fixtures::{LeafOrder, divergent_cells_pair, grown, path_at, rooted};
+use super::{LocalSession, join_oracle};
 use crate::tree::Root;
-use crate::tree::arb::leaf_parent_redaction_pair;
-use crate::tree::mirror::streaming::message::initiates;
-use crate::tree::mirror::streaming::stats::{Recorder, SessionStats};
-use crate::tree::mirror::streaming::window::WindowConfig;
-use crate::tree::mirror::streaming::{
-    Local, Root as StreamingRoot, materialized::Handshaking, mirror as drive_streaming,
+use crate::tree::arb::{
+    arb_forgotten_siblings, forgotten_sibling_pair, leaf_parent_redaction_pair,
 };
-
-use super::fixtures::LeafOrder;
+use crate::tree::mirror::streaming::message::initiates;
+use crate::tree::mirror::streaming::stats::SessionStats;
+use crate::tree::mirror::streaming::{Local, Root as StreamingRoot};
 
 /// Reconcile `a` and `b` through the streaming local backend with a
 /// recorder on each side, returning both reconciled roots and both
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 22:
>
> Imports: the builder and join_oracle from the parent, the new arb fixtures; the direct session construction imports leave.

<a id="hunk-103"></a>
### src/tree/mirror/streaming/tests/stats.rs `@@ -40,24 +37,10 @@ use super::fixtures::LeafOrder;`

```diff
@@ -40,24 +37,10 @@ use super::fixtures::LeafOrder;
 /// harness pins the walk's counters (disputes, gains, sheds) and the
 /// window grant.
 fn mirror_with_stats(a: Root, b: Root) -> (Root, Root, SessionStats, SessionStats) {
-    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
-    let a_recorder = Recorder::default();
-    let b_recorder = Recorder::default();
-    let client = Handshaking::start(Local, a)
-        .window(WindowConfig::FLOOR)
-        .stats(a_recorder.clone());
-    let server = Handshaking::start(Local, b)
-        .window(WindowConfig::FLOOR)
-        .stats(b_recorder.clone());
-    let (ours, theirs) = run_to_quiescence(drive_streaming(client, server))
-        .expect("streaming mirror became quiescent before completion")
-        .expect("local mirror speaks no violations");
-    (
-        ours.into(),
-        theirs.into(),
-        a_recorder.snapshot(),
-        b_recorder.snapshot(),
-    )
+    let outcome = LocalSession::new(a, b).stats().run();
+    let (a_stats, b_stats) = *outcome.stats();
+    let (ours, theirs) = outcome.sides();
+    (ours, theirs, a_stats, b_stats)
 }
 
 /// The live-leaf count of a tree root, for conservation checks.
```

<!-- annotation -->
> **streaming-tests-3** (T127), line 40:
>
> mirror_with_stats through the builder's stats instrument; SessionStats is Copy, so the pair is dereferenced, not cloned (the first box gate caught the clone as clippy's clone_on_copy).

<a id="hunk-104"></a>
### src/tree/mirror/streaming/tests/stats.rs `@@ -217,9 +200,81 @@ fn honored_redaction_counts_as_shed() {`

```diff
@@ -217,9 +200,81 @@ fn honored_redaction_counts_as_shed() {
     assert_eq!(b_stats.disputed_scopes, 16);
 }
 
+/// A leaf-parent only the holder occupies is judged leaf by leaf: the
+/// forgotten sibling sheds, the concurrent one crosses, and both sides
+/// hold the join oracle's tree in either orientation.
+///
+/// The counterparty holds nothing under the parent, so no leaf-parent
+/// dispute answers for the leaves; the verdict is the streaming filter's
+/// own, at leaf height.
+#[test]
+fn forgotten_sibling_is_judged_at_leaf_height() {
+    let (holder, forgetter, expected) = forgotten_sibling_pair();
+    assert_eq!(
+        join_oracle(holder.clone(), forgetter.clone()),
+        expected,
+        "the fixture's expectation is the join oracle's"
+    );
+    for holder_first in [true, false] {
+        let (left, right) = if holder_first {
+            (holder.clone(), forgetter.clone())
+        } else {
+            (forgetter.clone(), holder.clone())
+        };
+        let (ours, theirs, left_stats, right_stats) = mirror_with_stats(left, right);
+        assert_eq!(ours, expected, "left side holds the join oracle's tree");
+        assert_eq!(theirs, expected, "right side holds the join oracle's tree");
+        let (holder_stats, forgetter_stats) = if holder_first {
+            (left_stats, right_stats)
+        } else {
+            (right_stats, left_stats)
+        };
+        assert_eq!(
+            holder_stats.messages_shed, 1,
+            "the holder sheds the forgotten leaf"
+        );
+        assert_eq!(holder_stats.messages_gained, 0);
+        assert_eq!(forgetter_stats.messages_shed, 0);
+        assert_eq!(
+            forgetter_stats.messages_gained, 1,
+            "the forgetter gains the concurrent leaf"
+        );
+    }
+}
+
 proptest! {
     #![proptest_config(ProptestConfig::with_cases(64))]
 
+    /// For any forgotten subset of a holder's sibling leaves, both sides
+    /// hold the join oracle's tree in either orientation; the holder sheds
+    /// exactly the forgotten leaves and the forgetter gains exactly the
+    /// rest.
+    #[test]
+    fn forgotten_siblings_match_the_join_oracle(
+        (holder, forgetter, forgotten) in arb_forgotten_siblings(),
+        holder_first in any::<bool>(),
+    ) {
+        let expected = join_oracle(holder.clone(), forgetter.clone());
+        let total = live(&holder);
+        let (left, right) = if holder_first {
+            (holder, forgetter)
+        } else {
+            (forgetter, holder)
+        };
+        let (ours, theirs, left_stats, right_stats) = mirror_with_stats(left, right);
+        prop_assert_eq!(&ours, &expected);
+        prop_assert_eq!(&theirs, &expected);
+        let (holder_stats, forgetter_stats) = if holder_first {
+            (left_stats, right_stats)
+        } else {
+            (right_stats, left_stats)
+        };
+        prop_assert_eq!(holder_stats.messages_shed, forgotten as u64);
+        prop_assert_eq!(holder_stats.messages_gained, 0);
+        prop_assert_eq!(forgetter_stats.messages_shed, 0);
+        prop_assert_eq!(forgetter_stats.messages_gained, total - forgotten as u64);
+    }
+
     /// Across generated antichain corpora, both sides' counters match
     /// the oracle exactly.
     ///
```

<!-- annotation -->
> **materialized-27** (T26), line 210:
>
> The cross-peer pin the entry asked for: forgotten_sibling_pair in both orientations, both endpoints at the join oracle, holder sheds one and gains none, forgetter gains one and sheds none. Placed here rather than tests.rs because the shed and gain counts need the stats harness.

<!-- annotation -->
> **materialized-27** (T26), line 250:
>
> The generalization: any forgotten subset, Tree::join as oracle, plus the exact shed and gain counts (sheds equal the forgotten count, gains the rest), which cost nothing and also pin the whole-parent shed above one. Negative control: with the leaf verdict inverted both tests fail (commit message).

