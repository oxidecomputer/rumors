<!-- CAVEAT LECTOR: review packet for lane p1-causality, written by Claude (Fable 5.1) for Finch under triage/WORKFLOW.md. The goal, rulings, stack position, acceptance rows, fresh-eyes rounds, and stops are the coordinator's record; nothing in this packet is authored or endorsed by Finch. Read with the ground rules in .agent-notes/README.md. -->

# Review packet: p1-causality

## Goal

The bookmark causality harness (`tests/bookmark_causality.rs` and the
two bookmark suites beside it) simulates crashes, recoveries, and
bookmark faults across a fleet and asserts that no version is ever
recycled and no party leaks. The review found the simulation's network
ids drawn from a global source, its sessions driven under a runtime
that could hide a hang, session failures classified as disruptions
without checking whether they were codec bugs, and the recycle check
stated in terms of versions rather than the consequence a recycle has.
This lane makes the simulation deterministic per world, drives it under
the closed-world poller, asserts every outcome the harness cannot
legitimately fail, and checks recycling by its consequence, each with a
committed negative control. Production code is untouched.

## Rulings landed

- T8: the four causality entries land per their stated resolutions,
  with the lane deciding the open experiment (whether `join!` under the
  closed-world poller delivers a faulted side's EOF to the survivor; it
  does, so no timeout was added).
- T141: the prose standard, applied in a final pass over the lane's
  files.

## Stack position

- Base: `81993ee0` (main after the swarm merge and the renderer's
  T138 ruling)
- Parent: `main`
- Children: none

## Acceptance table

Every row is one the coordinator's verification runner ran on the illumos
box against the lane at `1e3a8825` and `7fb513f7`, then a detached
scratch worktree at `8e7f41a3` under a processor set at the committed case counts,
with whole logs kept; mutations were reversible swaps restored to an
empty diff.

| entry | commit | acceptance command | decisive output |
|---|---|---|---|
| all | `1e3a8825` | `git diff --stat 81993ee0...1e3a8825 -- src proptest-regressions` | empty: production code and seeds untouched |
| all | `1e3a8825` | `cargo nextest run -p rumors --all-features --locked --no-fail-fast -E 'binary(bookmark_causality) \| binary(bookmark_attach) \| binary(bookmark_transmit_window)'` (box) | `17 tests run: 17 passed` |
| `tests-bookmark-8` | `bb023662` | the three `reconstructed_` tests, two separate processes | `3 passed` both runs, same three names |
| `tests-bookmark-11` | `4b4b3d04` | `grep block_on tests/bookmark_attach.rs`; `git grep -E 'tokio::spawn\|tokio_block_on'` over the three suites | `use crate::common::wire::block_on` at line 17, used at 54 and 81; grep empty |
| (negative control) | | side a's gossip replaced by `std::future::pending()`, the committed seed set aside, `bookmarking_never_recycles_a_version` | `FAIL ... closed in-memory future became quiescent: Stalled`, shrinking to a two-node clean gossip; seed written on the box only, tree restored |
| `tests-bookmark-12` | `28c07188` | a `panic!` planted in the bootstrap serve path, the whole binary | `6 failed`, every one at the planted panic's line naming `MUTATION: planted panic in the bootstrap serve path` |
| `tests-bookmark-9` | `f69cdb7e` | the reclaim mutant (`Version::new()` alias push in `src/bookmark.rs`), `reconstructed_reclaim_after_crash_keeps_durable_content` | `FAIL: messages [0] were live in network ... when the heal began and never redacted there, but node 0 does not hold them after it`; `src` restored to identical |
| (negative control, part 3) | | `negative_control_unledgered_loss_fails_the_survival_check` on the pristine tree; then the ledger push disabled | PASS on the tree; FAIL at the survival check under the mutation |
| prose | `1e3a8825` | em-dash counts per file at base and head; `grep -i honest` | 33 to 31 in the causality suite, others unchanged (none added); no "honest" |
| all | `1e3a8825` | `just gate` on the box (lane log `gate6_prose_illumos.log`) | seven streams ok; `fuzz` failed on `FuzzerPlatform.h:72:2: error: #error "Support for your platform has not been implemented"` (the accepted illumos leg); the first four commits also gated clean on the Mac before the box rule |
| repair `ceef955f` | `7fb513f7` | the three bookmark binaries (box) | `18 tests run: 18 passed` |
| known-bad artifact | `ceef955f` | `known_bad_stale_record_destroys_durable_content` with success output | PASS as `should_panic`, panicking `messages [0] were live in network ... before heal gossip 0<->2 ... node 0 does not hold them after it` against unmodified `reclaim` |
| (negative control, per-session check) | | the reclaim mutant plus one gossip before the heal in the reconstructed test | FAIL at the pre-heal session: `... were live in network ... before gossip 2<->1 ... node 2 does not hold them after it` |
| (negative control, classifier) | | the wildcard arm flipped to admit | `negative_control_classifier_rejects_codec_bugs` FAILS: `an unconditional crate bug must fail the step` |
| (liveness floor) | | `!world.holds(a, 0)` inverted, then removed, on pristine `src` | inverted: FAIL `A' revived from a peer that never saw m` (the premise is real); removed: the test still passes (the assertion is a floor on the shape's premise, not the verdict) |
| prose | `7fb513f7` | em-dash count in the causality suite at base and head | 33 to 29 |
| all | `7fb513f7` | `just gate` on the box after the rebase onto `main` (lane log `gate7_fresh1_illumos.log`) | seven streams ok; `fuzz` the accepted illumos leg |
| repair `8e7f41a3` (round 2) | `8e7f41a3` | the three bookmark binaries under `pset-run -n 40` (box, scratch worktree) | `18 tests run: 18 passed`; `src/` and `crates/` byte-identical to `main`; `Cargo.lock` gains only `rand_chacha` under the rumors package; `cargo metadata --locked` ok |
| known-bad artifact | `8e7f41a3` | with success output | PASS as `should_panic`, panicking `... were live in network ... before gossip 2<->1 ... node 2 does not hold them after it` |
| (negative control, discrimination) | | every `assert_session_preserved` call disabled | the artifact FAILS: `test did not panic as expected` (the heal-window half alone no longer catches the shape) |
| (negative control, classifier) | | the `Decode(Record(_))` arm flipped to admit | `negative_control_classifier_rejects_codec_bugs` FAILS: `an unconditional crate bug must fail the step` |
| (negative control, pending ledger) | | the redaction promotion in `secure` disabled | both properties false-positive at once (`messages [1] were live ... before gossip 0<->1` on a two-node plan), so the promotion is load-bearing |
| prose | `8e7f41a3` | `ChaCha8Rng` at the world's seed sites; "observable non-recycling" in the module doc; em-dash lines | present at 99, 406, 466; lines 27 and 147; 28 (from 33 at base) |
| all | `8e7f41a3` | `just gate` on the box (gate10, unbound) plus `pset-run -n 40 -- just test-all` (run25) | gate10's streams ok except `fuzz` and a load-only timeout in `workspace`; run25: `1834 tests run: 1834 passed`; the bookmark binaries under a pset: `18 passed` |

## Fresh-eyes rounds

**Round 1** (surface correctness with operational validity), against
`1e3a8825`, by sha. The reviewer confirmed by reading that the faulted
side's EOF reaches the survivor under `join!` (the memory acceptor
returns `UnexpectedEof` when its channel closes and tokio's `MaybeDone`
drops the finished future), that the closed-world poller cannot mask a
hang (`Stalled` or `PollBudget`, both fatal), and that the per-world
RNG, the sequence counter, and the trie's iteration order make each
world's path deterministic. Findings, all repaired: the recycle check by
consequence watched only the heal window, and the reviewer's
construction (the committed destructive shape with one gossip before
the heal, under the reclaim mutant) was run first and confirmed: the
heal-window check read a destroyed durable message as success. The
repair checks survival at every same-network session: before each,
`U = (live(a) ∪ live(b)) \ ledger(network)`; after a session both sides
complete, each side holds `U`, with the soundness premise stated at the
check from `Rumors::gossip`'s contract (a failed session leaves content
unchanged or commits whole) and no false positive at the committed case
counts. The outcome classifier is now an inverted match naming what a
truncation, an injected fault, or a mismatch can produce, with the
wildcard as the bug arm (`Error` is `#[non_exhaustive]`, so the compiler
cannot hold totality from an integration test; the wildcard is the safe
default); `Ok` on both sides of a cross-network session and
`Retire::Declined` panic unconditionally. The reconstructed test's
discriminating premise (A' revives from a peer that never saw `m`) is
asserted as a liveness floor. A known-bad artifact against unmodified
production code (A's store bytes reverted after its crash) is committed
`should_panic` on the survival check's message and fires. Prose: the
determinism paragraph states what is exact (clean plans; faulted plans
up to which frame a cut lands on), a false "leaves both replicas
unchanged" is gone, the classifier control matches its message by
substring.

**Round 2** (operational validity and tree interaction), against
`7fb513f7`: the reviewer stated the per-session check's soundness
premise from the gossip, retire, and bootstrap contracts (traced to
the one content commit in `gossip_inner`), hunted false-positive paths
across every session kind (none found), and confirmed the check runs
on every same-network session and cannot be skipped by schedule shape.
Findings, landed in the final repair commit: the known-bad artifact did
not discriminate the per-session half (no session between the
re-issued sends and the heal, and a shared `should_panic` substring),
so it gains the pre-heal gossip and pins the per-session message; the
classifier's source-chain rule admitted a record truncated inside a
fully received run (a `DecodeLeafError` wrapping `UnexpectedEof`), now
a bug before the chain is consulted, with `Mirror` cases in both
directions added to the control; a redaction lost with its crashed
redacter exempted its message from the check forever, now modeled as
a pending ledger promoted on the redacter's next completed session;
the soundness sentence named completeness (whole-or-nothing commit)
as soundness (frontier domination); the world RNG is `ChaCha8Rng`, a
portable generator, so the pinned path no longer depends on rand's
platform choice. The rounds stopped here.

Reviewer notes not acted on: the chain rule's freedom from false
positives rests on the memory link depositing an I/O error in the wave
a peer completes (specific to memory links and the poller); the
`networks.len() > 1` assertion states world shape rather than the
reachability argument beside it; the case-count question (256 against
the 2048 at which the random search reaches the destructive shape under
the mutant) stays the owner's, the deterministic tests being the
instrument for that shape.

## Stops

None. Of the three questions the lane raised, the proptest case count
is ruled (T148: committed counts stay gate-fast; CI runs the suites in
release with a large measured count through one helper, a later lane);
the optional party-alias comparison in `promote` is ruled dropped as
a model (T149: any mechanism able to recycle recycles observably on
the plans that place durable content at the re-issued coordinates,
which the deterministic tests construct); the remaining one is in the
owner's queue: the optional
party-alias strengthening of `promote` (not landed), and the attach
helper's shared-driver clause (blocked on `tests-bookmark-3`).

## Reading order

### new tests and negative controls

- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:101` ([hunk](#hunk-6))
- tests-bookmark-8 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:99` ([hunk](#hunk-6))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:578` ([hunk](#hunk-12))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:849` ([hunk](#hunk-20))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:876` ([hunk](#hunk-21))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:987` ([hunk](#hunk-24))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:1750` ([hunk](#hunk-37))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:1772` ([hunk](#hunk-38))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:1933` ([hunk](#hunk-38))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:1901` ([hunk](#hunk-38))
- tests-bookmark-12 (T8) at `tests/common/flaky.rs:111` ([hunk](#hunk-52))

### tests and prose

- tests-bookmark-8 (fresh-eyes repair, round 2) at `Cargo.lock:0` ([hunk](#hunk-2))
- tests-bookmark-8 (fresh-eyes repair, round 2) at `Cargo.toml:165` ([hunk](#hunk-3))
- tests-bookmark-11 (T8) at `tests/bookmark_attach.rs:17` ([hunk](#hunk-4))
- tests-bookmark-11 (T8) at `tests/bookmark_attach.rs:22` ([hunk](#hunk-4))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:35` ([hunk](#hunk-5))
- prose (T141) at `tests/bookmark_causality.rs:35` ([hunk](#hunk-5))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:42` ([hunk](#hunk-5))
- tests-bookmark-9 (T149) at `tests/bookmark_causality.rs:27` ([hunk](#hunk-5))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:42` ([hunk](#hunk-5))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:69` ([hunk](#hunk-6))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:98` ([hunk](#hunk-6))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:118` ([hunk](#hunk-6))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:71` ([hunk](#hunk-6))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:111` ([hunk](#hunk-6))
- prose (T141) at `tests/bookmark_causality.rs:71` ([hunk](#hunk-6))
- prose (fresh-eyes repair) at `tests/bookmark_causality.rs:81` ([hunk](#hunk-6))
- prose (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:82` ([hunk](#hunk-6))
- tests-bookmark-8 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:118` ([hunk](#hunk-6))
- tests-bookmark-12 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:100` ([hunk](#hunk-6))
- tests-bookmark-9 (T149) at `tests/bookmark_causality.rs:147` ([hunk](#hunk-7))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:296` ([hunk](#hunk-8))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:336` ([hunk](#hunk-8))
- prose (T141) at `tests/bookmark_causality.rs:296` ([hunk](#hunk-8))
- tests-bookmark-12 (fresh-eyes repair) at `tests/bookmark_causality.rs:296` ([hunk](#hunk-8))
- tests-bookmark-12 (fresh-eyes repair) at `tests/bookmark_causality.rs:323` ([hunk](#hunk-8))
- tests-bookmark-12 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:308` ([hunk](#hunk-8))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:370` ([hunk](#hunk-9))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:398` ([hunk](#hunk-10))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:442` ([hunk](#hunk-10))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:460` ([hunk](#hunk-10))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:422` ([hunk](#hunk-10))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:422` ([hunk](#hunk-10))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:435` ([hunk](#hunk-10))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:560` ([hunk](#hunk-11))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:663` ([hunk](#hunk-13))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:610` ([hunk](#hunk-13))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:642` ([hunk](#hunk-13))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:635` ([hunk](#hunk-13))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:716` ([hunk](#hunk-15))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:711` ([hunk](#hunk-15))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:757` ([hunk](#hunk-16))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:788` ([hunk](#hunk-17))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:816` ([hunk](#hunk-18))
- prose (fresh-eyes repair) at `tests/bookmark_causality.rs:832` ([hunk](#hunk-19))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:845` ([hunk](#hunk-20))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:848` ([hunk](#hunk-20))
- tests-bookmark-12 (fresh-eyes repair) at `tests/bookmark_causality.rs:899` ([hunk](#hunk-21))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:924` ([hunk](#hunk-22))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:953` ([hunk](#hunk-23))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:955` ([hunk](#hunk-23))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:1026` ([hunk](#hunk-24))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:965` ([hunk](#hunk-24))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:971` ([hunk](#hunk-24))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:1041` ([hunk](#hunk-25))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:1074` ([hunk](#hunk-26))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:1067` ([hunk](#hunk-26))
- prose (fresh-eyes repair) at `tests/bookmark_causality.rs:1064` ([hunk](#hunk-26))
- prose (fresh-eyes repair) at `tests/bookmark_causality.rs:1085` ([hunk](#hunk-27))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:1102` ([hunk](#hunk-28))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:1104` ([hunk](#hunk-28))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:1114` ([hunk](#hunk-29))
- tests-bookmark-12 (T8) at `tests/bookmark_causality.rs:1140` ([hunk](#hunk-29))
- prose (T141) at `tests/bookmark_causality.rs:1133` ([hunk](#hunk-29))
- tests-bookmark-12 (fresh-eyes repair) at `tests/bookmark_causality.rs:1169` ([hunk](#hunk-29))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:1175` ([hunk](#hunk-29))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:1212` ([hunk](#hunk-30))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:1265` ([hunk](#hunk-31))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:1282` ([hunk](#hunk-31))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:1302` ([hunk](#hunk-32))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:1354` ([hunk](#hunk-33))
- prose (T141) at `tests/bookmark_causality.rs:1354` ([hunk](#hunk-33))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:1348` ([hunk](#hunk-33))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:1630` ([hunk](#hunk-34))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:1655` ([hunk](#hunk-35))
- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:1688` ([hunk](#hunk-36))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:1849` ([hunk](#hunk-38))
- prose (T141) at `tests/bookmark_causality.rs:1849` ([hunk](#hunk-38))
- prose (T141) at `tests/bookmark_causality.rs:1933` ([hunk](#hunk-38))
- tests-bookmark-12 (fresh-eyes repair) at `tests/bookmark_causality.rs:1773` ([hunk](#hunk-38))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:1870` ([hunk](#hunk-38))
- tests-bookmark-12 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:1772` ([hunk](#hunk-38))
- tests-bookmark-9 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:1901` ([hunk](#hunk-38))
- tests-bookmark-9 (T8) at `tests/bookmark_causality.rs:1968` ([hunk](#hunk-39))
- tests-bookmark-9 (fresh-eyes repair) at `tests/bookmark_causality.rs:1968` ([hunk](#hunk-39))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:2000` ([hunk](#hunk-40))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:2012` ([hunk](#hunk-41))
- tests-bookmark-8 (T8) at `tests/bookmark_causality.rs:2040` ([hunk](#hunk-42))
- tests-bookmark-8 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:2036` ([hunk](#hunk-42))
- tests-bookmark-8 (fresh-eyes repair, round 2) at `tests/bookmark_causality.rs:2074` ([hunk](#hunk-43))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:28` ([hunk](#hunk-44))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:44` ([hunk](#hunk-45))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:159` ([hunk](#hunk-47))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:261` ([hunk](#hunk-48))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:351` ([hunk](#hunk-49))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:526` ([hunk](#hunk-50))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:606` ([hunk](#hunk-51))
- tests-bookmark-12 (T8) at `tests/common/flaky.rs:161` ([hunk](#hunk-53))

### nits

- tests-bookmark-11 (T8) at `tests/bookmark_causality.rs:690` ([hunk](#hunk-14))
- tests-bookmark-11 (T8) at `tests/bookmark_transmit_window.rs:65` ([hunk](#hunk-46))

## The change

<a id="hunk-1"></a>
### .agent-notes/2026-09-01-holistic-review-rumors/triage/annotations/p1-causality.tsv `@@ -0,0 +1,112 @@`

```diff
@@ -0,0 +1,112 @@
+# Lane p1-causality, ruling T8: the bookmark causality harness. Line numbers are new-side lines of
+# `git diff 81993ee0...HEAD` in the final tree of the branch; a deletion is annotated at the line that follows it.
+tests/bookmark_causality.rs	69	tests-bookmark-8	T8	The entry: the module doc promised a fully deterministic schedule while every universe's Network came from OsRng, and it named tokio's unbiased select! as a second, undemonstrated source. I rewrote the Determinism paragraph to say what the harness now fixes (message ids, emission sequence numbers, and every network id, from per-world sources) and to calibrate the residual with the resolution's sentence: deterministic up to select! branch order in the session internals, with the mechanism stated (tokio's thread-local RNG is seeded per process; the order picks which ready stream a session polls first, so a fixed-offset cut can land on a different frame). I chose the calibration sentence over the optional ByteMeter replay-identity check: two replays inside one process would share one thread-local RNG state and so could not witness the cross-process source the sentence names.
+tests/bookmark_causality.rs	98	tests-bookmark-8	T8	SmallRng and SeedableRng imports for the per-world network RNG, the idiom the crate's other suites use with Peer::seed_rng.
+tests/bookmark_causality.rs	118	tests-bookmark-8	T8	NETWORK_SEED is the one seed every World's RNG starts from. I considered carrying a seed in Plan so proptest could vary which fresh peer wins a tie, and rejected it: it changes the strategy's shape, which re-maps the two committed regression seeds (the file comment above the reconstructed tests already records that hazard), and a constant seed already makes every plan replay. The value is 0, the sibling suites' idiom.
+tests/bookmark_causality.rs	398	tests-bookmark-8	T8	World gains the per-world sources the plan does not carry: rng (the network source), networks (every id seeded, so pairwise distinctness is asserted rather than assumed, per the resolution), and path (the record of every place a network id decided the outcome). The path log replaces a first draft that logged only successful bootstraps, which could not distinguish an undetected mismatch from a re-bootstrap that failed under bookmark faults; the reconstructed test needed that distinction to pin its winner.
+tests/bookmark_causality.rs	442	tests-bookmark-8	T8	PathEvent names the three places the tie-break or a revival decides what happens: a mismatch resolution (winner and loser), a bootstrap attempt with its outcome, and a re-seed. Equality on the whole path is what the reconstructed tests assert.
+tests/bookmark_causality.rs	460	tests-bookmark-8	T8	World::empty and seed_universe replace the three inline Peer::seed() calls with one seeded path; seed_universe draws from the world's RNG and asserts the id is new to this world. World::seed and single_network now build through empty() so the RNG exists before the first universe is drawn.
+tests/bookmark_causality.rs	924	tests-bookmark-8	T8	resolve_mismatch records the winner and loser on the path before bootstrapping the loser.
+tests/bookmark_causality.rs	1026	tests-bookmark-8	T8	bootstrap_into records the attempt and its outcome on the path; a failed attempt is as much a fact about the plan's path as a success.
+tests/bookmark_causality.rs	1074	tests-bookmark-8	T8	revive's fresh-universe fallback records the re-seed and draws its network from the world's RNG instead of OsRng.
+tests/bookmark_causality.rs	1630	tests-bookmark-8	T8	The library-level retire regression seeds from NETWORK_SEED too: it has no World, so it seeds a local SmallRng. Not required by the resolution (this test has no tie-break); swept so no Peer::seed() call remains in the suite and its network is the same on every run.
+tests/bookmark_causality.rs	1750	tests-bookmark-8	T8	The negative control's network is seeded the same way, for the same reason as the library-level test above.
+tests/bookmark_causality.rs	2012	tests-bookmark-8	T8	The crash-pair reconstruction pins its path: each crash re-seeds (no live member to reboot from), the cross-network retire is skipped, and heal collapses node 1 into node 0. Observed identically on two separate runs before pinning; the doc comment states the path in English.
+tests/bookmark_causality.rs	2040	tests-bookmark-8	T8	The cut-gossip reconstruction pins its path, which is the acceptance's asserted winner: the cut session still exchanges greetings, so the mismatch resolves with node 2 the winner by network id; node 1's re-bootstrap fails on its scheduled bookmark faults; the retire re-seeds node 1; heal collapses 1 and 2 into 0. Observed identically on two separate runs (two processes) before pinning.
+tests/bookmark_causality.rs	71	tests-bookmark-11	T8	The Determinism paragraph now names the closed-world poller as the runtime and states the consequence the entry wanted: a session that stops making progress fails at its source instead of hanging until the test runner's 180-second kill loses the proptest seed.
+tests/bookmark_causality.rs	111	tests-bookmark-11	T8	The suite imports common::wire::block_on (run_to_quiescence) in place of tokio_block_on. Every session in the file is a closed, in-memory future, so the stall detector applies to all of them.
+tests/bookmark_causality.rs	690	tests-bookmark-11	T8	Nit: the send's race-freedom comment spoke of the current-thread schedule and other tasks; there are no tasks now, so it states the actual reason (every session is its own single-threaded block_on).
+tests/bookmark_causality.rs	849	tests-bookmark-11	T8	The experiment the resolution asked for, decided here: World::gossip is a join! over two async move blocks that each own their faulted link, under the closed-world poller. The faulted side's EOF does reach the survivor: the full faulted proptest (256 cases) and both reconstructed cases pass, so no timeout was added. The comment states the mechanism (join! drops a completed block, link included) and what would deadlock (links declared outside the blocks). Negative control (reversible mutation, recorded in the commit message): side a's gossip replaced by std::future::pending() fails bookmarking_never_recycles_a_version in 41 ms with `closed in-memory future became quiescent: Stalled` and writes the seed `[Gossip(0, 1, clean, clean)]` once the committed seeds are set aside; with them present, the cut-gossip seed reproduces the failure first and proptest writes nothing, as AGENTS.md documents.
+tests/bookmark_causality.rs	965	tests-bookmark-11	T8	bootstrap_into on join!. The serve side's result is still discarded at this commit (`let _ = serve_out`) and the boot side still folds errors to None: binding and asserting them is tests-bookmark-12's change, landed in the next commit, so this commit stays purely structural.
+tests/bookmark_causality.rs	1114	tests-bookmark-11	T8	retire on join!: the retiree's try_into_peer and retire run in one block, the absorber's gossip in the other; the tuple destructures straight out of block_on.
+tests/bookmark_causality.rs	1265	tests-bookmark-11	T8	clean_gossip on join!, so the heal's sessions get the stall detector too.
+tests/bookmark_causality.rs	1655	tests-bookmark-11	T8	The library-level retire regression on join!: no JoinHandle, so the retire outcome and the absorber's result are matched directly.
+tests/bookmark_causality.rs	1688	tests-bookmark-11	T8	boot_from_async on join!, same shape as bootstrap_into.
+tests/bookmark_attach.rs	17	tests-bookmark-11	T8	The acceptance's import: common::wire::block_on, not tokio_block_on. The suite's sessions are closed in-memory futures.
+tests/bookmark_attach.rs	22	tests-bookmark-11	T8	bootstrap_unbookmarked keeps its local shape (join! over two owning blocks) rather than moving onto the shared driver: common::wire::bootstrap_fork is generic over Rumors<T> with the default bookmark, and this server is Rumors<String, FlakyInMemoryBookmark>; generalizing the driver over B: Bookmark is tests-bookmark-3 (a P5 entry), so that clause of the resolution is reported open, not done here.
+tests/bookmark_transmit_window.rs	28	tests-bookmark-11	T8	The resolution names the transmit suite too. Its module doc gains the sentence that every session runs under the closed-world poller and what that buys.
+tests/bookmark_transmit_window.rs	44	tests-bookmark-11	T8	common::wire::block_on replaces tokio_block_on for the transmit suite.
+tests/bookmark_transmit_window.rs	65	tests-bookmark-11	T8	Nit: the GatedBookmark doc said the test body ran on the same current-thread runtime; restated as the same thread under the same poller, which is what makes the interleaving exact.
+tests/bookmark_transmit_window.rs	159	tests-bookmark-11	T8	boot_from and the gossip helper on join! over owning blocks.
+tests/bookmark_transmit_window.rs	261	tests-bookmark-11	T8	The gated session is the resolution's exact shape: join!(ga, gb, async { entered; send(M1); release }); Notify needs no runtime.
+tests/bookmark_transmit_window.rs	351	tests-bookmark-11	T8	Cancellation without JoinHandle::abort: the two session futures are pinned in a block and driven by a biased select! against the bookmark's `entered` signal, and the block's end drops them mid-persist. My first draft moved the Pin<&mut _> into select!, which drops the reference and not the future, so the parked persist stayed alive holding the bookmark and the next session stalled (the poller caught it in 17 ms, which is the instrument the entry restores doing its job on my own bug). The old assertion that both JoinHandles reported an abort has no analogue and is gone; the drop is the cancellation.
+tests/bookmark_transmit_window.rs	526	tests-bookmark-11	T8	The two donation-abort tests on join!; the serve block is bound first so the closure captures read naturally, and the results are matched directly instead of through unwrap().
+tests/common/flaky.rs	111	tests-bookmark-12	T8	FlakyError::injected_write builds the error value the committed classifier negative control needs for its honest-disruption arm (Bookmark(Io(_)) must pass); the struct's field is private to the module, so a constructor is the smallest opening.
+tests/common/flaky.rs	161	tests-bookmark-12	T8	FaultFeed::may_fail is the resolution's emptiness accessor, made precise: a feed can fail iff it is enabled and a `true` decision remains in either queue. An exhausted or all-false queue defaults to success (next_read/next_write), so asserting Ok over such feeds is sound and strictly stronger than asserting only over empty ones; a step the plan cannot fail is asserted even inside a faulted plan.
+tests/bookmark_causality.rs	101	tests-bookmark-12	T8	Imports for the classifier's bounds and arms (BookmarkError, BookmarkIo) and the flaky error value the negative control constructs.
+tests/bookmark_causality.rs	296	tests-bookmark-12	T8	The classifier the entry asked to extract from retire, applied to every session result on both sides: Io(InvalidData), HandOffMalformed, Bookmark(Format(_)), and PartyOverlap are unconditional crate bugs whatever faults the step scheduled. It is generic over the bookmark type because the bootstrap's joining side reports Error<NoBookmark> and the rest report Error<FlakyInMemoryBookmark>. I kept the arm list exactly as the resolution names it rather than adding Mirror(_) or the preamble defects, which I have no evidence a cut wire cannot produce.
+tests/bookmark_causality.rs	336	tests-bookmark-12	T8	BootFailure names the three ways a bootstrap's joining side comes up empty, replacing `.ok().flatten()?` and `.ok()`, which folded a wire error, a peerless join, and a failed attach into one None. Each arm is classified separately below.
+tests/bookmark_causality.rs	663	tests-bookmark-12	T8	The fault regime is decided per step, not by a world-wide flag: a session may fail legitimately iff a cut is scheduled on either wire or a bookmark fault remains on either feed, read before the session because the session consumes the schedule. I chose this over the resolution's `reliable: bool` because it also asserts Ok for the clean steps inside faulted plans, and single_network worlds fall out as the case where nothing is ever scheduled.
+tests/bookmark_causality.rs	845	tests-bookmark-12	T8	gossip records the regime and whether the pair shared a network before the session (the session may re-bootstrap the loser).
+tests/bookmark_causality.rs	876	tests-bookmark-12	T8	Every gossip result is now judged on both sides: Ok passes; NetworkMismatch is admitted only across networks; any other error is classified and then admitted only if the step could fail or the counterparty mismatched (the mismatching side aborts and closes the wire, so the other side may see EOF). A clean cross-network session must surface its mismatch. Negative control (reversible mutation, recorded in the commit message): the witness's injection, every third World::gossip returning Err(PartyOverlap) on both sides, fails bookmarking_prevents_party_leakage at the first injected step with `gossip 0<->1, node 0: a protocol, codec, or bookmark-format bug, not an honest disruption: PartyOverlap`, where the suite before this change passed through 452 such injections.
+tests/bookmark_causality.rs	953	tests-bookmark-12	T8	A bootstrap's wire is clean, so only the two feeds decide whether it may fail.
+tests/bookmark_causality.rs	971	tests-bookmark-12	T8	The joining side keeps every outcome: a peerless join (impossible against a gossiping server) panics outright; a join error is classified and admitted only under a possible fault; an attach failure is a bug if it is Format(_) (the crate wrote every byte the store holds) and otherwise admitted only under a possible fault.
+tests/bookmark_causality.rs	987	tests-bookmark-12	T8	The comment the entry asked to rewrite: on this clean wire the server fails only through its own donating persist, whose abort closes the wire on the newcomer as a truncated hand-off, and the newcomer only through its own attach persist after the session; neither can fail over reliable bookmarks. The serve result is no longer discarded: it is classified and asserted Ok whenever nothing could fail. Under join! a serve-side panic propagates to the test directly, which is the acceptance's `serve_out.expect` without a JoinHandle. Negative controls (reversible mutations, recorded in the commit message): a planted Io(InvalidData) in the serve block fails bookmarking_prevents_party_leakage at `bootstrap of 1 from 0, serving side` with the classified error, at the offending step and not at heal; a planted panic! in the serve block fails bookmarking_never_recycles_a_version instead of leaving the node dormant.
+tests/bookmark_causality.rs	1067	tests-bookmark-12	T8	revive's fresh-universe fallback asserts it is unreachable in a single-network world: the crash guard keeps a live member and reliable bookmarks cannot fail the rejoin, so re-seeding there would silently abandon the one-network premise the leakage property rests on (the entry's `revive then seeds a fresh universe on a failed rejoin` finding).
+tests/bookmark_causality.rs	1102	tests-bookmark-12	T8	retire computes its regime the same way (clean wire, two feeds).
+tests/bookmark_causality.rs	1140	tests-bookmark-12	T8	retire's inline classifier is replaced by the shared one, applied to the absorber's error and, as the resolution asks, to Retire::Recovered { error } and Retire::Uncertain { error }. Over reliable bookmarks the absorber must succeed and the retiree must land as Retired: Declined arises only when the counterparty is itself retiring, which this harness never does, and both proptests (512 cases) confirm no other outcome occurs on HEAD.
+tests/bookmark_causality.rs	1772	tests-bookmark-12	T8	Committed negative control for the classifier: each of the four bug arms fires (constructed values, including FormatError::Truncated and HandOffDefect::NotPartyTagged), and three honest disruptions (a severed wire, a truncated hand-off, an injected bookmark fault) pass. catch_unwind under AssertUnwindSafe because the closure only reads the error.
+tests/bookmark_causality.rs	35	tests-bookmark-9	T8	The module doc gains the property's second half: the recycle checked by its consequence. The entry showed the version order cannot see a reclaimed region's first emission (it compares Greater or incomparable once the reclaimer carries another region's progress), so the paragraph states the mechanism, the ledger, and the soundness argument for the survival check (a frontier dominates a durable emission only by having merged it or a redacter's frontier). The paragraph at L26-28 (the version-only argument) stands: part (2) of the resolution, the party-alias strengthening of promote, is not landed, so that argument is still what promote implements.
+tests/bookmark_causality.rs	422	tests-bookmark-9	T8	The per-network redaction ledger (id and version of each redaction, in the network it was performed in) and the heal-start snapshot of the winning network's live ids, which assert_healed holds the converged fleet to. Scoped to the winning network because the collapse discards every other network's content.
+tests/bookmark_causality.rs	578	tests-bookmark-9	T8	Two read helpers (holds, leaf_version) for the constructed-shape test and the negative control; the snapshot lookups the tests would otherwise spell out.
+tests/bookmark_causality.rs	716	tests-bookmark-9	T8	redact now records what it redacted. The doc drops the claim that a redaction is pure pressure: it is also the one legitimate way a message leaves the fleet, which is exactly why the ledger exists.
+tests/bookmark_causality.rs	1212	tests-bookmark-9	T8	heal snapshots the winning network's live content once everyone is revived and before the collapse. Taken after the revivals so a revived node's content is its server's; the collapse does not change the winner's members' content.
+tests/bookmark_causality.rs	1354	tests-bookmark-9	T8	The survival check: every id live in the winning network at heal start, less that network's ledger, is live at every peer after the heal. It runs from assert_healed, so both proptests and every reconstructed test get it. Its doc carries the soundness argument beside the check. On the unmutated tree it raised no false positive across the two proptests' 512 cases, which is the evidence for the premise.
+tests/bookmark_causality.rs	1849	tests-bookmark-9	T8	The entry's constructed shape as a committed test, from the witness's plan: m durable by propagation to B, C advancing without meeting m, A crashing and reviving from C, then gossiping and sending, then the heal. On HEAD m survives at every node. Under the reclaim mutant (src/bookmark.rs, the alias pushed at Version::new(), applied as a reversible string swap and restored, src/ verified identical to HEAD) this test fails at the survival check with `messages [0] were live in network ... node 0 does not hold them after it`; the commit message records the run. At the committed 256 cases the random recycle proptest does not reach the shape under the mutant; at PROPTEST_CASES=2048 both proptests fail, the recycle property through the survival check on an unshrunk case and through the version-only oracle on the shrunk five-step plan. I left the case count alone: tuning it against one mutant is a resource trade with an unmeasured sign, and the committed deterministic test is the reliable instrument; the report raises the count as an owner question.
+tests/bookmark_causality.rs	1933	tests-bookmark-9	T8	Negative control for the survival check on HEAD, no src mutation: a redacted, propagated message legitimately leaves the fleet; with the ledger emptied the same converged world fails assert_healed naming message 0. A suppressed ledger entry is the shape of a redaction the harness forgot to record, and of a destroyed message.
+tests/bookmark_causality.rs	1968	tests-bookmark-9	T8	The recycle proptest's doc gains the third clause, the survival check, stated as the recycle's consequence.
+tests/bookmark_causality.rs	35	prose	T141	Prose pass over the lane's diff, ruling T141. The lane's diff is net longer in prose: the four entries add a classifier, a fault-regime rule, a path log, and a survival check, each of which the entries asked to be stated beside its code, and every added paragraph is justified in its own row above. This pass then shortened what it could: the property's second half here keeps the mechanism and the soundness premise and drops a restated clause.
+tests/bookmark_causality.rs	71	prose	T141	The Determinism paragraph drops the test-runner clause and the parenthetical restating the tie-break, and states the select! residual once.
+tests/bookmark_causality.rs	296	prose	T141	The classifier's doc and message, the gossip doc, and the two negative-control docs and messages no longer say `honest`: T50 reserves that word for the trust premise, and what these mean is a disruption the harness injects or provokes.
+tests/bookmark_causality.rs	1133	prose	T141	The retire comment I had already edited said the absorber `can only fail honestly`; it now says what fails it, in fewer words, without the reserved word.
+tests/bookmark_causality.rs	1354	prose	T141	The survival check's doc restated the module doc's paragraph; it now states the invariant in one sentence and the two facts a maintainer at this method needs (why promote cannot see the destruction; why the ledger's entries are the only legitimate losses).
+tests/bookmark_causality.rs	1849	prose	T141	The constructed-shape test's doc opens with its invariant in one sentence and keeps the plan and the failure shape; the narrative of what a correct bookmark does went, since the assertion says it.
+tests/bookmark_causality.rs	1933	prose	T141	`genuinely` (an intensifier, T58) is gone and the doc is two sentences: the invariant, then the construction.
+tests/bookmark_causality.rs	42	tests-bookmark-9	fresh-eyes repair	Item 1. The reviewer's construction was confirmed first: under the reclaim mutant, with a gossip(a, b) inserted before the heal, assert_healed passed and only the test's own holds() assertion fired, so the heal-window check read a destroyed durable message as success. The module doc now states the per-session form and the atomicity premise it rests on.
+tests/bookmark_causality.rs	296	tests-bookmark-12	fresh-eyes repair	Item 2. The classifier is inverted: the match names what a truncation, an injected fault, or a mismatch can produce (Io and Epilogue carrying a non-InvalidData I/O error, PreambleTruncated, HandOffTruncated, NetworkMismatch, Bookmark(Io), and a Mirror error whose source chain bottoms out in a non-InvalidData I/O error) and the wildcard is the bug arm. Error is #[non_exhaustive], so the compiler cannot hold totality from an integration test; the wildcard-as-bug is the safe default and the doc says so. For Mirror I chose the source-chain rule over enumerating the proxy error's fifteen nested arms: the transport failures inside it all carry an I/O source, and a decode rejection does not; the faulted proptest at 256 cases is the false-positive check.
+tests/bookmark_causality.rs	323	tests-bookmark-12	fresh-eyes repair	The source-chain walk the Mirror arm uses; downcasts each node, so a #[source] I/O error at any depth is found.
+tests/bookmark_causality.rs	610	tests-bookmark-9	fresh-eyes repair	Item 1 helpers: the live ids of a node and the network's ledger as id sets.
+tests/bookmark_causality.rs	642	tests-bookmark-9	fresh-eyes repair	Item 1. The per-session survival check: before each same-network session U = (live(a) union live(b)) minus the ledger; after a session both sides complete, each side holds U. Soundness at the check: the frontier-domination premise, plus the session contract I verified in Rumors::gossip's docs: on Err the replica's content is unchanged, and the three exceptions (Epilogue: every local effect committed; a donation failure: identity only; Bookmark after absorbing a retiree: content and identity committed) commit whole, so an unchecked failed session never applies content in part. No false positive across the two proptests at 256 cases on the box.
+tests/bookmark_causality.rs	848	tests-bookmark-9	fresh-eyes repair	gossip takes U before the session and checks both sides after a double Ok.
+tests/bookmark_causality.rs	899	tests-bookmark-12	fresh-eyes repair	Item 2 sibling: Ok on both sides of a cross-network session is a bug in any regime and panics unconditionally; the earlier assert (a clean cross-network session must surface its mismatch) stays for the error case.
+tests/bookmark_causality.rs	955	tests-bookmark-9	fresh-eyes repair	A bootstrap copies the server's content whole, so the newcomer is checked against the server's live set less the ledger; same helper, same premise.
+tests/bookmark_causality.rs	1104	tests-bookmark-9	fresh-eyes repair	retire takes U before the session and checks the absorber after Retired with the absorber Ok.
+tests/bookmark_causality.rs	1169	tests-bookmark-12	fresh-eyes repair	Item 2 sibling: Declined arises only from a retiring counterparty, which the absorber never is here, so it panics unconditionally like BootFailure::NoPeer.
+tests/bookmark_causality.rs	1282	tests-bookmark-9	fresh-eyes repair	The heal's sessions get the per-session check too; this is where the known-bad artifact fires.
+tests/bookmark_causality.rs	1064	prose	fresh-eyes repair	Item 5: em-dash to spaced double hyphen in the revive comment this lane extended.
+tests/bookmark_causality.rs	1085	prose	fresh-eyes repair	Item 5: em-dash to spaced double hyphen in the retire doc this lane extended.
+tests/bookmark_causality.rs	1348	tests-bookmark-9	fresh-eyes repair	The heal-window check stays as the second half and its doc says which half it is.
+tests/bookmark_causality.rs	1773	tests-bookmark-12	fresh-eyes repair	Item 5: the classifier control now matches the classifier's message substring, so it fails by name rather than on any panic; the bug list gains Epilogue(InvalidData), LinkPoisoned, IntentInvalid, and BootstrapRetireConflict, and the admitted list gains Epilogue(UnexpectedEof) and PreambleTruncated.
+tests/bookmark_causality.rs	1870	tests-bookmark-9	fresh-eyes repair	Item 3: the instrument's liveness floor. The shape discriminates only if revive picked a server that never saw m; a change to revive's choice would otherwise retire the test silently.
+tests/bookmark_causality.rs	1901	tests-bookmark-9	fresh-eyes repair	Item 4: the destructive shape as a committed failing artifact against unmodified src/: A's store bytes snapshotted right after the fleet forms and put back after its crash, so a correct reclaim re-owns A's region on the first update (the lost-write hazard the bookmark's docs describe). should_panic on the survival message; on the box the panic lands in the heal's first session that meets A''s re-issued versions.
+tests/bookmark_causality.rs	1968	tests-bookmark-9	fresh-eyes repair	The proptest's third clause now names both halves of the check.
+tests/bookmark_causality.rs	81	prose	fresh-eyes repair	Item 5: the determinism paragraph no longer claims byte-for-byte replay for everything else; the select! RNG is per-thread state advanced by every select!, so faulted cases in one process can diverge, and shrinking and replay are exact for clean plans and, for faulted ones, up to which frame a cut lands on.
+tests/bookmark_causality.rs	832	prose	fresh-eyes repair	Item 5: the gossip doc no longer says a failed session leaves both replicas unchanged, which Epilogue contradicts.
+Cargo.toml	165	tests-bookmark-8	fresh-eyes repair, round 2	Item 6: rand_chacha joins the dev-dependencies (workspace-inherited, per manifestlint) so the suite can seed from ChaCha8, whose output for a seed is fixed across platforms and rand versions; SmallRng's is documented as neither, and the reconstructed tests pin paths derived from the ids.
+tests/bookmark_causality.rs	27	tests-bookmark-9	T149	Ruling T149 drops part 2 of the entry (the party-alias comparison in promote) as a model. The property paragraph states the harness's invariant as observable non-recycling in one sentence, in the ruling's terms, and says why the emitter's id-region enters no comparison; the Emission doc, which cited the old version-only argument, now cites this statement.
+tests/bookmark_causality.rs	42	tests-bookmark-9	fresh-eyes repair, round 2	Items 4 and 5: the module doc says the ledger holds redactions the network has learned of, and separates soundness (the frontier-domination argument) from completeness (whole-or-nothing commit, so a destruction cannot hide inside an unchecked failed session).
+tests/bookmark_causality.rs	82	prose	fresh-eyes repair, round 2	Item 7: `that RNG` had no antecedent noun; now `the RNG behind that order`.
+tests/bookmark_causality.rs	99	tests-bookmark-8	fresh-eyes repair, round 2	Item 6: ChaCha8Rng replaces SmallRng at every seed site (the World's RNG, the library-level retire regression, the classifier and recycle negative controls).
+tests/bookmark_causality.rs	118	tests-bookmark-8	fresh-eyes repair, round 2	Item 6: the seed doc says which generator backs the claim of sameness across runs and replays.
+tests/bookmark_causality.rs	100	tests-bookmark-12	fresh-eyes repair, round 2	Items 2 and 3: the error paths the classifier and its control match. The round named the record arm through CodecDecodeError; in this tree adapter::DecodeError is re-exported as ReplyDecodeError, and that is the type RemoteError::Decode carries, so the match goes through it.
+tests/bookmark_causality.rs	308	tests-bookmark-12	fresh-eyes repair, round 2	Item 2: a supplied record that does not decode is excluded before the source chain is consulted. Its chain bottoms out in the UnexpectedEof the record parser wraps a short record in, though the run's bytes were received whole, so the chain rule alone admitted the doc's own decode-failure class. Finding for the crate (outside this lane): parse_record and the payload decoder could classify a short record as InvalidData so the chain rule would not need the exception.
+tests/bookmark_causality.rs	370	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: a redaction is modeled like an emission. It waits on its redacter until another live same-network peer's frontier dominates the redacter's post-redaction frontier (the deletion propagated), then joins the network's ledger; a crash before then discards it with the incarnation, so a message whose only redaction died with its redacter stays owed to the fleet. Persistence is not a promotion route: a persisted frontier carries no deletion to anyone. No false positive across both proptests at 256 cases on the box.
+tests/bookmark_causality.rs	422	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: the ledger's doc states the model.
+tests/bookmark_causality.rs	435	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: a redaction records its network and the redacter's frontier afterwards, which the promotion test compares against.
+tests/bookmark_causality.rs	711	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: redact pushes onto the redacter's pending list instead of the ledger, capturing the post-redaction frontier.
+tests/bookmark_causality.rs	757	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: secure promotes pending redactions by propagation only; secure_and_lose discards them with the incarnation. The bootstrap, retire, and heal checks now run after the secure calls, so a redaction the session itself propagated is in the ledger when the session's survival check reads it.
+tests/bookmark_causality.rs	1175	tests-bookmark-9	fresh-eyes repair, round 2	Item 4, found by the proptests at 256 cases on the box (shrunk plan: Send(0), Gossip(0,1), Redact(1,0), Retire(0,1)): a redaction pending at the absorber propagates to the retiree during the retire, and the retiree then vanishes, so secure's observer rule never sees a live peer dominating it and the ledger never learns of it, a false positive at the absorber's check. On Retired the absorber's pending redactions are promoted outright: the retiree reconciled with them before donating.
+tests/bookmark_causality.rs	635	tests-bookmark-9	fresh-eyes repair, round 2	Item 5: the per-session check's doc names the frontier-domination argument as the soundness premise and whole-or-nothing commit as what makes the checks complete, and says why a partial commit could never have produced a false positive (expected is recomputed from live content before every session).
+tests/bookmark_causality.rs	1772	tests-bookmark-12	fresh-eyes repair, round 2	Item 3: the control gains three Mirror cases: a server-side HandshakeRead(UnexpectedEof) passes; a client-side MaterializedError::Violation(UnaskedReply) fires; a server-side Decode(Record(Version(UnexpectedEof))) fires.
+tests/bookmark_causality.rs	1901	tests-bookmark-9	fresh-eyes repair, round 2	Item 1: the artifact now gossips A' with B before the heal (the round-1 construction), so the destruction lands in that session and the pin is the per-session message's phrase `before gossip 2<->1`; the heal-window message says `when the heal began` and cannot satisfy it. Discrimination verified with every assert_session_preserved call disabled (reversible): the artifact fails, recorded in the commit message.
+Cargo.lock	0	tests-bookmark-8	fresh-eyes repair, round 2	Item 6: the lock gains rand_chacha in the rumors package's dependency list, the one line a dev-dependency on an already-locked crate adds. The round's earlier box runs were not --locked, so the box updated its own copy silently and the change never reached the tree; a --locked run on the box surfaced it.
+tests/bookmark_causality.rs	2036	tests-bookmark-8	fresh-eyes repair, round 2	Item 6: the path is re-pinned from a run under ChaCha8 and the doc restated from a probe of the run: the retire of 1 into 2 ends Recovered on node 1's scheduled bookmark write fault (the absorber's side fails too), so nothing is re-seeded and the heal collapses 1 and 2 into node 0.
+tests/bookmark_causality.rs	2074	tests-bookmark-8	fresh-eyes repair, round 2	Item 6: the pinned path under ChaCha8 ids: node 1 wins the mismatch and node 2 boots into it; no Reseeded event, since the retire hands node 1 back; the heal boots both from node 0.
+tests/bookmark_causality.rs	147	tests-bookmark-9	T149	The Emission doc's reason for not recording the emitter's party cites the module doc's statement of the invariant instead of restating the version-only argument the ruling retired.
+tests/bookmark_causality.rs	560	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: every node the fleet constructors build starts with no pending redactions, like its pending emissions.
+tests/bookmark_causality.rs	788	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: the promotion loop applies the emission rule's propagated half to redactions, the redacter's post-redaction frontier being what an observer must dominate; there is no persisted half.
+tests/bookmark_causality.rs	816	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: a crash or re-bootstrap discards the incarnation's pending redactions along with its pending emissions.
+tests/bookmark_causality.rs	1041	tests-bookmark-9	fresh-eyes repair, round 2	Item 4: the bootstrap's check runs after both secure calls, so a redaction the bootstrap itself propagated is in the ledger the check reads.
+tests/bookmark_causality.rs	1302	tests-bookmark-9	T8	assert_healed's doc names the heal-window survival check alongside the two checks it already stated.
+tests/bookmark_transmit_window.rs	606	tests-bookmark-11	T8	The second donation-abort test's session on join!, the same shape as the first.
+tests/bookmark_causality.rs	2000	tests-bookmark-8	T8	The crash-pair reconstruction's doc states the path it pins; the assertion below carries the pin.
```

*(review record; no annotation expected)*

<a id="hunk-2"></a>
### Cargo.lock `@@ -1266,6 +1266,7 @@ dependencies = [`

```diff
@@ -1266,6 +1266,7 @@ dependencies = [
  "pollster",
  "proptest",
  "rand 0.8.6",
+ "rand_chacha 0.3.1",
  "rumors",
  "seq-macro",
  "serde",
```

<!-- annotation -->
> **tests-bookmark-8** (fresh-eyes repair, round 2), line 0:
>
> Item 6: the lock gains rand_chacha in the rumors package's dependency list, the one line a dev-dependency on an already-locked crate adds. The round's earlier box runs were not --locked, so the box updated its own copy silently and the change never reached the tree; a --locked run on the box surfaced it.

<a id="hunk-3"></a>
### Cargo.toml `@@ -159,6 +159,10 @@ tokio = { workspace = true, default-features = true, features = [`

```diff
@@ -159,6 +159,10 @@ tokio = { workspace = true, default-features = true, features = [
     "test-util",
 ] }
 rand = { workspace = true, features = ["small_rng"] }
+# The bookmark causality simulation seeds its network ids from ChaCha8: its
+# output is fixed across platforms and rand versions, which SmallRng's is not,
+# and the suite pins paths derived from those ids.
+rand_chacha = { workspace = true }
 pollster = { workspace = true }
 # Counting global allocator for the decoder allocation meter
 # (`tests/decode_alloc.rs`): prices decodes in bytes requested from the
```

<!-- annotation -->
> **tests-bookmark-8** (fresh-eyes repair, round 2), line 165:
>
> Item 6: rand_chacha joins the dev-dependencies (workspace-inherited, per manifestlint) so the suite can seed from ChaCha8, whose output for a seed is fixed across platforms and rand versions; SmallRng's is documented as neither, and the reconstructed tests pin paths derived from the ids.

<a id="hunk-4"></a>
### tests/bookmark_attach.rs `@@ -14,29 +14,30 @@ use std::sync::{Arc, Mutex};`

```diff
@@ -14,29 +14,30 @@ use std::sync::{Arc, Mutex};
 use rumors::{Peer, Rumors, Unbookmarked};
 
 use crate::common::flaky::{FaultFeed, FlakyInMemoryBookmark, persisted_record};
-use crate::common::wire::tokio_block_on as block_on;
+use crate::common::wire::block_on;
 
 /// Capacity for each in-memory link stream carrying a bootstrap session.
 const LINK_BUF: usize = 64 * 1024;
 
 /// Bootstrap a fresh, still-unbookmarked peer from `server` over a clean
-/// in-memory link. Both sides run as spawned tasks so a finished one drops
-/// its end; the wires are reliable, so the bootstrap succeeds.
+/// in-memory link. Each side owns its end inside its own block, so a
+/// finished side drops it; the wires are reliable, so the bootstrap
+/// succeeds.
 async fn bootstrap_unbookmarked(server: &Rumors<String, FlakyInMemoryBookmark>) -> Peer<String> {
     let server = server.clone();
     let (boot_link, serve_link) = rumors::link::memory_with_capacity(LINK_BUF);
-    let boot = tokio::spawn(async move {
-        let mut link = boot_link;
-        Peer::<String>::bootstrap().join(&mut link).await
-    });
-    let serve = tokio::spawn(async move {
-        let mut link = serve_link;
-        server.gossip(&mut link).await
-    });
-    let (boot_out, serve_out) = tokio::join!(boot, serve);
-    serve_out.unwrap().expect("serve the bootstrap");
+    let (boot_out, serve_out) = tokio::join!(
+        async move {
+            let mut link = boot_link;
+            Peer::<String>::bootstrap().join(&mut link).await
+        },
+        async move {
+            let mut link = serve_link;
+            server.gossip(&mut link).await
+        },
+    );
+    serve_out.expect("serve the bootstrap");
     boot_out
-        .unwrap()
         .expect("bootstrap ok")
         .expect("got a peer")
         .sync_window_floor()
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 17:
>
> The acceptance's import: common::wire::block_on, not tokio_block_on. The suite's sessions are closed in-memory futures.

<!-- annotation -->
> **tests-bookmark-11** (T8), line 22:
>
> bootstrap_unbookmarked keeps its local shape (join! over two owning blocks) rather than moving onto the shared driver: common::wire::bootstrap_fork is generic over Rumors<T> with the default bookmark, and this server is Rumors<String, FlakyInMemoryBookmark>; generalizing the driver over B: Bookmark is tests-bookmark-3 (a P5 entry), so that clause of the resolution is reported open, not done here.

<a id="hunk-5"></a>
### tests/bookmark_causality.rs `@@ -24,8 +24,31 @@`

```diff
@@ -24,8 +24,31 @@
 //! is stated and checked per network; within a network every party is a fork of
 //! one seed, so all versions are comparable, and a later version can be `<=` an
 //! earlier one only by rolling backwards (concurrent versions compare
-//! incomparable, never `<=`). The id-region need not enter the comparison at all
-//! — it would only rule out collisions the version order already forbids.
+//! incomparable, never `<=`). The invariant is observable non-recycling: a
+//! recycle re-issues coordinates without knowing whether durable content
+//! occupies them, so any bookkeeping able to recycle at all does so
+//! observably on the plans that place durable content there (the
+//! reconstructed test and the known-bad artifact construct such plans, and
+//! the survival checks catch the loss), and the emitter's id-region
+//! therefore enters no comparison.
+//!
+//! A recycle is also checked by its consequence. A rebooted peer that
+//! re-owns a region below a frontier some replica durably holds emits
+//! versions the causal sieve reads as already deleted wherever the message
+//! they collide with is held, and the fleet converges without it. Such an
+//! emission compares `Greater` or incomparable to the message it destroys
+//! whenever the reclaimer's frontier carries any other region's progress,
+//! so the version order alone cannot see it. The [`World`] keeps a
+//! per-network ledger of every redaction the network has learned of and
+//! checks survival at every session: after a session both sides complete,
+//! each holds every message either held before it and never redacted in
+//! their network, and after the heal every message the winning network held
+//! at heal start and never redacted is live at every peer. The checks are
+//! sound because a correct bookmark's frontier dominates a durable emission
+//! only by having merged it or a redacter's frontier, so the ledger's
+//! entries are the only legitimate losses; they are complete because a
+//! failed session leaves content unchanged or commits it whole, so a
+//! destruction cannot hide inside an unchecked failed session.
 //!
 //! Durability is the load-bearing qualifier. A plain `send` neither persists
 //! (only sessions do) nor propagates, so a local emission lost to a crash before
```

<!-- annotation -->
> **tests-bookmark-9** (T8), line 35:
>
> The module doc gains the property's second half: the recycle checked by its consequence. The entry showed the version order cannot see a reclaimed region's first emission (it compares Greater or incomparable once the reclaimer carries another region's progress), so the paragraph states the mechanism, the ledger, and the soundness argument for the survival check (a frontier dominates a durable emission only by having merged it or a redacter's frontier). The paragraph at L26-28 (the version-only argument) stands: part (2) of the resolution, the party-alias strengthening of promote, is not landed, so that argument is still what promote implements.

<!-- annotation -->
> **prose** (T141), line 35:
>
> Prose pass over the lane's diff, ruling T141. The lane's diff is net longer in prose: the four entries add a classifier, a fault-regime rule, a path log, and a survival check, each of which the entries asked to be stated beside its code, and every added paragraph is justified in its own row above. This pass then shortened what it could: the property's second half here keeps the mechanism and the soundness premise and drops a restated clause.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 42:
>
> Item 1. The reviewer's construction was confirmed first: under the reclaim mutant, with a gossip(a, b) inserted before the heal, assert_healed passed and only the test's own holds() assertion fired, so the heal-window check read a destroyed durable message as success. The module doc now states the per-session form and the atomicity premise it rests on.

<!-- annotation -->
> **tests-bookmark-9** (T149), line 27:
>
> Ruling T149 drops part 2 of the entry (the party-alias comparison in promote) as a model. The property paragraph states the harness's invariant as observable non-recycling in one sentence, in the ruling's terms, and says why the emitter's id-region enters no comparison; the Emission doc, which cited the old version-only argument, now cites this statement.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 42:
>
> Items 4 and 5: the module doc says the ledger holds redactions the network has learned of, and separates soundness (the frontier-domination argument) from completeness (whole-or-nothing commit, so a destruction cannot hide inside an unchecked failed session).

<a id="hunk-6"></a>
### tests/bookmark_causality.rs `@@ -45,36 +68,62 @@`

```diff
@@ -45,36 +68,62 @@
 //!
 //! # Determinism
 //!
-//! Unlike `disruption.rs`, this simulation runs on a *current-thread* runtime
-//! with a fully deterministic, plan-driven schedule (each session is its own
-//! `block_on`). The bug class is about the *ordering* of
-//! emit/gossip/crash/retire/persist-fail events and the persistence-fault
-//! sequence, not watch-channel thread races; determinism makes counterexamples
-//! replay byte-for-byte, makes shrinking sound, and makes capturing each
-//! message's emitted version race-free. Message ids and emission sequence
-//! numbers are assigned by the [`World`], so replay does not depend on any
-//! process-global counter shared with other proptest cases.
+//! Unlike `disruption.rs`, this simulation runs single-threaded under the
+//! closed-world poller ([`common::wire::block_on`]) with a plan-driven
+//! schedule: each session is its own `block_on`, and a session that stops
+//! making progress fails at its source instead of hanging the case. The bug
+//! class is about the *ordering* of emit/gossip/crash/retire/persist-fail
+//! events and the persistence-fault sequence, not watch-channel thread
+//! races. Every input the plan does not carry is fixed by the [`World`]:
+//! message ids and emission sequence numbers come from a per-world counter,
+//! and every universe's [`Network`] identifier, the tie-break between two
+//! fresh peers, comes from a per-world RNG seeded with [`NETWORK_SEED`].
+//! The schedule is deterministic up to tokio's `select!` branch order
+//! inside the session internals: the RNG behind that order is per-thread
+//! state advanced by every `select!`, so a wire cut at a fixed byte offset
+//! can land on a different frame across runs and across the cases of one
+//! run. Shrinking
+//! and replay are therefore exact for clean plans, and for faulted plans
+//! exact up to which frame a cut lands on. Each message's emitted version
+//! is captured race-free.
 
 mod common;
 
 use std::cmp::Ordering;
-use std::collections::BTreeMap;
+use std::collections::{BTreeMap, BTreeSet};
 use std::sync::{Arc, Mutex};
 
 use before::Party;
 use proptest::prelude::*;
-use rumors::{Error, MERKLE_HASH_LEN, Network, Peer, Retire, Rumors, Version};
+use rand::SeedableRng;
+use rand_chacha::ChaCha8Rng;
+use rumors::error::{RemoteError, ReplyDecodeError};
+use rumors::{
+    BookmarkError, BookmarkIo, Error, MERKLE_HASH_LEN, MirrorError, Network, Peer, Retire, Rumors,
+    Version,
+};
 
 use crate::common::fault::{self, FaultPlan};
-use crate::common::flaky::{DurableStore, FaultFeed, FlakyInMemoryBookmark, persisted_record};
+use crate::common::flaky::{
+    DurableStore, FaultFeed, FlakyError, FlakyInMemoryBookmark, persisted_record,
+};
 use crate::common::sim::arb_fault;
-use crate::common::wire::tokio_block_on as block_on;
+use crate::common::wire::block_on;
 
 /// The message payload: a simulation-unique id that is also the message's
 /// emission sequence number, so a single per-[`World`] counter assigns both at
 /// once.
 type Msg = u64;
 
+/// The seed of every [`World`]'s network RNG.
+///
+/// Each universe the simulation creates draws its [`Network`] identifier
+/// from this stream, so the `(min_ticks, network)` tie-break between fresh
+/// peers is the same on every run and every replay of a plan. The stream is
+/// ChaCha8, whose output for a seed is fixed across platforms and `rand`
+/// versions; the reconstructed tests pin paths derived from it.
+const NETWORK_SEED: u64 = 0;
+
 /// Capacity for every in-memory link stream; the mirror protocol alternates
 /// within a session, so a modest buffer suffices and exercises backpressure.
 const LINK_BUF: usize = 8 * 1024;
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 69:
>
> The entry: the module doc promised a fully deterministic schedule while every universe's Network came from OsRng, and it named tokio's unbiased select! as a second, undemonstrated source. I rewrote the Determinism paragraph to say what the harness now fixes (message ids, emission sequence numbers, and every network id, from per-world sources) and to calibrate the residual with the resolution's sentence: deterministic up to select! branch order in the session internals, with the mechanism stated (tokio's thread-local RNG is seeded per process; the order picks which ready stream a session polls first, so a fixed-offset cut can land on a different frame). I chose the calibration sentence over the optional ByteMeter replay-identity check: two replays inside one process would share one thread-local RNG state and so could not witness the cross-process source the sentence names.

<!-- annotation -->
> **tests-bookmark-8** (T8), line 98:
>
> SmallRng and SeedableRng imports for the per-world network RNG, the idiom the crate's other suites use with Peer::seed_rng.

<!-- annotation -->
> **tests-bookmark-8** (T8), line 118:
>
> NETWORK_SEED is the one seed every World's RNG starts from. I considered carrying a seed in Plan so proptest could vary which fresh peer wins a tie, and rejected it: it changes the strategy's shape, which re-maps the two committed regression seeds (the file comment above the reconstructed tests already records that hazard), and a constant seed already makes every plan replay. The value is 0, the sibling suites' idiom.

<!-- annotation -->
> **tests-bookmark-11** (T8), line 71:
>
> The Determinism paragraph now names the closed-world poller as the runtime and states the consequence the entry wanted: a session that stops making progress fails at its source instead of hanging until the test runner's 180-second kill loses the proptest seed.

<!-- annotation -->
> **tests-bookmark-11** (T8), line 111:
>
> The suite imports common::wire::block_on (run_to_quiescence) in place of tokio_block_on. Every session in the file is a closed, in-memory future, so the stall detector applies to all of them.

<!-- annotation -->
> **tests-bookmark-12** (T8), line 101:
>
> Imports for the classifier's bounds and arms (BookmarkError, BookmarkIo) and the flaky error value the negative control constructs.

<!-- annotation -->
> **prose** (T141), line 71:
>
> The Determinism paragraph drops the test-runner clause and the parenthetical restating the tie-break, and states the select! residual once.

<!-- annotation -->
> **prose** (fresh-eyes repair), line 81:
>
> Item 5: the determinism paragraph no longer claims byte-for-byte replay for everything else; the select! RNG is per-thread state advanced by every select!, so faulted cases in one process can diverge, and shrinking and replay are exact for clean plans and, for faulted ones, up to which frame a cut lands on.

<!-- annotation -->
> **prose** (fresh-eyes repair, round 2), line 82:
>
> Item 7: `that RNG` had no antecedent noun; now `the RNG behind that order`.

<!-- annotation -->
> **tests-bookmark-8** (fresh-eyes repair, round 2), line 99:
>
> Item 6: ChaCha8Rng replaces SmallRng at every seed site (the World's RNG, the library-level retire regression, the classifier and recycle negative controls).

<!-- annotation -->
> **tests-bookmark-8** (fresh-eyes repair, round 2), line 118:
>
> Item 6: the seed doc says which generator backs the claim of sameness across runs and replays.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair, round 2), line 100:
>
> Items 2 and 3: the error paths the classifier and its control match. The round named the record arm through CodecDecodeError; in this tree adapter::DecodeError is re-exported as ReplyDecodeError, and that is the type RemoteError::Decode carries, so the match goes through it.

<a id="hunk-7"></a>
### tests/bookmark_causality.rs `@@ -94,8 +143,8 @@ const MAX_HEAL_ROUNDS_PER_PEER: usize = 16;`

```diff
@@ -94,8 +143,8 @@ const MAX_HEAL_ROUNDS_PER_PEER: usize = 16;
 /// seed; concurrent pairs compare [`None`]), so a later emission can be `<=`
 /// an earlier one only by rolling backwards over a version the network
 /// already durably held — exactly a recycle. The emitting party's identity
-/// is deliberately *not* recorded: as the module docs argue, it would only
-/// rule out collisions the version order already forbids.
+/// is deliberately *not* recorded: the module docs state the invariant as
+/// observable non-recycling, which the survival checks judge.
 struct Emission {
     network: Network,
     seq: u64,
```

<!-- annotation -->
> **tests-bookmark-9** (T149), line 147:
>
> The Emission doc's reason for not recording the emitter's party cites the module doc's statement of the invariant instead of restating the version-only argument the ruling retired.

<a id="hunk-8"></a>
### tests/bookmark_causality.rs `@@ -221,6 +270,79 @@ fn store_parties(store: &DurableStore, network: Network) -> Vec<Party> {`

```diff
@@ -221,6 +270,79 @@ fn store_parties(store: &DurableStore, network: Network) -> Vec<Party> {
         .collect()
 }
 
+// ---- the session error classifier -------------------------------------------
+
+/// Fail the test if `error` is one no session can report except through a
+/// crate bug, whatever wire faults or bookmark faults the step schedules.
+///
+/// The admitted errors are exactly what a severed wire, an injected
+/// bookmark fault, a counterparty closing the wire after its own fault, or
+/// a network mismatch can produce: a transport failure (`Io` or `Epilogue`
+/// carrying an I/O error other than `InvalidData`, or a `Mirror` error
+/// whose source chain bottoms out in one), a truncated preamble or
+/// hand-off, an injected bookmark fault, or the mismatch itself. One
+/// `Mirror` error is excluded before its chain is consulted: a supplied
+/// record that does not decode (`ReplyDecodeError::Record`) wraps the
+/// short read as an `UnexpectedEof` I/O error, though the run's bytes were
+/// received whole, so it is a decode failure whatever its chain says. Every
+/// other error is a bug: a fully received frame that does not decode, a
+/// bookmark file that does not parse when the crate wrote every byte the
+/// store holds, an overlapping retiring party, a poisoned link (every
+/// session here runs on a fresh one), a malformed preamble, an invalid
+/// intent, a bootstrap conflict, a magic, version, or payload-depth
+/// mismatch, and any variant added later. `Error` is `#[non_exhaustive]`,
+/// so the compiler cannot hold that totality from outside the crate; the
+/// wildcard arm is the bug arm, which is the safe default.
+fn assert_not_codec_bug<B>(step: &str, error: &Error<B>)
+where
+    B: BookmarkError + std::fmt::Debug,
+    B::Error: std::fmt::Debug,
+{
+    let transport = |io: &std::io::Error| io.kind() != std::io::ErrorKind::InvalidData;
+    let admitted = match error {
+        Error::Io(io) | Error::Epilogue(io) => transport(io),
+        Error::PreambleTruncated { .. }
+        | Error::HandOffTruncated
+        | Error::NetworkMismatch { .. }
+        | Error::Bookmark(BookmarkIo::Io(_)) => true,
+        Error::Mirror(MirrorError::Server(RemoteError::Decode(ReplyDecodeError::Record(_)))) => {
+            false
+        }
+        Error::Mirror(mirror) => io_source(mirror).is_some_and(transport),
+        _ => false,
+    };
+    assert!(
+        admitted,
+        "{step}: a protocol, codec, or bookmark-format bug, not an injected disruption: {error:?}",
+    );
+}
+
+/// The I/O error an error's source chain bottoms out in, if any: what
+/// separates a mirror session that died on the transport from one that
+/// rejected a frame.
+fn io_source<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<&'a std::io::Error> {
+    let mut current = Some(error);
+    while let Some(error) = current {
+        if let Some(io) = error.downcast_ref::<std::io::Error>() {
+            return Some(io);
+        }
+        current = error.source();
+    }
+    None
+}
+
+/// Why a bootstrap's joining side came up without a live peer.
+#[derive(Debug)]
+enum BootFailure {
+    /// The join session failed before any peer existed.
+    Join(Error),
+    /// The join ended without a peer, which only a server that was itself
+    /// bootstrapping could cause.
+    NoPeer,
+    /// The eager attach persist failed; the handed-back peer is dropped.
+    Attach(BookmarkIo<FlakyError>),
+}
+
 // ---- the fleet --------------------------------------------------------------
 
 /// One participant across all its incarnations: its live handle (or `Dormant`
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 296:
>
> The classifier the entry asked to extract from retire, applied to every session result on both sides: Io(InvalidData), HandOffMalformed, Bookmark(Format(_)), and PartyOverlap are unconditional crate bugs whatever faults the step scheduled. It is generic over the bookmark type because the bootstrap's joining side reports Error<NoBookmark> and the rest report Error<FlakyInMemoryBookmark>. I kept the arm list exactly as the resolution names it rather than adding Mirror(_) or the preamble defects, which I have no evidence a cut wire cannot produce.

<!-- annotation -->
> **tests-bookmark-12** (T8), line 336:
>
> BootFailure names the three ways a bootstrap's joining side comes up empty, replacing `.ok().flatten()?` and `.ok()`, which folded a wire error, a peerless join, and a failed attach into one None. Each arm is classified separately below.

<!-- annotation -->
> **prose** (T141), line 296:
>
> The classifier's doc and message, the gossip doc, and the two negative-control docs and messages no longer say `honest`: T50 reserves that word for the trust premise, and what these mean is a disruption the harness injects or provokes.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair), line 296:
>
> Item 2. The classifier is inverted: the match names what a truncation, an injected fault, or a mismatch can produce (Io and Epilogue carrying a non-InvalidData I/O error, PreambleTruncated, HandOffTruncated, NetworkMismatch, Bookmark(Io), and a Mirror error whose source chain bottoms out in a non-InvalidData I/O error) and the wildcard is the bug arm. Error is #[non_exhaustive], so the compiler cannot hold totality from an integration test; the wildcard-as-bug is the safe default and the doc says so. For Mirror I chose the source-chain rule over enumerating the proxy error's fifteen nested arms: the transport failures inside it all carry an I/O source, and a decode rejection does not; the faulted proptest at 256 cases is the false-positive check.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair), line 323:
>
> The source-chain walk the Mirror arm uses; downcasts each node, so a #[source] I/O error at any depth is found.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair, round 2), line 308:
>
> Item 2: a supplied record that does not decode is excluded before the source chain is consulted. Its chain bottoms out in the UnexpectedEof the record parser wraps a short record in, though the run's bytes were received whole, so the chain rule alone admitted the doc's own decode-failure class. Finding for the crate (outside this lane): parse_record and the payload decoder could classify a short record as InvalidData so the chain rule would not need the exception.

<a id="hunk-9"></a>
### tests/bookmark_causality.rs `@@ -239,6 +361,13 @@ struct Node {`

```diff
@@ -239,6 +361,13 @@ struct Node {
     /// neither persisted nor seen by another peer, so still erasable by a crash.
     /// Promoted to the durable [`EmissionLog`] once persisted or propagated.
     pending: Vec<Emission>,
+    /// Redactions made by this incarnation that no other peer has seen.
+    ///
+    /// A crash discards them with the incarnation, and the message they
+    /// deleted stays owed to the fleet. Promoted to the network's ledger
+    /// ([`World::redacted`]) once another live peer's frontier dominates the
+    /// redacter's.
+    pending_redactions: Vec<Redacted>,
     label: usize,
 }
 
```

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 370:
>
> Item 4: a redaction is modeled like an emission. It waits on its redacter until another live same-network peer's frontier dominates the redacter's post-redaction frontier (the deletion propagated), then joins the network's ledger; a crash before then discards it with the incarnation, so a message whose only redaction died with its redacter stays owed to the fleet. Persistence is not a promotion route: a persisted frontier carries no deletion to anyone. No false positive across both proptests at 256 cases on the box.

<a id="hunk-10"></a>
### tests/bookmark_causality.rs `@@ -266,44 +395,127 @@ impl Node {`

```diff
@@ -266,44 +395,127 @@ impl Node {
     }
 }
 
-/// The whole simulated world: the fleet and the shared emission log.
+/// The whole simulated world: the fleet, the shared emission log, and the
+/// per-world sources of every input the plan does not carry.
 struct World {
     nodes: Vec<Node>,
     emissions: EmissionLog,
     next_seq: u64,
+    /// The source of every universe's [`Network`] identifier, seeded with
+    /// [`NETWORK_SEED`] so the tie-break between fresh peers replays.
+    rng: ChaCha8Rng,
+    /// Every network this world has seeded, to assert they are pairwise
+    /// distinct: two universes sharing an identifier would gossip as one
+    /// network while holding incomparable histories.
+    networks: BTreeSet<Network>,
+    /// The path this world took through every place a network identifier
+    /// decides the outcome, which a reconstructed counterexample pins.
+    path: Vec<PathEvent>,
+    /// Every redaction the network has learned of, by network: the messages
+    /// a session or the heal may legitimately end without.
+    ///
+    /// A redaction is modeled like an emission: it waits in its redacter's
+    /// [`pending_redactions`](Node::pending_redactions) until some other live
+    /// peer's frontier dominates the redacter's (the deletion propagated),
+    /// and a crash before then discards it, so a message whose only
+    /// redaction died with its redacter stays owed to the fleet.
+    redacted: BTreeMap<Network, Vec<Redacted>>,
+    /// The winning network's live content when the heal began, and the
+    /// network itself: what [`assert_healed`](World::assert_healed) holds the
+    /// converged fleet to, less the ledger. `None` until a heal has run.
+    heal_start: Option<(Network, BTreeSet<u64>)>,
+}
+
+/// One redaction as the harness performed it.
+///
+/// The network, the message's id, the version the application handed to
+/// `redact`, and the redacter's frontier afterwards, which another peer must
+/// dominate to have learned of the deletion.
+#[derive(Debug, Clone, PartialEq, Eq)]
+struct Redacted {
+    network: Network,
+    seq: u64,
+    version: Version,
+    frontier: Version,
+}
+
+/// One step of the path a plan takes through the places where a universe's
+/// identifier decides what happens; the reconstructed counterexamples pin
+/// the whole path.
+#[derive(Debug, Clone, PartialEq, Eq)]
+enum PathEvent {
+    /// A cross-network session resolved: `loser` re-bootstraps into `winner`.
+    Mismatch { winner: usize, loser: usize },
+    /// `newcomer` bootstrapped from `server`, or stayed dormant on failure.
+    Bootstrap {
+        newcomer: usize,
+        server: usize,
+        booted: bool,
+    },
+    /// `who` seeded a fresh universe: no live member of its network remained.
+    Reseeded(usize),
 }
 
 impl World {
-    /// Build `n` nodes, each its own freshly-seeded universe.
-    fn seed(n: usize, read_faults: Vec<Vec<bool>>, write_faults: Vec<Vec<bool>>) -> Self {
-        let nodes = (0..n)
-            .map(|label| {
-                let store = Arc::new(Mutex::new(None));
-                let faults = Arc::new(Mutex::new(FaultFeed::new(
-                    read_faults[label].clone(),
-                    write_faults[label].clone(),
-                )));
-                let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), label);
-                let peer = block_on(Peer::<Msg>::seed().sync_window_floor().bookmark(bookmark))
-                    .expect("a pristine seed attaches its bookmark without touching storage");
-                let network = peer.network();
-                Node {
-                    state: NodeState::Live(Box::new(peer.into_rumors())),
-                    store,
-                    faults,
-                    network,
-                    pending: Vec::new(),
-                    label,
-                }
-            })
-            .collect();
+    /// A world with no nodes yet and fresh per-world sources.
+    fn empty() -> Self {
         World {
-            nodes,
+            nodes: Vec::new(),
             emissions: EmissionLog::default(),
             next_seq: 0,
+            rng: ChaCha8Rng::seed_from_u64(NETWORK_SEED),
+            networks: BTreeSet::new(),
+            path: Vec::new(),
+            redacted: BTreeMap::new(),
+            heal_start: None,
         }
     }
 
+    /// Seed a fresh universe for `bookmark`'s node, drawing its network from
+    /// the world's RNG and asserting the identifier is new to this world.
+    fn seed_universe(
+        &mut self,
+        bookmark: FlakyInMemoryBookmark,
+    ) -> Peer<Msg, FlakyInMemoryBookmark> {
+        let peer = block_on(
+            Peer::<Msg>::seed_rng(&mut self.rng)
+                .sync_window_floor()
+                .bookmark(bookmark),
+        )
+        .expect("a pristine seed attaches its bookmark without touching storage");
+        assert!(
+            self.networks.insert(peer.network()),
+            "the network RNG handed out {:?} twice: two universes would share an identifier",
+            peer.network(),
+        );
+        peer
+    }
+
+    /// Build `n` nodes, each its own freshly-seeded universe.
+    fn seed(n: usize, read_faults: Vec<Vec<bool>>, write_faults: Vec<Vec<bool>>) -> Self {
+        let mut world = World::empty();
+        for label in 0..n {
+            let store = Arc::new(Mutex::new(None));
+            let faults = Arc::new(Mutex::new(FaultFeed::new(
+                read_faults[label].clone(),
+                write_faults[label].clone(),
+            )));
+            let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), label);
+            let peer = world.seed_universe(bookmark);
+            let network = peer.network();
+            world.nodes.push(Node {
+                state: NodeState::Live(Box::new(peer.into_rumors())),
+                store,
+                faults,
+                network,
+                pending: Vec::new(),
+                pending_redactions: Vec::new(),
+                label,
+            });
+        }
+        world
+    }
+
     /// Build `n` nodes that all share *one* network: node 0 seeds it, and nodes
     /// `1..n` bootstrap into it over clean wires.
     ///
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 398:
>
> World gains the per-world sources the plan does not carry: rng (the network source), networks (every id seeded, so pairwise distinctness is asserted rather than assumed, per the resolution), and path (the record of every place a network id decided the outcome). The path log replaces a first draft that logged only successful bootstraps, which could not distinguish an undetected mismatch from a re-bootstrap that failed under bookmark faults; the reconstructed test needed that distinction to pin its winner.

<!-- annotation -->
> **tests-bookmark-8** (T8), line 442:
>
> PathEvent names the three places the tie-break or a revival decides what happens: a mismatch resolution (winner and loser), a bootstrap attempt with its outcome, and a re-seed. Equality on the whole path is what the reconstructed tests assert.

<!-- annotation -->
> **tests-bookmark-8** (T8), line 460:
>
> World::empty and seed_universe replace the three inline Peer::seed() calls with one seeded path; seed_universe draws from the world's RNG and asserts the id is new to this world. World::seed and single_network now build through empty() so the RNG exists before the first universe is drawn.

<!-- annotation -->
> **tests-bookmark-9** (T8), line 422:
>
> The per-network redaction ledger (id and version of each redaction, in the network it was performed in) and the heal-start snapshot of the winning network's live ids, which assert_healed holds the converged fleet to. Scoped to the winning network because the collapse discards every other network's content.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 422:
>
> Item 4: the ledger's doc states the model.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 435:
>
> Item 4: a redaction records its network and the redacter's frontier afterwards, which the promotion test compares against.

<a id="hunk-11"></a>
### tests/bookmark_causality.rs `@@ -322,36 +534,33 @@ impl World {`

```diff
@@ -322,36 +534,33 @@ impl World {
         assert!(n >= 1, "a fleet needs at least one node");
         let reliable = || Arc::new(Mutex::new(FaultFeed::new(Vec::new(), Vec::new())));
 
+        let mut world = World::empty();
         let store = Arc::new(Mutex::new(None));
         let faults = reliable();
         let bookmark = FlakyInMemoryBookmark::new(store.clone(), faults.clone(), 0);
-        let peer = block_on(Peer::<Msg>::seed().sync_window_floor().bookmark(bookmark))
-            .expect("a pristine seed attaches its bookmark without touching storage");
+        let peer = world.seed_universe(bookmark);
         let network = peer.network();
-        let mut nodes = vec![Node {
+        world.nodes.push(Node {
             state: NodeState::Live(Box::new(peer.into_rumors())),
             store,
             faults,
             network,
             pending: Vec::new(),
+            pending_redactions: Vec::new(),
             label: 0,
-        }];
+        });
         // The rest start dormant in node 0's network; the bootstraps below make
         // them live forks of its identity.
-        nodes.extend((1..n).map(|label| Node {
+        world.nodes.extend((1..n).map(|label| Node {
             state: NodeState::Dormant,
             store: Arc::new(Mutex::new(None)),
             faults: reliable(),
             network,
             pending: Vec::new(),
+            pending_redactions: Vec::new(),
             label,
         }));
 
-        let mut world = World {
-            nodes,
-            emissions: EmissionLog::default(),
-            next_seq: 0,
-        };
         for who in 1..n {
             assert!(
                 world.bootstrap_into(who, 0),
```

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 560:
>
> Item 4: every node the fleet constructors build starts with no pending redactions, like its pending emissions.

<a id="hunk-12"></a>
### tests/bookmark_causality.rs `@@ -365,6 +574,25 @@ impl World {`

```diff
@@ -365,6 +574,25 @@ impl World {
         self.nodes.len()
     }
 
+    /// Whether live node `who` holds the message `seq`.
+    fn holds(&self, who: usize, seq: u64) -> bool {
+        self.nodes[who]
+            .live()
+            .is_some_and(|rumors| rumors.snapshot().iter().any(|(_, value)| *value == seq))
+    }
+
+    /// The version stamped on message `seq` at live node `who`.
+    fn leaf_version(&self, who: usize, seq: u64) -> Version {
+        self.nodes[who]
+            .live()
+            .expect("a live node")
+            .snapshot()
+            .iter()
+            .find(|(_, value)| **value == seq)
+            .map(|(version, _)| version.clone())
+            .expect("the node holds the message")
+    }
+
     /// Whether some peer *other than* `who` is live in `who`'s network: a peer
     /// `who` could reboot from.
     ///
```

<!-- annotation -->
> **tests-bookmark-9** (T8), line 578:
>
> Two read helpers (holds, leaf_version) for the constructed-shape test and the negative control; the snapshot lookups the tests would otherwise spell out.

<a id="hunk-13"></a>
### tests/bookmark_causality.rs `@@ -378,6 +606,76 @@ impl World {`

```diff
@@ -378,6 +606,76 @@ impl World {
             .any(|k| k != who && self.nodes[k].is_live() && self.nodes[k].network == network)
     }
 
+    /// The ids of every message live at `who`, or none while dormant.
+    fn live_ids(&self, who: usize) -> BTreeSet<u64> {
+        self.nodes[who]
+            .live()
+            .map(|rumors| rumors.snapshot().iter().map(|(_, value)| *value).collect())
+            .unwrap_or_default()
+    }
+
+    /// The ids of every message redacted in `network`.
+    fn ledger(&self, network: Network) -> BTreeSet<u64> {
+        self.redacted
+            .get(&network)
+            .map(|entries| entries.iter().map(|entry| entry.seq).collect())
+            .unwrap_or_default()
+    }
+
+    /// After a session both sides completed, every message `expected`
+    /// (what the sides held before it, less the network's ledger) is live at
+    /// each of `sides`.
+    ///
+    /// The recycle check by consequence at session granularity: a reclaimed
+    /// region's re-issued versions make the causal sieve delete the message
+    /// they collide with at whichever session first meets them, which may be
+    /// long before the heal. Sound because a correct bookmark's frontier
+    /// dominates a durable emission only by having merged it or a redacter's
+    /// frontier, so the ledger's entries are the only legitimate losses.
+    /// Complete because the session contract commits whole or not at all:
+    /// on `Err` the replica's content is unchanged, and the exceptions
+    /// (`Epilogue`, and a `Bookmark` error after absorbing a retiree) commit
+    /// the session whole, so a destruction cannot hide inside an unchecked
+    /// failed session; `expected` is recomputed from live content before
+    /// every session, so a partial commit could never have produced a false
+    /// positive either way.
+    fn assert_session_preserved(
+        &self,
+        step: &str,
+        network: Network,
+        expected: &BTreeSet<u64>,
+        sides: &[usize],
+    ) {
+        for &k in sides {
+            let held = self.live_ids(k);
+            let destroyed: Vec<u64> = expected.difference(&held).copied().collect();
+            assert!(
+                destroyed.is_empty(),
+                "messages {destroyed:?} were live in network {network:?} before {step} and never \
+                 redacted there, but node {k} does not hold them after it: a rebooted peer \
+                 re-issued versions below a frontier the fleet durably held, and the causal \
+                 sieve read them as deletions",
+            );
+        }
+    }
+
+    /// Whether `who`'s bookmark still has an injected failure scheduled.
+    fn bookmark_may_fail(&self, who: usize) -> bool {
+        self.nodes[who].faults.lock().unwrap().may_fail()
+    }
+
+    /// Whether a session between `a` and `b` can fail for a reason other
+    /// than a crate bug or a network mismatch: a wire cut scheduled on either
+    /// side, or a bookmark fault still queued on either side's feed.
+    ///
+    /// Decided before the session, since the session consumes the schedule.
+    fn session_may_fail(&self, a: usize, b: usize, fault_a: FaultPlan, fault_b: FaultPlan) -> bool {
+        !fault_a.is_clean()
+            || !fault_b.is_clean()
+            || self.bookmark_may_fail(a)
+            || self.bookmark_may_fail(b)
+    }
+
     /// Emit a fresh unique message from `who`, capturing its full causal
     /// coordinate and holding it *pending* until it is persisted or propagated.
     fn send(&mut self, who: usize) {
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 663:
>
> The fault regime is decided per step, not by a world-wide flag: a session may fail legitimately iff a cut is scheduled on either wire or a bookmark fault remains on either feed, read before the session because the session consumes the schedule. I chose this over the resolution's `reliable: bool` because it also asserts Ok for the clean steps inside faulted plans, and single_network worlds fall out as the case where nothing is ever scheduled.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 610:
>
> Item 1 helpers: the live ids of a node and the network's ledger as id sets.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 642:
>
> Item 1. The per-session survival check: before each same-network session U = (live(a) union live(b)) minus the ledger; after a session both sides complete, each side holds U. Soundness at the check: the frontier-domination premise, plus the session contract I verified in Rumors::gossip's docs: on Err the replica's content is unchanged, and the three exceptions (Epilogue: every local effect committed; a donation failure: identity only; Bookmark after absorbing a retiree: content and identity committed) commit whole, so an unchecked failed session never applies content in part. No false positive across the two proptests at 256 cases on the box.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 635:
>
> Item 5: the per-session check's doc names the frontier-domination argument as the soundness premise and whole-or-nothing commit as what makes the checks complete, and says why a partial commit could never have produced a false positive (expected is recomputed from live content before every session).

<a id="hunk-14"></a>
### tests/bookmark_causality.rs `@@ -389,9 +687,10 @@ impl World {`

```diff
@@ -389,9 +687,10 @@ impl World {
         };
         let network = rumors.network();
         rumors.send(id).unwrap(); // one commit per send
-        // Read back the leaf's version. Under the current-thread schedule no
-        // other task runs between the commit and here, so the lookup is
-        // race-free and the just-sent unique id is present exactly once.
+        // Read back the leaf's version. Nothing else runs between the commit
+        // and here (every session is its own single-threaded `block_on`), so
+        // the lookup is race-free and the just-sent unique id is present
+        // exactly once.
         let snapshot = rumors.snapshot();
         let mut version = None;
         for (leaf_version, value) in snapshot.iter() {
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 690:
>
> Nit: the send's race-freedom comment spoke of the current-thread schedule and other tasks; there are no tasks now, so it states the actual reason (every session is its own single-threaded block_on).

<a id="hunk-15"></a>
### tests/bookmark_causality.rs `@@ -408,19 +707,35 @@ impl World {`

```diff
@@ -408,19 +707,35 @@ impl World {
         });
     }
 
-    /// Redact one of `who`'s live messages, indexed mod the live count. Pure
-    /// adversarial pressure: it advances the clock without a tracked emission.
+    /// Redact one of `who`'s live messages, indexed mod the live count, and
+    /// hold it pending until another peer learns of it.
+    ///
+    /// Adversarial pressure on the version order (a clock advance with no
+    /// tracked emission), and the one legitimate way a message leaves the
+    /// fleet.
     fn redact(&mut self, who: usize, which: usize) {
         self.revive(who);
         let Some(rumors) = self.nodes[who].live() else {
             return;
         };
+        let network = rumors.network();
         let snapshot = rumors.snapshot();
-        let versions: Vec<Version> = snapshot.iter().map(|(v, _)| v.clone()).collect();
-        if versions.is_empty() {
+        let leaves: Vec<(Version, u64)> = snapshot
+            .iter()
+            .map(|(version, value)| (version.clone(), *value))
+            .collect();
+        if leaves.is_empty() {
             return;
         }
-        rumors.redact(&versions[which % versions.len()]);
+        let (version, seq) = leaves[which % leaves.len()].clone();
+        rumors.redact(&version);
+        let frontier = rumors.snapshot().latest().clone();
+        self.nodes[who].pending_redactions.push(Redacted {
+            network,
+            seq,
+            version,
+            frontier,
+        });
     }
 
     /// Promote every pending emission of `who` that has become **known to the
```

<!-- annotation -->
> **tests-bookmark-9** (T8), line 716:
>
> redact now records what it redacted. The doc drops the claim that a redaction is pure pressure: it is also the one legitimate way a message leaves the fleet, which is exactly why the ledger exists.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 711:
>
> Item 4: redact pushes onto the redacter's pending list instead of the ledger, capturing the post-redaction frontier.

<a id="hunk-16"></a>
### tests/bookmark_causality.rs `@@ -438,6 +753,10 @@ impl World {`

```diff
@@ -438,6 +753,10 @@ impl World {
     /// Either way the network can no longer forget the version, so reusing it
     /// would be a recycle. Only an emission that is *neither* persisted *nor*
     /// propagated remains pending, erasable by a crash with no recycle.
+    ///
+    /// A pending redaction is promoted to the network's ledger by the second
+    /// route only: a persisted frontier carries no deletion to anyone, so
+    /// only propagation makes a redaction one the fleet has learned of.
     fn secure(&mut self, who: usize) {
         let record = decompose_store(&self.nodes[who].store);
         // Every other live peer's `(network, frontier)`: a peer knows `who`'s
```

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 757:
>
> Item 4: secure promotes pending redactions by propagation only; secure_and_lose discards them with the incarnation. The bootstrap, retire, and heal checks now run after the secure calls, so a redaction the session itself propagated is in the ledger when the session's survival check reads it.

<a id="hunk-17"></a>
### tests/bookmark_causality.rs `@@ -465,6 +784,23 @@ impl World {`

```diff
@@ -465,6 +784,23 @@ impl World {
             }
         }
         self.nodes[who].pending = still_pending;
+
+        let pending_redactions = std::mem::take(&mut self.nodes[who].pending_redactions);
+        let mut still_pending = Vec::new();
+        for redaction in pending_redactions {
+            let propagated = observers.iter().any(|(network, frontier)| {
+                *network == redaction.network && redaction.frontier <= *frontier
+            });
+            if propagated {
+                self.redacted
+                    .entry(redaction.network)
+                    .or_default()
+                    .push(redaction);
+            } else {
+                still_pending.push(redaction);
+            }
+        }
+        self.nodes[who].pending_redactions = still_pending;
     }
 
     /// Secure what is now known to the network, then discard the rest.
```

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 788:
>
> Item 4: the promotion loop applies the emission rule's propagated half to redactions, the redacter's post-redaction frontier being what an observer must dominate; there is no persisted half.

<a id="hunk-18"></a>
### tests/bookmark_causality.rs `@@ -477,6 +813,7 @@ impl World {`

```diff
@@ -477,6 +813,7 @@ impl World {
     fn secure_and_lose(&mut self, who: usize) {
         self.secure(who);
         self.nodes[who].pending.clear();
+        self.nodes[who].pending_redactions.clear();
     }
 
     /// Drop `who`'s in-memory state, keeping its durable store and schedule: a
```

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 816:
>
> Item 4: a crash or re-bootstrap discards the incarnation's pending redactions along with its pending emissions.

<a id="hunk-19"></a>
### tests/bookmark_causality.rs `@@ -492,7 +829,9 @@ impl World {`

```diff
@@ -492,7 +829,9 @@ impl World {
     /// A cross-network pair surfaces
     /// [`Error::NetworkMismatch`] on at least one side; the loser of the
     /// `(min_ticks, network)` tie-break re-bootstraps into the winner. Any other
-    /// error is an honest disruption that leaves both replicas unchanged.
+    /// error is a disruption, admitted only when the step scheduled a fault
+    /// or met a mismatch; a step that cannot fail must succeed on both sides,
+    /// and a session both sides complete loses no unredacted message.
     fn gossip(&mut self, a: usize, b: usize, fault_a: FaultPlan, fault_b: FaultPlan) {
         if a == b {
             return;
```

<!-- annotation -->
> **prose** (fresh-eyes repair), line 832:
>
> Item 5: the gossip doc no longer says a failed session leaves both replicas unchanged, which Epilogue contradicts.

<a id="hunk-20"></a>
### tests/bookmark_causality.rs `@@ -503,23 +842,28 @@ impl World {`

```diff
@@ -503,23 +842,28 @@ impl World {
             return;
         };
         let (ra, rb) = (ra.clone(), rb.clone());
-        // Each side owns its faulted link inside its own task, so when a wire
-        // fault kills one side it returns and *drops* its link, surfacing EOF
-        // to the counterparty. A bare `join!` would instead hold both sides'
-        // links until both finished, deadlocking the survivor on a read that
-        // never completes.
+        let may_fail = self.session_may_fail(a, b, fault_a, fault_b);
+        let same_network = self.nodes[a].network == self.nodes[b].network;
+        let network = self.nodes[a].network;
+        let held_before: BTreeSet<u64> = &self.live_ids(a) | &self.live_ids(b);
+        // Each side owns its faulted link inside its own `async move` block,
+        // so when a wire fault kills one side its block completes and
+        // `join!` drops the block, link included, surfacing EOF to the
+        // counterparty. Links declared outside the blocks would live until
+        // both sides finished, deadlocking the survivor on a read that never
+        // completes.
         let (out_a, out_b) = block_on(async {
             let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
-            let task_a = tokio::spawn(async move {
-                let mut link = fault::faulty(side_a, fault_a);
-                ra.gossip(&mut link).await
-            });
-            let task_b = tokio::spawn(async move {
-                let mut link = fault::faulty(side_b, fault_b);
-                rb.gossip(&mut link).await
-            });
-            let (out_a, out_b) = tokio::join!(task_a, task_b);
-            (out_a.expect("gossip task a"), out_b.expect("gossip task b"))
+            tokio::join!(
+                async move {
+                    let mut link = fault::faulty(side_a, fault_a);
+                    ra.gossip(&mut link).await
+                },
+                async move {
+                    let mut link = fault::faulty(side_b, fault_b);
+                    rb.gossip(&mut link).await
+                },
+            )
         });
 
         // The session ran each side's bookmark update before any mismatch, so
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 849:
>
> The experiment the resolution asked for, decided here: World::gossip is a join! over two async move blocks that each own their faulted link, under the closed-world poller. The faulted side's EOF does reach the survivor: the full faulted proptest (256 cases) and both reconstructed cases pass, so no timeout was added. The comment states the mechanism (join! drops a completed block, link included) and what would deadlock (links declared outside the blocks). Negative control (reversible mutation, recorded in the commit message): side a's gossip replaced by std::future::pending() fails bookmarking_never_recycles_a_version in 41 ms with `closed in-memory future became quiescent: Stalled` and writes the seed `[Gossip(0, 1, clean, clean)]` once the committed seeds are set aside; with them present, the cut-gossip seed reproduces the failure first and proptest writes nothing, as AGENTS.md documents.

<!-- annotation -->
> **tests-bookmark-12** (T8), line 845:
>
> gossip records the regime and whether the pair shared a network before the session (the session may re-bootstrap the loser).

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 848:
>
> gossip takes U before the session and checks both sides after a double Ok.

<a id="hunk-21"></a>
### tests/bookmark_causality.rs `@@ -529,6 +873,40 @@ impl World {`

```diff
@@ -529,6 +873,40 @@ impl World {
 
         let mismatched = matches!(out_a, Err(Error::NetworkMismatch { .. }))
             || matches!(out_b, Err(Error::NetworkMismatch { .. }));
+        for (side, out) in [(a, &out_a), (b, &out_b)] {
+            match out {
+                Ok(_) => {}
+                Err(Error::NetworkMismatch { .. }) => assert!(
+                    !same_network,
+                    "gossip {a}<->{b}: node {side} reported a network mismatch inside one network",
+                ),
+                Err(error) => {
+                    assert_not_codec_bug(&format!("gossip {a}<->{b}, node {side}"), error);
+                    assert!(
+                        may_fail || mismatched,
+                        "gossip {a}<->{b}: node {side} failed on a clean wire over reliable \
+                         bookmarks with no mismatch: {error:?}",
+                    );
+                }
+            }
+        }
+        assert!(
+            same_network || may_fail || mismatched,
+            "gossip {a}<->{b}: a cross-network session on a clean wire must surface the mismatch",
+        );
+        assert!(
+            same_network || out_a.is_err() || out_b.is_err(),
+            "gossip {a}<->{b}: a cross-network session completed on both sides",
+        );
+        if same_network && out_a.is_ok() && out_b.is_ok() {
+            let expected = &held_before - &self.ledger(network);
+            self.assert_session_preserved(
+                &format!("gossip {a}<->{b}"),
+                network,
+                &expected,
+                &[a, b],
+            );
+        }
         if mismatched {
             self.resolve_mismatch(a, b);
         }
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 876:
>
> Every gossip result is now judged on both sides: Ok passes; NetworkMismatch is admitted only across networks; any other error is classified and then admitted only if the step could fail or the counterparty mismatched (the mismatching side aborts and closes the wire, so the other side may see EOF). A clean cross-network session must surface its mismatch. Negative control (reversible mutation, recorded in the commit message): the witness's injection, every third World::gossip returning Err(PartyOverlap) on both sides, fails bookmarking_prevents_party_leakage at the first injected step with `gossip 0<->1, node 0: a protocol, codec, or bookmark-format bug, not an honest disruption: PartyOverlap`, where the suite before this change passed through 452 such injections.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair), line 899:
>
> Item 2 sibling: Ok on both sides of a cross-network session is a bug in any regime and panics unconditionally; the earlier assert (a clean cross-network session must surface its mismatch) stays for the error case.

<a id="hunk-22"></a>
### tests/bookmark_causality.rs `@@ -543,6 +921,7 @@ impl World {`

```diff
@@ -543,6 +921,7 @@ impl World {
             return;
         };
         let (winner, loser) = if ta >= tb { (a, b) } else { (b, a) };
+        self.path.push(PathEvent::Mismatch { winner, loser });
         self.bootstrap_into(loser, winner);
     }
 
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 924:
>
> resolve_mismatch records the winner and loser on the path before bootstrapping the loser.

<a id="hunk-23"></a>
### tests/bookmark_causality.rs `@@ -564,12 +943,16 @@ impl World {`

```diff
@@ -564,12 +943,16 @@ impl World {
     /// bookmarks on both sides stay flaky: the server's donating `slice`/`write`
     /// and `who`'s eager identity persist can each fail, which is precisely the
     /// adversarial persistence path. On any failure `who` is left dormant for a
-    /// later attempt.
+    /// later attempt. Over reliable bookmarks nothing can fail, and the step
+    /// asserts that both sides succeed.
     fn bootstrap_into(&mut self, who: usize, server: usize) -> bool {
         let Some(server_rumors) = self.nodes[server].live() else {
             return false;
         };
         let server_rumors = server_rumors.clone();
+        let may_fail = self.bookmark_may_fail(who) || self.bookmark_may_fail(server);
+        let network = self.nodes[server].network;
+        let served: BTreeSet<u64> = self.live_ids(server);
         // The prior incarnation's memory is about to vanish: secure what its
         // store persisted, lose the rest.
         self.secure_and_lose(who);
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 953:
>
> A bootstrap's wire is clean, so only the two feeds decide whether it may fail.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 955:
>
> A bootstrap copies the server's content whole, so the newcomer is checked against the server's live set less the ledger; same helper, same premise.

<a id="hunk-24"></a>
### tests/bookmark_causality.rs `@@ -577,30 +960,74 @@ impl World {`

```diff
@@ -577,30 +960,74 @@ impl World {
         // Drop any prior incarnation before creating the new one.
         self.nodes[who].state = NodeState::Dormant;
 
-        let booted = block_on(async {
+        let (boot_out, serve_out) = block_on(async {
             let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
-            // Spawn both sides so a failing one drops its link (see `gossip`).
-            let boot = tokio::spawn(async move {
-                let mut link = boot_side;
-                // `Ok(None)` cannot happen (the server is gossiping, not
-                // bootstrapping); a wire fault drops us to `None`, as does an
-                // injected persistence failure in the eager bookmark attach.
-                let peer = Peer::<Msg>::bootstrap()
-                    .join(&mut link)
-                    .await
-                    .ok()
-                    .flatten()?;
-                peer.sync_window_floor().bookmark(bookmark).await.ok()
-            });
-            let serve = tokio::spawn(async move {
-                let mut link = serve_side;
-                server_rumors.gossip(&mut link).await
-            });
-            let (boot_out, serve_out) = tokio::join!(boot, serve);
-            let _ = serve_out;
-            boot_out.expect("bootstrap task")
+            // Each side owns its link inside its block so a failing one drops
+            // it (see `gossip`).
+            tokio::join!(
+                async move {
+                    let mut link = boot_side;
+                    let peer = match Peer::<Msg>::bootstrap().join(&mut link).await {
+                        Ok(Some(peer)) => peer,
+                        Ok(None) => return Err(BootFailure::NoPeer),
+                        Err(error) => return Err(BootFailure::Join(error)),
+                    };
+                    peer.sync_window_floor()
+                        .bookmark(bookmark)
+                        .await
+                        .map_err(|unbookmarked| BootFailure::Attach(unbookmarked.error))
+                },
+                async move {
+                    let mut link = serve_side;
+                    server_rumors.gossip(&mut link).await
+                },
+            )
         });
 
+        // The wire is clean, so each side fails only through the bookmarks:
+        // the server through its own donating persist, whose abort closes
+        // the wire on the newcomer (a truncated hand-off), and the newcomer
+        // through its own eager attach persist after the session. Neither
+        // can fail over reliable bookmarks.
+        let step = format!("bootstrap of {who} from {server}");
+        if let Err(error) = &serve_out {
+            assert_not_codec_bug(&format!("{step}, serving side"), error);
+            assert!(
+                may_fail,
+                "{step}: the serve failed on a clean wire over reliable bookmarks: {error:?}",
+            );
+        }
+        let booted = match boot_out {
+            Ok(peer) => Some(peer),
+            Err(BootFailure::NoPeer) => {
+                panic!("{step}: the server was gossiping, so the join cannot end without a peer")
+            }
+            Err(BootFailure::Join(error)) => {
+                assert_not_codec_bug(&format!("{step}, joining side"), &error);
+                assert!(
+                    may_fail,
+                    "{step}: the join failed on a clean wire over reliable bookmarks: {error:?}",
+                );
+                None
+            }
+            Err(BootFailure::Attach(error)) => {
+                assert!(
+                    !matches!(error, BookmarkIo::Format(_)),
+                    "{step}: the attach rejected a bookmark file the crate wrote: {error:?}",
+                );
+                assert!(
+                    may_fail,
+                    "{step}: the attach persist failed over a reliable bookmark: {error:?}",
+                );
+                None
+            }
+        };
+
+        self.path.push(PathEvent::Bootstrap {
+            newcomer: who,
+            server,
+            booted: booted.is_some(),
+        });
         match booted {
             Some(peer) => {
                 self.nodes[who].network = peer.network();
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 1026:
>
> bootstrap_into records the attempt and its outcome on the path; a failed attempt is as much a fact about the plan's path as a success.

<!-- annotation -->
> **tests-bookmark-11** (T8), line 965:
>
> bootstrap_into on join!. The serve side's result is still discarded at this commit (`let _ = serve_out`) and the boot side still folds errors to None: binding and asserting them is tests-bookmark-12's change, landed in the next commit, so this commit stays purely structural.

<!-- annotation -->
> **tests-bookmark-12** (T8), line 971:
>
> The joining side keeps every outcome: a peerless join (impossible against a gossiping server) panics outright; a join error is classified and admitted only under a possible fault; an attach failure is a bug if it is Format(_) (the crate wrote every byte the store holds) and otherwise admitted only under a possible fault.

<!-- annotation -->
> **tests-bookmark-12** (T8), line 987:
>
> The comment the entry asked to rewrite: on this clean wire the server fails only through its own donating persist, whose abort closes the wire on the newcomer as a truncated hand-off, and the newcomer only through its own attach persist after the session; neither can fail over reliable bookmarks. The serve result is no longer discarded: it is classified and asserted Ok whenever nothing could fail. Under join! a serve-side panic propagates to the test directly, which is the acceptance's `serve_out.expect` without a JoinHandle. Negative controls (reversible mutations, recorded in the commit message): a planted Io(InvalidData) in the serve block fails bookmarking_prevents_party_leakage at `bootstrap of 1 from 0, serving side` with the classified error, at the offending step and not at heal; a planted panic! in the serve block fails bookmarking_never_recycles_a_version instead of leaving the node dormant.

<a id="hunk-25"></a>
### tests/bookmark_causality.rs `@@ -609,6 +1036,9 @@ impl World {`

```diff
@@ -609,6 +1036,9 @@ impl World {
                 // the server's donating persist secures its emissions too.
                 self.secure(who);
                 self.secure(server);
+                // A bootstrap copies the server's content whole.
+                let expected = &served - &self.ledger(network);
+                self.assert_session_preserved(&step, network, &expected, &[who]);
                 true
             }
             None => false,
```

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 1041:
>
> Item 4: the bootstrap's check runs after both secure calls, so a redaction the bootstrap itself propagated is in the ledger the check reads.

<a id="hunk-26"></a>
### tests/bookmark_causality.rs `@@ -631,13 +1061,19 @@ impl World {`

```diff
@@ -631,13 +1061,19 @@ impl World {
             return;
         }
         // No reachable member of its old network (or the rejoin's persistence
-        // failed): start fresh. Its old network's identity is left stranded —
+        // failed): start fresh. Its old network's identity is left stranded --
         // a harmless leak, never a corruption. The old incarnation's memory
-        // vanishes, so secure what was persisted and lose the rest.
+        // vanishes, so secure what was persisted and lose the rest. A
+        // single-network world never gets here: its crash guard keeps a live
+        // member, and its reliable bookmarks cannot fail the rejoin.
+        assert!(
+            self.networks.len() > 1,
+            "node {who} of a single-network world found no live member to reboot from",
+        );
         self.secure_and_lose(who);
+        self.path.push(PathEvent::Reseeded(who));
         let bookmark = self.nodes[who].bookmark();
-        let peer = block_on(Peer::<Msg>::seed().sync_window_floor().bookmark(bookmark))
-            .expect("a pristine seed attaches its bookmark without touching storage");
+        let peer = self.seed_universe(bookmark);
         self.nodes[who].network = peer.network();
         self.nodes[who].state = NodeState::Live(Box::new(peer.into_rumors()));
     }
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 1074:
>
> revive's fresh-universe fallback records the re-seed and draws its network from the world's RNG instead of OsRng.

<!-- annotation -->
> **tests-bookmark-12** (T8), line 1067:
>
> revive's fresh-universe fallback asserts it is unreachable in a single-network world: the crash guard keeps a live member and reliable bookmarks cannot fail the rejoin, so re-seeding there would silently abandon the one-network premise the leakage property rests on (the entry's `revive then seeds a fresh universe on a failed rejoin` finding).

<!-- annotation -->
> **prose** (fresh-eyes repair), line 1064:
>
> Item 5: em-dash to spaced double hyphen in the revive comment this lane extended.

<a id="hunk-27"></a>
### tests/bookmark_causality.rs `@@ -646,9 +1082,10 @@ impl World {`

```diff
@@ -646,9 +1082,10 @@ impl World {
     ///
     /// Same-network
     /// only: a cross-network retire cannot be absorbed, so it is skipped. The
-    /// retiree's durable store may later resurrect it — exercising party reuse
+    /// retiree's durable store may later resurrect it -- exercising party reuse
     /// across a donation, where a failed `slice` would let the donated region
-    /// live twice.
+    /// live twice. Over reliable bookmarks the clean-wire retirement cannot
+    /// fail: the absorber must succeed and the retiree must report `Retired`.
     fn retire(&mut self, retiree: usize, absorber: usize) {
         if retiree == absorber {
             return;
```

<!-- annotation -->
> **prose** (fresh-eyes repair), line 1085:
>
> Item 5: em-dash to spaced double hyphen in the retire doc this lane extended.

<a id="hunk-28"></a>
### tests/bookmark_causality.rs `@@ -662,6 +1099,9 @@ impl World {`

```diff
@@ -662,6 +1099,9 @@ impl World {
             return;
         };
         let absorber_rumors = absorber_rumors.clone();
+        let may_fail = self.session_may_fail(retiree, absorber, FaultPlan::NONE, FaultPlan::NONE);
+        let network = self.nodes[absorber].network;
+        let held_before: BTreeSet<u64> = &self.live_ids(retiree) | &self.live_ids(absorber);
         // Take the retiree's sole handle so it can become a `Peer` immediately.
         let NodeState::Live(retiree_rumors) =
             std::mem::replace(&mut self.nodes[retiree].state, NodeState::Dormant)
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 1102:
>
> retire computes its regime the same way (clean wire, two feeds).

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 1104:
>
> retire takes U before the session and checks the absorber after Retired with the absorber Ok.

<a id="hunk-29"></a>
### tests/bookmark_causality.rs `@@ -669,64 +1109,83 @@ impl World {`

```diff
@@ -669,64 +1109,83 @@ impl World {
             return;
         };
 
-        let outcome = block_on(async {
+        let (outcome, absorbed) = block_on(async {
             let (ret_side, abs_side) = rumors::link::memory_with_capacity(LINK_BUF);
-            // Spawn both sides so a failing one drops its link (see `gossip`).
-            // The retiree becomes a `Peer` inside its task: it holds the sole
-            // handle to its set, so `try_into_peer` resolves at once.
-            let retire = tokio::spawn(async move {
-                let mut link = ret_side;
-                let peer = retiree_rumors
-                    .try_into_peer()
-                    .await
-                    .expect("the node holds the sole handle to its set");
-                peer.retire(&mut link).await
-            });
-            let absorb = tokio::spawn(async move {
-                let mut link = abs_side;
-                absorber_rumors.gossip(&mut link).await
-            });
-            let (retire_out, gossip_out) = tokio::join!(retire, absorb);
-            (
-                retire_out.expect("retire task"),
-                gossip_out.expect("absorb task"),
+            // Each side owns its link inside its block so a failing one drops
+            // it (see `gossip`). The retiree becomes a `Peer` inside its
+            // block: it holds the sole handle to its set, so `try_into_peer`
+            // resolves at once.
+            tokio::join!(
+                async move {
+                    let mut link = ret_side;
+                    let peer = retiree_rumors
+                        .try_into_peer()
+                        .await
+                        .expect("the node holds the sole handle to its set");
+                    peer.retire(&mut link).await
+                },
+                async move {
+                    let mut link = abs_side;
+                    absorber_rumors.gossip(&mut link).await
+                },
             )
         });
-        let (outcome, absorbed) = outcome;
-        // Never swallow the absorber's result: a retirement's whole point is the
-        // hand-off, and silently dropping a failed absorption is exactly what hid
-        // the codec leak this test was written to catch. The retire session runs
-        // over a *clean* wire, so the absorber can only fail honestly by an
-        // injected bookmark fault (`Error::Bookmark`) or by the retiree safely
-        // aborting its own bookmark fault and closing the wire (the typed
-        // `HandOffTruncated`, or `UnexpectedEof` elsewhere in the session).
-        // A decode failure (`InvalidData`, `HandOffMalformed`) means a
-        // fully-received frame was malformed — a protocol/codec bug like the
-        // non-canonical party that motivated this check — so surface it loudly.
+        // Never swallow the absorber's result: a retirement's whole point is
+        // the hand-off, and a silently dropped failed absorption is what hid
+        // the codec leak this test was written to catch. The wire is clean,
+        // so the absorber fails only by an injected bookmark fault
+        // (`Error::Bookmark`) or by the retiree aborting on its own bookmark
+        // fault and closing the wire (`HandOffTruncated`, or `UnexpectedEof`
+        // elsewhere in the session); over reliable bookmarks it cannot fail.
+        let step = format!("retire of {retiree} into {absorber}");
         if let Err(error) = &absorbed {
-            let codec_bug = matches!(
-                error,
-                Error::Io(io) if io.kind() == std::io::ErrorKind::InvalidData,
-            ) || matches!(error, Error::HandOffMalformed { .. });
+            assert_not_codec_bug(&format!("{step}, absorber"), error);
             assert!(
-                !codec_bug,
-                "retire absorber failed to decode on a clean wire: a protocol/codec bug, \
-                 not an honest disruption: {error:?}",
+                may_fail,
+                "{step}: the absorber failed on a clean wire over reliable bookmarks: {error:?}",
             );
         }
+        if let Retire::Recovered { error, .. } | Retire::Uncertain { error } = &outcome {
+            assert_not_codec_bug(&format!("{step}, retiree"), error);
+        }
+        assert!(
+            may_fail || matches!(outcome, Retire::Retired),
+            "{step}: a clean-wire retirement over reliable bookmarks must land as \
+             `Retired`, not {outcome:?}",
+        );
 
+        let retired = matches!(outcome, Retire::Retired) && absorbed.is_ok();
         match outcome {
             // Donated: the retiree's memory is consumed, so secure what it
             // persisted and lose the rest.
             Retire::Retired | Retire::Uncertain { .. } => self.secure_and_lose(retiree),
             // Unchanged: hand the intact peer back to life; its persisted
             // emissions are durable, its unpersisted ones remain pending.
-            Retire::Declined { peer } | Retire::Recovered { peer, .. } => {
+            Retire::Recovered { peer, .. } => {
                 self.nodes[retiree].state = NodeState::Live(Box::new(peer.into_rumors()));
                 self.secure(retiree);
             }
+            // The absorber gossips; only a retiring counterparty declines.
+            Retire::Declined { .. } => {
+                panic!("{step}: the absorber was gossiping, so the retirement cannot be declined")
+            }
         }
         self.secure(absorber);
+        if retired {
+            // The retiree reconciled with the absorber before donating and
+            // then vanished, so every redaction the absorber held pending
+            // reached a peer that is no longer live to witness it: promote
+            // them all, since `secure`'s observer rule cannot see them.
+            let learned = std::mem::take(&mut self.nodes[absorber].pending_redactions);
+            for redaction in learned {
+                self.redacted
+                    .entry(redaction.network)
+                    .or_default()
+                    .push(redaction);
+            }
+            let expected = &held_before - &self.ledger(network);
+            self.assert_session_preserved(&step, network, &expected, &[absorber]);
+        }
     }
 
     /// Drive the fleet to a single network and a content fixed point over clean
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 1114:
>
> retire on join!: the retiree's try_into_peer and retire run in one block, the absorber's gossip in the other; the tuple destructures straight out of block_on.

<!-- annotation -->
> **tests-bookmark-12** (T8), line 1140:
>
> retire's inline classifier is replaced by the shared one, applied to the absorber's error and, as the resolution asks, to Retire::Recovered { error } and Retire::Uncertain { error }. Over reliable bookmarks the absorber must succeed and the retiree must land as Retired: Declined arises only when the counterparty is itself retiring, which this harness never does, and both proptests (512 cases) confirm no other outcome occurs on HEAD.

<!-- annotation -->
> **prose** (T141), line 1133:
>
> The retire comment I had already edited said the absorber `can only fail honestly`; it now says what fails it, in fewer words, without the reserved word.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair), line 1169:
>
> Item 2 sibling: Declined arises only from a retiring counterparty, which the absorber never is here, so it panics unconditionally like BootFailure::NoPeer.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 1175:
>
> Item 4, found by the proptests at 256 cases on the box (shrunk plan: Send(0), Gossip(0,1), Redact(1,0), Retire(0,1)): a redaction pending at the absorber propagates to the retiree during the retire, and the retiree then vanishes, so secure's observer rule never sees a live peer dominating it and the ledger never learns of it, a false positive at the absorber's check. On Retired the absorber's pending redactions are promoted outright: the retiree reconciled with them before donating.

<a id="hunk-30"></a>
### tests/bookmark_causality.rs `@@ -746,6 +1205,22 @@ impl World {`

```diff
@@ -746,6 +1205,22 @@ impl World {
             .max_by_key(|&k| self.tuple(k))
             .expect("a non-empty fleet");
         let winning_network = self.nodes[winner].network;
+        // What the winning network holds as the heal begins: every message
+        // live at any of its live members. The other networks' content is
+        // discarded by the collapse below, so only the winner's is held to
+        // survive.
+        let live_at_start: BTreeSet<u64> = (0..self.n())
+            .filter(|&k| self.nodes[k].network == winning_network)
+            .filter_map(|k| self.nodes[k].live())
+            .flat_map(|rumors| {
+                rumors
+                    .snapshot()
+                    .iter()
+                    .map(|(_, value)| *value)
+                    .collect::<Vec<_>>()
+            })
+            .collect();
+        self.heal_start = Some((winning_network, live_at_start));
         for who in 0..self.n() {
             if who != winner && self.nodes[who].network != winning_network {
                 assert!(
```

<!-- annotation -->
> **tests-bookmark-9** (T8), line 1212:
>
> heal snapshots the winning network's live content once everyone is revived and before the collapse. Taken after the revivals so a revived node's content is its server's; the collapse does not change the winner's members' content.

<a id="hunk-31"></a>
### tests/bookmark_causality.rs `@@ -783,26 +1258,32 @@ impl World {`

```diff
@@ -783,26 +1258,32 @@ impl World {
             return;
         };
         let (ra, rb) = (ra.clone(), rb.clone());
+        let network = self.nodes[a].network;
+        let held_before: BTreeSet<u64> = &self.live_ids(a) | &self.live_ids(b);
         let (out_a, out_b) = block_on(async {
             let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
-            let task_a = tokio::spawn(async move {
-                let mut link = side_a;
-                ra.gossip(&mut link).await
-            });
-            let task_b = tokio::spawn(async move {
-                let mut link = side_b;
-                rb.gossip(&mut link).await
-            });
-            let (out_a, out_b) = tokio::join!(task_a, task_b);
-            (
-                out_a.expect("heal gossip task A"),
-                out_b.expect("heal gossip task B"),
+            tokio::join!(
+                async move {
+                    let mut link = side_a;
+                    ra.gossip(&mut link).await
+                },
+                async move {
+                    let mut link = side_b;
+                    rb.gossip(&mut link).await
+                },
             )
         });
         out_a.expect("clean heal gossip A");
         out_b.expect("clean heal gossip B");
         self.secure(a);
         self.secure(b);
+        let expected = &held_before - &self.ledger(network);
+        self.assert_session_preserved(
+            &format!("heal gossip {a}<->{b}"),
+            network,
+            &expected,
+            &[a, b],
+        );
     }
 
     /// Each live peer's `(hash, latest)` fingerprint, for fixed-point detection.
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 1265:
>
> clean_gossip on join!, so the heal's sessions get the stall detector too.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 1282:
>
> The heal's sessions get the per-session check too; this is where the known-bad artifact fires.

<a id="hunk-32"></a>
### tests/bookmark_causality.rs `@@ -817,8 +1298,9 @@ impl World {`

```diff
@@ -817,8 +1298,9 @@ impl World {
             .collect()
     }
 
-    /// After a clean heal: every live peer holds identical content (equal hash
-    /// and frontier), and their live parties are pairwise disjoint.
+    /// After a clean heal: every live peer holds identical content, their live
+    /// parties are pairwise disjoint, and every unredacted message the winning
+    /// network held at heal start is live at every peer.
     fn assert_healed(&self) {
         let live: Vec<usize> = (0..self.n()).filter(|&k| self.nodes[k].is_live()).collect();
 
```

<!-- annotation -->
> **tests-bookmark-9** (T8), line 1302:
>
> assert_healed's doc names the heal-window survival check alongside the two checks it already stated.

<a id="hunk-33"></a>
### tests/bookmark_causality.rs `@@ -857,6 +1339,49 @@ impl World {`

```diff
@@ -857,6 +1339,49 @@ impl World {
         }
 
         self.assert_live_content_is_durable(&live);
+        self.assert_durable_content_survived(&live);
+    }
+
+    /// Every message live in the winning network when the heal began, and
+    /// never redacted there, is live at every peer after it.
+    ///
+    /// The heal-window half of the recycle check by consequence
+    /// ([`assert_session_preserved`](World::assert_session_preserved) is the
+    /// per-session half): a reclaimed region's re-issued versions compare
+    /// `Greater` or incomparable to the message they destroy, so
+    /// [`EmissionLog::promote`] cannot see the destruction and these checks
+    /// can.
+    fn assert_durable_content_survived(&self, live: &[usize]) {
+        let (network, live_at_start) = self
+            .heal_start
+            .as_ref()
+            .expect("assert_healed runs after a heal");
+        let redacted: BTreeSet<u64> = self
+            .redacted
+            .get(network)
+            .map(|entries| entries.iter().map(|entry| entry.seq).collect())
+            .unwrap_or_default();
+        for &k in live {
+            let held: BTreeSet<u64> = self.nodes[k]
+                .live()
+                .unwrap()
+                .snapshot()
+                .iter()
+                .map(|(_, value)| *value)
+                .collect();
+            let destroyed: Vec<u64> = live_at_start
+                .iter()
+                .filter(|seq| !redacted.contains(seq) && !held.contains(seq))
+                .copied()
+                .collect();
+            assert!(
+                destroyed.is_empty(),
+                "messages {destroyed:?} were live in network {network:?} when the heal began \
+                 and never redacted there, but node {k} does not hold them after it: a \
+                 rebooted peer re-issued versions below a frontier the fleet durably held, \
+                 and the causal sieve read them as deletions",
+            );
+        }
     }
 
     /// Verify the recycle assertion is not passing vacuously: after the
```

<!-- annotation -->
> **tests-bookmark-9** (T8), line 1354:
>
> The survival check: every id live in the winning network at heal start, less that network's ledger, is live at every peer after the heal. It runs from assert_healed, so both proptests and every reconstructed test get it. Its doc carries the soundness argument beside the check. On the unmutated tree it raised no false positive across the two proptests' 512 cases, which is the evidence for the premise.

<!-- annotation -->
> **prose** (T141), line 1354:
>
> The survival check's doc restated the module doc's paragraph; it now states the invariant in one sentence and the two facts a maintainer at this method needs (why promote cannot see the destruction; why the ledger's entries are the only legitimate losses).

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 1348:
>
> The heal-window check stays as the second half and its doc says which half it is.

<a id="hunk-34"></a>
### tests/bookmark_causality.rs `@@ -1102,7 +1627,7 @@ fn retire_into_rebooted_absorber_absorbs_cleanly() {`

```diff
@@ -1102,7 +1627,7 @@ fn retire_into_rebooted_absorber_absorbs_cleanly() {
     block_on(async move {
         // A seeds, B bootstraps from A. Then each reboots once, reclaiming its
         // region from its bookmark (drop = crash; re-bootstrap = revive).
-        let a = Peer::<Msg>::seed()
+        let a = Peer::<Msg>::seed_rng(&mut ChaCha8Rng::seed_from_u64(NETWORK_SEED))
             .sync_window_floor()
             .bookmark(bm_a())
             .await
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 1630:
>
> The library-level retire regression seeds from NETWORK_SEED too: it has no World, so it seeds a local SmallRng. Not required by the resolution (this test has no tie-break); swept so no Peer::seed() call remains in the suite and its network is the same on every run.

<a id="hunk-35"></a>
### tests/bookmark_causality.rs `@@ -1127,24 +1652,23 @@ fn retire_into_rebooted_absorber_absorbs_cleanly() {`

```diff
@@ -1127,24 +1652,23 @@ fn retire_into_rebooted_absorber_absorbs_cleanly() {
         // holding the whole seed identity.
         let (ret_side, abs_side) = rumors::link::memory_with_capacity(LINK_BUF);
         let absorber = a.clone();
-        let retire = tokio::spawn(async move {
-            let mut link = ret_side;
-            let peer = b.try_into_peer().await.expect("sole handle");
-            peer.retire(&mut link).await
-        });
-        let absorb = tokio::spawn(async move {
-            let mut link = abs_side;
-            absorber.gossip(&mut link).await
-        });
-        let (retire_out, absorb_out) = tokio::join!(retire, absorb);
+        let (retire_out, absorb_out) = tokio::join!(
+            async move {
+                let mut link = ret_side;
+                let peer = b.try_into_peer().await.expect("sole handle");
+                peer.retire(&mut link).await
+            },
+            async move {
+                let mut link = abs_side;
+                absorber.gossip(&mut link).await
+            },
+        );
 
         assert!(
-            matches!(retire_out.expect("retire task"), Retire::Retired),
+            matches!(retire_out, Retire::Retired),
             "the retiree should have retired",
         );
-        absorb_out
-            .expect("absorb task")
-            .expect("the absorber's gossip must not fail while taking a retirement");
+        absorb_out.expect("the absorber's gossip must not fail while taking a retirement");
         assert_eq!(
             a.dangerously_alias_party().expect("A live"),
             Party::seed(),
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 1655:
>
> The library-level retire regression on join!: no JoinHandle, so the retire outcome and the absorber's result are matched directly.

<a id="hunk-36"></a>
### tests/bookmark_causality.rs `@@ -1161,28 +1685,29 @@ async fn boot_from_async(`

```diff
@@ -1161,28 +1685,29 @@ async fn boot_from_async(
 ) -> Rumors<Msg, FlakyInMemoryBookmark> {
     let server = server.clone();
     let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
-    let boot = tokio::spawn(async move {
-        let mut link = boot_side;
-        let peer = Peer::<Msg>::bootstrap()
-            .join(&mut link)
-            .await
-            .expect("bootstrap ok")
-            .expect("got a peer")
-            .sync_window_floor();
-        // Clean wires, reliable store: the eager persist of the reclaimed
-        // identity must succeed.
-        match peer.bookmark(bm).await {
-            Ok(peer) => peer,
-            Err(_) => panic!("bookmark ok"),
-        }
-    });
-    let serve = tokio::spawn(async move {
-        let mut link = serve_side;
-        server.gossip(&mut link).await
-    });
-    let (boot_out, serve_out) = tokio::join!(boot, serve);
-    serve_out.unwrap().expect("serve bootstrap");
-    boot_out.unwrap().into_rumors()
+    let (boot_out, serve_out) = tokio::join!(
+        async move {
+            let mut link = boot_side;
+            let peer = Peer::<Msg>::bootstrap()
+                .join(&mut link)
+                .await
+                .expect("bootstrap ok")
+                .expect("got a peer")
+                .sync_window_floor();
+            // Clean wires, reliable store: the eager persist of the reclaimed
+            // identity must succeed.
+            match peer.bookmark(bm).await {
+                Ok(peer) => peer,
+                Err(_) => panic!("bookmark ok"),
+            }
+        },
+        async move {
+            let mut link = serve_side;
+            server.gossip(&mut link).await
+        },
+    );
+    serve_out.expect("serve bootstrap");
+    boot_out.into_rumors()
 }
 
 /// Execute a reliable-recovery plan to its post-heal end state.
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 1688:
>
> boot_from_async on join!, same shape as bootstrap_into.

<a id="hunk-37"></a>
### tests/bookmark_causality.rs `@@ -1222,7 +1747,9 @@ fn run_reliable_plan(plan: Plan) -> World {`

```diff
@@ -1222,7 +1747,9 @@ fn run_reliable_plan(plan: Plan) -> World {
 #[should_panic(expected = "recycled version identifier")]
 fn negative_control_recycled_durable_emission_panics() {
     let log = EmissionLog::default();
-    let network = Peer::<Msg>::seed().sync_window_floor().network();
+    let network = Peer::<Msg>::seed_rng(&mut ChaCha8Rng::seed_from_u64(NETWORK_SEED))
+        .sync_window_floor()
+        .network();
     let mut version = Version::new();
     version.tick(&Party::seed());
 
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 1750:
>
> The negative control's network is seeded the same way, for the same reason as the library-level test above.

<a id="hunk-38"></a>
### tests/bookmark_causality.rs `@@ -1238,6 +1765,195 @@ fn negative_control_recycled_durable_emission_panics() {`

```diff
@@ -1238,6 +1765,195 @@ fn negative_control_recycled_durable_emission_panics() {
     });
 }
 
+/// Negative control for the session error classifier: each error the
+/// harness deems an unconditional crate bug fails the step it is reported
+/// on, and an injected disruption does not.
+#[test]
+fn negative_control_classifier_rejects_codec_bugs() {
+    fn fires(error: Error<FlakyInMemoryBookmark>) -> bool {
+        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
+            assert_not_codec_bug("negative control", &error)
+        }))
+        .err()
+        .and_then(|payload| payload.downcast::<String>().ok())
+        .is_some_and(|message| message.contains("a protocol, codec, or bookmark-format bug"))
+    }
+    let io = |kind, text| Error::Io(std::io::Error::new(kind, text));
+    for bug in [
+        io(
+            std::io::ErrorKind::InvalidData,
+            "a fully received frame that does not decode",
+        ),
+        Error::Epilogue(std::io::Error::new(
+            std::io::ErrorKind::InvalidData,
+            "a bad marker",
+        )),
+        Error::HandOffMalformed {
+            defect: rumors::error::HandOffDefect::NotPartyTagged,
+        },
+        Error::Bookmark(BookmarkIo::Format(rumors::FormatError::Truncated {
+            len: 0,
+        })),
+        Error::PartyOverlap,
+        Error::LinkPoisoned,
+        Error::IntentInvalid { byte: 0xff },
+        Error::BootstrapRetireConflict,
+        Error::Mirror(MirrorError::Client(
+            rumors::error::MaterializedError::Violation(
+                rumors::error::MaterializedViolation::UnaskedReply,
+            ),
+        )),
+        Error::Mirror(MirrorError::Server(RemoteError::Decode(
+            ReplyDecodeError::Record(rumors::error::DecodeLeafError::Version(
+                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "a short record"),
+            )),
+        ))),
+    ] {
+        assert!(fires(bug), "an unconditional crate bug must fail the step");
+    }
+    for disruption in [
+        io(std::io::ErrorKind::UnexpectedEof, "a severed wire"),
+        Error::Epilogue(std::io::Error::new(
+            std::io::ErrorKind::UnexpectedEof,
+            "a severed wire",
+        )),
+        Error::PreambleTruncated {
+            received: 0,
+            expected: 1,
+        },
+        Error::HandOffTruncated,
+        Error::Bookmark(BookmarkIo::Io(FlakyError::injected_write())),
+        Error::Mirror(MirrorError::Server(RemoteError::HandshakeRead(
+            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "a severed wire"),
+        ))),
+    ] {
+        assert!(
+            !fires(disruption),
+            "an injected disruption must not fail the step",
+        );
+    }
+}
+
+/// A message made durable by propagation survives its emitter's crash, a
+/// rejoin from a peer that never saw it, and the emitter's reclaim and later
+/// sends.
+///
+/// A sends `m` and gossips it to B; C sends three times without meeting `m`;
+/// A crashes, revives from C (the lowest-index live member), gossips with C
+/// so its bookmark update runs, and sends again. A bookmark that re-admits
+/// every stored region on reboot has A' re-issue coordinates below `m`'s,
+/// compared `Greater` to `m`'s version because A' carries C's ticks, and the
+/// heal's sieve deletes `m` fleet-wide: the survival check catches that and
+/// the version order cannot.
+#[test]
+fn reconstructed_reclaim_after_crash_keeps_durable_content() {
+    let (a, b, c) = (2, 1, 0);
+    let mut world = World::single_network(3);
+    world.send(a);
+    world.gossip(a, b, FaultPlan::NONE, FaultPlan::NONE);
+    let network = world.nodes[b].network;
+    assert!(
+        world
+            .emissions
+            .contains_exact(network, 0, &world.leaf_version(b, 0)),
+        "m is durable: it propagated to B",
+    );
+    for _ in 0..3 {
+        world.send(c);
+    }
+    world.crash(a);
+    world.gossip(a, c, FaultPlan::NONE, FaultPlan::NONE);
+    // The shape discriminates only if A' rebooted from a peer that never
+    // saw m; a change to how `revive` picks its server would retire it.
+    assert!(
+        !world.holds(a, 0),
+        "A' revived from a peer that never saw m"
+    );
+    for _ in 0..4 {
+        world.send(a);
+    }
+    world.heal();
+    world.assert_healed();
+    for k in [a, b, c] {
+        assert!(
+            world.holds(k, 0),
+            "the never-redacted durable message m must survive the heal at node {k}",
+        );
+    }
+}
+
+/// Known-bad artifact for the survival checks, against unmodified
+/// production code: a stale bookmark record makes a correct `reclaim`
+/// destroy a durable message, and the checks must catch it.
+///
+/// The shape of [`reconstructed_reclaim_after_crash_keeps_durable_content`]
+/// with one change: A's store is snapshotted right after the fleet forms
+/// and put back after A's crash, so A' reboots from a record that predates
+/// `m` (the lost-write hazard the bookmark's own docs describe). C's
+/// frontier dominates that stale record, so A''s first update reclaims A's
+/// old region, its sends re-issue coordinates below `m`'s, and the next
+/// session with B sieves `m` out of it. That session is the one the
+/// per-session check names, so the pin is on that check alone: without it,
+/// `m` is live nowhere when the heal begins and the heal-window check would
+/// pass.
+#[test]
+#[should_panic(expected = "before gossip 2<->1")]
+fn known_bad_stale_record_destroys_durable_content() {
+    let (a, b, c) = (2, 1, 0);
+    let mut world = World::single_network(3);
+    let stale = world.nodes[a].store.lock().unwrap().clone();
+    world.send(a);
+    world.gossip(a, b, FaultPlan::NONE, FaultPlan::NONE);
+    for _ in 0..3 {
+        world.send(c);
+    }
+    world.crash(a);
+    *world.nodes[a].store.lock().unwrap() = stale;
+    world.gossip(a, c, FaultPlan::NONE, FaultPlan::NONE);
+    assert!(
+        !world.holds(a, 0),
+        "A' revived from a peer that never saw m"
+    );
+    for _ in 0..4 {
+        world.send(a);
+    }
+    world.gossip(a, b, FaultPlan::NONE, FaultPlan::NONE);
+    world.heal();
+    world.assert_healed();
+}
+
+/// Negative control for the survival check: a message that left the fleet
+/// fails the check unless the ledger accounts for it.
+///
+/// One peer redacts a propagated message and the heal carries the
+/// redaction everywhere; with the ledger emptied the same converged fleet
+/// fails the check naming the message, which is what a destroyed message or
+/// an unrecorded redaction looks like.
+#[test]
+fn negative_control_unledgered_loss_fails_the_survival_check() {
+    let mut world = World::single_network(2);
+    world.send(0);
+    world.gossip(0, 1, FaultPlan::NONE, FaultPlan::NONE);
+    world.redact(0, 0);
+    world.heal();
+    world.assert_healed();
+    assert!(
+        !world.holds(0, 0) && !world.holds(1, 0),
+        "the redaction reached every peer",
+    );
+    world.redacted.clear();
+    let unledgered =
+        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| world.assert_healed()));
+    let message = unledgered
+        .expect_err("with the ledger emptied, the redacted message reads as destroyed")
+        .downcast::<String>()
+        .expect("a formatted assertion message");
+    assert!(
+        message.contains("messages [0] were live"),
+        "the survival check must name the missing message: {message}",
+    );
+}
+
 proptest! {
     /// Under arbitrary interleavings of sends, redactions, faulted gossip,
     /// crashes, and retirements, the identity bookmark never recycles a
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 1772:
>
> Committed negative control for the classifier: each of the four bug arms fires (constructed values, including FormatError::Truncated and HandOffDefect::NotPartyTagged), and three honest disruptions (a severed wire, a truncated hand-off, an injected bookmark fault) pass. catch_unwind under AssertUnwindSafe because the closure only reads the error.

<!-- annotation -->
> **tests-bookmark-9** (T8), line 1849:
>
> The entry's constructed shape as a committed test, from the witness's plan: m durable by propagation to B, C advancing without meeting m, A crashing and reviving from C, then gossiping and sending, then the heal. On HEAD m survives at every node. Under the reclaim mutant (src/bookmark.rs, the alias pushed at Version::new(), applied as a reversible string swap and restored, src/ verified identical to HEAD) this test fails at the survival check with `messages [0] were live in network ... node 0 does not hold them after it`; the commit message records the run. At the committed 256 cases the random recycle proptest does not reach the shape under the mutant; at PROPTEST_CASES=2048 both proptests fail, the recycle property through the survival check on an unshrunk case and through the version-only oracle on the shrunk five-step plan. I left the case count alone: tuning it against one mutant is a resource trade with an unmeasured sign, and the committed deterministic test is the reliable instrument; the report raises the count as an owner question.

<!-- annotation -->
> **tests-bookmark-9** (T8), line 1933:
>
> Negative control for the survival check on HEAD, no src mutation: a redacted, propagated message legitimately leaves the fleet; with the ledger emptied the same converged world fails assert_healed naming message 0. A suppressed ledger entry is the shape of a redaction the harness forgot to record, and of a destroyed message.

<!-- annotation -->
> **prose** (T141), line 1849:
>
> The constructed-shape test's doc opens with its invariant in one sentence and keeps the plan and the failure shape; the narrative of what a correct bookmark does went, since the assertion says it.

<!-- annotation -->
> **prose** (T141), line 1933:
>
> `genuinely` (an intensifier, T58) is gone and the doc is two sentences: the invariant, then the construction.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair), line 1773:
>
> Item 5: the classifier control now matches the classifier's message substring, so it fails by name rather than on any panic; the bug list gains Epilogue(InvalidData), LinkPoisoned, IntentInvalid, and BootstrapRetireConflict, and the admitted list gains Epilogue(UnexpectedEof) and PreambleTruncated.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 1870:
>
> Item 3: the instrument's liveness floor. The shape discriminates only if revive picked a server that never saw m; a change to revive's choice would otherwise retire the test silently.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 1901:
>
> Item 4: the destructive shape as a committed failing artifact against unmodified src/: A's store bytes snapshotted right after the fleet forms and put back after its crash, so a correct reclaim re-owns A's region on the first update (the lost-write hazard the bookmark's docs describe). should_panic on the survival message; on the box the panic lands in the heal's first session that meets A''s re-issued versions.

<!-- annotation -->
> **tests-bookmark-12** (fresh-eyes repair, round 2), line 1772:
>
> Item 3: the control gains three Mirror cases: a server-side HandshakeRead(UnexpectedEof) passes; a client-side MaterializedError::Violation(UnaskedReply) fires; a server-side Decode(Record(Version(UnexpectedEof))) fires.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair, round 2), line 1901:
>
> Item 1: the artifact now gossips A' with B before the heal (the round-1 construction), so the destruction lands in that session and the pin is the per-session message's phrase `before gossip 2<->1`; the heal-window message says `when the heal began` and cannot satisfy it. Discrimination verified with every assert_session_preserved call disabled (reversible): the artifact fails, recorded in the commit message.

<a id="hunk-39"></a>
### tests/bookmark_causality.rs `@@ -1248,7 +1964,13 @@ proptest! {`

```diff
@@ -1248,7 +1964,13 @@ proptest! {
     ///    (checked as the durable set grows in [`EmissionLog::promote`], a
     ///    message becoming durable once it is persisted or reaches another peer);
     /// 2. after a clean heal, all surviving peers converge to identical content
-    ///    and their live parties are pairwise disjoint.
+    ///    and their live parties are pairwise disjoint;
+    /// 3. no session both sides complete loses a message either side held
+    ///    and no one redacted, and every message the winning network held
+    ///    when the heal began, and never redacted, survives at every peer:
+    ///    the recycle checked by its consequence
+    ///    ([`World::assert_session_preserved`] and
+    ///    [`World::assert_durable_content_survived`]).
     ///
     /// The fleet starts fragmented into per-peer networks and converges by
     /// the `(min_ticks, network)` tie-break, with each peer's bookmark reads
```

<!-- annotation -->
> **tests-bookmark-9** (T8), line 1968:
>
> The recycle proptest's doc gains the third clause, the survival check, stated as the recycle's consequence.

<!-- annotation -->
> **tests-bookmark-9** (fresh-eyes repair), line 1968:
>
> The proptest's third clause now names both halves of the check.

<a id="hunk-40"></a>
### tests/bookmark_causality.rs `@@ -1270,7 +1992,10 @@ proptest! {`

```diff
@@ -1270,7 +1992,10 @@ proptest! {
 /// crashed-and-rebooted peer retires into the sender.
 ///
 /// The bookmark must not recycle a version across the crash/retire
-/// pair, and the heal must still converge.
+/// pair, and the heal must still converge. The path is pinned: each crash
+/// re-seeds its node (neither has a live member to reboot from), the
+/// cross-network retire is skipped, and the heal collapses node 1 into
+/// node 0, whose one send outranks a fresh universe.
 #[test]
 fn reconstructed_crash_pair_then_retire() {
     let world = run_plan(Plan {
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 2000:
>
> The crash-pair reconstruction's doc states the path it pins; the assertion below carries the pin.

<a id="hunk-41"></a>
### tests/bookmark_causality.rs `@@ -1284,6 +2009,19 @@ fn reconstructed_crash_pair_then_retire() {`

```diff
@@ -1284,6 +2009,19 @@ fn reconstructed_crash_pair_then_retire() {
         read_faults: vec![vec![], vec![]],
         write_faults: vec![vec![], vec![]],
     });
+    assert_eq!(
+        world.path,
+        vec![
+            PathEvent::Reseeded(0),
+            PathEvent::Reseeded(1),
+            PathEvent::Bootstrap {
+                newcomer: 1,
+                server: 0,
+                booted: true,
+            },
+        ],
+        "the plan's path through revival and heal",
+    );
     world.assert_healed();
 }
 
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 2012:
>
> The crash-pair reconstruction pins its path: each crash re-seeds (no live member to reboot from), the cross-network retire is skipped, and heal collapses node 1 into node 0. Observed identically on two separate runs before pinning; the doc comment states the path in English.

<a id="hunk-42"></a>
### tests/bookmark_causality.rs `@@ -1291,7 +2029,13 @@ fn reconstructed_crash_pair_then_retire() {`

```diff
@@ -1291,7 +2029,13 @@ fn reconstructed_crash_pair_then_retire() {
 /// mid-frame, then a retirement, under bookmark read/write fail
 /// schedules on every node.
 ///
-/// Versions must survive the faulted persistence without recycling.
+/// Versions must survive the faulted persistence without recycling. The
+/// path is pinned: the cut session still exchanges greetings, so the
+/// mismatch between the two fresh peers resolves by network identifier
+/// with node 1 the winner and node 2 re-bootstrapping into it; the retire
+/// of 1 into 2 fails on the scheduled bookmark faults and hands node 1 back
+/// intact, so nothing is re-seeded; the heal then collapses both into node
+/// 0, whose send outranks every fresh universe.
 #[test]
 fn reconstructed_cut_gossip_then_retire_under_bookmark_faults() {
     let world = run_plan(Plan {
```

<!-- annotation -->
> **tests-bookmark-8** (T8), line 2040:
>
> The cut-gossip reconstruction pins its path, which is the acceptance's asserted winner: the cut session still exchanges greetings, so the mismatch resolves with node 2 the winner by network id; node 1's re-bootstrap fails on its scheduled bookmark faults; the retire re-seeds node 1; heal collapses 1 and 2 into 0. Observed identically on two separate runs (two processes) before pinning.

<!-- annotation -->
> **tests-bookmark-8** (fresh-eyes repair, round 2), line 2036:
>
> Item 6: the path is re-pinned from a run under ChaCha8 and the doc restated from a probe of the run: the retire of 1 into 2 ends Recovered on node 1's scheduled bookmark write fault (the absorber's side fails too), so nothing is re-seeded and the heal collapses 1 and 2 into node 0.

<a id="hunk-43"></a>
### tests/bookmark_causality.rs `@@ -1323,6 +2067,31 @@ fn reconstructed_cut_gossip_then_retire_under_bookmark_faults() {`

```diff
@@ -1323,6 +2067,31 @@ fn reconstructed_cut_gossip_then_retire_under_bookmark_faults() {
             vec![false, false, false, false, true, false],
         ],
     });
+    assert_eq!(
+        world.path,
+        vec![
+            PathEvent::Mismatch {
+                winner: 1,
+                loser: 2
+            },
+            PathEvent::Bootstrap {
+                newcomer: 2,
+                server: 1,
+                booted: true,
+            },
+            PathEvent::Bootstrap {
+                newcomer: 1,
+                server: 0,
+                booted: true,
+            },
+            PathEvent::Bootstrap {
+                newcomer: 2,
+                server: 0,
+                booted: true,
+            },
+        ],
+        "the plan's path through mismatch resolution, revival, and heal",
+    );
     world.assert_healed();
 }
 
```

<!-- annotation -->
> **tests-bookmark-8** (fresh-eyes repair, round 2), line 2074:
>
> Item 6: the pinned path under ChaCha8 ids: node 1 wins the mismatch and node 2 boots into it; no Reseeded event, since the retire hands node 1 back; the heal boots both from node 0.

<a id="hunk-44"></a>
### tests/bookmark_transmit_window.rs `@@ -27,7 +27,9 @@`

```diff
@@ -27,7 +27,9 @@
 //!
 //! [`GatedBookmark`] makes the write's in-flight window a deterministic
 //! interleaving point: the test parks the session inside the persist, commits
-//! a send, and releases.
+//! a send, and releases. Every session runs under the closed-world poller
+//! ([`common::wire::block_on`]), so a session that stops making progress
+//! fails at its source rather than hanging the test.
 
 mod common;
 
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 28:
>
> The resolution names the transmit suite too. Its module doc gains the sentence that every session runs under the closed-world poller and what that buys.

<a id="hunk-45"></a>
### tests/bookmark_transmit_window.rs `@@ -39,7 +41,7 @@ use rumors::{Bookmark, BookmarkError, Peer, Rumors, Serialized};`

```diff
@@ -39,7 +41,7 @@ use rumors::{Bookmark, BookmarkError, Peer, Rumors, Serialized};
 use tokio::sync::Notify;
 
 use crate::common::flaky::{DurableStore, persisted_record};
-use crate::common::wire::tokio_block_on as block_on;
+use crate::common::wire::block_on;
 
 /// The message payload: a test-unique id.
 type Msg = u64;
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 44:
>
> common::wire::block_on replaces tokio_block_on for the transmit suite.

<a id="hunk-46"></a>
### tests/bookmark_transmit_window.rs `@@ -60,7 +62,8 @@ const MAX_HEAL_ROUNDS: usize = 16;`

```diff
@@ -60,7 +62,8 @@ const MAX_HEAL_ROUNDS: usize = 16;
 /// Disarmed (the default) it persists synchronously, like the sibling suites'
 /// in-memory bookmarks. Armed, the next `store` signals `entered`, then parks
 /// until `release` is notified; the test body runs in that window, on the
-/// same current-thread runtime, so the interleaving is exact and replayable.
+/// same thread under the same poller, so the interleaving is exact and
+/// replayable.
 #[derive(Clone, Debug)]
 struct GatedBookmark {
     store: DurableStore,
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 65:
>
> Nit: the GatedBookmark doc said the test body ran on the same current-thread runtime; restated as the same thread under the same poller, which is what makes the interleaving exact.

<a id="hunk-47"></a>
### tests/bookmark_transmit_window.rs `@@ -153,39 +156,41 @@ async fn boot_from(`

```diff
@@ -153,39 +156,41 @@ async fn boot_from(
 ) -> Rumors<Msg, GatedBookmark> {
     let server = server.clone();
     let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
-    let boot = tokio::spawn(async move {
-        let mut link = boot_side;
-        let peer = Peer::<Msg>::bootstrap()
-            .join(&mut link)
-            .await
-            .expect("bootstrap ok")
-            .expect("the server is established");
-        peer.bookmark(bm).await.expect("in-memory persist")
-    });
-    let serve = tokio::spawn(async move {
-        let mut link = serve_side;
-        server.gossip(&mut link).await
-    });
-    let (boot_out, serve_out) = tokio::join!(boot, serve);
-    serve_out.unwrap().expect("serve bootstrap");
-    boot_out.unwrap().into_rumors()
+    let (boot_out, serve_out) = tokio::join!(
+        async move {
+            let mut link = boot_side;
+            let peer = Peer::<Msg>::bootstrap()
+                .join(&mut link)
+                .await
+                .expect("bootstrap ok")
+                .expect("the server is established");
+            peer.bookmark(bm).await.expect("in-memory persist")
+        },
+        async move {
+            let mut link = serve_side;
+            server.gossip(&mut link).await
+        },
+    );
+    serve_out.expect("serve bootstrap");
+    boot_out.into_rumors()
 }
 
 /// One clean gossip session between two peers, both sides required to succeed.
 async fn gossip(a: &Rumors<Msg, GatedBookmark>, b: &Rumors<Msg, GatedBookmark>) {
     let (a, b) = (a.clone(), b.clone());
     let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
-    let task_a = tokio::spawn(async move {
-        let mut link = side_a;
-        a.gossip(&mut link).await
-    });
-    let task_b = tokio::spawn(async move {
-        let mut link = side_b;
-        b.gossip(&mut link).await
-    });
-    let (out_a, out_b) = tokio::join!(task_a, task_b);
-    out_a.unwrap().expect("gossip side a");
-    out_b.unwrap().expect("gossip side b");
+    let (out_a, out_b) = tokio::join!(
+        async move {
+            let mut link = side_a;
+            a.gossip(&mut link).await
+        },
+        async move {
+            let mut link = side_b;
+            b.gossip(&mut link).await
+        },
+    );
+    out_a.expect("gossip side a");
+    out_b.expect("gossip side b");
 }
 
 /// The version stamped on the live leaf carrying `payload`, if present.
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 159:
>
> boot_from and the gossip helper on join! over owning blocks.

<a id="hunk-48"></a>
### tests/bookmark_transmit_window.rs `@@ -241,24 +246,25 @@ async fn transmit_during_persist() -> Scene {`

```diff
@@ -241,24 +246,25 @@ async fn transmit_during_persist() -> Scene {
     let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
     let ga = {
         let a = a.clone();
-        tokio::spawn(async move {
+        async move {
             let mut link = side_a;
             a.gossip(&mut link).await
-        })
+        }
     };
     let gb = {
         let b = b.clone();
-        tokio::spawn(async move {
+        async move {
             let mut link = side_b;
             b.gossip(&mut link).await
-        })
+        }
     };
-    bm_a.entered().await;
-    a.send(M1).unwrap();
-    bm_a.release();
-    let (out_a, out_b) = tokio::join!(ga, gb);
-    out_a.unwrap().expect("gated gossip side a");
-    out_b.unwrap().expect("gated gossip side b");
+    let (out_a, out_b, ()) = tokio::join!(ga, gb, async {
+        bm_a.entered().await;
+        a.send(M1).unwrap();
+        bm_a.release();
+    });
+    out_a.expect("gated gossip side a");
+    out_b.expect("gated gossip side b");
 
     Scene {
         a,
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 261:
>
> The gated session is the resolution's exact shape: join!(ga, gb, async { entered; send(M1); release }); Notify needs no runtime.

<a id="hunk-49"></a>
### tests/bookmark_transmit_window.rs `@@ -342,28 +348,23 @@ fn cancelled_persist_never_suppresses_the_next_update() {`

```diff
@@ -342,28 +348,23 @@ fn cancelled_persist_never_suppresses_the_next_update() {
         a.send(M1).unwrap();
         bm_a.arm();
         let (side_a, side_b) = rumors::link::memory_with_capacity(LINK_BUF);
-        let ga = {
-            let a = a.clone();
-            tokio::spawn(async move {
-                let mut link = side_a;
-                a.gossip(&mut link).await
-            })
-        };
-        let gb = {
-            let b = b.clone();
-            tokio::spawn(async move {
-                let mut link = side_b;
-                b.gossip(&mut link).await
-            })
-        };
-        bm_a.entered().await;
-        ga.abort();
-        gb.abort();
-        let (out_a, out_b) = tokio::join!(ga, gb);
-        assert!(
-            out_a.is_err() && out_b.is_err(),
-            "both session futures were dropped mid-persist",
-        );
+        // Drive the session until A's persist parks, then drop both session
+        // futures there (the block ends): the cancellation lands inside the
+        // durable write.
+        {
+            let (a, b) = (a.clone(), b.clone());
+            let mut session = std::pin::pin!(async move {
+                let (mut side_a, mut side_b) = (side_a, side_b);
+                tokio::join!(a.gossip(&mut side_a), b.gossip(&mut side_b))
+            });
+            tokio::select! {
+                biased;
+                () = bm_a.entered() => {}
+                _ = &mut session => {
+                    panic!("the gated session cannot complete while its persist is parked")
+                }
+            }
+        }
 
         // The next session runs on a fresh link. It must persist M1's
         // frontier before transmitting M1: a suppression token surviving
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 351:
>
> Cancellation without JoinHandle::abort: the two session futures are pinned in a block and driven by a biased select! against the bookmark's `entered` signal, and the block's end drops them mid-persist. My first draft moved the Pin<&mut _> into select!, which drops the reference and not the future, so the parked persist stayed alive holding the bookmark and the next session stalled (the poller caught it in 17 ms, which is the instrument the entry restores doing its job on my own bug). The old assertion that both JoinHandles reported an abort has no analogue and is gone; the drop is the cancellation.

<a id="hunk-50"></a>
### tests/bookmark_transmit_window.rs `@@ -524,24 +525,26 @@ fn donation_persist_failure_aborts_before_the_wire() {`

```diff
@@ -524,24 +525,26 @@ fn donation_persist_failure_aborts_before_the_wire() {
         // slice's write is the next store call.
         bm_a.fail_at(1);
         let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
-        let boot = tokio::spawn(async move {
-            let mut link = boot_side;
-            Peer::<Msg>::bootstrap().join(&mut link).await
-        });
         let serve = {
             let a = a.clone();
-            tokio::spawn(async move {
+            async move {
                 let mut link = serve_side;
                 a.gossip(&mut link).await
-            })
+            }
         };
-        let (boot_out, serve_out) = tokio::join!(boot, serve);
+        let (boot_out, serve_out) = tokio::join!(
+            async move {
+                let mut link = boot_side;
+                Peer::<Msg>::bootstrap().join(&mut link).await
+            },
+            serve,
+        );
         assert!(
-            matches!(serve_out.unwrap(), Err(rumors::Error::Bookmark(_))),
+            matches!(serve_out, Err(rumors::Error::Bookmark(_))),
             "the serve must surface the failed donation persist",
         );
         assert!(
-            !matches!(boot_out.unwrap(), Ok(Some(_))),
+            !matches!(boot_out, Ok(Some(_))),
             "the newcomer must not receive a party the donor could not persist away",
         );
 
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 526:
>
> The two donation-abort tests on join!; the serve block is bound first so the closure captures read naturally, and the results are matched directly instead of through unwrap().

<a id="hunk-51"></a>
### tests/bookmark_transmit_window.rs `@@ -600,24 +603,26 @@ fn repeated_donation_aborts_normalize() {`

```diff
@@ -600,24 +603,26 @@ fn repeated_donation_aborts_normalize() {
             // donation slice's write (the second): fail the second.
             bm_a.fail_at(2);
             let (boot_side, serve_side) = rumors::link::memory_with_capacity(LINK_BUF);
-            let boot = tokio::spawn(async move {
-                let mut link = boot_side;
-                Peer::<Msg>::bootstrap().join(&mut link).await
-            });
             let serve = {
                 let a = a.clone();
-                tokio::spawn(async move {
+                async move {
                     let mut link = serve_side;
                     a.gossip(&mut link).await
-                })
+                }
             };
-            let (boot_out, serve_out) = tokio::join!(boot, serve);
+            let (boot_out, serve_out) = tokio::join!(
+                async move {
+                    let mut link = boot_side;
+                    Peer::<Msg>::bootstrap().join(&mut link).await
+                },
+                serve,
+            );
             assert!(
-                matches!(serve_out.unwrap(), Err(rumors::Error::Bookmark(_))),
+                matches!(serve_out, Err(rumors::Error::Bookmark(_))),
                 "round {round}: the serve must surface the failed donation persist",
             );
             assert!(
-                !matches!(boot_out.unwrap(), Ok(Some(_))),
+                !matches!(boot_out, Ok(Some(_))),
                 "round {round}: the newcomer must not receive a party",
             );
             assert_eq!(
```

<!-- annotation -->
> **tests-bookmark-11** (T8), line 606:
>
> The second donation-abort test's session on join!, the same shape as the first.

<a id="hunk-52"></a>
### tests/common/flaky.rs `@@ -105,6 +105,14 @@ pub struct FlakyError {`

```diff
@@ -105,6 +105,14 @@ pub struct FlakyError {
     op: &'static str,
 }
 
+impl FlakyError {
+    /// The error an injected write failure reports, for tests that need the
+    /// value without a scheduled fault.
+    pub fn injected_write() -> Self {
+        FlakyError { op: "write" }
+    }
+}
+
 impl fmt::Display for FlakyError {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
         write!(f, "flaky bookmark: injected {} failure", self.op)
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 111:
>
> FlakyError::injected_write builds the error value the committed classifier negative control needs for its honest-disruption arm (Bookmark(Io(_)) must pass); the struct's field is private to the module, so a constructor is the smallest opening.

<a id="hunk-53"></a>
### tests/common/flaky.rs `@@ -145,6 +153,15 @@ impl FaultFeed {`

```diff
@@ -145,6 +153,15 @@ impl FaultFeed {
         self.enabled = false;
     }
 
+    /// Whether a scheduled failure remains: the feed is enabled and a `true`
+    /// decision is still queued for a read or a write.
+    ///
+    /// An exhausted or all-`false` queue never fails, so a session over such
+    /// a feed cannot legitimately report a bookmark error.
+    pub fn may_fail(&self) -> bool {
+        self.enabled && (self.reads.contains(&true) || self.writes.contains(&true))
+    }
+
     fn next_read(&mut self) -> bool {
         self.enabled && self.reads.pop_front().unwrap_or(false)
     }
```

<!-- annotation -->
> **tests-bookmark-12** (T8), line 161:
>
> FaultFeed::may_fail is the resolution's emptiness accessor, made precise: a feed can fail iff it is enabled and a `true` decision remains in either queue. An exhausted or all-false queue defaults to success (next_read/next_write), so asserting Ok over such feeds is sound and strictly stronger than asserting only over empty ones; a step the plan cannot fail is asserted even inside a faulted plan.

