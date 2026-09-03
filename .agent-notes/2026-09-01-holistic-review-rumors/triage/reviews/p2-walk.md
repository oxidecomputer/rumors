<!-- CAVEAT LECTOR: review packet for lane p2-walk, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p2-walk

## Goal

The streaming mirror's materialized walk classified the violations a
counterparty's reply can commit in two places: the descending stages
through the `Resolver`'s taxonomy, and the walk's two end legs (the
opening and the terminal absorb) through hand-written arms that named
fewer shapes and could disagree with the stages. The review also
recorded a liveness gap it could not reproduce: a walk error before the
opening's first yield leaving the wire session quiescent. This lane
routes the terminal leg through the `Resolver`, classifies the opening
leg per arm in the `Resolver`'s order (its reply shape genuinely
differs), widens the injection families to every reply-shaped
violation, and pins the opening-failure position over a wire with a
control that stalls when the error item is never published. A prose
sweep across `src/` and `tests/` replaces the "fails typed" family with
"returns an error".

## Rulings landed

- T40: the walk's end legs share the `Resolver`'s classifier where the
  reply shape allows it and classify per arm in the `Resolver`'s order
  where it does not; the injection families widen to every reply-shaped
  violation; the "aborts typed" family of phrases leaves the prose for
  "returns an error".
- T141: the prose standard, applied to every touched paragraph.

## Stack position

- Base: `b1e8a397` (main at launch; carries the `before` allow)
- Parent: `main`
- Children: none. The renderer lane's capture files were left untouched
  by the sweep, as arranged.

## Acceptance table

Every row is one the coordinator's verification runner ran on the illumos
box against the lane at `18ebb1ad`, then a detached scratch worktree at
`eb97720b` under a processor set, with whole logs kept.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| `materialized-14` step 1 | `e03434a3` | the two opening-failure pins by name (box) | `materialized_backend_failures_are_fail_fast_over_the_wire` PASS; `opening_failure_before_the_first_yield_returns_over_the_wire` PASS |
| (negative control) | | `park_after_published_error` moved before the error item is published in `materialized/work.rs`'s `pump` | both pins FAIL with `Stalled`: `materialized backend failure left the wire session quiescent` (the committed seed replays the shape) and `an opening failure must terminate both sessions, not stall them` |
| step 2, 3 | `c033be4f` | the three injection families by name (box) | `terminal_absorb_reports_exact_violation`, `opening_fault_reports_exact_violation`, `connected_violation_aborts_without_mutating_root`: `3 passed` |
| (negative control) | | the opening leg's `Match` arm reverted to report `UnexpectedQuery` (the old arm) | `opening_fault_reports_exact_violation` FAILS: `left: UnexpectedQuery, right: UnexpectedMatch: Match { trailing: false }` |
| all | `18ebb1ad` | `cargo nextest run -p rumors --all-features --locked -E 'test(tree::mirror::streaming) \| binary(seed_liveness)'` (box) | `276 tests run: 276 passed` |
| vocabulary sweep | `18ebb1ad` | the brief's regenerating grep; `git grep -E 'fails (the (session\|decode) )?typed\|aborts typed\|exit typed'` | both empty |
| `absorb` | `c033be4f` | `git grep '\.clone()' -- materialized.rs` filtered to `absorb` | one `stats.clone()` (the `Recorder` handed to `Resolver::new`), no leaf clone |
| all | `18ebb1ad` | snapshot and capture-file diffs | 0 snapshot files; the renderer lane's capture files untouched |
| seeds | `e03434a3`, `c033be4f` | `git diff --stat -- proptest-regressions` | two seed files, one row each (the proxy pin's shrunk pair; a violations-family case) |
| all | `18ebb1ad` | `just gate` on the box (lane log, two runs) | seven streams ok; `fuzz` the accepted illumos leg |
| repair `eb97720b` | `eb97720b` | the wider "typed" family grep over the nine repaired files; whole tree | 0 in the lane's files; 29 sites elsewhere, the P3 vocabulary sweep's (recorded in `triage/new-findings.md`) |
| (same) | `eb97720b` | the streaming suite plus seed liveness under `pset-run -n 40` (box, scratch worktree) | `276 tests run: 276 passed` |
| (negative control, item 3) | | `park_after_published_error` moved before the error item is published | both twins FAIL with `Stalled`: the in-process `materialized_backend_failures_are_fail_fast` (now drawn from the wide generator) and its wire twin |
| (same) | `eb97720b` | the `Faulting` arms' stated couplings; the in-process twin's generator | the `UnexpectedSupply` arm names the comb's spine as its premise; `arb_wide_divergent_pair` imported and drawn at `faults.rs:205` |
| all | `eb97720b` | `pset-run -n 40 -- just gate` on the box (lane log `gate-3.log`) | seven streams ok; `fuzz` the accepted illumos leg |

## Fresh-eyes rounds

**Round 1** (surface correctness with operational validity), by sha:
no defect in the production change. The reviewer traced the terminal
leg over an empty fan (every old violation surfaces with a truer
variant; the leaf-radix pre-check is load-bearing), the opening leg's
legitimate shapes in-process and over a wire (no conforming reply is
newly rejected; over a wire the proxy folds early supplies into the
next stage, so the opening's supply arms are in-process-only), the
trailing check's liveness (no cycle of stream ends), the yield census
(none moved), and both harness-script repairs. Repairs landed in
`eb97720b`: the adjectival kin of the excised phrase family in the
lane's files (seventeen sites), the two `Faulting` scripts' fixture
couplings stated at their arms, and the in-process fail-fast twin
drawn from the wide generator so the pump-park mutant fails it too.
The rounds stopped here.

Reviewer notes not acted on, and hand-backs recorded in
`triage/new-findings.md`: `absorb` keeps one `stats.clone()` (a
refcount bump; the `Resolver` borrowing its recorder is the clean fix,
a P5 proposal); the wider "typed" family survives at 33 sites outside
this lane's files (the P3 vocabulary sweep, with the regenerating grep
in the lane's report); the opening's precedence for a leading supply
at a held radix is a judgment call pinned by its test; the widened
connected suite's correctness rests on comb geometry a fixture change
would misclassify silently rather than fail loudly.

## Stops

None. Findings handed back, recorded in `triage/new-findings.md`: the in-process `materialized_backend_failures_are_fail_fast`
in `tests/faults.rs` is likely inert on the response-stream path for
the reason its wire twin was under the mutant (few-leaf pairs rarely
dispute a scope); the `internal_level` doc's claim that a mid-loop
failure would strand the counterparty's pump is undemonstrated. One
follow-up nit the lane proposes rather than lands: the `Resolver` could
borrow its `Recorder` instead of taking it owned, removing one
`stats.clone()` per request at four sites. <!-- STOPS -->

## Reading order

### deviations from a stated resolution

- materialized-14 (T40) at `src/tree/mirror/streaming/testing/faulting.rs:198` ([hunk](#hunk-52))

### new tests and negative controls

- materialized-14 (T40) at `src/tree/mirror/streaming/remote/proxy/tests.rs:552` ([hunk](#hunk-47))

### production edits

- materialized-14 (T40) at `src/link.rs:0` ([hunk](#hunk-4))
- materialized-14 (T40) at `src/testing/transport.rs:0` ([hunk](#hunk-5))
- materialized-14 (T40) at `src/tree/mirror/streaming/driver.rs:0` ([hunk](#hunk-14))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized.rs:106` ([hunk](#hunk-15))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized.rs:110` ([hunk](#hunk-15))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized.rs:858` ([hunk](#hunk-16))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized.rs:883` ([hunk](#hunk-17))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized.rs:909` ([hunk](#hunk-17))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized.rs:911` ([hunk](#hunk-17))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work.rs:26` ([hunk](#hunk-26))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/levels.rs:165` ([hunk](#hunk-27))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/levels.rs:209` ([hunk](#hunk-28))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/levels.rs:219` ([hunk](#hunk-28))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/levels.rs:254` ([hunk](#hunk-28))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/levels.rs:256` ([hunk](#hunk-28))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/levels.rs:275` ([hunk](#hunk-29))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec.rs:0` ([hunk](#hunk-39))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/error.rs:0` ([hunk](#hunk-45))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/error.rs:0` ([hunk](#hunk-45))
- materialized-14 (T40) at `src/tree/mirror/streaming/testing/faulting.rs:234` ([hunk](#hunk-53))
- materialized-14 (T40) at `src/tree/mirror/streaming/testing/faulting.rs:245` ([hunk](#hunk-53))
- materialized-14 (T40) at `src/tree/mirror/streaming/testing/faulting.rs:224` ([hunk](#hunk-53))
- materialized-14 (T40) at `src/tree/mirror/streaming/testing/faulting.rs:236` ([hunk](#hunk-53))

### tests and prose

- materialized-14 (T40) at `proptest-regressions/tree/mirror/streaming/materialized/work/tests/violations.txt:9` ([hunk](#hunk-2))
- materialized-14 (T40) at `proptest-regressions/tree/mirror/streaming/remote/proxy/tests.txt:11` ([hunk](#hunk-3))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-6))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-6))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-7))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-7))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-8))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-8))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-9))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-9))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-10))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-10))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-11))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-11))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-12))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-12))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-13))
- materialized-14 (T40) at `src/tree/mirror/handshake/tests.rs:0` ([hunk](#hunk-13))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-18))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-19))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-20))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-21))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-22))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-23))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-24))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/tests.rs:0` ([hunk](#hunk-25))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/tests/violations.rs:0` ([hunk](#hunk-30))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/tests/violations.rs:0` ([hunk](#hunk-31))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/tests/violations.rs:0` ([hunk](#hunk-32))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/tests/violations.rs:0` ([hunk](#hunk-33))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/tests/violations.rs:0` ([hunk](#hunk-34))
- materialized-14 (T40) at `src/tree/mirror/streaming/materialized/work/tests/violations.rs:0` ([hunk](#hunk-35))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:0` ([hunk](#hunk-36))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/adapter/tests/opening.rs:0` ([hunk](#hunk-37))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/adapter/tests/opening.rs:0` ([hunk](#hunk-37))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/adapter/tests/opening.rs:0` ([hunk](#hunk-38))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/adapter/tests/opening.rs:0` ([hunk](#hunk-38))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-40))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-40))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-41))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-41))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-42))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-42))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-43))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/decode/tests.rs:0` ([hunk](#hunk-43))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:0` ([hunk](#hunk-44))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/proxy/tests.rs:303` ([hunk](#hunk-46))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:0` ([hunk](#hunk-48))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:0` ([hunk](#hunk-49))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:0` ([hunk](#hunk-50))
- materialized-14 (T40) at `src/tree/mirror/streaming/remote/proxy/work/tests.rs:0` ([hunk](#hunk-51))
- materialized-14 (T40) at `src/tree/mirror/streaming/tests/faults.rs:0` ([hunk](#hunk-54))
- materialized-14 (T40) at `src/tree/mirror/streaming/tests/faults.rs:10` ([hunk](#hunk-54))
- materialized-14 (T40) at `src/tree/mirror/streaming/tests/faults.rs:0` ([hunk](#hunk-55))
- materialized-14 (T40) at `src/tree/mirror/streaming/tests/faults.rs:0` ([hunk](#hunk-56))
- materialized-14 (T40) at `src/tree/mirror/streaming/tests/faults.rs:199` ([hunk](#hunk-56))
- materialized-14 (T40) at `tests/handshake.rs:0` ([hunk](#hunk-57))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-58))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-58))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-59))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-59))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-60))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-60))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-61))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-61))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-62))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-62))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-63))
- materialized-14 (T40) at `tests/payload_depth.rs:0` ([hunk](#hunk-63))
- materialized-14 (T40) at `tests/single_peer.rs:0` ([hunk](#hunk-64))
- materialized-14 (T40) at `tests/single_peer.rs:0` ([hunk](#hunk-64))
- materialized-14 (T40) at `tests/single_peer.rs:0` ([hunk](#hunk-65))
- materialized-14 (T40) at `tests/single_peer.rs:0` ([hunk](#hunk-65))
- materialized-14 (T40) at `tests/single_peer.rs:0` ([hunk](#hunk-66))
- materialized-14 (T40) at `tests/single_peer.rs:0` ([hunk](#hunk-66))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p2-walk.tsv `@@ -0,0 +1,50 @@`

```diff
@@ -0,0 +1,50 @@
+# Lane p2-walk, ruling T40 (materialized-14, fix-amended; the "fails typed" vocabulary sweep). Line numbers are
+# new-side lines of `git diff b1e8a397...HEAD`; a deletion is annotated at the line that follows it.
+src/tree/mirror/streaming/remote/proxy/tests.rs	303	materialized-14	T40	The probe the recorded commit (50c8b0a3) held back needs a wire session whose materialized walk fails; every existing wire harness puts the failing backend on the proxy (reconcile_with_stacked_failures), so this helper is its twin with the Failing backend on the walk. walk_injected_operation reads the error from whichever endpoint holds the walk, the same Client/Server convention the proxy helpers use.
+src/tree/mirror/streaming/remote/proxy/tests.rs	552	materialized-14	T40	Step 1 of the brief: construct the probe, observe the stall, fix. The stall does not reproduce on this tree: both tests pass against base. Why the recorded construction (an understated set_len rewrite) cannot reach the walk's early-supply loop over a wire: the proxy's replayed opening carries only the root question (the early supplies cross on the opening stream and are folded into root-request replies at the first descending stage, already so at 50c8b0a3), and since 08f2899b the wire decoder charges the ledger at ingress ahead of the walk. The position the record names is still reachable by a backend failure in the responder's opening (answer::internal explodes the disputed first root child before the yield): the deterministic test pins exactly that, operation 1 on early_first_child_dispute_pair. Negative control (reversible mutation, restored, work.rs diff empty): parking the walk's pump before it publishes an error item fails the deterministic test with `an opening failure must terminate both sessions, not stall them: Stalled`, and the proptest twin with `materialized backend failure left the wire session quiescent: Stalled` at operations = 1, fail_left = false. The twin draws from arb_wide_divergent_pair deliberately: with arb_divergent_pair it passed under the mutant, because its few-leaf trees rarely dispute a scope, so no failing operation lands inside a stage loop (the response-stream path); the in-process twin materialized_backend_failures_are_fail_fast shares that weakness and is reported as a finding, not changed here.
+proptest-regressions/tree/mirror/streaming/remote/proxy/tests.txt	11	materialized-14	T40	The seed the mutant run minted on the box, fetched before the next sync could delete it and committed per the pool's precedent for flip-minted entries: it replays the minimal stalling case (a wide pair with a disputed root child, operations = 1, the walk as responder) ahead of fresh generation, so a reintroduced stall fails on the first case.
+src/tree/mirror/streaming/materialized/work.rs	26	materialized-14	T40	The terminal leg lives in materialized.rs, outside `work`, so the classifier is re-exported one level up (pub-in-private is the recorded convention, T46); nothing else outside `work` uses it.
+src/tree/mirror/streaming/materialized.rs	106	materialized-14	T40	Deletion of the `contained` import: the hand-written arm was its only use here; the containment check now runs inside the Resolver's supply arm.
+src/tree/mirror/streaming/materialized.rs	110	materialized-14	T40	`Resolver` joins `Work` in the import from `work`.
+src/tree/mirror/streaming/materialized.rs	858	materialized-14	T40	The doc states the one rule the terminal leg keeps for itself and why the Resolver cannot express it: a leaf request names a radix, the Resolver classifies against a fan. The credit sentence now says the resolver's supply arm does the crediting, because it does (the old arm credited 1 by hand; a leaf's len() is 1, so the count is unchanged).
+src/tree/mirror/streaming/materialized.rs	883	materialized-14	T40	The 'alternatively' branch of the resolution, tried first per T40's amendment: the reply is fed reaction by reaction to a Resolver over `Query { prefix: parent, ours: [] }`, so Match -> UnexpectedMatch, Query -> UnexpectedQuery, a duplicate radix -> InvalidSupply, containment, the ledger, and the stats credit are all the Resolver's. The terminal-only rule sits ahead of `react`: a supply at any radix but the requested one is InvalidSupply, as the old arm already classified `[Supply(other)]`; it precedes containment so structure is judged before content, the Resolver's own order. Unpacking the Resolution: `pop()` yields the accepted leaf by move (no clone, the acceptance criterion), the `Pending` arm is `unreachable!` because an empty fan routes no query (the mutants policy's form for a truly unreachable branch), and a debug_assert pins that at most one slot exists (one radix admitted, its duplicate rejected). Rejected alternative: per-arm reclassification (the resolution's primary text) would restate the Resolver's containment/ledger/credit logic a second time, the duplication the goal is against.
+src/tree/mirror/streaming/materialized.rs	909	materialized-14	T40	Deletion: the `_ => UnfinishedReply` catch-all and the wrong-radix arm are gone; the hunk's remaining lines are the unchanged pass-up.
+src/tree/mirror/streaming/materialized.rs	911	materialized-14	T40	rustfmt re-flow of the unchanged pass-up after the block above it changed shape; no semantic change.
+src/tree/mirror/streaming/materialized/work/levels.rs	165	materialized-14	T40	Doc paragraph of `responder_level` rewritten to state the opening reply's grammar and the checks its early supplies pass, at the reader's altitude; the parenthetical about what a failure 'can strand' is gone (the liveness at this position is now pinned by the wire probe, and the sentence explained nothing a caller holds). Em-dashes replaced.
+src/tree/mirror/streaming/materialized/work/levels.rs	209	materialized-14	T40	The opening leg keeps its own arms: per-arm reclassification, the brief's fallback. Why the Resolver cannot serve here: it pairs reactions positionally against the held fan, but the opening reply is one Query about the root scope itself followed by supplies at initiator-exclusive radices, with no positional reaction for the responder's held children (the merge-join answers those on this side), so fed to a Resolver over the fan the root Query would pair with the first held child and every early supply past a held radix would read as InvalidSupply. The comment says so at the site.
+src/tree/mirror/streaming/materialized/work/levels.rs	219	materialized-14	T40	The arms, judged in reply order so the first offending reaction names the fault: a second Query -> UnexpectedQuery (was the catch-all); Match anywhere -> UnexpectedMatch (was UnexpectedQuery); a supply is checked in the Resolver's order, ascending radix first (InvalidSupply), then not held (UnexpectedSupply), then, only for a supply ahead of the question, InvalidSupply as out of order; then containment, ledger, explode. Judgment call: a leading supply is judged by its radix before the missing question is held against it, so the connected suite's UnexpectedSupply corruption (a held-radix supply inserted at the front) classifies as UnexpectedSupply at the opening, matching what it means at every other height.
+src/tree/mirror/streaming/materialized/work/levels.rs	254	materialized-14	T40	Deletion: the old loop body, whose containment and ledger checks moved into the Supply arm above unchanged.
+src/tree/mirror/streaming/materialized/work/levels.rs	256	materialized-14	T40	A reply with no root question is UnfinishedReply, per the resolution (the reply ended before reacting to the one thing it owed); the resolution's 'documented new variant' alternative is a stop the amendment closes, and no new variant was needed.
+src/tree/mirror/streaming/materialized/work/levels.rs	275	materialized-14	T40	The trailing `requests.next()` check every descending walk performs, placed after the yield block. Liveness argument: the opening stream's end now waits for the initiator's opening stream to end, which happens once its one reply and root query are out; that end does not depend on anything this stage publishes, so no new wait edge closes a cycle, and no yield point moves (`yield_resolve_query!` is untouched).
+src/tree/mirror/streaming/materialized/tests.rs	0	materialized-14	T40	The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.
+proptest-regressions/tree/mirror/streaming/materialized/work/tests/violations.txt	9	materialized-14	T40	Seed minted by the opening family's first run against the old arms (`DescendingSupplies, radixes = {0}`), fetched from the box and committed per the pool's rule: it replays the failing baseline ahead of fresh generation.
+src/tree/mirror/streaming/materialized/work/tests/violations.rs	0	materialized-14	T40	Opening-leg injections, as the resolution asks. `reported_violation` gains a count of well-formed replies the script answers before its fault (0 at every query height; 1 for the opening's unasked-reply shape, whose extra reply necessarily trails the opening's own answer), so a well-formed reply ahead of the fault is asserted answered rather than mistaken for acceptance. The opening family is its own proptest rather than a 33rd height of `injected_fault_reports_exact_violation`: the opening's reply grammar (one root Query, then supplies) is not a query height's (positional reactions over a held fan), so the script differs in kind. Twelve shapes, one per clause of the grammar, including both positions of a leading supply and the ledger arm. On the old arms it fails at the first case (`the malformed reply produced a successful response`, DescendingSupplies at radixes {0}: the out-of-order supplies were absorbed and parked).
+src/tree/mirror/streaming/testing/faulting.rs	198	materialized-14	T40	Deviation from nothing stated, but a harness repair the widening needed: the UnaskedReply script forwarded one honest reply and then ended, so at any stage with two or more queries its extra empty reply paired with the second query and classified as UnfinishedReply (observed: `left: UnfinishedReply, right: UnaskedReply` at `violation = UnaskedReply, server_steps = 0, client_steps = 1`). It now forwards every honest reply and then appends the unasked one, which is what the name means.
+src/tree/mirror/streaming/testing/faulting.rs	234	materialized-14	T40	The InvalidSupply script's duplicated radix moves from 0 to 0xff, the radix the UncontainedSupply script already assumes no fixture holds: at the opening no honest reaction precedes the corruption, so a held radix 0 would classify as UnexpectedSupply there (correctly: it lands on a held child), while a fresh radix duplicated is InvalidSupply at every height. The comment records the assumption beside the one it copies.
+src/tree/mirror/streaming/testing/faulting.rs	245	materialized-14	T40	The two radix literals of the change described at line 227.
+src/tree/mirror/streaming/tests/faults.rs	0	materialized-14	T40	`arb_connected_violation` widened from two variants to every reply-shaped violation `Faulting` can script (eight; OverdrawnSupply is a greeting lie, scripted by the greeting family). The doc's first paragraph is shortened to satisfy doclint's 220-character summary bound, which the first wording exceeded (the gate's first leg caught it). The step range stays 0..=15 per the brief's hazard. Cost: the property now runs about 27 s on the box (was under 2 s with two variants); the greeting-lie family beside it already runs 46 s.
+src/link.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1. One word in a module-doc paragraph; T100's owner passages in this file are elsewhere and untouched.
+src/testing/transport.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+src/tree/mirror/handshake/tests.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.
+src/tree/mirror/streaming/driver.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+src/tree/mirror/streaming/remote/adapter/tests/malformed.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+src/tree/mirror/streaming/remote/adapter/tests/opening.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1. Kin the grep misses ('fails the decode typed').
+src/tree/mirror/streaming/remote/codec.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 5. Two em-dashes in the re-worded sentence became commas.
+src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+src/tree/mirror/streaming/remote/error.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+src/tree/mirror/streaming/remote/proxy/tests/declarations.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. These three are kin the brief's grep misses ('fails the session typed'): the goal names the family, so they go too, and the regex gap is reported.
+src/tree/mirror/streaming/remote/proxy/work/tests.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2. The sentence before the hit ('A typed backend failure') dropped its adjective too, for one voice across the two lines.
+tests/handshake.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+tests/payload_depth.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. One grep hit and one kin the grep misses ('exit typed'), the latter re-flowed over three lines.
+tests/single_peer.rs	0	materialized-14	T40	Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.
+src/tree/mirror/streaming/remote/error.rs	0	materialized-14	T40	Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 1. 'the typed adapter, stream-layer, and codec failures' loses its adjective; the sentence already names the three layers.
+src/tree/mirror/handshake/tests.rs	0	materialized-14	T40	Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.
+src/tree/mirror/streaming/remote/codec/decode/tests.rs	0	materialized-14	T40	Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.
+src/tree/mirror/streaming/remote/adapter/tests/opening.rs	0	materialized-14	T40	Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 1.
+tests/payload_depth.rs	0	materialized-14	T40	Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 4. The two em-dashes on the re-flowed line at 54-55 became commas.
+tests/single_peer.rs	0	materialized-14	T40	Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.
+src/tree/mirror/streaming/testing/faulting.rs	224	materialized-14	T40	Fresh-eyes repair, item 2: the UnexpectedSupply script's radix 0 is a fixture coupling the arm now states, as the 0xff arms state theirs: it classifies as UnexpectedSupply only where the answered fan's first child is radix 0, which full_depth_comb_pair's spine guarantees at every scope the step range reaches; at a higher first child it is a legal sibling mid-walk, and at the opening it is out of order.
+src/tree/mirror/streaming/testing/faulting.rs	236	materialized-14	T40	Fresh-eyes repair, item 2: the InvalidSupply script absorbs its first 0xff copy before the duplicate fires, so the receiver's ledger needs one leaf of slack under the corrupting side's declared length; the comment names why every fixture has it (true declarations; the receiver absorbs at most that side's exclusive leaves ahead of the corruption).
+src/tree/mirror/streaming/tests/faults.rs	10	materialized-14	T40	Fresh-eyes repair, item 3: the in-process fail-fast twin draws from the wide generator, as its wire twin does and as the generator's doc says wire-liveness properties should. Under the pump-parking mutant it passed on arb_divergent_pair and now fails at its first case (`backend failure left materialized reconciliation quiescent: Stalled`, operations = 1, fail_client = false); verbatim in the commit message. The import is the only other change.
+src/tree/mirror/streaming/tests/faults.rs	199	materialized-14	T40	See the row at line 10: the generator change and the doc sentence that says why (a stage loop explodes nodes only under disputed scopes).
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### proptest-regressions/tree/mirror/streaming/materialized/work/tests/violations.txt `@@ -6,3 +6,4 @@`

```diff
@@ -6,3 +6,4 @@
 # everyone who runs the test benefits from these saved cases.
 # The case below was minted against a deliberately neutralized resolver containment check during mutation review; it passes on real code and deterministically exercises the resolver chokepoint.
 cc 2ff181faf59f338e6b61b11eb3b507b7457d5d343163f1ddbfaedba1bbf10122 # shrinks to injection = UncontainedSupply, parent = 0, radixes = {0}, schedule = []
+cc 000e7cb205b90efce9c06e1113c90b3f5d88b211671e19c97e8f51b3f4ad8220 # shrinks to injection = DescendingSupplies, radixes = {0}, schedule = []
```

<!-- annotation -->
> **materialized-14** (T40), line 9:
>
> Seed minted by the opening family's first run against the old arms (`DescendingSupplies, radixes = {0}`), fetched from the box and committed per the pool's rule: it replays the failing baseline ahead of fresh generation.

<a id="hunk-3"></a>
### proptest-regressions/tree/mirror/streaming/remote/proxy/tests.txt `@@ -8,3 +8,4 @@ cc 596d12a22c3a5079905ab0a260bb878acad52a9a0bb159f7081df5d847fcb951 # shrinks to`

```diff
@@ -8,3 +8,4 @@ cc 596d12a22c3a5079905ab0a260bb878acad52a9a0bb159f7081df5d847fcb951 # shrinks to
 cc a8e144179d2225ef4f994669a999b5cbc4f8faf69a12208bcc8510bb509cddc4 # shrinks to (a, b) = (Root { ceiling: (0, (0, 0, 3), 5), root: Some(Node { prefix: "", children: Branch { ceiling: OnceLock(<uninit>), floor: OnceLock(<uninit>), leaves: 6, children: {13: Node { prefix: "b9bc6c0254ae5e131aac9c4ff0d10a18ab41bc4913a3bbcfe8a73c09a79439", children: Leaf { version: (0, 0, 3), message: () } }, 17: Node { prefix: "1c316ce374f8ef1812d853ba5b99e657c4a986609f5f4a4b6f6c0cd0746e4f", children: Leaf { version: (0, (0, 0, 2), 5), message: () } }, 20: Node { prefix: "b34806daa1d5e83a5ac0a71e42fba5a4cb891f34068c0d27e52a51860129c5", children: Leaf { version: (0, (0, 0, 1), 5), message: () } }, 142: Node { prefix: "1da3a707c8d4df241ab5be602cefcb11e464e5e8c94723c16adb253a81cf0a", children: Leaf { version: (0, 0, 5), message: () } }, 160: Node { prefix: "f4921e9d21dbfd835420175ea1dc5a644c43e9549724daf06fb44ec2014163", children: Leaf { version: (0, 0, 4), message: () } }, 224: Node { prefix: "cd3e3252fa8aa03361796d1b1d2a9449261c84d35d357607b43b1d71102e14", children: Leaf { version: (0, 0, 2), message: () } }} } }) }, Root { ceiling: (0, (0, (0, 0, 4), 0), 5), root: Some(Node { prefix: "", children: Branch { ceiling: OnceLock(<uninit>), floor: OnceLock(<uninit>), leaves: 9, children: {13: Node { prefix: "b9bc6c0254ae5e131aac9c4ff0d10a18ab41bc4913a3bbcfe8a73c09a79439", children: Leaf { version: (0, 0, 3), message: () } }, 17: Node { prefix: "4abe8bf7b2cd54e0eb4be0340ec80e8c1b6c620ee7e6ad44e6e48825fbaf70", children: Leaf { version: (0, (0, (0, 0, 3), 0), 5), message: () } }, 32: Node { prefix: "4d3be44e01d4d26c35fefb7ccc1825c9e1b643e4653608e7ba1d50ec07740a", children: Leaf { version: (0, 0, 1), message: () } }, 44: Node { prefix: "1cfb9e4b866dbffcfe2ef0f6874dfe6148727b647d9efa95d9b71913a3c4d4", children: Leaf { version: (0, (0, (0, 0, 1), 0), 5), message: () } }, 99: Node { prefix: "e41191dd1129bbca03b410af6425c18f7e53151cf348a28079242c40a609ae", children: Leaf { version: (0, (0, (0, 0, 2), 0), 5), message: () } }, 142: Node { prefix: "1da3a707c8d4df241ab5be602cefcb11e464e5e8c94723c16adb253a81cf0a", children: Leaf { version: (0, 0, 5), message: () } }, 159: Node { prefix: "40ce48b992b8515b2933f97887d487e94b833e2e995b3d6544d71b9f931bdc", children: Leaf { version: (0, (0, (0, 0, 4), 0), 5), message: () } }, 160: Node { prefix: "f4921e9d21dbfd835420175ea1dc5a644c43e9549724daf06fb44ec2014163", children: Leaf { version: (0, 0, 4), message: () } }, 224: Node { prefix: "cd3e3252fa8aa03361796d1b1d2a9449261c84d35d357607b43b1d71102e14", children: Leaf { version: (0, 0, 2), message: () } }} } }) }), schedule = []
 cc 0976f4a3e086191f4cad19af21868382af4641a33c728a43f591d9643a47b9d3 # shrinks to (a, b) = (Root { ceiling: 0, root: None }, Root { ceiling: (0, (0, (0, 0, 3), 0), 0), root: Some(Node { prefix: "", children: Branch { ceiling: OnceLock(<uninit>), floor: OnceLock(<uninit>), leaves: 3, children: {80: Node { prefix: "09cabf1e315343f1e40384237d2eeddc6919a3dbed8e64014638926442a28e", children: Leaf { version: (0, (0, (0, 0, 1), 0), 0), message: () } }, 107: Node { prefix: "6f2b879ff46d0291707155c648caff10bafeec067343cc7ddbf072e5e53cdf", children: Leaf { version: (0, (0, (0, 0, 3), 0), 0), message: () } }, 197: Node { prefix: "435d9afb7520ad826c543d168f27bcef96c0f0f6ffada7e375566a91b156d1", children: Leaf { version: (0, (0, (0, 0, 2), 0), 0), message: () } }} } }) }), operations = 0, fail_left = true
 cc 6b8224ab83109e8360142a1ec7d513259665c0ca64455c0d816c8fe2e3769159 # shrinks to (a, b) = (Root { ceiling: (0, (0, 0, 1), 1), root: None }, Root { ceiling: (0, (0, (0, 0, 1), 0), 1), root: Some(Node { prefix: "", children: Branch { ceiling: OnceLock(<uninit>), floor: OnceLock(<uninit>), leaves: 2, children: {32: Node { prefix: "4d3be44e01d4d26c35fefb7ccc1825c9e1b643e4653608e7ba1d50ec07740a", children: Leaf { version: (0, 0, 1), message: () } }, 226: Node { prefix: "a41a14212a9e89a9ea63e6166048a98c3874ca0109239598ee5bd071f11450", children: Leaf { version: (0, (0, (0, 0, 1), 0), 1), message: () } }} } }) }), operations = 0, fail_left = false
+cc 3c541531c3d1b57497946c27bec666bfc323727fbdd6876c240532f7e1599b4b # shrinks to (a, b) = (Root { ceiling: (0, (0, 0, 14), 0), root: Some(Node { prefix: "", children: Branch { bounds: OnceLock(<uninit>), leaves: 14, version_bytes: OnceLock(<uninit>), children: {4: Node { prefix: "229973eac7a239e0a3ff07cc0b203ecf43bc848bb6aa2fc8d8a58a12e4ad02", children: Leaf { version: (0, (0, 0, 11), 0), message: Message { serialized: "f6", .. } } }, 49: Node { prefix: "169d886ecce867a4394a40e693b3e926ec34c80a4fdfce8b8c19501490c286", children: Leaf { version: (0, (0, 0, 4), 0), message: Message { serialized: "f6", .. } } }, 54: Node { prefix: "0004e80f412ba9ee21c3ed67c91e6dc87ec4484c89ca6e187e6354690dceb9", children: Leaf { version: (0, (0, 0, 12), 0), message: Message { serialized: "f6", .. } } }, 67: Node { prefix: "bcd511e7470459d0fc429e5384b1c377bf79a7b20e05095440f20a5ee0c7ab", children: Leaf { version: (0, (0, 0, 13), 0), message: Message { serialized: "f6", .. } } }, 75: Node { prefix: "b3c94e7908067fb2195af2e0f2807c0325dbac70fa65b4e8504b92b18fb880", children: Leaf { version: (0, (0, 0, 3), 0), message: Message { serialized: "f6", .. } } }, 84: Node { prefix: "27cf328711dd7b2feb88bb25ddb0d633fe284cc96339d69e72b27e90d1c2b9", children: Leaf { version: (0, (0, 0, 1), 0), message: Message { serialized: "f6", .. } } }, 90: Node { prefix: "37cf127b0680355a6a0a77db83b716e3da3d434253eaa2ea0b06f3096270f7", children: Leaf { version: (0, (0, 0, 8), 0), message: Message { serialized: "f6", .. } } }, 122: Node { prefix: "577856476ac645ad4054bc915d258125500af000ee729c78b64735d0e77c6d", children: Leaf { version: (0, (0, 0, 10), 0), message: Message { serialized: "f6", .. } } }, 123: Node { prefix: "475fa257b49762cfb9ebdffcfd853439a15809cab21a51a02ef0c37519579f", children: Leaf { version: (0, (0, 0, 7), 0), message: Message { serialized: "f6", .. } } }, 137: Node { prefix: "a199b0999c5e694cc1641781421584618e87a425b81aae195e76eeb05cc2fe", children: Leaf { version: (0, (0, 0, 9), 0), message: Message { serialized: "f6", .. } } }, 149: Node { prefix: "abf4ad1dddb5ae922f4852ca5ff091a1a6281c0c5a0dbfce0e8b86ed5b8fe6", children: Leaf { version: (0, (0, 0, 14), 0), message: Message { serialized: "f6", .. } } }, 195: Node { prefix: "6f7f0fb8cf023ba9f391b04935e587c068ff64add9e92048d83b1326721446", children: Leaf { version: (0, (0, 0, 6), 0), message: Message { serialized: "f6", .. } } }, 221: Node { prefix: "d2b8d4f506952f945f9ac099c09190ebdc22b428ae7e26c3a1a7023e73fa22", children: Leaf { version: (0, (0, 0, 5), 0), message: Message { serialized: "f6", .. } } }, 249: Node { prefix: "e01f97a9b3ee5093a763286f4c51d4b948d0f47b6cd04fd2dc769403099b6a", children: Leaf { version: (0, (0, 0, 2), 0), message: Message { serialized: "f6", .. } } }} } }) }, Root { ceiling: (0, (0, (0, 0, 6), 0), 0), root: Some(Node { prefix: "", children: Branch { bounds: OnceLock(<uninit>), leaves: 6, version_bytes: OnceLock(<uninit>), children: {30: Node { prefix: "28f12de5c77004e23ececa92fb1dfd060dfd06d284e06394f319ffcd4e71e2", children: Leaf { version: (0, (0, (0, 0, 2), 0), 0), message: Message { serialized: "f6", .. } } }, 89: Node { prefix: "9e8505912bab736064186f61e1d40e5b02e9aedf339eea36576eb458889852", children: Leaf { version: (0, (0, (0, 0, 4), 0), 0), message: Message { serialized: "f6", .. } } }, 116: Node { prefix: "6efc726d62ceb52c68b43e96019408d93079b7f72a4c73dce510039cb1617a", children: Leaf { version: (0, (0, (0, 0, 1), 0), 0), message: Message { serialized: "f6", .. } } }, 149: Node { prefix: "588068b912400db5da17799f3814ff116250ef170248985c416132c71b63cb", children: Leaf { version: (0, (0, (0, 0, 6), 0), 0), message: Message { serialized: "f6", .. } } }, 191: Node { prefix: "9e0c7b8e2b00bc7813932689072b2d52c6ef409f891ba1763fca7553616a0d", children: Leaf { version: (0, (0, (0, 0, 3), 0), 0), message: Message { serialized: "f6", .. } } }, 236: Node { prefix: "53b94132bbe5ed2c9b809fee4b211a3c87e88a50bd1891197d8340f3fd2db2", children: Leaf { version: (0, (0, (0, 0, 5), 0), 0), message: Message { serialized: "f6", .. } } }} } }) }), operations = 1, fail_left = false, schedule = []
```

<!-- annotation -->
> **materialized-14** (T40), line 11:
>
> The seed the mutant run minted on the box, fetched before the next sync could delete it and committed per the pool's precedent for flip-minted entries: it replays the minimal stalling case (a wide pair with a disputed root child, operations = 1, the walk as responder) ahead of fresh generation, so a reintroduced stall fails on the first case.

<a id="hunk-4"></a>
### src/link.rs `@@ -118,7 +118,7 @@`

```diff
@@ -118,7 +118,7 @@
 //! peers and securing the transport to the application. It is worth
 //! being concrete about that division, because the protocol's own
 //! validation can look like security and is not. The protocol does
-//! reject malformed and mismatched sessions with typed errors, trusts
+//! reject malformed and mismatched sessions with an error, trusts
 //! nothing peer-declared before the fixed preamble validates, and leaves
 //! the caller's timeout as the sole liveness backstop against a silent
 //! peer; all of that machinery exists to catch nonconforming peers, and
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1. One word in a module-doc paragraph; T100's owner passages in this file are elsewhere and untouched.

<a id="hunk-5"></a>
### src/testing/transport.rs `@@ -182,7 +182,7 @@ impl State {`

```diff
@@ -182,7 +182,7 @@ impl State {
         delay
     }
 
-    /// Return a typed failure once its configured prefix has completed.
+    /// Return an error once its configured prefix has completed.
     fn failure(&mut self, operation: Operation) -> Option<io::Error> {
         let fault = self.plan.fault?;
         if fault.operation != operation {
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<a id="hunk-6"></a>
### src/tree/mirror/handshake/tests.rs `@@ -95,7 +95,7 @@ fn fragmented_exchange_is_symmetric() {`

```diff
@@ -95,7 +95,7 @@ fn fragmented_exchange_is_symmetric() {
 /// place.
 ///
 /// The two defined values are accepted, other small uint items are the
-/// typed intent rejection, and bytes that are no one-byte uint item at
+/// intent rejection, and bytes that are no one-byte uint item at
 /// all are the malformed-preamble class.
 #[test]
 fn intent_byte_space_is_exhaustive() {
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-7"></a>
### src/tree/mirror/handshake/tests.rs `@@ -120,7 +120,7 @@ fn intent_byte_space_is_exhaustive() {`

```diff
@@ -120,7 +120,7 @@ fn intent_byte_space_is_exhaustive() {
 }
 
 /// A peer that closes the connection at any point inside the preamble
-/// surfaces a typed truncation, never a hang and never a partial decode.
+/// surfaces the truncation error, never a hang and never a partial decode.
 ///
 /// Every strict prefix of the fixed item is a structurally distinct
 /// truncation, so the whole prefix space is swept, each cut resolving to
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-8"></a>
### src/tree/mirror/handshake/tests.rs `@@ -156,7 +156,7 @@ fn every_truncation_boundary_is_typed() {`

```diff
@@ -156,7 +156,7 @@ fn every_truncation_boundary_is_typed() {
                 );
             }
             other => {
-                panic!("cut after {cut} bytes must be a typed truncation, got {other:?}")
+                panic!("cut after {cut} bytes must be the truncation error, got {other:?}")
             }
         }
     }
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-9"></a>
### src/tree/mirror/handshake/tests.rs `@@ -179,14 +179,14 @@ fn magic_mismatch_is_diagnosed_first() {`

```diff
@@ -179,14 +179,14 @@ fn magic_mismatch_is_diagnosed_first() {
             &result,
             Err(Error::MagicMismatch { remote_magic }) if remote_magic == b"SROMUR",
         ),
-        "expected the magic's typed rejection, got {result:?}",
+        "expected the magic's rejection, got {result:?}",
     );
 }
 
 /// A wrong wire version is diagnosed before the semantic fields.
 ///
 /// With a correct opening but a foreign version, the item's (invalid)
-/// intent must never be reached: the typed rejection is
+/// intent must never be reached: the rejection is
 /// [`Error::VersionMismatch`] carrying the remote's declared version, so a
 /// dialect skew is reported as such rather than as a garbled body.
 #[test]
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-10"></a>
### src/tree/mirror/handshake/tests.rs `@@ -203,7 +203,7 @@ fn version_mismatch_is_diagnosed_before_intent() {`

```diff
@@ -203,7 +203,7 @@ fn version_mismatch_is_diagnosed_before_intent() {
                 local_protocol: Protocol::V2,
             }),
         ),
-        "expected the version's typed rejection",
+        "expected the version's rejection",
     );
 }
 
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-11"></a>
### src/tree/mirror/handshake/tests.rs `@@ -211,7 +211,7 @@ proptest! {`

```diff
@@ -211,7 +211,7 @@ proptest! {
     /// Any complete V2 preamble whose fields are canonically spelled
     /// decodes exactly as the field-by-field oracle predicts.
     ///
-    /// The prediction: a typed error naming the first invalid field in
+    /// The prediction: an error naming the first invalid field in
     /// diagnostic order, or the valid preamble — never a panic.
     #[test]
     fn arbitrary_preamble_decodes_by_the_oracle(
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-12"></a>
### src/tree/mirror/handshake/tests.rs `@@ -254,7 +254,7 @@ proptest! {`

```diff
@@ -254,7 +254,7 @@ proptest! {
         prop_assert!(as_oracle, "decode disagreed with the oracle: {:?}", result);
     }
 
-    /// Arbitrary bytes in the preamble's place decode to a typed error or
+    /// Arbitrary bytes in the preamble's place decode to an error or
     /// a valid preamble, never a panic: the parser is total over its
     /// fixed-width input.
     #[test]
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-13"></a>
### src/tree/mirror/handshake/tests.rs `@@ -333,7 +333,7 @@ fn widened_version_spelling_is_the_version_defect() {`

```diff
@@ -333,7 +333,7 @@ fn widened_version_spelling_is_the_version_defect() {
     );
 }
 
-/// A network item that is not a 16-byte byte string is the typed network
+/// A network item that is not a 16-byte byte string is the network
 /// defect: a byte-string head declaring 17 bytes fails the length filter,
 /// and the defect names the network field.
 #[test]
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 7. 'a typed truncation' twice became 'the truncation error', the noun the sentence needed once 'typed' went.

<a id="hunk-14"></a>
### src/tree/mirror/streaming/driver.rs `@@ -171,7 +171,7 @@ where`

```diff
@@ -171,7 +171,7 @@ where
     }
 }
 
-/// Divert one producer's typed errors while forwarding its responses.
+/// Divert one producer's errors while forwarding its responses.
 ///
 /// On error the stream parks rather than ending, because EOF means successful
 /// phase completion to its consumer. [`race_session`] observes the routed
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<a id="hunk-15"></a>
### src/tree/mirror/streaming/materialized.rs `@@ -104,11 +104,10 @@ use std::sync::atomic::{AtomicU64, Ordering};`

```diff
@@ -104,11 +104,10 @@ use std::sync::atomic::{AtomicU64, Ordering};
 
 use crate::message::PayloadDepthLimit;
 use crate::tree::{
-    mirror::contained,
     mirror::streaming::{
         Backend, ErasedNode, Leaf, Root,
         erased::{self, Reaction, Reply},
-        materialized::work::Work,
+        materialized::work::{Resolver, Work},
         message::Greeting,
         protocol::{self, BoxResponses, Requests},
         remote::DEFAULT_TARGET_MESSAGE_SIZE,
```

<!-- annotation -->
> **materialized-14** (T40), line 106:
>
> Deletion of the `contained` import: the hand-written arm was its only use here; the containment check now runs inside the Resolver's supply arm.

<!-- annotation -->
> **materialized-14** (T40), line 110:
>
> `Resolver` joins `Work` in the import from `work`.

<a id="hunk-16"></a>
### src/tree/mirror/streaming/materialized.rs `@@ -856,9 +855,14 @@ where`

```diff
@@ -856,9 +855,14 @@ where
 /// final [`Reply`] and pass its provision up, prefix-less, like every
 /// return.
 ///
-/// Each absorbed leaf is content this replica just learned, credited as
-/// [`messages_gained`](crate::SessionStats::messages_gained) exactly like
-/// the resolver's supply arm.
+/// The reply is classified by the [`Resolver`] every descending stage
+/// uses, over a request that holds nothing. One rule is the terminal
+/// leg's own: a leaf request names a single radix where a scope request
+/// names a subtree, so a supply at any other radix is out of order
+/// ([`Violation::InvalidSupply`]). Each absorbed leaf is content this
+/// replica just learned, credited as
+/// [`messages_gained`](crate::SessionStats::messages_gained) by the
+/// resolver's supply arm.
 async fn absorb<B>(
     their_version: Version,
     ledger: SupplyLedger,
```

<!-- annotation -->
> **materialized-14** (T40), line 858:
>
> The doc states the one rule the terminal leg keeps for itself and why the Resolver cannot express it: a leaf request names a radix, the Resolver classifies against a fan. The credit sentence now says the resolver's supply arm does the crediting, because it does (the old arm credited 1 by hand; a leaf's len() is 1, so the count is unchanged).

<a id="hunk-17"></a>
### src/tree/mirror/streaming/materialized.rs `@@ -876,24 +880,38 @@ where`

```diff
@@ -876,24 +880,38 @@ where
             return violation(Violation::UnansweredQuery);
         };
 
-        // The last radix of the prefix is the one we expect to be supplied.
-        let (_, expected) = prefix.pop();
-
-        // Only if we received exactly that radix paired with a leaf whose
-        // version the sender's declared version contains, do we absorb it.
-        let supply = match replies.as_slice() {
-            [] => None,
-            [Reaction::Supply(radix, leaf)] if *radix == expected => {
-                if !contained(leaf.span().hi(), &their_version) {
-                    return violation(Violation::UncontainedSupply);
-                }
-                ledger.absorb(leaf.len() as u64)?;
-                stats.gained(1);
-                Some(leaf.clone())
+        // The request names one leaf: its scope is the parent, and the
+        // radix it asks for is the path's last byte.
+        let (scope, expected) = prefix.pop();
+        let request = Query {
+            prefix: scope.erase(),
+            ours: Vec::new(),
+        };
+        let mut resolver = Resolver::<B>::new(request, &their_version, &ledger, stats.clone());
+        for reaction in replies {
+            if let Reaction::Supply(radix, _) = &reaction
+                && *radix != expected
+            {
+                return violation(Violation::InvalidSupply);
+            }
+            let routed = resolver.react(reaction)?;
+            debug_assert!(
+                routed.is_none(),
+                "a request that holds nothing routes no query"
+            );
+        }
+        let Resolution { mut resolved, .. } = resolver.finish()?;
+        let supply = match resolved.pop() {
+            None => None,
+            Some((_, Resolve::Ready(leaf))) => leaf,
+            Some((_, Resolve::Pending)) => {
+                unreachable!("a request that holds nothing routes no query, so no slot is pending")
             }
-            [Reaction::Supply(_, _)] => return violation(Violation::InvalidSupply),
-            _ => return violation(Violation::UnfinishedReply),
         };
+        debug_assert!(
+            resolved.is_empty(),
+            "one radix is admitted, and the resolver rejects its duplicate"
+        );
 
         // Then we send that (optional) leaf upwards.
         if returns.send(supply).await.is_err() {
```

<!-- annotation -->
> **materialized-14** (T40), line 883:
>
> The 'alternatively' branch of the resolution, tried first per T40's amendment: the reply is fed reaction by reaction to a Resolver over `Query { prefix: parent, ours: [] }`, so Match -> UnexpectedMatch, Query -> UnexpectedQuery, a duplicate radix -> InvalidSupply, containment, the ledger, and the stats credit are all the Resolver's. The terminal-only rule sits ahead of `react`: a supply at any radix but the requested one is InvalidSupply, as the old arm already classified `[Supply(other)]`; it precedes containment so structure is judged before content, the Resolver's own order. Unpacking the Resolution: `pop()` yields the accepted leaf by move (no clone, the acceptance criterion), the `Pending` arm is `unreachable!` because an empty fan routes no query (the mutants policy's form for a truly unreachable branch), and a debug_assert pins that at most one slot exists (one radix admitted, its duplicate rejected). Rejected alternative: per-arm reclassification (the resolution's primary text) would restate the Resolver's containment/ledger/credit logic a second time, the duplication the goal is against.

<!-- annotation -->
> **materialized-14** (T40), line 909:
>
> Deletion: the `_ => UnfinishedReply` catch-all and the wrong-radix arm are gone; the hunk's remaining lines are the unchanged pass-up.

<!-- annotation -->
> **materialized-14** (T40), line 911:
>
> rustfmt re-flow of the unchanged pass-up after the block above it changed shape; no semantic change.

<a id="hunk-18"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -1,21 +1,22 @@`

```diff
@@ -1,21 +1,22 @@
-//! The initiator's terminal absorb loop: the [`Completing`](super::Completing)
-//! seam's containment enforcement.
+//! The initiator's terminal absorb loop: the closing leg's classification
+//! of every reply it can receive.
 //!
-//! The session's closing leg is the one ingress the descent walks never
+//! The session's closing leg is the one ingress the descending walks never
 //! see: the initiator's pending leaf requests are answered directly by the
 //! counterparty's terminal supplies and absorbed by [`absorb`](super::absorb),
-//! so its containment check is a chokepoint of its own and gets its own
-//! scripted counterparty here.
+//! so it gets its own scripted counterparty here, for the accepted shape
+//! and for every malformed one.
 
 use std::convert::Infallible;
 
 use futures::stream;
+use proptest::prelude::*;
 
 use super::{
     Error, SupplyLedger, Violation, absorb,
-    channel::{QueueKind, QueueRole, channel},
+    channel::{QueueKind, QueueRole, channel, with_schedule},
 };
-use crate::tree::mirror::streaming::erased;
+use crate::tree::mirror::streaming::erased::{Reaction, Reply};
 use crate::tree::mirror::streaming::stats::Recorder;
 use crate::{
     Version,
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-19"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -30,6 +31,13 @@ use crate::{`

```diff
@@ -30,6 +31,13 @@ use crate::{
     },
 };
 
+/// The in-memory backend's erased node representation, which the closing
+/// leg's replies carry.
+type Erased = <Local as Backend>::Erased;
+
+/// The radix the one scripted request asks for: the last byte of its path.
+const REQUESTED: u8 = 0;
+
 /// One tick on the disjoint party `index` (see [`nth_party`]).
 fn ticked(index: usize) -> Version {
     let mut version = Version::new();
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-20"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -37,11 +45,17 @@ fn ticked(index: usize) -> Version {`

```diff
@@ -37,11 +45,17 @@ fn ticked(index: usize) -> Version {
     version
 }
 
-/// Drive [`absorb`](super::absorb) against one scripted closing-leg reply.
+/// A leaf supply at `radix` carrying `version`.
+fn supply(radix: u8, version: Version) -> Reaction<Erased> {
+    let leaf = typed::Node::leaf(version, Message::new(()));
+    Reaction::Supply(radix, <Local as Backend>::erase(leaf))
+}
+
+/// Drive [`absorb`](super::absorb) against one scripted closing leg.
 ///
-/// A single pending leaf request, answered by a single leaf supply carrying
-/// `leaf_version`, from a counterparty whose greeting declared `declared`
-/// and whose set-length ledger is `ledger`.
+/// A single pending leaf request at radix [`REQUESTED`], answered by the
+/// scripted `replies`, from a counterparty whose greeting declared
+/// `declared` and whose set-length ledger is `ledger`.
 ///
 /// Returns the loop's result and what, if anything, it passed up to the
 /// assembly above it.
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-21"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -49,33 +63,28 @@ fn ticked(index: usize) -> Version {`

```diff
@@ -49,33 +63,28 @@ fn ticked(index: usize) -> Version {
 fn absorb_scripted(
     declared: Version,
     ledger: SupplyLedger,
-    leaf_version: Version,
+    replies: Vec<Reply<Erased>>,
 ) -> (
     Result<(), Error<Infallible>>,
     Option<Option<typed::Node<Z>>>,
 ) {
-    // The request whose answer the script supplies: the leaf radix is the
-    // path's last byte, zero here.
-    let path = Path::from([0u8; 32]);
+    let mut bytes = [0u8; 32];
+    bytes[31] = REQUESTED;
+    let path = Path::from(bytes);
     let (queries, queries_rx) =
         channel::<Prefix<Z>>(QueueRole::new(QueueKind::LeafRequests, Z::HEIGHT), 1);
     pollster::block_on(queries.send(Prefix::containing(&path))).expect("the loop is live");
     drop(queries);
 
-    let (returns, mut returns_rx) = channel::<Option<<Local as Backend>::Erased>>(
+    let (returns, mut returns_rx) = channel::<Option<Erased>>(
         QueueRole::new(QueueKind::TerminalLeafResolutions, Z::HEIGHT),
         1,
     );
 
-    let leaf = typed::Node::leaf(leaf_version, Message::new(()));
-    let requests = stream::iter(vec![erased::Reply {
-        replies: vec![erased::Reaction::Supply(0, <Local as Backend>::erase(leaf))],
-    }]);
-
     let result = pollster::block_on(absorb::<Local>(
         declared,
         ledger,
-        requests,
+        stream::iter(replies),
         queries_rx,
         returns,
         Recorder::default(),
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-22"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -85,18 +94,25 @@ fn absorb_scripted(`

```diff
@@ -85,18 +94,25 @@ fn absorb_scripted(
     (result, returned)
 }
 
+/// The accepted shape: the one requested leaf, supplied once.
+fn requested(version: Version) -> Vec<Reply<Erased>> {
+    vec![Reply {
+        replies: vec![supply(REQUESTED, version)],
+    }]
+}
+
 /// A terminal leaf supply whose version the declared greeting version
 /// contains is absorbed and passed up to the assembly.
 ///
-/// The happy path that keeps the rejections below honest: the scripted
-/// shape differs from theirs only in the supplied version.
+/// The accepted shape, from which the rejections below differ only in the
+/// scripted reply.
 #[test]
 fn terminal_absorb_accepts_a_contained_supply() {
     let declared = ticked(0);
     let (result, returned) = absorb_scripted(
         declared.clone(),
         SupplyLedger::new(u64::MAX),
-        declared.clone(),
+        requested(declared.clone()),
     );
     assert!(result.is_ok(), "a contained supply is absorbed: {result:?}");
     let leaf = returned
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-23"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -120,7 +136,8 @@ fn terminal_absorb_rejects_a_dominating_supply() {`

```diff
@@ -120,7 +136,8 @@ fn terminal_absorb_rejects_a_dominating_supply() {
     let declared = ticked(0);
     let mut escaped = declared.clone();
     escaped.tick(&nth_party(0));
-    let (result, returned) = absorb_scripted(declared, SupplyLedger::new(u64::MAX), escaped);
+    let (result, returned) =
+        absorb_scripted(declared, SupplyLedger::new(u64::MAX), requested(escaped));
     assert!(
         matches!(result, Err(Error::Violation(Violation::UncontainedSupply))),
         "a dominating supply is rejected: {result:?}",
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-24"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -138,7 +155,8 @@ fn terminal_absorb_rejects_a_dominating_supply() {`

```diff
@@ -138,7 +155,8 @@ fn terminal_absorb_rejects_a_dominating_supply() {
 fn terminal_absorb_rejects_an_incomparable_supply() {
     let declared = ticked(0);
     let escaped = ticked(31);
-    let (result, returned) = absorb_scripted(declared, SupplyLedger::new(u64::MAX), escaped);
+    let (result, returned) =
+        absorb_scripted(declared, SupplyLedger::new(u64::MAX), requested(escaped));
     assert!(
         matches!(result, Err(Error::Violation(Violation::UncontainedSupply))),
         "an incomparable supply is rejected: {result:?}",
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-25"></a>
### src/tree/mirror/streaming/materialized/tests.rs `@@ -152,15 +170,124 @@ fn terminal_absorb_rejects_an_incomparable_supply() {`

```diff
@@ -152,15 +170,124 @@ fn terminal_absorb_rejects_an_incomparable_supply() {
 /// The closing leg is the one ingress the connected greeting-lie family
 /// cannot reach: an empty declaration trips at the session's first
 /// absorbed supply, never at a terminal leaf, so the terminal arm of the
-/// set-length guard is pinned here at its own seam — a spent ledger, one
-/// contained honest leaf.
+/// set-length guard is pinned here directly: a spent ledger, one contained
+/// leaf.
 #[test]
 fn terminal_absorb_rejects_an_overdrawn_supply() {
     let declared = ticked(0);
-    let (result, returned) = absorb_scripted(declared.clone(), SupplyLedger::new(0), declared);
+    let (result, returned) =
+        absorb_scripted(declared.clone(), SupplyLedger::new(0), requested(declared));
     assert!(
         matches!(result, Err(Error::Violation(Violation::OverdrawnSupply))),
         "a supply past the declared set length is rejected: {result:?}",
     );
     assert!(returned.is_none(), "nothing is passed up past a rejection");
 }
+
+/// One deliberately malformed closing-leg script and the exact violation
+/// it must surface.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+enum Injection {
+    /// The reply stream ends with the request outstanding.
+    UnansweredQuery,
+    /// A second reply follows the one the request claimed.
+    UnaskedReply,
+    /// A `Match`, leading or after the requested supply.
+    Match { after_supply: bool },
+    /// A `Query`, leading or after the requested supply.
+    Query { after_supply: bool },
+    /// The requested radix supplied twice.
+    DuplicateSupply,
+    /// A supply at a radix nobody requested, alone or after the requested
+    /// one.
+    ForeignSupply { after_supply: bool },
+}
+
+impl Injection {
+    fn expected(self) -> Violation {
+        match self {
+            Self::UnansweredQuery => Violation::UnansweredQuery,
+            Self::UnaskedReply => Violation::UnaskedReply,
+            Self::Match { .. } => Violation::UnexpectedMatch,
+            Self::Query { .. } => Violation::UnexpectedQuery,
+            Self::DuplicateSupply | Self::ForeignSupply { .. } => Violation::InvalidSupply,
+        }
+    }
+
+    /// The replies answering the one request, every supply carrying the
+    /// contained `version`.
+    fn script(self, version: Version) -> Vec<Reply<Erased>> {
+        let requested = || supply(REQUESTED, version.clone());
+        let one = |after_supply: bool, reaction: Reaction<Erased>| {
+            let mut replies = Vec::new();
+            if after_supply {
+                replies.push(requested());
+            }
+            replies.push(reaction);
+            vec![Reply { replies }]
+        };
+        match self {
+            Self::UnansweredQuery => Vec::new(),
+            Self::UnaskedReply => vec![
+                Reply {
+                    replies: vec![requested()],
+                },
+                Reply {
+                    replies: Vec::new(),
+                },
+            ],
+            Self::Match { after_supply } => one(after_supply, Reaction::Match),
+            Self::Query { after_supply } => one(after_supply, Reaction::Query(Vec::new())),
+            Self::DuplicateSupply => one(true, requested()),
+            Self::ForeignSupply { after_supply } => {
+                one(after_supply, supply(REQUESTED + 1, version.clone()))
+            }
+        }
+    }
+}
+
+fn arb_injection() -> impl Strategy<Value = Injection> {
+    prop_oneof![
+        Just(Injection::UnansweredQuery),
+        Just(Injection::UnaskedReply),
+        any::<bool>().prop_map(|after_supply| Injection::Match { after_supply }),
+        any::<bool>().prop_map(|after_supply| Injection::Query { after_supply }),
+        Just(Injection::DuplicateSupply),
+        any::<bool>().prop_map(|after_supply| Injection::ForeignSupply { after_supply }),
+    ]
+}
+
+proptest! {
+    /// Every malformed closing-leg reply is reported as its exact public
+    /// `Violation`, under arbitrary channel poll order.
+    ///
+    /// The closing leg classifies through the resolver every descending
+    /// stage uses; its one rule of its own is that a leaf request names a
+    /// single radix, so a supply at any other radix is out of order. Only
+    /// the reply that trails an accepted one passes anything up: every
+    /// other rejection precedes the pass-up.
+    #[test]
+    fn terminal_absorb_reports_exact_violation(
+        injection in arb_injection(),
+        schedule in proptest::collection::vec(0u8..=2, 0..=64),
+    ) {
+        let declared = ticked(0);
+        let (result, returned) = with_schedule(schedule, || {
+            absorb_scripted(
+                declared.clone(),
+                SupplyLedger::new(u64::MAX),
+                injection.script(declared.clone()),
+            )
+        });
+        let expected = injection.expected();
+        prop_assert!(
+            matches!(result, Err(Error::Violation(actual)) if actual == expected),
+            "{injection:?} reported {result:?}, expected {expected:?}",
+        );
+        prop_assert_eq!(
+            returned.is_some(),
+            injection == Injection::UnaskedReply,
+            "only the accepted reply ahead of an unasked one is passed up",
+        );
+    }
+}
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> The absorb script is generalized from one hard-coded supply to a scripted reply list so one harness drives the accepted shape and every malformed one; the four terminal_absorb_* tests keep their names and assertions, calling it through `requested`. The new proptest family is the entry's Construction line made committed: the absorb_scripted shapes `[Match]`, `[Query]`, `[Supply(0), Supply(0)]`, `[Supply(1)]` plus `[Supply(0), Match]`, `[Supply(0), Query]`, `[Supply(0), Supply(1)]`, and the two stream shapes, under `with_schedule`. On the old arms it fails at the first case (`Match { after_supply: false } reported Err(Violation(UnfinishedReply)), expected UnexpectedMatch`). The pass-up assertion is exact rather than 'nothing passed up': the unasked-reply shape necessarily passes the accepted first answer up before the extra reply is seen. Prose: the module doc no longer names the `Completing` type or a 'seam' (T49), 'honest' is out of the two test docs (T50), em-dashes replaced in the paragraphs touched.

<a id="hunk-26"></a>
### src/tree/mirror/streaming/materialized/work.rs `@@ -23,6 +23,8 @@ mod levels;`

```diff
@@ -23,6 +23,8 @@ mod levels;
 mod queues;
 mod resolver;
 
+pub(super) use resolver::Resolver;
+
 #[cfg(test)]
 use super::{progress, transcript};
 use crate::tree::{
```

<!-- annotation -->
> **materialized-14** (T40), line 26:
>
> The terminal leg lives in materialized.rs, outside `work`, so the classifier is re-exported one level up (pub-in-private is the recorded convention, T46); nothing else outside `work` uses it.

<a id="hunk-27"></a>
### src/tree/mirror/streaming/materialized/work/levels.rs `@@ -162,15 +162,14 @@ where`

```diff
@@ -162,15 +162,14 @@ where
     /// the root's children, so this stage starts from that listing rather
     /// than exploding the root itself.
     ///
-    /// The opening request may trail *early supplies* behind its query: the
-    /// initiator's exclusive root children, shipped whole without waiting to
-    /// be asked. The merge-join below still emits its Right-arm empty
-    /// queries for them — the reply/question pairing on every stream is
-    /// untouched — and the supplied nodes are exploded into their children
-    /// here (this stage's one reply is the only thing a failure can strand)
-    /// and handed to the next level through the returned channel, where
-    /// they resolve those queries' now-empty answers without touching the
-    /// backend mid-loop.
+    /// The opening reply is the root question followed by the initiator's
+    /// *early supplies*: its exclusive root children, shipped whole without
+    /// waiting to be asked. They pass the checks every solicited supply
+    /// passes (ascending radices this side does not hold, containment, the
+    /// ledger), are exploded into their children here, ahead of the yield,
+    /// and are handed to the next level through the returned channel,
+    /// where they answer the merge-join's now-empty queries for those
+    /// radices without touching the backend mid-loop.
     #[allow(clippy::type_complexity)]
     pub fn responder_level(
         &mut self,
```

<!-- annotation -->
> **materialized-14** (T40), line 165:
>
> Doc paragraph of `responder_level` rewritten to state the opening reply's grammar and the checks its early supplies pass, at the reader's altitude; the parenthetical about what a failure 'can strand' is gone (the liveness at this position is now pinned by the wire probe, and the sentence explained nothing a caller holds). Em-dashes replaced.

<a id="hunk-28"></a>
### src/tree/mirror/streaming/materialized/work/levels.rs `@@ -207,28 +206,56 @@ where`

```diff
@@ -207,28 +206,56 @@ where
             let Some(Reply { replies }) = requests.next().await else {
                 return violation(Violation::UnansweredQuery)?;
             };
-            let mut reactions = replies.into_iter();
-            let Some(Reaction::Query(theirs)) = reactions.next() else {
-                return violation(Violation::UnexpectedQuery)?;
-            };
+            // The opening reply is not paired positionally against the
+            // held fan, so the resolver cannot classify it; its arms are
+            // its own, and the first offending reaction names the fault.
+            // A supply is judged as an early supply wherever it sits,
+            // with the resolver's structural checks first (ascending
+            // radices, none this side holds), and one ahead of the root
+            // question is out of order.
+            let mut theirs = None;
+            let mut last = None;
             let mut early = Vec::new();
-            for reaction in reactions {
-                let Reaction::Supply(radix, node) = reaction else {
-                    return violation(Violation::UnexpectedQuery)?;
-                };
-                // Early supplies are absorbed here, ahead of the descent's
-                // resolver, so they pass the same containment and ledger
-                // checks every other supply does. The ledger charges at
-                // this wire ingestion, never at the claim downstream: the
-                // claim consumes nodes already counted here.
-                if !contained(node.span().hi(), &their_version) {
-                    return violation(Violation::UncontainedSupply)?;
+            for reaction in replies {
+                match reaction {
+                    Reaction::Query(listing) => {
+                        if theirs.is_some() {
+                            return violation(Violation::UnexpectedQuery)?;
+                        }
+                        theirs = Some(listing);
+                    }
+                    Reaction::Match => return violation(Violation::UnexpectedMatch)?,
+                    Reaction::Supply(radix, node) => {
+                        if last.is_some_and(|last| radix <= last) {
+                            return violation(Violation::InvalidSupply)?;
+                        }
+                        if fan.iter().any(|(held, _)| *held == radix) {
+                            return violation(Violation::UnexpectedSupply)?;
+                        }
+                        if theirs.is_none() {
+                            return violation(Violation::InvalidSupply)?;
+                        }
+                        // Early supplies are absorbed here, ahead of the
+                        // descent's resolver, so they pass the same
+                        // containment and ledger checks every other supply
+                        // does. The ledger charges at this wire ingestion,
+                        // never at the claim downstream: the claim consumes
+                        // nodes already counted here.
+                        if !contained(node.span().hi(), &their_version) {
+                            return violation(Violation::UncontainedSupply)?;
+                        }
+                        ledger.absorb(node.len() as u64)?;
+                        let children =
+                            erased::ops::children_of(&backend, root_scope.push(radix), node)
+                                .await?;
+                        last = Some(radix);
+                        early.push((radix, children));
+                    }
                 }
-                ledger.absorb(node.len() as u64)?;
-                let children =
-                    erased::ops::children_of(&backend, root_scope.push(radix), node).await?;
-                early.push((radix, children));
             }
+            let Some(theirs) = theirs else {
+                return violation(Violation::UnfinishedReply)?;
+            };
             // Filled before this reply yields, so the level consuming it
             // never waits: its first query cannot arrive earlier.
             let _ = early_tx.send(early);
```

<!-- annotation -->
> **materialized-14** (T40), line 209:
>
> The opening leg keeps its own arms: per-arm reclassification, the brief's fallback. Why the Resolver cannot serve here: it pairs reactions positionally against the held fan, but the opening reply is one Query about the root scope itself followed by supplies at initiator-exclusive radices, with no positional reaction for the responder's held children (the merge-join answers those on this side), so fed to a Resolver over the fan the root Query would pair with the first held child and every early supply past a held radix would read as InvalidSupply. The comment says so at the site.

<!-- annotation -->
> **materialized-14** (T40), line 219:
>
> The arms, judged in reply order so the first offending reaction names the fault: a second Query -> UnexpectedQuery (was the catch-all); Match anywhere -> UnexpectedMatch (was UnexpectedQuery); a supply is checked in the Resolver's order, ascending radix first (InvalidSupply), then not held (UnexpectedSupply), then, only for a supply ahead of the question, InvalidSupply as out of order; then containment, ledger, explode. Judgment call: a leading supply is judged by its radix before the missing question is held against it, so the connected suite's UnexpectedSupply corruption (a held-radix supply inserted at the front) classifies as UnexpectedSupply at the opening, matching what it means at every other height.

<!-- annotation -->
> **materialized-14** (T40), line 254:
>
> Deletion: the old loop body, whose containment and ledger checks moved into the Supply arm above unchanged.

<!-- annotation -->
> **materialized-14** (T40), line 256:
>
> A reply with no root question is UnfinishedReply, per the resolution (the reply ended before reacting to the one thing it owed); the resolution's 'documented new variant' alternative is a stop the amendment closes, and no new variant was needed.

<a id="hunk-29"></a>
### src/tree/mirror/streaming/materialized/work/levels.rs `@@ -245,6 +272,10 @@ where`

```diff
@@ -245,6 +272,10 @@ where
                 };
                 asked => next_queries;
             );
+
+            if requests.next().await.is_some() {
+                return violation(Violation::UnaskedReply)?;
+            }
         };
 
         let (returns, returns_rx) = responder_root_returns::<B>();
```

<!-- annotation -->
> **materialized-14** (T40), line 275:
>
> The trailing `requests.next()` check every descending walk performs, placed after the yield block. Liveness argument: the opening stream's end now waits for the initiator's opening stream to end, which happens once its one reply and root query are out; that end does not depend on anything this stage publishes, so no new wait edge closes a cycle, and no yield point moves (`yield_resolve_query!` is untouched).

<a id="hunk-30"></a>
### src/tree/mirror/streaming/materialized/work/tests/violations.rs `@@ -21,8 +21,8 @@ use crate::{`

```diff
@@ -21,8 +21,8 @@ use crate::{
         window::Window,
     },
     tree::typed::{
-        self, Path, Prefix,
-        height::{Height, S, Z},
+        self, Hash, Path, Prefix,
+        height::{Height, S, UnderRoot, Z},
     },
 };
 
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Opening-leg injections, as the resolution asks. `reported_violation` gains a count of well-formed replies the script answers before its fault (0 at every query height; 1 for the opening's unasked-reply shape, whose extra reply necessarily trails the opening's own answer), so a well-formed reply ahead of the fault is asserted answered rather than mistaken for acceptance. The opening family is its own proptest rather than a 33rd height of `injected_fault_reports_exact_violation`: the opening's reply grammar (one root Query, then supplies) is not a query height's (positional reactions over a held fan), so the script differs in kind. Twelve shapes, one per clause of the grammar, including both positions of a leading supply and the ledger arm. On the old arms it fails at the first case (`the malformed reply produced a successful response`, DescendingSupplies at radixes {0}: the out-of-order supplies were absorbed and parked).

<a id="hunk-31"></a>
### src/tree/mirror/streaming/materialized/work/tests/violations.rs `@@ -215,15 +215,29 @@ where`

```diff
@@ -215,15 +215,29 @@ where
 }
 
 /// Drive a walk's response pump until it surfaces the injected violation.
+///
+/// `well_formed` is the number of replies the script answers before its
+/// fault: each must be answered, and the violation must follow the last.
 fn reported_violation<H: Height>(
     work: Work<Local>,
     mut responses: BoxResponses<Local, H, Error<Infallible>>,
+    well_formed: usize,
 ) -> Violation {
     let response = pollster::block_on(async move {
         let drive = work.execute(Box::pin(std::future::pending::<
             Result<(), Error<Infallible>>,
         >()));
         tokio::pin!(drive);
+        for _ in 0..well_formed {
+            let answered = tokio::select! {
+                response = responses.next() => response,
+                result = &mut drive => panic!("the pending driver unexpectedly completed: {result:?}"),
+            };
+            assert!(
+                matches!(answered, Some(Ok(_))),
+                "a well-formed reply ahead of the fault was not answered",
+            );
+        }
         tokio::select! {
             response = responses.next() => response,
             result = &mut drive => panic!("the pending driver unexpectedly completed: {result:?}"),
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Opening-leg injections, as the resolution asks. `reported_violation` gains a count of well-formed replies the script answers before its fault (0 at every query height; 1 for the opening's unasked-reply shape, whose extra reply necessarily trails the opening's own answer), so a well-formed reply ahead of the fault is asserted answered rather than mistaken for acceptance. The opening family is its own proptest rather than a 33rd height of `injected_fault_reports_exact_violation`: the opening's reply grammar (one root Query, then supplies) is not a query height's (positional reactions over a held fan), so the script differs in kind. Twelve shapes, one per clause of the grammar, including both positions of a leading supply and the ledger arm. On the old arms it fails at the first case (`the malformed reply produced a successful response`, DescendingSupplies at radixes {0}: the out-of-order supplies were absorbed and parked).

<a id="hunk-32"></a>
### src/tree/mirror/streaming/materialized/work/tests/violations.rs `@@ -254,7 +268,7 @@ impl InjectHeight for Z {`

```diff
@@ -254,7 +268,7 @@ impl InjectHeight for Z {
             stream::iter(requests),
             queries,
         );
-        reported_violation(work, responses)
+        reported_violation(work, responses, 0)
     }
 }
 
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Opening-leg injections, as the resolution asks. `reported_violation` gains a count of well-formed replies the script answers before its fault (0 at every query height; 1 for the opening's unasked-reply shape, whose extra reply necessarily trails the opening's own answer), so a well-formed reply ahead of the fault is asserted answered rather than mistaken for acceptance. The opening family is its own proptest rather than a 33rd height of `injected_fault_reports_exact_violation`: the opening's reply grammar (one root Query, then supplies) is not a query height's (positional reactions over a held fan), so the script differs in kind. Twelve shapes, one per clause of the grammar, including both positions of a leading supply and the ledger arm. On the old arms it fails at the first case (`the malformed reply produced a successful response`, DescendingSupplies at radixes {0}: the out-of-order supplies were absorbed and parked).

<a id="hunk-33"></a>
### src/tree/mirror/streaming/materialized/work/tests/violations.rs `@@ -269,7 +283,7 @@ impl InjectHeight for S<Z> {`

```diff
@@ -269,7 +283,7 @@ impl InjectHeight for S<Z> {
             stream::iter(requests),
             queries,
         );
-        reported_violation(work, responses)
+        reported_violation(work, responses, 0)
     }
 }
 
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Opening-leg injections, as the resolution asks. `reported_violation` gains a count of well-formed replies the script answers before its fault (0 at every query height; 1 for the opening's unasked-reply shape, whose extra reply necessarily trails the opening's own answer), so a well-formed reply ahead of the fault is asserted answered rather than mistaken for acceptance. The opening family is its own proptest rather than a 33rd height of `injected_fault_reports_exact_violation`: the opening's reply grammar (one root Query, then supplies) is not a query height's (positional reactions over a held fan), so the script differs in kind. Twelve shapes, one per clause of the grammar, including both positions of a leading supply and the ledger arm. On the old arms it fails at the first case (`the malformed reply produced a successful response`, DescendingSupplies at radixes {0}: the out-of-order supplies were absorbed and parked).

<a id="hunk-34"></a>
### src/tree/mirror/streaming/materialized/work/tests/violations.rs `@@ -292,7 +306,7 @@ where`

```diff
@@ -292,7 +306,7 @@ where
             stream::iter(requests),
             queries,
         );
-        reported_violation(work, responses)
+        reported_violation(work, responses, 0)
     }
 }
 
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Opening-leg injections, as the resolution asks. `reported_violation` gains a count of well-formed replies the script answers before its fault (0 at every query height; 1 for the opening's unasked-reply shape, whose extra reply necessarily trails the opening's own answer), so a well-formed reply ahead of the fault is asserted answered rather than mistaken for acceptance. The opening family is its own proptest rather than a 33rd height of `injected_fault_reports_exact_violation`: the opening's reply grammar (one root Query, then supplies) is not a query height's (positional reactions over a held fan), so the script differs in kind. Twelve shapes, one per clause of the grammar, including both positions of a leading supply and the ledger arm. On the old arms it fails at the first case (`the malformed reply produced a successful response`, DescendingSupplies at radixes {0}: the out-of-order supplies were absorbed and parked).

<a id="hunk-35"></a>
### src/tree/mirror/streaming/materialized/work/tests/violations.rs `@@ -349,4 +363,173 @@ proptest! {`

```diff
@@ -349,4 +363,173 @@ proptest! {
             prop_assert_eq!(actual, expected, "query height {}", height);
         }
     }
+
+    /// Every malformed opening reply is reported as its exact public
+    /// `Violation`.
+    ///
+    /// The responder's opening leg reads the root question and the
+    /// initiator's early supplies with arms of its own: the opening reply
+    /// is not paired positionally against the held fan, so the resolver
+    /// cannot classify it. An arbitrary held fan and channel poll order
+    /// pin those arms to the taxonomy.
+    #[test]
+    fn opening_fault_reports_exact_violation(
+        injection in arb_opening_injection(),
+        radixes in proptest::collection::btree_set(any::<u8>(), 1..=8),
+        schedule in proptest::collection::vec(0u8..=2, 0..=64),
+    ) {
+        let expected = injection.expected();
+        let actual = with_schedule(schedule, || opening_violation(injection, &radixes));
+        prop_assert_eq!(actual, expected, "{:?}", injection);
+    }
+}
+
+/// One deliberately malformed opening reply and the exact violation it
+/// must surface.
+///
+/// The opening reply is the root question followed by the initiator's
+/// early supplies, in ascending radix order and at radices the responder
+/// does not hold; each shape below breaks one clause of that grammar.
+#[derive(Clone, Copy, Debug)]
+enum OpeningInjection {
+    /// The reply stream ends before the opening reply.
+    UnansweredQuery,
+    /// A second reply follows the opening.
+    UnaskedReply,
+    /// An empty opening reply: no root question.
+    UnfinishedReply,
+    /// A `Match`, ahead of the root question or after it.
+    Match { trailing: bool },
+    /// A supply ahead of the root question, at a radix this side holds or
+    /// at a fresh one.
+    LeadingSupply { held: bool },
+    /// A second `Query` after the root question.
+    TrailingQuery,
+    /// Two early supplies in descending radix order.
+    DescendingSupplies,
+    /// The same early radix supplied twice.
+    DuplicateSupply,
+    /// An early supply at a radix this side holds.
+    HeldSupply,
+    /// An early supply whose version escapes the declared version.
+    UncontainedSupply,
+    /// An early supply past the declared set length.
+    OverdrawnSupply,
+}
+
+impl OpeningInjection {
+    fn expected(self) -> Violation {
+        match self {
+            Self::UnansweredQuery => Violation::UnansweredQuery,
+            Self::UnaskedReply => Violation::UnaskedReply,
+            Self::UnfinishedReply => Violation::UnfinishedReply,
+            Self::Match { .. } => Violation::UnexpectedMatch,
+            Self::LeadingSupply { held: true } | Self::HeldSupply => Violation::UnexpectedSupply,
+            Self::LeadingSupply { held: false }
+            | Self::DescendingSupplies
+            | Self::DuplicateSupply => Violation::InvalidSupply,
+            Self::TrailingQuery => Violation::UnexpectedQuery,
+            Self::UncontainedSupply => Violation::UncontainedSupply,
+            Self::OverdrawnSupply => Violation::OverdrawnSupply,
+        }
+    }
+
+    /// The declared set length the script runs under: spent only where
+    /// the overrun is the fault.
+    fn ledger(self) -> SupplyLedger {
+        match self {
+            Self::OverdrawnSupply => SupplyLedger::new(0),
+            _ => SupplyLedger::new(u64::MAX),
+        }
+    }
+}
+
+fn arb_opening_injection() -> impl Strategy<Value = OpeningInjection> {
+    prop_oneof![
+        Just(OpeningInjection::UnansweredQuery),
+        Just(OpeningInjection::UnaskedReply),
+        Just(OpeningInjection::UnfinishedReply),
+        any::<bool>().prop_map(|trailing| OpeningInjection::Match { trailing }),
+        any::<bool>().prop_map(|held| OpeningInjection::LeadingSupply { held }),
+        Just(OpeningInjection::TrailingQuery),
+        Just(OpeningInjection::DescendingSupplies),
+        Just(OpeningInjection::DuplicateSupply),
+        Just(OpeningInjection::HeldSupply),
+        Just(OpeningInjection::UncontainedSupply),
+        Just(OpeningInjection::OverdrawnSupply),
+    ]
+}
+
+/// Inject one malformed opening reply through the responder's opening leg,
+/// whose root fan holds one child per radix in `radixes`.
+///
+/// The root question matches the held fan exactly, so a script whose
+/// fault lies past the question is answered without a dispute. The
+/// declared version is snapshotted after every contained node is built;
+/// the `UncontainedSupply` script alone ticks past the snapshot.
+fn opening_violation(injection: OpeningInjection, radixes: &BTreeSet<u8>) -> Violation {
+    let mut version = Version::new();
+    let ours = radixes
+        .iter()
+        .map(|&radix| (radix, UnderRoot::node(&mut version)))
+        .collect::<Vec<_>>();
+    let theirs = ours
+        .iter()
+        .map(|(radix, node)| (*radix, node.hash()))
+        .collect::<Vec<(u8, Hash)>>();
+    let fan = ours
+        .into_iter()
+        .map(|(radix, node)| (radix, <Local as Backend>::erase(node)))
+        .collect::<Vec<(u8, Erased)>>();
+    let contained = [UnderRoot::node(&mut version), UnderRoot::node(&mut version)];
+    let declared = version.clone();
+    let escaped = UnderRoot::node(&mut version);
+
+    let held = *radixes.first().expect("the strategy produces a child");
+    let mut fresh = (0..=u8::MAX).filter(|radix| !radixes.contains(radix));
+    let low = fresh.next().expect("at most eight radices are held");
+    let high = fresh.next().expect("at most eight radices are held");
+    let question = || Reaction::Query(theirs.clone());
+    let supply = |radix: u8, index: usize| Reaction::Supply(radix, contained[index].clone());
+    let one = |replies: Vec<Reaction<Local, UnderRoot>>| vec![Reply { replies }];
+    let requests = match injection {
+        OpeningInjection::UnansweredQuery => Vec::new(),
+        OpeningInjection::UnaskedReply => vec![
+            Reply {
+                replies: vec![question()],
+            },
+            Reply {
+                replies: Vec::new(),
+            },
+        ],
+        OpeningInjection::UnfinishedReply => one(Vec::new()),
+        OpeningInjection::Match { trailing: false } => one(vec![Reaction::Match]),
+        OpeningInjection::Match { trailing: true } => one(vec![question(), Reaction::Match]),
+        OpeningInjection::LeadingSupply { held: true } => one(vec![supply(held, 0), question()]),
+        OpeningInjection::LeadingSupply { held: false } => one(vec![supply(low, 0), question()]),
+        OpeningInjection::TrailingQuery => one(vec![question(), Reaction::Query(Vec::new())]),
+        OpeningInjection::DescendingSupplies => {
+            one(vec![question(), supply(high, 0), supply(low, 1)])
+        }
+        OpeningInjection::DuplicateSupply => one(vec![question(), supply(low, 0), supply(low, 1)]),
+        OpeningInjection::HeldSupply => one(vec![question(), supply(held, 0)]),
+        OpeningInjection::UncontainedSupply => {
+            one(vec![question(), Reaction::Supply(low, escaped)])
+        }
+        OpeningInjection::OverdrawnSupply => one(vec![question(), supply(low, 0)]),
+    };
+
+    let mut work = Work::new(Local, Window::FLOOR, Recorder::default());
+    // The channels the leg publishes into stay open for the run: a closed
+    // one ends the leg quietly instead of reporting.
+    let (responses, _asked, _returns, _early, _finish) = work.responder_level(
+        declared.clone(),
+        injection.ledger(),
+        declared,
+        fan,
+        stream::iter(requests),
+    );
+    // The unasked reply trails the opening's well-formed answer.
+    let well_formed = usize::from(matches!(injection, OpeningInjection::UnaskedReply));
+    reported_violation(work, responses, well_formed)
 }
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Opening-leg injections, as the resolution asks. `reported_violation` gains a count of well-formed replies the script answers before its fault (0 at every query height; 1 for the opening's unasked-reply shape, whose extra reply necessarily trails the opening's own answer), so a well-formed reply ahead of the fault is asserted answered rather than mistaken for acceptance. The opening family is its own proptest rather than a 33rd height of `injected_fault_reports_exact_violation`: the opening's reply grammar (one root Query, then supplies) is not a query height's (positional reactions over a held fan), so the script differs in kind. Twelve shapes, one per clause of the grammar, including both positions of a leading supply and the ledger arm. On the old arms it fails at the first case (`the malformed reply produced a successful response`, DescendingSupplies at radixes {0}: the out-of-order supplies were absorbed and parked).

<a id="hunk-36"></a>
### src/tree/mirror/streaming/remote/adapter/tests/malformed.rs `@@ -594,7 +594,7 @@ fn whole_root_supply_reply(cases: &[LeafCase]) -> Vec<Frame> {`

```diff
@@ -594,7 +594,7 @@ fn whole_root_supply_reply(cases: &[LeafCase]) -> Vec<Frame> {
     )]
 }
 
-/// A reply streaming past the declared `set_len` fails typed at its first
+/// A reply streaming past the declared `set_len` returns an error at its first
 /// over-declaration record, under node residency independent of the
 /// overrun; a declaration exactly covering the stream admits it whole.
 ///
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<a id="hunk-37"></a>
### src/tree/mirror/streaming/remote/adapter/tests/opening.rs `@@ -176,7 +176,7 @@ fn opening_supplies_decode_by_radix_group() {`

```diff
@@ -176,7 +176,7 @@ fn opening_supplies_decode_by_radix_group() {
 }
 
 /// The opening-supply reply is held to the declared set length record by
-/// record: the first record past the allowance fails the decode typed,
+/// record: the first record past the allowance fails the decode,
 /// while the one opening reply is still open.
 ///
 /// The same fixture as the radix-group decode above, under an allowance
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1. Kin the grep misses ('fails the decode typed').

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 1.

<a id="hunk-38"></a>
### src/tree/mirror/streaming/remote/adapter/tests/opening.rs `@@ -290,7 +290,7 @@ fn positional_reaction_in_opening_supplies_is_rejected() {`

```diff
@@ -290,7 +290,7 @@ fn positional_reaction_in_opening_supplies_is_rejected() {
 }
 
 /// Every semantic opening shape is either the canonical query-then-supplies
-/// form or its exact typed rejection.
+/// form or its exact rejection.
 #[test]
 fn opening_rejections_are_exhaustive() {
     let empty = Reply::<<Local as Backend>::Erased> {
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1. Kin the grep misses ('fails the decode typed').

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 1.

<a id="hunk-39"></a>
### src/tree/mirror/streaming/remote/codec.rs `@@ -158,7 +158,7 @@ pub(crate) fn lone_record_run(len: usize) -> Vec<u8> {`

```diff
@@ -158,7 +158,7 @@ pub(crate) fn lone_record_run(len: usize) -> Vec<u8> {
 ///
 /// The allocator meter (`tests/decode_alloc.rs`) drives the supply read path
 /// through this to price a supply body in bytes requested from the
-/// allocator; the decoded value is noise to that meter, but the typed error
+/// allocator; the decoded value is noise to that meter, but the error
 /// passes through so the meter can also assert how a failure classified.
 /// The budget is the meter's to choose: the framing ceiling keeps every
 /// well-framed declaration on the body-read path being priced, while a
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<a id="hunk-40"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -100,8 +100,8 @@ fn arb_flow() -> impl Strategy<Value = Flow> {`

```diff
@@ -100,8 +100,8 @@ fn arb_flow() -> impl Strategy<Value = Flow> {
     prop_oneof![Just(Flow::Continue), Just(Flow::End)]
 }
 
-/// The stream constructor rejects an index past the stream range with a
-/// typed error naming the index.
+/// The stream constructor rejects an index past the stream range with an
+/// error naming the index.
 #[test]
 fn out_of_range_stream_index_is_rejected() {
     assert_eq!(
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 5. Two em-dashes in the re-worded sentence became commas.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.

<a id="hunk-41"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -885,8 +885,8 @@ proptest! {`

```diff
@@ -885,8 +885,8 @@ proptest! {
     /// record's heads.
     ///
     /// A multi-record supply frame decodes when its charged wire size is
-    /// within the budget and fails typed as `OverbatchedRun` — carrying
-    /// that wire size and the budget — when it is past it. The rejection
+    /// within the budget and returns `OverbatchedRun`, carrying
+    /// that wire size and the budget, when it is past it. The rejection
     /// is decided ahead of the rest of the body: a stream ending right
     /// after the first record's heads still classifies as the budget
     /// violation, never as a truncation. Both decoders (the async reader
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 5. Two em-dashes in the re-worded sentence became commas.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.

<a id="hunk-42"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -927,11 +927,11 @@ proptest! {`

```diff
@@ -927,11 +927,11 @@ proptest! {
             (stream, Frame::Reaction(Reaction::Supply(expected), flow))
         );
 
-        // Past the budget: typed rejection naming the frame and the budget.
+        // Past the budget: a rejection naming the frame and the budget.
         let over = RunBudget::from_bytes(wire_size.saturating_sub(deficit));
         let error = decode_both(speaker, over, &encoded).expect_err(
             "undetected over-budget batching: a multi-record frame past the \
-             budget must fail typed",
+             budget must return an error",
         );
         prop_assert_eq!(error.origin, Origin::stream(speaker, stream));
         let typed = matches!(
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 5. Two em-dashes in the re-worded sentence became commas.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.

<a id="hunk-43"></a>
### src/tree/mirror/streaming/remote/codec/decode/tests.rs `@@ -1089,7 +1089,7 @@ fn an_over_deep_supplied_payload_dies_typed_at_ingress() {`

```diff
@@ -1089,7 +1089,7 @@ fn an_over_deep_supplied_payload_dies_typed_at_ingress() {
     };
     let codec = PayloadCodec::new::<Arr>(limit);
 
-    // One scope past the limit: typed rejection at the record iterator.
+    // One scope past the limit: a rejection at the record iterator.
     let over = record_with_payload(&deep_payload(limit.get() as usize + 1));
     let run = LeafRun::from_encoded(raw_record(&over)).unwrap();
     let error = run.records(codec).next().unwrap().unwrap_err();
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 5. Two em-dashes in the re-worded sentence became commas.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.

<a id="hunk-44"></a>
### src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs `@@ -122,7 +122,7 @@ fn one_record_run<T: Serialize + Send + Sync + 'static>(version: Version, value:`

```diff
@@ -122,7 +122,7 @@ fn one_record_run<T: Serialize + Send + Sync + 'static>(version: Version, value:
     run
 }
 
-/// Every feasible typed failure pins its fields and source chain, and its
+/// Every feasible failure pins its fields and source chain, and its
 /// origin where one exists (the record-level witnesses carry none;
 /// `record_errors` says why).
 #[test]
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<a id="hunk-45"></a>
### src/tree/mirror/streaming/remote/error.rs `@@ -1,6 +1,6 @@`

```diff
@@ -1,6 +1,6 @@
-//! Typed failures surfaced by a wire-bound streaming participant.
+//! Errors surfaced by a wire-bound streaming participant.
 //!
-//! [`RemoteError`] is the protocol-facing sum. Its variants retain the typed
+//! [`RemoteError`] is the protocol-facing sum. Its variants retain the
 //! adapter, stream-layer, and codec failures below, all of which are
 //! re-exported here so a caller can match a failure down to its precise cause
 //! without depending on the private implementation modules.
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 1. 'the typed adapter, stream-layer, and codec failures' loses its adjective; the sentence already names the three layers.

<a id="hunk-46"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -300,6 +300,58 @@ async fn reconcile_with_stacked_failures(`

```diff
@@ -300,6 +300,58 @@ async fn reconcile_with_stacked_failures(
     )
 }
 
+/// Reconcile with exactly one materialized participant using the supplied
+/// failing backend; both proxies keep whole backends.
+async fn reconcile_with_failing_walk(
+    a: TreeRoot,
+    b: TreeRoot,
+    failing: Failing<Local>,
+    fail_left: bool,
+) -> (Result<(), LeftFailure>, Result<(), RightFailure>) {
+    let whole = || Failing::after(Local, usize::MAX);
+    let (left_backend, right_backend) = if fail_left {
+        (failing, whole())
+    } else {
+        (whole(), failing)
+    };
+    let a = Handshaking::start(left_backend, failing_root(a)).window(WindowConfig::FLOOR);
+    let b = Handshaking::start(right_backend, failing_root(b)).window(WindowConfig::FLOOR);
+
+    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
+    let remote_b = RemoteHandshaking::start(
+        whole(),
+        a_link,
+        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
+    )
+    .window(WindowConfig::FLOOR);
+    let remote_a = RemoteHandshaking::start(
+        whole(),
+        b_link,
+        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
+    )
+    .window(WindowConfig::FLOOR);
+
+    let (left, right) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));
+    (left.map(|_| ()), right.map(|_| ()))
+}
+
+/// The operation a failing materialized participant reported, read from the
+/// endpoint that holds it.
+fn walk_injected_operation(
+    fail_left: bool,
+    (left, right): &(Result<(), LeftFailure>, Result<(), RightFailure>),
+) -> Result<Operation, TestCaseError> {
+    match (fail_left, left, right) {
+        (true, Err(MirrorError::Client(MaterializedError::Backend(Failure::Injected(op)))), _)
+        | (false, _, Err(MirrorError::Server(MaterializedError::Backend(Failure::Injected(op))))) => {
+            Ok(*op)
+        }
+        (_, left, right) => Err(TestCaseError::fail(format!(
+            "materialized failure was masked: left {left:?}, right {right:?}",
+        ))),
+    }
+}
+
 /// Extract the injected backend operation from a proxy conversion failure.
 fn injected_operation(error: &ProxyFailure) -> Option<Operation> {
     use crate::tree::mirror::streaming::remote::{ReplyDecodeError, ReplyEncodeError};
```

<!-- annotation -->
> **materialized-14** (T40), line 303:
>
> The probe the recorded commit (50c8b0a3) held back needs a wire session whose materialized walk fails; every existing wire harness puts the failing backend on the proxy (reconcile_with_stacked_failures), so this helper is its twin with the Failing backend on the walk. walk_injected_operation reads the error from whichever endpoint holds the walk, the same Client/Server convention the proxy helpers use.

<a id="hunk-47"></a>
### src/tree/mirror/streaming/remote/proxy/tests.rs `@@ -497,6 +549,88 @@ proptest! {`

```diff
@@ -497,6 +549,88 @@ proptest! {
             prop_assert!(result.1.is_ok(), "right endpoint failed without injection: {:?}", result.1);
         }
     }
+
+    /// Every reached materialized backend failure terminates both wire
+    /// endpoints and surfaces from the failing walk with its exact
+    /// operation identity; every unreached failure is inert.
+    ///
+    /// The wire twin of `materialized_backend_failures_are_fail_fast`: a
+    /// walk's error item reaches the session through the proxy pump that
+    /// carries its response stream, wherever in the walk it is raised. The
+    /// wide generator is what puts failures on that path: a stage loop
+    /// explodes nodes only under disputed scopes, and the small generator's
+    /// few leaves rarely collide into one.
+    #[test]
+    fn materialized_backend_failures_are_fail_fast_over_the_wire(
+        (a, b) in arb_wide_divergent_pair(),
+        operations in 0usize..32,
+        fail_left in any::<bool>(),
+        schedule in vec(0_u8..=2, 0..128),
+    ) {
+        let failing = Failing::after(Local, operations);
+        let result = with_schedule(schedule, || {
+            run_to_quiescence(reconcile_with_failing_walk(
+                a,
+                b,
+                failing.clone(),
+                fail_left,
+            ))
+        })
+        .map_err(|stopped| TestCaseError::fail(format!(
+            "materialized backend failure left the wire session quiescent: {stopped:?}",
+        )))?;
+        let history = failing.history();
+
+        if let Some(expected) = history.get(operations).copied() {
+            let actual = walk_injected_operation(fail_left, &result)?;
+            prop_assert_eq!(actual, expected);
+        } else {
+            prop_assert!(result.0.is_ok(), "left endpoint failed without injection: {:?}", result.0);
+            prop_assert!(result.1.is_ok(), "right endpoint failed without injection: {:?}", result.1);
+        }
+    }
+}
+
+/// A materialized error raised before the responder's opening reply yields
+/// returns over a wire instead of stalling the joint session.
+///
+/// The responder's opening explodes every disputed root child before it
+/// yields its one reply, so a backend failure there is an error item ahead
+/// of any reply on the walk's response stream. The proxy carrying that
+/// stream must observe the item without waiting on wire progress the
+/// missing reply would have unlocked: the faulted endpoint reports the
+/// injected operation, and its counterparty terminates on the cut.
+#[test]
+fn opening_failure_before_the_first_yield_returns_over_the_wire() {
+    let (a, b) = early_first_child_dispute_pair();
+    // The failing walk is the elected responder, whose opening explodes
+    // the disputed first root child.
+    let fail_left = !harness::left_initiates(&a, &b);
+    // Operation 0 is the greeting's root explosion; operation 1 is the
+    // opening's, at the root child's height.
+    let failing = Failing::after(Local, 1);
+    let result = run_to_quiescence(reconcile_with_failing_walk(
+        a,
+        b,
+        failing.clone(),
+        fail_left,
+    ))
+    .expect("an opening failure must terminate both sessions, not stall them");
+    let expected = Operation::Children { height: 31 };
+    assert_eq!(
+        failing.history().get(1).copied(),
+        Some(expected),
+        "the failure lands in the responder's opening: {:?}",
+        failing.history(),
+    );
+    assert_eq!(
+        walk_injected_operation(fail_left, &result).unwrap_or_else(|masked| panic!("{masked}")),
+        expected,
+    );
+    assert!(
+        result.0.is_err() && result.1.is_err(),
+        "the counterparty must terminate on the cut: {result:?}",
+    );
 }
 
 /// Wide-budget divergence still matches the materialized oracle with both
```

<!-- annotation -->
> **materialized-14** (T40), line 552:
>
> Step 1 of the brief: construct the probe, observe the stall, fix. The stall does not reproduce on this tree: both tests pass against base. Why the recorded construction (an understated set_len rewrite) cannot reach the walk's early-supply loop over a wire: the proxy's replayed opening carries only the root question (the early supplies cross on the opening stream and are folded into root-request replies at the first descending stage, already so at 50c8b0a3), and since 08f2899b the wire decoder charges the ledger at ingress ahead of the walk. The position the record names is still reachable by a backend failure in the responder's opening (answer::internal explodes the disputed first root child before the yield): the deterministic test pins exactly that, operation 1 on early_first_child_dispute_pair. Negative control (reversible mutation, restored, work.rs diff empty): parking the walk's pump before it publishes an error item fails the deterministic test with `an opening failure must terminate both sessions, not stall them: Stalled`, and the proptest twin with `materialized backend failure left the wire session quiescent: Stalled` at operations = 1, fail_left = false. The twin draws from arb_wide_divergent_pair deliberately: with arb_divergent_pair it passed under the mutant, because its few-leaf trees rarely dispute a scope, so no failing operation lands inside a stage loop (the response-stream path); the in-process twin materialized_backend_failures_are_fail_fast shares that weakness and is reported as a finding, not changed here.

<a id="hunk-48"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -79,7 +79,7 @@ fn batched_uneven_pair() -> (crate::tree::Root, crate::tree::Root) {`

```diff
@@ -79,7 +79,7 @@ fn batched_uneven_pair() -> (crate::tree::Root, crate::tree::Root) {
     (small.root, large.root)
 }
 
-/// A peer batching supply runs past the session minimum fails the session typed.
+/// A peer batching supply runs past the session minimum fails the session.
 ///
 /// The deceived side hears the bulk peer's `target_message_size` as zero,
 /// so it negotiates a zero session run budget while the peer keeps
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. These three are kin the brief's grep misses ('fails the session typed'): the goal names the family, so they go too, and the regex gap is reported.

<a id="hunk-49"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -166,7 +166,7 @@ fn overstated_target_message_size_still_converges() {`

```diff
@@ -166,7 +166,7 @@ fn overstated_target_message_size_still_converges() {
     }
 }
 
-/// A supplied version over the declared `max_version_bytes` fails the session typed.
+/// A supplied version over the declared `max_version_bytes` fails the session.
 ///
 /// The receiving side reports `OversizedVersion` at the first offending
 /// record — the declared aggregate covers every version the peer's tree
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. These three are kin the brief's grep misses ('fails the session typed'): the goal names the family, so they go too, and the regex gap is reported.

<a id="hunk-50"></a>
### src/tree/mirror/streaming/remote/proxy/tests/declarations.rs `@@ -205,7 +205,7 @@ fn understated_version_bytes_fail_the_session() {`

```diff
@@ -205,7 +205,7 @@ fn understated_version_bytes_fail_the_session() {
     }
 }
 
-/// A supply stream past the declared `set_len` fails the session typed.
+/// A supply stream past the declared `set_len` fails the session.
 ///
 /// The dual of the oversized-version guard, completing the declaration
 /// matrix: the declared set length is a premise of the window solve's
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. These three are kin the brief's grep misses ('fails the session typed'): the goal names the family, so they go too, and the regex gap is reported.

<a id="hunk-51"></a>
### src/tree/mirror/streaming/remote/proxy/work/tests.rs `@@ -162,8 +162,8 @@ fn deposited_supply_failure_outranks_a_racing_consequence() {`

```diff
@@ -162,8 +162,8 @@ fn deposited_supply_failure_outranks_a_racing_consequence() {
     }
 }
 
-/// A typed backend failure racing a dead stream supply surfaces as itself:
-/// the supply-outranking terminal exempts backend-typed errors.
+/// A backend failure racing a dead stream supply surfaces as itself:
+/// the supply-outranking terminal exempts backend errors.
 ///
 /// The terminal outranks protocol errors with a deposited supply failure
 /// because a consequence of the dead transport is a symptom, not a cause —
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 2. The sentence before the hit ('A typed backend failure') dropped its adjective too, for one voice across the two lines.

<a id="hunk-52"></a>
### src/tree/mirror/streaming/testing/faulting.rs `@@ -195,7 +195,11 @@ where`

```diff
@@ -195,7 +195,11 @@ where
         }
 
         if violation == Violation::UnaskedReply {
-            if let Some(item) = responses.next().await {
+            // Every honest reply passes, then one nobody asked for: the
+            // consumer's trailing check is what must catch it, so no
+            // honest reply is dropped ahead of it (a dropped one would
+            // pair the extra reply with a live query instead).
+            while let Some(item) = responses.next().await {
                 yield item;
             }
             yield Ok(message::Reply { replies: Vec::new() });
```

<!-- annotation -->
> **materialized-14** (T40), line 198:
>
> Deviation from nothing stated, but a harness repair the widening needed: the UnaskedReply script forwarded one honest reply and then ended, so at any stage with two or more queries its extra empty reply paired with the second query and classified as UnfinishedReply (observed: `left: UnfinishedReply, right: UnaskedReply` at `violation = UnaskedReply, server_steps = 0, client_steps = 1`). It now forwards every honest reply and then appends the unasked one, which is what the name means.

<a id="hunk-53"></a>
### src/tree/mirror/streaming/testing/faulting.rs `@@ -217,12 +221,29 @@ where`

```diff
@@ -217,12 +221,29 @@ where
                 reply.replies.push(message::Reaction::Query(Vec::new()));
             }
             Violation::UnexpectedSupply => {
+                // Ahead of the honest reply, at radix 0, which assumes the
+                // fan the corrupted reply answers holds radix 0 as its
+                // first child: `full_depth_comb_pair`'s spine does at
+                // every scope the connected suite's step range reaches.
+                // At a fan whose first child is higher, a radix-0 supply
+                // is a legal sibling mid-walk, and at the opening it is
+                // out of order (`InvalidSupply`) rather than held.
                 reply.replies.insert(0, message::Reaction::Supply(0, B::node::<H>()));
             }
             Violation::InvalidSupply => {
+                // A duplicated radix, past the honest reply. Radix 0xff
+                // assumes no fixture holds that child (as the escape below
+                // does): at the opening, where no honest reaction precedes
+                // the corruption to place it out of order, a held radix
+                // would be rejected as `UnexpectedSupply` instead. The
+                // first copy is absorbed before the duplicate fires, so
+                // the receiver's ledger must have one leaf of slack under
+                // the corrupting side's declared set length: every fixture
+                // declares its true length, and the receiver absorbs at
+                // most that side's exclusive leaves ahead of this one.
                 let node = B::node::<H>();
-                reply.replies.push(message::Reaction::Supply(0, node.clone()));
-                reply.replies.push(message::Reaction::Supply(0, node));
+                reply.replies.push(message::Reaction::Supply(0xff, node.clone()));
+                reply.replies.push(message::Reaction::Supply(0xff, node));
             }
             Violation::UncontainedSupply => {
                 // Appended past the honest reply, which covers the whole
```

<!-- annotation -->
> **materialized-14** (T40), line 234:
>
> The InvalidSupply script's duplicated radix moves from 0 to 0xff, the radix the UncontainedSupply script already assumes no fixture holds: at the opening no honest reaction precedes the corruption, so a held radix 0 would classify as UnexpectedSupply there (correctly: it lands on a held child), while a fresh radix duplicated is InvalidSupply at every height. The comment records the assumption beside the one it copies.

<!-- annotation -->
> **materialized-14** (T40), line 245:
>
> The two radix literals of the change described at line 227.

<!-- annotation -->
> **materialized-14** (T40), line 224:
>
> Fresh-eyes repair, item 2: the UnexpectedSupply script's radix 0 is a fixture coupling the arm now states, as the 0xff arms state theirs: it classifies as UnexpectedSupply only where the answered fan's first child is radix 0, which full_depth_comb_pair's spine guarantees at every scope the step range reaches; at a higher first child it is a legal sibling mid-walk, and at the opening it is out of order.

<!-- annotation -->
> **materialized-14** (T40), line 236:
>
> Fresh-eyes repair, item 2: the InvalidSupply script absorbs its first 0xff copy before the duplicate fires, so the receiver's ledger needs one leaf of slack under the corrupting side's declared length; the comment names why every fixture has it (true declarations; the receiver absorbs at most that side's exclusive leaves ahead of the corruption).

<a id="hunk-54"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -7,7 +7,7 @@ use super::{`

```diff
@@ -7,7 +7,7 @@ use super::{
     streaming_mirror_sides,
 };
 use crate::testing::run_to_quiescence;
-use crate::tree::arb::arb_divergent_pair;
+use crate::tree::arb::arb_wide_divergent_pair;
 use crate::tree::mirror::streaming::window::WindowConfig;
 use crate::tree::mirror::{
     Error as MirrorError,
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> `arb_connected_violation` widened from two variants to every reply-shaped violation `Faulting` can script (eight; OverdrawnSupply is a greeting lie, scripted by the greeting family). The doc's first paragraph is shortened to satisfy doclint's 220-character summary bound, which the first wording exceeded (the gate's first leg caught it). The step range stays 0..=15 per the brief's hazard. Cost: the property now runs about 27 s on the box (was under 2 s with two variants); the greeting-lie family beside it already runs 46 s.

<!-- annotation -->
> **materialized-14** (T40), line 10:
>
> Fresh-eyes repair, item 3: the in-process fail-fast twin draws from the wide generator, as its wire twin does and as the generator's doc says wire-liveness properties should. Under the pump-parking mutant it passed on arb_divergent_pair and now fails at its first case (`backend failure left materialized reconciliation quiescent: Stalled`, operations = 1, fail_client = false); verbatim in the commit message. The import is the only other change.

<a id="hunk-55"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -28,19 +28,25 @@ fn failing_root(root: crate::tree::Root) -> StreamingRoot<Failing<Local>> {`

```diff
@@ -28,19 +28,25 @@ fn failing_root(root: crate::tree::Root) -> StreamingRoot<Failing<Local>> {
     }
 }
 
-/// The connected abort suite's injected faults: one structural shape
-/// (a reply reaction the pairing never asked for) and one content shape
-/// (a structurally legal supply whose version escapes the declared
-/// greeting version).
+/// The connected abort suite's injected faults: every reply-shaped
+/// violation [`Faulting`] can script.
 ///
-/// The escape rides a party no fixture ticks
-/// ([`Faulting`]'s injection machinery), so the supplied ceiling is
-/// *incomparable* with the declared version: the containment
-/// predicate's hard case crosses the connected driver end to end, not
-/// only the dominating regime the deterministic tripwires build.
+/// All but one are structural. The content shape is a structurally legal
+/// supply whose version escapes the declared greeting version; the escape
+/// rides a party no fixture ticks ([`Faulting`]'s injection machinery),
+/// so the supplied ceiling is *incomparable* with the declared version:
+/// the containment predicate's hard case crosses the connected driver end
+/// to end, not only the dominating regime the deterministic tripwires
+/// build.
 fn arb_connected_violation() -> impl Strategy<Value = Violation> {
     prop_oneof![
+        Just(Violation::UnaskedReply),
+        Just(Violation::UnansweredQuery),
+        Just(Violation::UnfinishedReply),
+        Just(Violation::UnexpectedMatch),
         Just(Violation::UnexpectedQuery),
+        Just(Violation::UnexpectedSupply),
+        Just(Violation::InvalidSupply),
         Just(Violation::UncontainedSupply),
     ]
 }
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> `arb_connected_violation` widened from two variants to every reply-shaped violation `Faulting` can script (eight; OverdrawnSupply is a greeting lie, scripted by the greeting family). The doc's first paragraph is shortened to satisfy doclint's 220-character summary bound, which the first wording exceeded (the gate's first leg caught it). The step range stays 0..=15 per the brief's hazard. Cost: the property now runs about 27 s on the box (was under 2 s with two variants); the greeting-lie family beside it already runs 46 s.

<a id="hunk-56"></a>
### src/tree/mirror/streaming/tests/faults.rs `@@ -190,9 +196,13 @@ proptest! {`

```diff
@@ -190,9 +196,13 @@ proptest! {
 
     /// Every reached materialized backend failure terminates the session and
     /// survives sibling cancellation with its exact operation identity.
+    ///
+    /// The wide generator is what puts failures on the response-stream
+    /// path: a stage loop explodes nodes only under disputed scopes, and
+    /// the small generator's few leaves rarely collide into one.
     #[test]
     fn materialized_backend_failures_are_fail_fast(
-        (client_root, server_root) in arb_divergent_pair(),
+        (client_root, server_root) in arb_wide_divergent_pair(),
         operations in 0usize..32,
         fail_client in any::<bool>(),
         schedule in proptest::collection::vec(0_u8..=2, 0..128),
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> `arb_connected_violation` widened from two variants to every reply-shaped violation `Faulting` can script (eight; OverdrawnSupply is a greeting lie, scripted by the greeting family). The doc's first paragraph is shortened to satisfy doclint's 220-character summary bound, which the first wording exceeded (the gate's first leg caught it). The step range stays 0..=15 per the brief's hazard. Cost: the property now runs about 27 s on the box (was under 2 s with two variants); the greeting-lie family beside it already runs 46 s.

<!-- annotation -->
> **materialized-14** (T40), line 199:
>
> See the row at line 10: the generator change and the doc sentence that says why (a stage loop explodes nodes only under disputed scopes).

<a id="hunk-57"></a>
### tests/handshake.rs `@@ -3,7 +3,7 @@`

```diff
@@ -3,7 +3,7 @@
 //! Drives [`rumors::Rumors::gossip`] against a counterparty whose control
 //! halves are driven by hand over an in-memory [`rumors::link`] pair,
 //! asserting that a mismatched magic, version, or intent surfaces as the
-//! typed error variant rather than corrupting the local rumor set. The V2
+//! error variant rather than corrupting the local rumor set. The V2
 //! preamble is one self-described CBOR item of exactly 30 bytes with no
 //! redundant length:
 //! `55799(["rumors", version: uint, network: bstr(16), intent: uint])`.
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<a id="hunk-58"></a>
### tests/payload_depth.rs `@@ -10,8 +10,8 @@`

```diff
@@ -10,8 +10,8 @@
 //! versioning-`enum` shape, whose decode recursion is type-dependent),
 //! reject one step past the limit at its author, reject a lossy value
 //! (`Some(None)`) at its author, and carry deep content across a fleet
-//! whose limit was raised in concert; the last pin holds the sender's
-//! exit typed when a counterparty aborts mid-session on a decode
+//! whose limit was raised in concert; the last pin holds that the sender
+//! exits with an error when a counterparty aborts mid-session on a decode
 //! failure.
 
 mod common;
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. One grep hit and one kin the grep misses ('exit typed'), the latter re-flowed over three lines.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 4. The two em-dashes on the re-flowed line at 54-55 became commas.

<a id="hunk-59"></a>
### tests/payload_depth.rs `@@ -51,8 +51,8 @@ fn a_payload_at_the_default_depth_round_trips() {`

```diff
@@ -51,8 +51,8 @@ fn a_payload_at_the_default_depth_round_trips() {
     );
 }
 
-/// One step past the limit is rejected at send with the typed depth
-/// error — at the author, at the moment of choice — and nothing is
+/// One step past the limit is rejected at send with the depth
+/// error, at the author, at the moment of choice, and nothing is
 /// stored, so no session can ever wedge on it.
 #[test]
 fn one_step_past_the_limit_is_rejected_at_send() {
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. One grep hit and one kin the grep misses ('exit typed'), the latter re-flowed over three lines.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 4. The two em-dashes on the re-flowed line at 54-55 became commas.

<a id="hunk-60"></a>
### tests/payload_depth.rs `@@ -62,7 +62,7 @@ fn one_step_past_the_limit_is_rejected_at_send() {`

```diff
@@ -62,7 +62,7 @@ fn one_step_past_the_limit_is_rejected_at_send() {
         .expect_err("one step past the limit is rejected");
     assert!(
         matches!(error, rumors::EncodeError::Depth { limit } if limit == DEFAULT_PAYLOAD_DEPTH_LIMIT),
-        "the rejection is the typed depth case naming the limit: {error:?}"
+        "the rejection is the depth case naming the limit: {error:?}"
     );
     assert_eq!(rumors.snapshot().len(), 0, "a rejected send stores nothing");
 }
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. One grep hit and one kin the grep misses ('exit typed'), the latter re-flowed over three lines.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 4. The two em-dashes on the re-flowed line at 54-55 became commas.

<a id="hunk-61"></a>
### tests/payload_depth.rs `@@ -109,7 +109,7 @@ fn the_deepest_admissible_enum_round_trips() {`

```diff
@@ -109,7 +109,7 @@ fn the_deepest_admissible_enum_round_trips() {
 }
 
 /// An enum payload whose own decode needs one step past the limit is
-/// rejected at send with the typed depth error, and nothing is stored:
+/// rejected at send with the depth error, and nothing is stored:
 /// the author learns at the moment of choice, and no receiver can ever
 /// see the value.
 #[test]
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. One grep hit and one kin the grep misses ('exit typed'), the latter re-flowed over three lines.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 4. The two em-dashes on the re-flowed line at 54-55 became commas.

<a id="hunk-62"></a>
### tests/payload_depth.rs `@@ -120,7 +120,7 @@ fn an_enum_needing_one_step_past_the_limit_is_rejected_at_send() {`

```diff
@@ -120,7 +120,7 @@ fn an_enum_needing_one_step_past_the_limit_is_rejected_at_send() {
         .expect_err("a decode needing limit + 1 is rejected");
     assert!(
         matches!(error, rumors::EncodeError::Depth { limit } if limit == DEFAULT_PAYLOAD_DEPTH_LIMIT),
-        "the rejection is the typed depth case naming the limit: {error:?}"
+        "the rejection is the depth case naming the limit: {error:?}"
     );
     assert_eq!(rumors.snapshot().len(), 0, "a rejected send stores nothing");
 }
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. One grep hit and one kin the grep misses ('exit typed'), the latter re-flowed over three lines.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 4. The two em-dashes on the re-flowed line at 54-55 became commas.

<a id="hunk-63"></a>
### tests/payload_depth.rs `@@ -313,7 +313,7 @@ fn mismatched_limits_abort_both_sides_at_the_handshake() {`

```diff
@@ -313,7 +313,7 @@ fn mismatched_limits_abort_both_sides_at_the_handshake() {
 
 /// When a counterparty aborts mid-session on a payload decode failure
 /// and discards its poisoned link, the sender's own `gossip` completes
-/// with a typed error rather than hanging.
+/// with an error rather than hanging.
 ///
 /// The sender's error is [`rumors::Error::Epilogue`]: its local session
 /// work committed, and only the peer's confirmation was lost. The
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 3. One grep hit and one kin the grep misses ('exit typed'), the latter re-flowed over three lines.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 4. The two em-dashes on the re-flowed line at 54-55 became commas.

<a id="hunk-64"></a>
### tests/single_peer.rs `@@ -253,7 +253,7 @@ impl Arr {`

```diff
@@ -253,7 +253,7 @@ impl Arr {
 /// the whole batch: earlier-queued actions included — the
 /// cancel-on-error pin.
 ///
-/// The typed error carries the configured limit, and the tree is
+/// The error carries the configured limit, and the tree is
 /// untouched.
 #[test]
 fn a_depth_error_cancels_the_whole_batch() {
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.

<a id="hunk-65"></a>
### tests/single_peer.rs `@@ -273,7 +273,7 @@ fn a_depth_error_cancels_the_whole_batch() {`

```diff
@@ -273,7 +273,7 @@ fn a_depth_error_cancels_the_whole_batch() {
         .expect_err("the second send exceeds the limit");
     assert!(
         matches!(error, rumors::EncodeError::Depth { limit: l } if l == limit),
-        "the rejection is the typed depth case naming the limit: {error:?}"
+        "the rejection is the depth case naming the limit: {error:?}"
     );
     assert_eq!(
         rumors.snapshot().len(),
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.

<a id="hunk-66"></a>
### tests/single_peer.rs `@@ -332,7 +332,7 @@ fn send_all_commits_nothing_when_a_message_is_rejected() {`

```diff
@@ -332,7 +332,7 @@ fn send_all_commits_nothing_when_a_message_is_rejected() {
         .expect_err("the third message exceeds the limit");
     assert!(
         matches!(error, rumors::EncodeError::Depth { limit: l } if l == limit),
-        "the rejection is the typed depth case naming the limit: {error:?}"
+        "the rejection is the depth case naming the limit: {error:?}"
     );
     assert_eq!(
         rumors.snapshot().len(),
```

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Vocabulary sweep (T40): the phrase family 'fails typed' / 'aborts typed' and kin becomes 'returns an error' or plain 'error'; the brief's regenerating grep runs to zero in the commit. Where the grep matched an adjectival 'typed error' or 'typed failure' the adjective is dropped (the type is the sentence's subject already); where it matched the adverb after a verb, the verb is restated ('returns an error at its first', 'must return an error'). Lines changed in this file: 1.

<!-- annotation -->
> **materialized-14** (T40), line 0:
>
> Fresh-eyes repair, item 1: adjectival kin of the excised family ('typed rejection', 'typed truncation', 'typed depth case', 'typed intent rejection', 'typed network defect') drop the adjective under the sweep's own rule, the error's type being the sentence's subject; the type-level sense (height-typed, typed facade) is untouched. Lines changed in this file: 2.

