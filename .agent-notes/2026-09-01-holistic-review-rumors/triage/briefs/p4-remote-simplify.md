<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T119, T126, and T132 (T63's row named, not landed; inventory-5 is a dup) in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: one election, one hand-off, one spelling per mechanism on the wire side

## Goal

The wire side (adapter, codec, proxy, streams, capture) elects the
initiator once and hands off through one struct (T119, after
remote-proxy-7 collapses the hand-off), reads the opening through
`read_reply`, derives scopes by one rule, parses and writes the greeting
as straight-line code (T126), and folds the duplicated pumps, the flush
block, the erase-and-box, the frame-to-signal projection, the two proxy
error names, the wrappers, and the `Option`/`unreachable!` states into
one spelling each. No wire byte moves: the `insta` snapshots are the
standing check. Effort: high.

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
  run in the background redirected to a log under `<scratchpad>/p4-remote-simplify/`,
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

1. **T119's series**: remote-proxy-7, then remote-proxy-4, then the
   premises bundle (remote-proxy-24's `Ingress`, remote-adapter-streams-4,
   remote-adapter-tests-2's helpers); TRIAGE.md orders remote-proxy-7
   before the bundle. Known-bad: `equal_versions_return_both_roots` and
   `tests/observe.rs`'s election assertions.
2. **Adapter**: remote-adapter-streams-3, then -5, -12, -9, -10;
   remote-adapter-tests-19.
3. **Streams**: remote-adapter-streams-20 (with inventory-6's
   `streams.rs` half), -24, -25.
4. **Proxy pumps**: remote-proxy-23, -28, -29; remote-proxy-tests-23.
5. **Codec and capture**: remote-codec-27, remote-capture-atlas-29, -4,
   -5, -16.

Oracle: `start.rs` has no call to `initiates` and no version
comparison; `state.rs` has no `unreachable!` and no `debug_assert_eq!`
on `Speaker`; `read_early` is gone or delegates;
`grep -rn 'map(erased::erase_reply' src` hits nothing outside
`erased.rs`; `greeting.rs` has no `unreachable!`, `expect`, or
`match key`; `grep -rn codec_stream_count src` empty;
`grep -rn 'Error as RemoteError' src` hits only `remote/error.rs`;
`git grep -n 'Symmetric with decode'` empty; exactly one
`Frame -> Signal` match in the tree; the wire snapshots byte-identical
(`just test`); the proxy and adapter suites unchanged.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **remote-adapter-streams-4** (low; T119). `src/tree/mirror/streaming/remote/adapter/decode.rs:203-210`. Resolution: Define a small `#[derive(Clone)]` struct in the adapter (the greeting's priced premises plus the peer's payload codec) with the three fields and their existing per-field docs; `Work` constructs it once; `Early` holds one field; `decode_reply(backend, premises: &Premises, scope, frames)`; `SupplyRuns::new(premises.version_bytes)`. Optionally a receiver-scoped context in streams.rs for `read_frames`. Acceptance: each adapter decode entry point takes four or fewer parameters; `Early` stores one premises field; the pump call sites shrink; adapter and proxy tests pass unchanged.
- **remote-adapter-tests-2** (medium; T119). `src/tree/mirror/streaming/remote/adapter/tests.rs:35-39`. Resolution: add to tests.rs a `fn codec() -> PayloadCodec` and thin wrappers that fix `Local, u64::MAX, unbounded(), codec()` for the common decode, leaf-decode, and early-supply shapes, leaving the raw six-argument calls only where a premise is varied so those sites stand out. Acceptance: the codec constructor appears in the helper and at the sites that vary it, nowhere else; the tests varying the version bound, ledger, or backend are the only ones spelling those arguments.
- **remote-proxy-24** (low; T119). `src/tree/mirror/streaming/remote/proxy/work/pump.rs:183-187`. Resolution: introduce a `Clone` struct in `work/` (say `Ingress<B> { backend: B, version_bytes: u64, ledger: SupplyLedger, codec: PayloadCodec }`) with `async fn decode(&self, scope, &mut incoming)` and `decode_leaf` wrappers over the adapter calls; each pump captures one `Ingress` and the `Progress`; `Early` holds an `Ingress`. Whether the adapter's entries take the same struct, and whether they take `&SupplyLedger`, is the adapter reviewer's call. Acceptance: each decode pump's prelude is two lets; the six-argument decode call appears once per adapter entry; `just clippy` clean; proxy suites pass.
- **remote-proxy-4** (medium; T119). `src/tree/mirror/streaming/remote/proxy/start.rs:168-196`. Resolution: add `fn connected(self, local: Greeting, remote: Greeting) -> Result<Connected<B, R, W, C, A>, Error<B::Error>>` on `impl<B, R, W, C, A, V> Handshaking<B, R, W, C, A, V>` (the free function never reads `versions`) holding the depth check, `window.resolve`, `run_budget`, and the hand-off; both impls reduce to their exchange plus `self.connected(ours, remote)`. Let the one remaining call-site comment shrink to the placement note ("before the equal-versions shortcut") and leave the full argument on `payload_depth_limits_match`. Replace `Connected::new`'s positional list with a struct literal. If remote-proxy-7 lands, `open` becomes the body of `initiator()`/`responder()` and the chain collapses further; whether the remaining premises get a named bundle is the open question below. Acceptance: one `window.resolve` and one `payload_depth_limits_match` call in start.rs; every surviving `too_many_arguments` allow carries a one-line rationale above the attribute; `start/tests.rs` and `proxy/tests.rs` pass unchanged.
- **remote-proxy-7** (medium; T119; first in this lane (TRIAGE.md: remote-proxy-7 before decision 98's bundle)). `src/tree/mirror/streaming/remote/proxy/start.rs:356-376`. Resolution: make `Connected` a struct holding the `Link`, backend, resolved `Window`/`RunBudget`, the remote greeting's `max_version_bytes`/`set_len`/`listing`, stats, codec, and observe handle. `complete_equal` destructures the link and returns the control halves. `Initiator::initiator` and `Responder::responder` fire `observe.elected(...)` and call `open` with `Speaker::Initiator`/`Speaker::Responder` respectively (the remote is the initiator exactly when the driver calls `initiator()` on the proxy), building `Session` there; equal sessions stay allocation-free because `open` runs only on the role paths. Delete `ConnectedState`, `Connected::new`, `Connected::equal`, `diverged()`, both `unreachable!`s, and the four `debug_assert_eq!`s. Acceptance: start.rs contains no call to `initiates` and no version comparison; state.rs contains no `unreachable!` and no `debug_assert_eq!` on `Speaker`; `just gate` clean; the proxy suites pass unchanged, including `equal_versions_return_both_roots` and `tests/observe.rs`'s election assertions.
- **remote-adapter-streams-3** (medium; T126). `src/tree/mirror/streaming/remote/adapter/decode.rs:136-200`. Resolution: Replace `read_early`'s body with `let read = read_reply::<B, _, _, Scope>(version_bytes, ledger, Scope::new(parent, &[]), &mut frames, <the interior question closure>, leaves, codec).await?; if read.is_none() { return Ok(()); } if frames.next().await.is_some() { return Err(DecodeError::ExtraOpeningReply); } Ok(())`, discarding the skeleton and (necessarily empty) questions. Extract `fn assembly<B>(backend: B, height: usize, rx: mpsc::Receiver<..>) -> Pin<Box<dyn Stream<Item = Result<(ErasedPrefix, B::Erased), B::Error>> + Send>>` holding the `ReceiverStream`/probe/`ops::assemble` setup; `assemble_supplies` becomes `assembly(...).map_err(DecodeError::Backend).try_collect().await` and `early_supplies` pins the same helper; drop both `pin!`s. The `#[cfg(test)]` probe hooks fall from four sites to two, and the `fan_probe` module doc (553) no longer needs to say both paths "hook the same counter". Acceptance: `read_early` is gone or is a handful of lines delegating to `read_reply`; `ledger.charge(1)` and `<B::Node<Z> as Leaf>::leaf` each appear once in decode.rs; `adapter/tests/{opening,malformed,fan_occupancy}.rs` pass unchanged (they pin `UnpositionedMatch`/`UnpositionedQuery`/`BareEndAfterReaction`/`ExtraOpeningReply` on the early stream and the FAN + 1 occupancy ceiling on both channels).
- **remote-adapter-streams-5** (medium; T126). `src/tree/mirror/streaming/remote/adapter/decode.rs:261-273`. Resolution: Introduce a two-variant `enum Level { Interior, Leaf }` (or two named functions with one signature) and one `fn derive_question(level: Level, scope: &mut Scope, listing: &[(u8, Hash)]) -> Result<Scope, ScopeError>` used by both `render` and `read_reply`; `render` handles `Match` and `Supply` itself as `read_reply` already does, so the `debug_assert!(question.is_none())` at encode.rs:183 dissolves (a `Supply` structurally derives no question). Make `Encoded { frame, question: Option<Scope> }`, `Decoded<E> { reply, questions: Vec<Scope> }`, `Frames<E>`; keep `encode_reply`/`encode_leaf_reply`/`decode_reply`/`decode_leaf_reply` as thin wrappers passing the level. Drop `Q` from `write_reply`/`write_encoded` in proxy/work/encode.rs. Acceptance: no type parameter in adapter/{encode,decode}.rs has a single instantiation; `git grep -n 'Symmetric with decode'` returns nothing; the scope-derivation rule appears once; `adapter/tests/{properties,malformed,opening}.rs` pass unchanged except for `into_parts` tuple types.
- **remote-codec-27** (medium; T126; keep `KEYS` as the roster's single spelling for remote-codec-26's sortedness test (`p4-drivers`)). `src/tree/mirror/streaming/remote/codec/greeting.rs:60-88`. Resolution: Factor `fn key(input: &mut &[u8], name: &'static str) -> Result<(), GreetingError>` for the text-key check and read the six entries in wire order as straight-line code (`key(&mut input, "listing")?; let listing = parse_listing_map(&mut input)...?; key(&mut input, "set_len")?; let set_len = uint(&mut input, ...)?; ...; Ok(Greeting { .. })`), mirroring `greeting_map` as six explicit writes. Keep `KEYS` as the single spelling of the roster and its order for the sortedness test of remote-codec-26 (and for the map count), so the roster's single-source role survives as a test rather than a runtime loop. Acceptance: `greeting.rs` contains no `unreachable!`, no `expect`, and no `match key`; `greetings_round_trip` and `greeting_key_roster_is_exact` pass unchanged; the greeting bytes in `tests/gossip_snapshot.rs` are byte-identical.
- **remote-adapter-streams-10** (nit; T132). ``src/tree/mirror/streaming/remote/adapter/decode.rs:530-539``: `SupplyOrder`'s "preceded" half is unreachable; `LeafOrder` fires first Resolution: Narrow the filter to `*previous == radix` and reword the variant to the reachable case, or comment why `<` cannot arrive
- **remote-adapter-streams-12** (low; T132). `src/tree/mirror/streaming/remote/adapter/encode.rs:161-233`. Resolution: `let wire = match reaction { Match => WireReaction::Match, Query(listing) => WireReaction::Query(listing), Supply(radix, node) => { <leaf loop with the mid-run flush yield>; assert!(!run.is_empty(), ..); WireReaction::Supply(run) } }; if let Some((previous, question)) = pending.replace((wire, question)) { yield Encoded { frame: Frame::Reaction(previous, Flow::Continue), question }; }`. Acceptance: two `yield Encoded { frame: Frame::Reaction(.., Flow::Continue), .. }` sites in `render`; `adapter/tests/{runs,properties}.rs` (frame sequences and Continue/End placement) pass unchanged; wire snapshots unchanged.
- **remote-adapter-streams-20** (low; T132; carries the `streams.rs` half of inventory-6). `src/tree/mirror/streaming/remote/streams.rs:179-234`. Resolution: Replace `enum SendState<Tx>` with `struct Opened<Tx> { write: FrameWrite<CountedWrite<Tx>>, done: Done<Tx> }` and `state: Option<Opened<Tx>>`; factor the frame write into `Opened::frame(&mut self, stream: Stream, frame: Frame)`; restructure `write`/`finish` as above. Acceptance: `git grep -n 'unreachable!' src/tree/mirror/streaming/remote/streams.rs` returns nothing from `StreamSender`; `unopened_sender_finishes_without_connecting`, `truncated_stream_is_reported_not_ended`, and `frames_flow_sender_to_claimed_receiver` pass unchanged.
- **remote-adapter-streams-24** (low; T132). `src/tree/mirror/streaming/remote/streams.rs:304-331`. Resolution: `StreamReceiver { frames: BoxStream<'static, Frame>, claimed: bool }`; `new` calls `Box::pin(read_frames(..))` directly; `poll_next` sets `claimed = true` before polling; `finish` checks `!self.claimed`. Delete `ReceiverStart` and `frames()`; move the per-field docs (stats and observe rationale) onto `read_frames`' parameters. Decide whether `Rx` stays as a phantom or the type loses the parameter (callers in state.rs name `StreamReceiver<A::Rx>`). Acceptance: no `expect` remains in `StreamReceiver`; `unpolled_receiver_finishes_vacuously`, `frames_flow_sender_to_claimed_receiver`, and `supply_failure_reaches_the_awaiting_receiver` pass unchanged.
- **remote-adapter-streams-25** (nit; T132). ``src/tree/mirror/streaming/remote/streams.rs:506``: The supply-failure deposit slot is spelled twice with two lock sites; a newtype names it once Resolution: `struct SupplyFailureSlot(Arc<Mutex<Option<io::Error>>>)` with `deposit(&self, io::Error)` (`get_or_insert`) and `take(&self) -> Option<io::Error>`
- **remote-adapter-streams-9** (low; T132). `src/tree/mirror/streaming/remote/adapter/decode.rs:520-526`. Resolution: `previous: previous.into(),`. Acceptance: no `try_into().expect` remains in decode.rs; `malformed.rs`'s `LeafOrder` pin passes unchanged.
- **remote-adapter-tests-19** (low; T132). `src/tree/mirror/streaming/remote/adapter/tests/properties.rs:21-23`. Resolution: collapse to one alias (`type Erased = <Local as Backend>::Erased;`) or use the path directly, and drop the "per payload" sentence. Acceptance: no `ErasedUnit`/`ErasedU64` identifiers remain in properties.rs.
- **remote-capture-atlas-16** (nit; T132; T138/T140 replaced the hand renderer: verify at base and quote the absence if the site is gone). ``src/tree/mirror/streaming/remote/codec/capture.rs:616-625``: `render_embedded` is a one-production-caller wrapper over `render_embedded_as` Resolution: Rename `render_embedded_as` to `render_embedded`, pass `Naming::Plain` at the four call sites, delete the wrapper
- **remote-capture-atlas-29** (low; T132). `src/tree/mirror/streaming/remote/codec/tests.rs:504-514`. Resolution: Add `pub(super) fn signal(&self) -> Signal` on `Frame` in frame.rs; have `FrameEncoding::new` compute `let signal = frame.signal();` and match only on the body; delete both test copies and call `frame.signal()`. Minimal alternative: delete error_atlas.rs:543-553 and `use super::frame_signal;`. Acceptance: exactly one `Frame -> Signal` match exists in the tree; codec and atlas snapshots are byte-identical.
- **remote-capture-atlas-4** (low; T132). `src/tree/mirror/streaming/remote.rs:87-91`. Resolution: Delete lines 87-91; in src/link/tests.rs:17 write `usize::from(crate::tree::mirror::streaming::remote::Stream::COUNT)`. Acceptance: `grep -rn codec_stream_count src` is empty; `stream_count_matches_the_codec` compiles and passes.
- **remote-capture-atlas-5** (low; T132). `src/tree/mirror/streaming/remote.rs:93`. Resolution: Delete `pub use proxy::Error;` at line 93 (`RemoteError` remains via the glob and is the name `crate::error` re-exports), change gossip.rs:1416 and 1423 to `streaming_remote::RemoteError`, and drop the `Error as RemoteError` renames in the five proxy test files. The `tree` module is crate-private, so this is not a public API change. Acceptance: `grep -rn 'Error as RemoteError' src` returns only remote/error.rs:17; `grep -rnE 'streaming_remote::Error\b' src` is empty.
- **remote-proxy-23** (low; T132). `src/tree/mirror/streaming/remote/proxy/work/pump.rs:98-99`. Resolution: add `pub(crate) fn erase_requests<B, H>(requests: impl Requests<B, H>) -> BoxStream<'static, Reply<B::Erased>>` to erased.rs next to `erase_reply`, with one `pub(crate)` alias there; replace the ten call sites and delete the two local aliases. Acceptance: `grep -rn 'map(erased::erase_reply' src` returns nothing outside erased.rs; one alias for the erased request stream.
- **remote-proxy-28** (low; T132). `src/tree/mirror/streaming/remote/proxy/work/pump.rs:371-401`. Resolution: give `leaf_decode_pump` a `next_scopes: Option<Sender<Scope>>` parameter; when `None` and `questions` is nonempty, fail with `TerminalQuery` (or, after remote-proxy-29, nothing), else record and yield; `complete_responder` passes `None`, `leaf_replies` passes `Some(next_scopes)`. Delete `terminal_decode_pump`. Acceptance: one leaf-height decode pump in pump.rs; `instrumented_channels_cover_every_proxy_edge` and the `Trace` assertions still pass.
- **remote-proxy-29** (low; T132; land the arm deletion; the variant's doc naming the local producer is remote-proxy-3, T63's (P6)). `src/tree/mirror/streaming/remote/proxy/work/pump.rs:393-395`. Resolution: delete the arm and let the codec's placement rejection be the diagnosis; the variant stays for the local encode-side use at encode.rs:67, and its doc then names the local producer (remote-proxy-3). Alternatively keep it with a comment in the style of decode.rs:178-181 stating the grammar excludes the frame and the arm exists for in-process construction only, plus a `pump/tests.rs` test that constructs the frame. Acceptance: either the arm is gone and `Error::TerminalQuery`'s doc names only the local producer, or the arm carries the in-process-only rationale and a test fires it.
- **remote-proxy-tests-23** (nit; T132). ``src/tree/mirror/streaming/remote/proxy/tests/harness.rs:456-456``: `capacity.max(1)` in `harness::reconcile` silences an assert for an input no caller passes Resolution: Pass `capacity` through unchanged

## Named here, landed elsewhere

These rows belong to this lane's pattern and are counted in its roster, but another ruling's lane lands them; this lane verifies the state at base, quotes it in the report, and edits nothing at their sites.

- **remote-adapter-streams-19** (medium, T63): T63 (owner decision 18, the P6 `rumors::error` pass): `ReplyFrame`'s move and `ReplyFrameError`'s deletion land there; verify at base and do not touch `streams.rs:73-106`.

## Hazards and stops

- `p2-vanish-liveness` (unmerged) changes the proxy's production code
  (T145: the session observes the control stream while awaiting a data
  stream) in `work.rs`, `error.rs`, `streams.rs`, and edits
  `proxy/tests.rs`; `p1-proptest-ci` (unmerged) edits the proxy test
  files. Launch after both merge.
- `p1-harness-crate` (merged) rebuilt the proxy harness on
  `harness::drive` (T22); re-anchor every test site.
- T99 (`p4-placement`) renames the election enum: launch after it, or
  rebase and read the surviving name.
- remote-adapter-streams-19 and remote-proxy-3 are T63's (P6):
  `ReplyFrame`, `ReplyFrameError`, and the variant docs stay as they
  are here.
