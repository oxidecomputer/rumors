<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T126, T128, T130, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: whole-root convergence witnesses and redaction in every generated population

## Goal

Every convergence witness compares whole roots, content and ceiling,
never a hash or a length alone; every generated population that a
redaction can reach draws redactions, with the whole-subtree shed count
pinned above one (one mutation, `stats.shed(node.len())` to
`shed(1)`, caught by three suites); the hop budget's known-bad regime is
a committed red cell; and `deep_trie_divergence` carries a depth floor.
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
  run in the background redirected to a log under `<scratchpad>/p4-oracles/`,
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

1. **Convergence**, one commit per suite: remote-proxy-tests-15 (the
   `join_oracle` helper), remote-proxy-tests-7, tests-observation-36,
   tests-resource-link-window-27, -16, tests-wire-format-5. Oracle:
   `git grep -n -E 'snapshot\(\)\.len\(\) *==|hash_of|union_hash' -- src tests`
   reviewed in the closing commit, every survivor beside a whole-snapshot
   equality or not a convergence witness. Known-bad: the entries'
   Constructions (a wrong ceiling, a dropped last record, a
   content-free retirement), each applied and quoted.
2. **Redaction**: the shared shed pin first (streaming-tests-26's
   walk-tier pin; materialized-28 and tests-observation-25 are the same
   defect from the other partitions), then the generators
   (`arb_deep_redaction_pair`, `arb_local_actions` in `session_stats`,
   `RedactA`/`RedactB` in `gossip_when`, `RedactB`/`RedactLearned` in
   `causal`, `Op::RedactAgain`/`HelperRedact` in `bookmark_when`), each
   with the liveness point case its entry names. Known-bad: the
   `shed(1)` mutant, applied once, failing three suites, quoted once.
3. **Hop budget**: verification-infra-16's floor cell against today's
   fixture; tests-disruption-handshake-10's equality waits on the shared
   fixture (T131, P5): report the split.
4. **Depth floor**: tests-wire-format-7.

Every seed file that appears is committed; every committed seed still
replays (draw new arms after the existing ones).

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **remote-proxy-tests-7** (medium; T126). `src/tree/mirror/streaming/remote/proxy/tests.rs:296-300`. Resolution: Return the roots from the helper (`left.map(|(root, _)| ..)`, `right.map(|(_, root)| ..)`), adding the inverse of `failing_root` (tests.rs:219-224) to convert `Root<Failing<Local>>` back to `TreeRoot`; compute `reconcile_locally(a, b)` in the property and assert any `Ok` result equals the oracle's side, as failures.rs:247-253 does. `stacked_backend_and_transport_failures_remain_distinct` can keep discarding at its own call sites. Acceptance: `proxy_backend_failures_are_fail_fast` contains a `prop_assert_eq!` against the local oracle for whichever endpoint returns `Ok` when the injection fired, and the helper's return type carries `TreeRoot`. Construction: Wrap the unfaulted proxy's backend to substitute one leaf's payload after N operations while the faulted side fails; the current property passes because the tree is never compared.
- **streaming-tests-26** (medium; T128). `src/tree/mirror/streaming/tests/stats.rs:195-218`. Resolution: Add a walk-tier pin with `grown` and `act`: a shared base of k >= 2 leaves under one controlled child (paths `[0x30, i, 0..]`) plus one shared leaf elsewhere so the root stays disputed; side b is the base plus one extra on party 1; side a is the base with the k leaves forgotten on party 2 (a lacks child 0x30 and its ceiling dominates the k versions). Assert `b_stats.messages_shed == k` and `live(&theirs) == b_before - k + b_stats.messages_gained`. Add a redaction step to `sessions_conserve_the_live_count`'s strategy (a drawn subset of the shared sends) so the law is checked with shed > 1 over the wire. Acceptance: a committed test asserts `messages_shed >= 2` from one whole-subtree prune; the public conservation proptest draws redactions; the `node.len()` -> `1` mutant at unknown.rs:104 and :150 is caught.
- **tests-observation-25** (medium; T130; the same mutant as streaming-tests-26 and materialized-28: one `stats.shed(1)` mutation, three suites, quoted once). `tests/session_stats.rs:297-335`. Resolution: Build both sides on a shared converged base and let each side act: a preamble of shared sends before the fork, then `a = build_local(bootstrap_fork(&seed), &a_actions)` and `b = build_local(bootstrap_fork(&seed), &b_actions)` with `arb_local_actions()` (which draws `Redact`), so cross-redactions of base messages held on the other side occur. Keep the conservation and byte-mirror assertions; add a point case with both sides shedding. Acceptance: over a default run some cases report `messages_shed > 0` on each side (pin with a dedicated point case); a mutant deleting the `stats.shed(1)` call at answer.rs:142 fails this suite.
- **tests-resource-link-window-16** (medium; T130). `tests/target_message_size.rs:224-267`. Resolution: Use `capture_gossip_returning` in the `frames` closure and assert `l.snapshot() == r.snapshot()` per cell, so each of the four cells also pins convergence. Acceptance: each cell asserts snapshot equality, and the assertion is reachable (an encoder that skips the last record of a split run fails it). Construction: In a scratch build, make the supply encoder drop the final record of any run it splits under a nonzero target; today every assertion in this test still passes (the frame counts stay equal across the mixed and uniform-small cells); with snapshot equality per cell it fails.
- **materialized-28** (low; T132). `src/tree/mirror/streaming/materialized/unknown.rs:103-106`. Resolution: Add a redaction schedule to `sessions_conserve_the_live_count` (a `Forget` over the shared base, as `arb_divergent_pair` already does in-crate) so `after = before + gained - shed` exercises shed > 0; and add a walk-tier pin where one side holds k >= 2 leaves under one root radix the other has seen and forgotten, asserting `messages_shed == k`. Acceptance: a committed test fails when either `stats.shed(node.len() as u64)` is replaced by `stats.shed(1)`.
- **remote-proxy-tests-15** (low; T132; the `join_oracle` helper lands in `harness.rs` (rewritten by `p1-harness-crate` under T22: re-anchor)). `src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:33-43`. Resolution: Add one `join_oracle(a: &TreeRoot, b: &TreeRoot) -> TreeRoot` to harness.rs (built on `Tree::join`, as tests.rs:335-340 does) and assert `assert_eq!(left, expected)` on whole roots in the seven tests; delete both `hash_of`/`union_hash` pairs. Acceptance: no convergence assertion in greeting.rs or declarations.rs goes through `Tree::hash()` alone; a deliberately wrong ceiling on one reconciled side fails the test. Construction: Patch the proxy's equal or diverged completion to return the local pre-session ceiling instead of the merged one; `empty_carried_listing_asks_for_everything` still passes because both hashes equal `populated.hash()`.
- **streaming-tests-7** (low; T132). `src/tree/mirror/streaming/tests.rs:194-196`. Resolution: Add `arb_deep_redaction_pair` beside `arb_deep_divergent_pair` in `arb.rs` (a shared spine of drawn depth carrying k drawn shared leaves; each side forgets a drawn subset on its own party and adds drawn concurrent extras), include it as an arm of `arb_oracle_pair`, and add two fixtures: (a) both sides hold prefix P, side a holds child Q under P with k >= 2 leaves that side b forgot (b's request for Q prunes to nothing on a); (b) each side forgets the other's sibling under one 31-byte prefix. Acceptance: `arb_oracle_pair` draws forgets under a shared prefix of depth >= 1; the two fixtures are committed with `join_oracle` as the expectation in both orientations.
- **tests-bookmark-26** (low; T132). `tests/bookmark_when.rs:629-655`. Resolution: add `Op::RedactAgain` (re-redact the most recently redacted version; model: no local change, so the next session drives no I/O) and `Op::HelperRedact(usize)` (a helper redacts one of its own held messages; model: pure hearsay). Both are a few lines in `apply` and the strategy. Acceptance: the proptest alphabet includes both ops and the model's predictions still match at every step. Construction: with `Op::RedactAgain` in the alphabet, a mutant `Rumors::redact` that ticks the own region on an unheld version produces `Delta { writes: 1 }` at the next `Gossip` where the model predicts `writes: 0`, and the proptest fails.
- **tests-disruption-handshake-10** (low; T132; the shared fixture is tests-disruption-handshake-31 (T131, the P5 harness lane): land the floor leg against today's fixture and report the equality as waiting on that lane). `tests/gossip_pipelining.rs:33-41`. Resolution: Once the fixture is shared (finding 31), add one committed equality `latency::session_hops(CAPACITY, DELAY, pair) == trace.hops()` tying the two instruments, and add a floor leg on the same fixture (`sync_window_floor()` both sides) asserting `floor_measured > HOP_BUDGET`; let HOP_BUDGET's doc quote the measured floor figure and drop the hand-derived ratios. Acceptance: one committed equality between the two hop instruments exists; a test fails if the floor drops below the budget; the constant's doc contains no arithmetic over unmeasured numbers. Construction: Build `diverged_insertions()` in hop_trace, run it through both `traced_session` and `latency::session_hops(CAPACITY, DELAY, ...)`, and assert equality; for the floor, build the same pair with `.sync_window_floor()` and print `session_hops`.
- **tests-disruption-handshake-18** (low; T132; the committed seed in `proptest-regressions/gossip_when.txt` must still replay: draw the new arms after the existing ones (T130's rule for tests-common-25)). `tests/gossip_when.rs:788-797`. Resolution: Add `RedactA` and `RedactB` arms that redact the peer's own latest live message when one exists and count executed redactions; assert at the end `len == sends - executed_redactions` beside the existing hash and latest equalities. Acceptance: `op_strategy()` produces redaction ops; the final assertion accounts for them; the committed seed in proptest-regressions/gossip_when.txt still replays. Construction: Extend the feeder's match with the two arms and re-run the proptest.
- **tests-observation-36** (low; T132). `tests/party_conservation.rs:402-441`. Resolution: After the retirement loop, `prop_assert_eq!(readout_multiset(&fleet[0].snapshot()), (0..providers.len() as u64).map(|i| (i, 1)).collect::<BTreeMap<_, _>>())` (importing `readout_multiset` from `crate::common::oracle`), or at least `prop_assert_eq!(fleet[0].snapshot().len(), providers.len())`. Acceptance: a mutant that skips content reconciliation in the retire path fails this test. Construction: Temporarily make the retiree's write-back in `retire_inner` skip the gossip round's content while still donating the party; today this test passes; with the readout assertion it fails.
- **tests-observation-5** (low; T132). `tests/causal.rs:359-386`. Resolution: Extend causal.rs's alphabet with `RedactB(usize)` (B redacts one of its own sends, tracked in `sent_b`) and `RedactLearned(usize)` (A redacts the idx-th version of `a.snapshot()`), keeping the existing assertions. For listen.rs, either add `Gossip` and `SendB` ops mirroring causal.rs or state in the testdoc that the multi-peer regime is covered by `disruption.rs`. Acceptance: a default run's shrunk Debug output shows redactions of B-originated and A-learned versions; a point test pins the staged-then-shed shape (A stages both, B redacts one, gossip, drain). Construction: In causal.rs add a point test: `a` seeds, `b = bootstrap_fork(&a)`, `b.send(1); b.send(2)`, `wire_gossip(&a,&b)`, `let mut obs = a.causal_messages(); step(&mut obs)` (ingests both, delivers one), `b.redact(&staged_version); wire_gossip(&a,&b); drain(&mut obs)`. The current strategy cannot generate this sequence; the point test states what the expected delivery is (the staged message is delivered once, per the ingest-liveness contract) and fails if the pop path re-reads the tree.
- **tests-resource-link-window-27** (low; T132). `tests/window_corners.rs:141-145`. Resolution: Compare `left.snapshot()` with `right.snapshot()` (or their `hash()`) at 141-145 and 195-199. Acceptance: no `snapshot().len()` equality stands alone as a convergence witness in the window suites.
- **tests-wire-format-5** (low; T132). `tests/gossip_snapshot.rs:418-421`. Resolution: Use `capture_gossip_returning` in these scenarios and assert the live sets the docs name (a sorted-payloads `assert_eq!` per side), or reword the docs to claim only the pinned wire form. Acceptance: each doc's stated live set is asserted after the capture, or the doc no longer states one. Construction: In `fork_insert_redact`, replace `capture_gossip(a, b)` with `capture_gossip_returning`, then assert both snapshots' payloads equal `[3, 4]`; the assertion passes today and would fail under a redaction-propagation regression that the snapshot alone would only catch as a byte diff a reviewer could re-accept.
- **tests-wire-format-7** (low; T132; the streams>=2 half is carried by T23 (`p1-collision-mode`, merged): the depth floor is this lane's; if the collision mode stages a `stream 2` header cheaply, use it and say so). `tests/gossip_snapshot.rs:492-515`. Resolution: Add a depth floor to `deep_trie_divergence` (`stream_frames(&capture, "Responder stream 1 (height 29)")` must be `Some`), so its stated purpose is tamper-evident. For the deeper labels, either stage one fixture with both peers holding leaves under a shared three-byte prefix (a shared pair at `shaped_pair(pool, 3, true)` staged before the fork, plus a divergent third leaf under the same prefix, as `shared_subtree_dispute_pins_a_nonempty_query` does at one byte; a birthday pool on the order of 2^12 sends), floored on a `stream 2` header; or, if the staging cost is judged too high, add a codec-level unit pin of the stream-label encoding for indexes 2..16 and say so in this module doc. Acceptance: `deep_trie_divergence` fails before the snapshot comparison if its depth degrades; either a committed snapshot contains a `stream 2` header or the module doc states where the deeper labels are pinned. Construction: Stage `send_pool(&a, 0, 4096)`, `shaped_pair(&pool(&a, 0, 4096), 3, true)`, `keep_only`, fork `b`, then land a third leaf under the same three-byte prefix on `a` via a targeted pool search on `leaf_path(v)[..3]`; capture and check for a `stream 2` header.
- **verification-infra-16** (low; T132). `tests/gossip_pipelining.rs:33-41`. Resolution: add a floor-window cell to gossip_pipelining (the same `diverged_pair` built with `sync_window_floor()`) asserting `measured >= SERIALIZED_FLOOR` with the floor derived from the disputed-scope count, so the budget's discrimination is measured rather than stated; in future_size, pin the measured sizes as constants and assert the budget is within a stated factor of them. Acceptance: the pipelining file carries two cells, one passing under the production window and one asserting the floor window exceeds `HOP_BUDGET`.

## Hazards and stops

- T151: no `cases`.
- `p2-vanish-liveness` (unmerged) edits `gossip_when.rs`;
  `p1-proptest-ci` (unmerged) edits `session_stats.rs`,
  `party_conservation.rs`, `stats.rs`, the proxy tests;
  `p2-commit-path` (unmerged) edits `arb.rs`. Launch after all merge.
- `arb.rs` is also `p4-testdocs`' (tree-core-34's generator): order
  testdocs first, or rebase.
- `window_corners.rs` (tests-resource-link-window-27) carries T139's
  landed census changes: re-anchor.
