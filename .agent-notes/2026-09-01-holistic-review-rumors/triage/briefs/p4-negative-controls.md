<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T126, T130, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: a committed demonstration for every conformance-bug detector

## Goal

Every conformance-bug detector (an arm that rejects a nonconforming
peer, a misbehaving backend, or a malformed input) has a committed
demonstration that a known-bad artifact reaches it: `label_item`'s
arms, the nine proxy error arms, the `Resolver`'s skip-past arm,
`Work::execute`'s accept arm, `read_early`'s rejections as a
recognizer differential, the backend-contract panics, the typed layer's
`debug_assert!` guards, the three older link checks, the legibility
walker, the six `snapshot_liveness` convictions, the greeting
rewriter's fired witness, and the two driver terminal arms. Each lands
as a test that fails when its arm is removed; the entry's Construction
is the artifact. Effort: high.

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
  run in the background redirected to a log under `<scratchpad>/p4-negative-controls/`,
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

One commit per entry: the test, plus the doc line the resolution names
(remote-adapter-streams-28's cancel-safety cross-reference). For each,
apply the reversible mutation the Acceptance names (delete the arm,
replace the fate, `if !any` to `true`, drop a guard to `<=`), observe
the failure, quote it in the commit message, restore. Where an arm is
judged unreachable by construction (remote-proxy-31's local-only group,
if so), the comment at the site states why and the report names it.

Order: tree-typed-7 and conformance-20 (independent of the wire);
materialized-37; remote-adapter-streams-14, -28; remote-proxy-31,
remote-proxy-tests-22, -26; remote-adapter-tests-15 (after
`p4-remote-simplify`'s remote-adapter-streams-3, or against the entry
points at base with a re-run after); tests-disruption-handshake-15;
tests-wire-format-18, -20.

Oracle: the committed tests exist and are named in the report; each
`should_panic` test carries its doc (`just testdoc`); the standing
checks are the tests themselves. No general grep.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **remote-adapter-tests-15** (medium; T126; T126 orders it after remote-adapter-streams-3 (`p4-remote-simplify`): pin the family against the entry points that exist at base and re-run after that lane merges). `src/tree/mirror/streaming/remote/adapter/tests/opening.rs:245-290`. Resolution: land the recognizer-differential family the note designs: generate short frame words over {Supply(Continue), Supply(End), End(Reply), End(Stream), Match, Query}, define the accepted language once (`Supply(Continue)* Supply(End)` or one bare `End(Reply)`; nothing after), and assert `early_supplies` accepts exactly it and rejects each other word with the variant the recognizer predicts; the existing point tests become witnesses of the family or are dropped. Alternatively make the malformed cases entry-point-parametric (one table run through both `decode_reply` over `Scope::opening(&[])` and `early_supplies` over the root prefix), or factor the shared record handling into one function both loops call. Add the empty-run `debug_assert!` to `read_early` or state at the site why the asymmetry is intended. Acceptance: a test in opening.rs fails when decode.rs:188's `if !any` is replaced by `true`; each of `BareEndAfterReaction`, `UnexpectedStreamEnd`, `TruncatedReply`, `UnpositionedQuery` is asserted at least once against `early_supplies`. Construction: `early_supplies` over `[Frame::Reaction(WireReaction::Supply(one record), Flow::Continue), Frame::End(End::Reply)]` must return `Err(DecodeError::BareEndAfterReaction)`; with the `!any` guard replaced by `true` it returns `Ok` with one node, which no committed test observes.
- **remote-proxy-tests-26** (medium; T126). `src/tree/mirror/streaming/remote/proxy/work/tests.rs:28-38`. Resolution: Add a test to work/tests.rs: from `peer.into_parts()`, `connector.connect()` a stream and write a label pair whose epoch item disagrees with `session.epoch()` (or whose stream item is `>= Stream::COUNT`), then `run_to_quiescence(work.execute(future::pending()))` and assert `Err(Error::Accept(AcceptError::Epoch { .. }))` (or `UnknownStream`) with the exact values and no panic. Optionally a full-stack dual via a label mutation in `ScriptedWrite` (it already parses past the two label items) reaching `MirrorError::Server(RemoteError::Accept(_))` in malformed.rs. Acceptance: a test in work/tests.rs whose asserted outcome is `Error::Accept(_)` from `execute`. Construction: In `parked_session()`'s peer link: connect, write two canonical unsigned-int heads with the wrong epoch, flush, then execute; expect `Error::Accept(AcceptError::Epoch { .. })`.
- **tests-disruption-handshake-15** (medium; T130). `tests/gossip_when.rs:438-442`. Resolution: Over `rumors::link::memory_with_capacity(1)`, let B run one-shot `gossip` and poll it a few times so its preamble crosses byte by byte; poll A's driver (`a_sessions.next().now_or_never()`) so `staged` holds between 1 and 29 bytes (assert this from the fixture, e.g. via a metered link). Then (a) drop A's cue sender and drive both under `run_to_quiescence`: A yields `Ok(Gossiped { led: Led::Remote, .. })` then None, B gets Ok; (b) in a second test drop A's driver instead and assert `run_to_quiescence(a.gossip(&mut a_link))` is `Ok(Err(Error::LinkPoisoned))`, while the existing empty-boundary drop leaves the link reusable. Acceptance: two new tests whose fixture asserts the staged byte count is strictly between 0 and 30; mutating gossip.rs:994-997 to `return None` and 1369-1371 to a no-op each fails exactly one of them. Construction: As in the resolution; the capacity-one link is the only shape that can leave a partial fill, since `Staged::fill` reads with one `read` per poll.
- **conformance-20** (low; T132). `src/conformance/link/tests.rs:872-877`. Resolution: Add a looped-back control fixture (rebuild one end with its own `control_write` joined to its `control_read` through `tokio::io::duplex`) and a `#[should_panic(expected = "not this side's own")]` test on `check_control`; a truncating `Tx` wrapper that drops the final byte on shutdown or drop with a `should_panic(expected = "exact bytes")` test on `check_streams`, optionally an `Rx` wrapper that never surfaces EOF asserted `Stalled`. For `check_sessions`, either a fixture that passes every focused probe and fails sessions, or reword the header to name the checks it covers. Acceptance: each new test fails its check as asserted; the header's claim is true of every check in `check`. Construction: wire `a.control_write` to `a.control_read` via `tokio::io::duplex` and run `check_control`: the a-side `read_exact` returns `CONTROL_PROBE_AB` and the `assert_eq!` against `CONTROL_PROBE_BA` panics.
- **materialized-37** (low; T132). `src/tree/mirror/streaming/materialized/work/resolver.rs:81-83`. Resolution: Add an `Injection::InvalidSupplySkipsHeld` script: `ours = {r}` (any held radix with `r < 255`), reply `[Supply(r + 1, supplied)]`, expected `InvalidSupply`; run it through the same 32-height dispatch. Acceptance: a committed injection fails when the arm at 81-83 is removed.
- **remote-adapter-streams-14** (low; T132). `src/tree/mirror/streaming/remote/adapter/encode.rs:249-262`. Resolution: Land the scope-A disposition: a test backend wrapping `Local` whose `leaves` override swaps two adjacent yields (or displaces one prefix) and whose `assemble` override drops the last node or ends early while leaves remain, with `#[should_panic(expected = ..)]` tests for each of the three encoder/decoder messages and the decode.rs:299 `unreachable!`. Either add order and containment checks to the conformance `leaves`/`assemble` wrappers so "convicts a violating override" is true for every listed clause, or narrow backend.rs:195-197 to the clauses the suite checks. Acceptance: a committed test panics with each message under a deliberately misbehaving backend; `Backend::leaves`/`assemble` rustdoc lists exactly the clauses that are enforced somewhere, and names where. Construction: Wrap `Local` in a test backend whose `leaves` collects the inner stream, reverses two adjacent items, and re-yields; call `encode_reply` on a two-leaf node under `#[should_panic(expected = "strict path order")]`. For `reify`, an `assemble` override that drops its last yielded node reaches the `expect` at decode.rs:427; one that ends its stream after the first node while leaves remain reaches decode.rs:299.
- **remote-adapter-streams-28** (low; T132). `src/tree/mirror/streaming/remote/streams.rs:756-778`. Resolution: Land the recorded disposition beside `accept_driver_rejects_unknown_stream_index`, using `raw_labeled`-style raw connects on a `memory()` link: (1) write `[0x40, 0x03]` (a byte-string head where the epoch belongs) and assert `AcceptError::Label { detail: "label item is not an unsigned int", .. }`; (2) write `[0x18, 0x00, 0x03]` (epoch 0 spelled with a one-byte argument) and assert `AcceptError::Label { detail: "label head is not canonical", .. }`; (3) write `[EPOCH]` alone and drop the writer, drive a receiver awaiting its claim through `first_reported_error`, and assert `StreamError::SupplyClosed { source: None, .. }` with `take_supply_failure()` yielding `UnexpectedEof`; (4) write `[0x18]` (the first byte of a two-byte head) and drop, asserting the same deferral. Cross-reference (3) from `StreamSender::frame`'s cancel-safety section so the doc claim names its pin. Acceptance: the four tests exist, each fails when its arm in `label_item` is replaced by a different `AcceptFate`, and the `frame()` cancel-safety text names the test that pins the peer-side classification. Construction: As in the resolution; all four are buildable from the existing `raw_labeled`, `claims`, `error_route`, and `first_reported_error` helpers.
- **remote-proxy-31** (low; T132). `src/tree/mirror/streaming/remote/proxy/work/pump.rs:479-521`. Resolution: remote-reachable group (pump.rs:481, 513, 519): land the scope-A note's `Early` pairing property (for ascending radix sets R requested and S supplied, `advance_to` over R against a scripted supply stream of S then `finish` yields each r answered iff r in S and ends in `UnaskedReply` iff S is not a subset of R), plus one full-proxy wire witness: extend the harness's greeting rewrite to drop one radix from the responder's listing as the initiator hears it, so the initiator early-ships a radix the responder never requests; place the orphan below a requested radix for the behind-cursor arm, as the highest early radix with a request following for the leftover-lookahead arm, and with no request after it for the unread-group arm. Local-only group (encode.rs sites): a `mirror(scripted_participant, proxy)` where a scripted in-process participant yields one reply too many, one too few, an opening that is not a query, or a terminal reply that asks a question; or, if these are judged unreachable by construction of the crate's own walk, say so at each site. Acceptance: each listed arm is reached by a committed test asserting the exact variant, or carries a comment stating why it is a local-programmer-error detector with no constructible peer input.
- **remote-proxy-tests-22** (low; T132). `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:284-297`. Resolution: Give `RewriteRead` a shared `Arc<AtomicBool>` set when it enters `RewriteState::Serving` with a rewritten item, return it from `reconcile_rewritten_greetings`, and assert it in the three convergence tests (and, for symmetry, the failing ones). Acceptance: making `GreetingRewrite::apply` return its input unchanged fails all seven declaration tests.
- **tests-wire-format-18** (low; T132; the streams>=2 half is T23's; the walker negatives and the `streams.len() >= 3` case are this lane's). `tests/wire_legibility.rs:109-116`. Resolution: Add three unit checks on the walker (a `[0xff]` sequence; `24(<< 0x00 0x00 >>)` with residue; `63(0)`) asserting `Err`. Add one deterministic case staged through `common::shape` (a shared pair at three bytes on both sides plus a divergent leaf) so at least one capture per run has a stream index of 2 or more, asserting `streams.len() >= 3` on that capture. Acceptance: negative walker tests committed and red on the bad inputs; one legibility run includes a capture with three or more data streams. Construction: `assert!(walk_sequence(&[0xff], "bad").is_err())`; build tag 24 around the two-byte string `[0x00, 0x00]` via `ciborium::ser::into_writer(&Value::Tag(24, Box::new(Value::Bytes(vec![0, 0]))), ..)` and assert `Err` mentioning "residue"; build `Value::Tag(63, Box::new(Value::Integer(0.into())))` and assert `Err` mentioning "byte string".
- **tests-wire-format-20** (low; T132). `tests/snapshot_liveness.rs:187-213`. Resolution: Extend the module fixture with `other__foo__tests__x.snap`, `rumors__foo__tests.snap`, `rumors__wrong__tests__x.snap`, a `notes.txt`, a nested directory, and a tests-side `nosep.snap`, asserting each conviction's message; use the by-path map in both fixture tests. Acceptance: each `Err(...)` literal in the sweep has a fixture line that produces it. Construction: Add `.file("src/foo/snapshots/other__foo__tests__x.snap", "")` to `module_snapshots_resolve_through_the_module_path` and assert its verdict contains "rumors__ prefix"; repeat for the other five.
- **tree-typed-7** (low; T132; `p2-commit-path` adds `src/tree/typed/prefix/tests.rs`: after its merge, extend that file rather than create it). `src/tree/typed/hash.rs:196-210`. Resolution: one `#[cfg(debug_assertions)] #[should_panic(expected = "...")]` test per guard in the guarded function's sibling tests.rs (prefix.rs gets a new `prefix/tests.rs` and `mod tests;`). Acceptance: each new test fails when its guard is deleted or weakened to `<=`, and passes with it present; the gate's `testdoc` sees a doc comment on each. Construction: hash/tests.rs: `Hash::branch(&[], [(2u8, Hash::default()), (1u8, Hash::default())])` expecting "strictly ascending radix order"; `Hash::branch(&[], [(0u8, Hash::default())])` expecting "one-child branch is unrepresentable". fan/tests.rs: `let mut f = Fan::new(); f.push(3, child()); f.push(3, child());` expecting "not greater than the current last". untyped/tests.rs: two entries sharing one `[u8; 32]` path through `Node::from_sorted_leaves(0, &mut entries)` expecting "strictly ascending by path"; a one-entry run whose leaf was pre-wrapped with `.beneath(0)` expecting "supplies bare leaf nodes". prefix/tests.rs: `Prefix::<Root>::new().erase().assume::<Z>()` expecting "re-tags at the height it was erased at".

## Hazards and stops

- T151: no `cases`.
- `p2-vanish-liveness` (unmerged) edits `gossip_when.rs`
  (tests-disruption-handshake-15) and the proxy tests; `p2-commit-path`
  adds `prefix/tests.rs` (tree-typed-7). Launch after both merge.
- The collision-schedule mode (T23, merged) makes deep geometry
  reachable behind `test-internals`: tests-wire-format-18's
  three-stream capture may use it; say which staging was used.
- remote-adapter-streams-14 touches `Backend`'s rustdoc (the clauses
  enforced, and where): the maintainer-facing doc T102 restates
  (`p4-placement`); coordinate the wording, land after it.
