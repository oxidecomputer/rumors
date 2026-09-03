<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T85, T124, T126, T128, T129, T130, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: every testdoc names exactly what its body checks

## Goal

Every test's doc names exactly what its body checks. Where the doc
claims more, the body grows to check it, with the entry's Construction
as the negative control; where the claim is not worth a body, the doc
shrinks to the body, per each entry's Resolution. Asserts implied by
the checks before them go (tests-bookmark-14, streaming-tests-18), and
`assert_parent_early` dissolves (T85). The gate's `testdoc` checks
presence only; accuracy has no mechanical check, so this is a one-time
pass whose report lists, per row, which arm was taken and the negative
control's observed failure. Effort: high (harness semantics: new
helpers in `tests/handshake.rs`, `redaction.rs`, `listen.rs`).

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
  run in the background redirected to a log under `<scratchpad>/p4-testdocs/`,
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

One commit per file, by family:

1. **Crate suites**: api-core-20, link-12, materialized-22,
   remote-adapter-tests-4, remote-capture-atlas-19, remote-codec-17,
   remote-proxy-tests-2, -9, -25, session-bookmark-18, -42,
   streaming-backend-window-15, -36, -37, streaming-tests-18,
   testing-infra-19, -21, tree-core-34, tree-typed-9.
2. **Integration suites**: tests-bookmark-4, -14, -21, tests-common-20,
   tests-disruption-handshake-21, -22, -23, -25, tests-lifecycle-4,
   -21, tests-observation-17, -20, tests-resource-link-window-7.

For every extension, apply the Construction as a reversible mutation,
observe the failure, quote it in the commit message, restore. For every
narrowing, quote the sentence removed and the assertion it exceeded.

Oracle: no general grep exists; per entry, its own
(`grep -rn '17-byte' src/tree/typed` empty, `grep -rn associativ src tests`
shows the new property, `grep -n MB` over `parking.rs` disagrees with no
pin, `grep -n pollster` untouched here). `just testdoc` stays green.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **materialized-22** (low; T85; T85 rules the dissolve arm: remove `assert_parent_early` and its test, and `assert_parent_last`'s doc stops pointing at it). `src/tree/mirror/streaming/materialized/progress/tests.rs:96-117`. Resolution: Either make the pin real (capture a session with `with_trace` as `streaming/tests.rs:83-95` does and run it through `assert_parent_early` under `#[should_panic]`), or dissolve `assert_parent_early` and this test, leaving the d5/d6 record to the model, and reword `assert_parent_last`'s doc (progress.rs:127-128) to stop pointing at the removed checker. Keeping a checker purely as a record is the owner's call; at minimum the testdoc must stop claiming a sensitivity it lacks. Acceptance: either a real-session trace reaches `assert_parent_early`, or the function and test are gone and no prose references them.
- **tree-core-34** (medium; T124; the generator lives in `arb.rs` (`p2-commit-path` edits it): after its merge). `src/tree/traverse/join/tests.rs:35-41`. Resolution: Add a three-way divergent generator to `arb.rs` (a common base of shared inserts on party 0; three forks on parties 1, 2, 3, each with its own inserts and an arbitrary redaction subset of the shared keys, ceilings built by `Tree::act`) and a `join_associative_with_redactions` proptest asserting `join_tree(join_tree(a, b), c) == join_tree(a, join_tree(b, c))` (`Root` equality covers ceiling and content). Rewrite the testdoc of `join_associative` to state what it tests and drop the transitive-coverage sentence. Acceptance: `grep -rn associativ src tests` shows the new property; the testdoc for `join_associative` names no coverage it does not provide and makes no appeal to the mirror-versus-join differential. Construction: generator `base.act(p0, n_shared inserts)`; for each side i in 1..=3, `t = base.clone(); t.act(p_i, n_i inserts); t.act(p_i, forgets of drawn shared keys)`. Assert both association orders equal, and optionally all six permutations (commutativity composed).
- **remote-proxy-tests-25** (medium; T126). `src/tree/mirror/streaming/remote/proxy/tests/malformed.rs:186-196`. Resolution: Give the receiving side's acceptor a `Done` that captures the returned receive half (the contract hands it back resting at the frame boundary); after the session, read from the captured half and assert exactly the duplicated frame's bytes remain. Alternatively compose `wrap_link` under the scripted connector and assert the receiving side's `read_bytes` equals the sender's `write_bytes` minus the duplicate's length. Acceptance: the test asserts, by byte count or by reading the returned half, that the duplicated frame's bytes were not consumed by the session. Construction: Temporarily make the receiver in streams.rs continue past `End(Stream)` and break on the second one; the current test still passes.
- **streaming-backend-window-37** (medium; T128). `src/tree/mirror/streaming/window/tests.rs:266-310`. Resolution: Rewrite the doc without the literals ("the crossover record size and the u64 BDP-scale window are the solve's own numbers, pinned here and quoted at `Peer::sync_memory_budget`") so the next re-pin cannot reopen the gap; the assertions and their messages remain the record. If figures must stay in the doc, name them once as constants used by both the assertion and the message. Acceptance: no number in the doc comment disagrees with the assertions below it or with peer.rs:422-441.
- **streaming-tests-18** (medium; T128; `faults.rs` was edited by `p1-harness-crate` and `p2-walk` (both merged): re-anchor). `src/tree/mirror/streaming/tests/faults.rs:61-100`. Resolution: Delete the `before` snapshots and the two closing `prop_assert_eq!`s (71, 100, 122, 188); drop "while both materialized input roots remain untouched" (61-62), "with both input roots untouched" (108-109), and "and lifecycle atomicity" (1); rename the first test to `connected_violation_aborts_with_its_typed_error`. If a walk-tier atomicity statement is wanted, its observable is the session's `Output` on the failing path, not the caller's clone. Acceptance: no assertion in faults.rs compares a pre-session clone of an input root to itself; the docs claim only error routing, classification, and unchanged reconciliation under tolerated lies.
- **testing-infra-21** (medium; T129). `src/tests.rs:423-427`. Resolution: Make `EPILOGUE_MARKER` `pub(crate)`; set `budget = PREAMBLE_LEN + greeting_frame_len(&child) + party_frame_len(&child) + EPILOGUE_MARKER.len() - 1`, which makes "minus one byte" true and pins over-counting by any amount; reword 408-409 to "the last byte of the two-byte epilogue marker". Add the dual `severed_after_marker_is_retired` with `budget = ... + EPILOGUE_MARKER.len()`, asserting `Retire::Retired` and the peer `Ok`, which pins under-counting. Acceptance: both tests exist; injecting `+ 1` into `party_frame_len` fails the minus-one test (`Retired`), and injecting `- 1` fails the full-budget test (`Uncertain`). Construction: Add `+ 1` to `party_frame_len`'s return: `severed_party_frame_is_uncertain` and `severed_epilogue_marker_is_uncertain` both still pass today.
- **tests-bookmark-21** (medium; T130). `tests/bookmark_transmit_window.rs:412-427`. Resolution: pick one shape. (a) Recommended: keep the L423 assert as the drift alarm, delete the arm and its comment, add `assert!(leaf_version(peer, M0).is_some(), ...)` for A', B, and C after the heal so the "destroy" claim has a witness, and restate the doc as what is checked (an own event committed inside the persist window is not transmitted; after A crashes and reboots from C the fleet reconverges holding every pre-crash durable message). (b) Positive-path: build a schedule where M1 becomes durable before the crash (send M1, clean gossip A with B, crash A, restart from C which never saw M1, heal) and assert M1 survives everywhere; modest value since the invariant is pinned at the transmit boundary. Acceptance: no branch of the test is unreachable under its own assertions; the doc's first sentence names exactly the assertions in the body; a pre-crash durable message is asserted present on every replica after the heal.
- **tests-disruption-handshake-22** (medium; T130; write the helper against `run_to_quiescence`, not pollster (`p4-drivers` removes it)). `tests/handshake.rs:94-97`. Resolution: Add one helper `async fn alice_against(reply: &[u8]) -> (Result<Gossiped, Error>, Rumors<String>)` that builds alice, runs the fake peer, asserts `got[..13] == preamble(V2_OPENING, Protocol::V2 as u8, INTENT_REMAIN)[..13]` and `got[29] == INTENT_REMAIN` (bytes 13..29 are the universe's network id, which has no public accessor; skip them), and drops the write half after writing (the truncation case passes `&partial[..6]`). Each rejection test becomes: build the reply, call the helper, match the variant, assert `alice.snapshot().is_empty()`. Add a `bootstrap_retire_conflict_surfaces_error` case (network `[0; 16]`, intent 1). Acceptance: one helper and six tests of a few lines each; changing `Protocol::V2 as u64` to `3` in `Preamble::encode` (src/tree/mirror/handshake.rs:102) fails every rejection test, not only the snapshot suite; every error test asserts the local set is unchanged; `Error::BootstrapRetireConflict` appears in tests/handshake.rs. Construction: In a scratch copy set the encoded version to 3 at handshake.rs:102; today only handshake_roundtrip_succeeds fails (both sides real) and no rejection test observes the outgoing byte.
- **tests-lifecycle-21** (medium; T130; its Construction mutates `Batch::commit`, which `p2-commit-path` rewrites: apply the mutation to the merged code). `tests/redaction.rs:94-133`. Resolution: In both tests, give the peer live content that survives the redaction so the readout comparison is not over an empty map; capture `peer.local.snapshot().latest().clone()` before the redaction and assert it is unchanged after; subscribe a `changes()` observer (as single_peer.rs:370-384 does, draining the immediate first tick) and assert `now_or_never()` yields `None` after the redaction. Rewrite the doc at 116-118 to cite the no-op clause `Rumors::redact` states; consider folding the two tests into one property over "a version not currently held (never held, or already redacted)". Acceptance: both tests fail if `Tree::act` (or `Batch::commit`) is mutated to advance the ceiling or report `true` on an ineffectual forget; the testdoc no longer claims the docs are silent. Construction: mutate `Batch::commit` at batch.rs:143 to `inner.tree.act(party, actions); true` (unconditional wake) or make the ceiling join run for every action: redaction.rs still passes today; the proposed `latest()`/`changes()` assertions fail.
- **tests-observation-17** (medium; T130). `tests/listen.rs:241-243`. Resolution: After the observer subscribes (line 250) and before the retirement, `survivor.send(8).unwrap();`; after the drain assert that the items contain both 7 and 8, with 8 named as the message the session learned. Acceptance: ending the retiree's observer ahead of the write-back (or dropping learned content from it) fails the test on message 8. Construction: Apply the two-line change above and temporarily swap the order of the write-back and the observer-ending step in `retire_inner`; the test must fail on 8. Without the change, the same swap passes.
- **api-core-20** (low; T132). `src/peer/bootstrap/tests.rs:6-8`. Resolution: Extend the bodies: assert `config.payload_depth_limit` and `peer.codec.limit()` against a non-default `PayloadDepthLimit`, and assert the attachment reaches the joined peer (a `pub(crate)` `Attachment::is_attached()` or the `Debug` form, since `Attachment` has no `PartialEq`); in `defaults_match_the_seed_configuration`, read the expected values off `Peer::<u64>::seed()`'s `window`, `run_budget`, and `codec`. Alternatively narrow every quantifier to the two sizing knobs and point at tests/payload_depth.rs and tests/observe.rs for the rest. Acceptance: each testdoc's quantifier matches the fields its body asserts; the defaults test reads its expected values off a seeded `Peer`.
- **link-12** (low; T132). `src/link/tests.rs:94-95`. Resolution: restate each to what the body checks: "a two-byte window still carries six bytes to completion"; "the control halves carry bytes in both directions, intact and in order"; "Epochs advance one per begun session and wrap at `u8::MAX`; the epoch is a mismatch check, never an identity. `finish` marks each session's clean end."; "bytes cross in both directions". If blocking is meant to be tested, add the observation (`now_or_never` on the over-capacity write before draining). Acceptance: each listed doc names only behavior its body asserts.
- **remote-adapter-tests-4** (low; T132). `src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:1-1`. Resolution: either add an `Operation::Leaf` injection point to `Failing` (in-crate test infrastructure) and a decode row that fails at the k-th record of a multi-record run, asserting the typed error, the history, the untouched sentinel, and via the census that no node from the failing record took custody; or narrow line 1 to "every backend traversal operation" and, at failing.rs:185, state why construction is exempt (the comment currently gives no reason beyond itself). Acceptance: a committed test drives `decode_reply` to `DecodeError::Backend(Failure::Injected(Operation::Leaf))`, or the module doc no longer claims every reachable operation.
- **remote-capture-atlas-19** (low; T132; `capture/tests.rs` was rewritten by the renderer lane: verify at base and quote the absence of any gone site). `src/tree/mirror/streaming/remote/codec/capture/tests.rs:41-42`. Resolution: (1) "the version's canonical bytes (as hex) and its event-tree rendering move on the same line". (2) Add a `#[should_panic(expected = "no known shape")]` companion feeding a bare uint item, or drop the clause. (3) Retitle to what it checks (`parse_node` rejects nesting past `MAX_DEPTH` with the depth reason), or drive it through `render_item` and `assert_depth_fallback`. Acceptance: each testdoc names only what its body asserts.
- **remote-codec-17** (low; T132). `src/tree/mirror/streaming/remote/codec/decode/tests.rs:1061-1070`. Resolution: Rename the depth test to `..._fails_typed_at_the_record_iterator` and restate its first sentence ("fails typed at the record iterator, run structure having already passed ingress"). In the greeting test either construct the two named cases (splice the `set_len` entry out and decrement the map head from `0xa6` to `0xa5`, expecting `Shape("greeting is not a map of one entry per roster key")`; swap the `listing` and `set_len` entries, expecting the roster `Shape`) or restate the doc to "a renamed key or trailing bytes". In the record-length test either write "at the one- and two-byte head widths" or extend the sweep until `head_len` reaches 3 and assert `== 3`. Acceptance: each doc's stated rejections and widths correspond to constructed inputs asserted in the body; no testdoc in the partition places a `DecodeLeafError` at ingress.
- **remote-proxy-tests-2** (low; T132). `src/tree/mirror/streaming/remote/proxy/start/tests.rs:186-194`. Resolution: Generate from a valid `encode_greeting` output and apply drawn mutations (byte flips at drawn offsets, truncation, insertion, drawn listing entries), or draw `Greeting` fields structurally plus a key permutation; if the pure-random arm stays, narrow its doc to the heads it reaches. Acceptance: a sampled run of the property produces at least one `Error::HandshakeListing` and at least one `GreetingError` past the first head, and the doc names only what the generator reaches. Construction: Temporarily tally the error variant per case; under the current generator the tally is `HandshakeDecode(Shape)` for every case that is not `Ok`, with no `HandshakeListing` and no version-atom error.
- **remote-proxy-tests-9** (low; T132). `src/tree/mirror/streaming/remote/proxy/tests.rs:357-371`. Resolution: Either split into a `preamble_and_session_share_the_control_halves` test (documenting the byte-isolation claim, unit payload) and run the payload test on `reconcile_symmetric_accepts::<u64>`, or fold the preamble claim into this test's name and doc. Use `nth_party(0)`/`nth_party(1)`. Acceptance: the doc of whichever test calls `reconcile_after_preamble` states the byte-isolation claim; no `Party::seed()` in the file.
- **session-bookmark-18** (low; T132). `src/peer/gossip/tests.rs:79-89`. Resolution: Re-state the module doc (6-8) and the testdoc (79-85) for the two-byte CBOR text item `"."`. Sweep both positions: either the full 65536-pair space (cheap) or the first byte over `u8` with the second fixed plus the existing sweep, asserting `InvalidData` for every non-marker pair. Add a named case feeding `[0xd9, 0xd9]` (the opening of `SELF_DESCRIBED_HEAD`) and asserting `Error::Epilogue` with `InvalidData`, so the preamble-desync claim is pinned rather than asserted. Acceptance: the docs say "two-byte item"; a committed case rejects a preamble opening; a mutant that compares only `marker[1]` fails that case.
- **session-bookmark-42** (nit; T132). ``src/observe/tests.rs:45-56``: testdoc says "whatever the session kind"; the body calls `begin(SessionKind::Gossip)` once Resolution: iterate the three kinds or drop the quantifier
- **streaming-backend-window-15** (low; T132). `src/tree/mirror/streaming/backend/local/adversarial.rs:137-156`. Resolution: Poll with a counting waker (a `std::task::Wake` impl over an `AtomicUsize`) and assert the count equals the scheduled delays (3 for `vec![2, 1]`), or narrow the doc to the Pending/Ready sequence it checks. Acceptance: the body asserts a wake count, or the doc no longer claims self-waking. Construction: Delete line 107 (`cx.waker().wake_by_ref();`) and run this test: it still passes, while any scheduled streaming test hangs. A counting waker turns that into a failing assertion here.
- **streaming-backend-window-36** (low; T132). `src/tree/mirror/streaming/window/tests.rs:150-167`. Resolution: Either iterate the depths and assert `capacity(KEY_DEPTH - depth) > 1` wherever `stage_population(n, n * n, depth) > 1` (the helpers are imported), or reword the doc to the point it checks: depth 4, the first stage whose population outgrows the structural caps, receives capacity above a full fan under the default budget. Acceptance: doc and assertions describe the same set of heights.
- **testing-infra-19** (nit; T132). ``src/tests.rs:82-88``: two testdocs omit an assertion the body makes (`Retire::Uncertain`; `peer_out.is_err()`) Resolution: one clause each
- **tests-bookmark-14** (low; T132). `tests/bookmark_causality.rs:885-892`. Resolution: delete L885-892 and `EmissionLog::len`; delete when L734-739 and keep the invariant statement in `Model`'s doc at L276-278. Do not replace the causality floor with "`next_seq > 0` implies some node holds content": every sent message can be legitimately redacted or crash-lost. Acceptance: the per-leaf witness loop and the global log checks remain; neither deleted block has a caller.
- **tests-bookmark-4** (nit; T132). ``tests/bookmark_attach.rs:45-55``: several assertions admit outcomes their docs exclude (a write-only fault feed under a "touches no storage" claim; `writes >= 1` where the model says one; `Ok(None)` accepted beside `Err`) Resolution: tighten each to the intended outcome (sites listed in the entry)
- **tests-common-20** (low; T132). `tests/common/schedule/arb.rs:248-250`. Resolution: restate 248-250, 58-60, and 355-360: `observed_log[p]` is the set of `EventIdx`s the live peer has observed, kept in the shadow's own sorted-per-pass order so redact targets are drawn deterministically; the live peer's order is the tree's and is not compared. Acceptance: no passage in arb.rs claims sequence agreement with the live peer.
- **tests-disruption-handshake-21** (low; T132). `tests/handshake.rs:57-61`. Resolution: Delete line 60; keep the discriminant pin, or fold `Protocol::V2 as u8` into the outgoing-preamble check proposed in finding 22 (byte 11 of the bytes alice writes), which pins the discriminant against the wire rather than against the enum. Acceptance: no assertion in tests/handshake.rs compares a local constant to a literal restating it.
- **tests-disruption-handshake-23** (low; T132). `tests/handshake.rs:247-262`. Resolution: Fold into magic_mismatch_surfaces_error as a second reply pattern (a small table of openings), or rewrite the doc to claim only what is asserted: any 30 bytes without the rumors opening are diagnosed as MagicMismatch quoting the first six. Acceptance: each handshake.rs test's doc names a distinct observed behavior.
- **tests-disruption-handshake-25** (nit; T132). ``tests/handshake_liveness.rs:132-134``: the fixture self-check comment states a purpose the `GREETING_FLOOR` doc contradicts Resolution: reword to the const doc's claim
- **tests-lifecycle-4** (low; T132). `tests/bootstrap.rs:52-59`. Resolution: Either drop "non-disjoint" and "creates a *disjoint* party" from the doc, cite `party_conservation.rs` for disjointness, and state the floor mechanism ("a stale floor would leave the newcomer's version dominated, and deletion honoring would drop it as already seen"); or sharpen the body so the claim is true: have the provider also `send` after the fork, gossip, and assert both payloads live on both sides. Apply the same to the `String` variant. Acceptance: the doc's stated failure mode is one the body demonstrably detects, and the adverb has its mechanism beside it.
- **tests-observation-20** (low; T132). `tests/listen.rs:616-620`. Resolution: Reword the doc to the two checked claims (the union covers the final live set; nothing re-fires after a completed pass) and add `prop_assert_eq!(first_versions.len(), first_run.len())` and `prop_assert_eq!(second_versions.len(), second_run.len())` so each run is duplicate-free; delete the 691-692 comment. Mirror the duplicate check in causal.rs:510-520. Acceptance: the doc names only checked properties; a within-run duplicate fails the test.
- **tests-resource-link-window-7** (low; T132). `tests/decode_alloc.rs:263-271`. Resolution: Run `supply_full_delivery_costs_at_most_payload_plus_chunk` (or a twin) through `decode_supply_frame_budgeted(&bytes[..], 0)`, making the lone record a true overhang at no extra cost, and reword lines 269-271 to name that test as the complement. Acceptance: a metered test decodes a lone record under a budget smaller than the record, asserts `Ok`, and holds the same `[N, N + chunk]` band.
- **tree-typed-9** (low; T132). `src/tree/typed/hash/tests.rs:10-12`. Resolution: replace "17-byte" with "`CHILD_RECORD_LEN`-byte" (or "`radix ‖ hash`") at both sites; open the two root tests with the root claim ("The root's ceiling is the join of every leaf's version") and point at `bounds_are_the_leaf_fold_at_every_node` for the per-node statement; replace "version", "branch versions", and "root version" in the helper docs with "ceiling" or "bounds", and replace "recomputed by `Node::branch`" with "memoized lazily from the rebuilt children". Acceptance: `grep -rn '17-byte' src/tree/typed` is empty; each testdoc's first sentence is checked by its own body; no helper doc in the file calls a bound a "version".

## Hazards and stops

- T151: no `cases` in any `ProptestConfig` added here.
- `p2-commit-path` (unmerged) edits `arb.rs` (tree-core-34's generator)
  and `batch.rs` (tests-lifecycle-21's mutation target);
  `p2-vanish-liveness` and `p1-proptest-ci` (unmerged) edit
  `proxy/tests.rs`, `gossip_when.rs`, `session_stats.rs`,
  `party_conservation.rs`, `observe.rs`. Launch after all merge.
- `p1-causality` (merged) rewrote `bookmark_transmit_window.rs`
  (tests-bookmark-21); `p1-harness-crate` and `p2-walk` rewrote
  `faults.rs` and the proxy tests: re-anchor.
- `p4-drivers` removes pollster: tests-disruption-handshake-22's helper
  is written against `run_to_quiescence`.
