<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as a lane brief derived from rulings T125, T128, and T132 in ../rulings.md; not authored, audited, or endorsed by Finch. -->

# P4 lane: one spelling per mechanism in the crate root, session, tree, link, and scaffolding

## Goal

In the crate root, the session and bookmark code, the message codec,
the tree, the link, the conformance suites, and the test scaffolding,
one mechanism has one spelling: duplicated halves fold onto one
function, one-caller wrappers and bypassed impls dissolve, guards that
recompute what the types establish go, the three observers share one
`Channel` state machine (T125), the two independence probes are one
probe over the stalled count (T128), and same-typed positional runs
take a name. Every change is behavior-preserving, judged by the
existing suites and the wire snapshots. Effort: high.

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
  run in the background redirected to a log under `<scratchpad>/p4-core-simplify/`,
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

Order by dependency, one commit per entry unless two share a helper:

1. **Tree** (after `p2-commit-path` merges): tree-core-7, tree-typed-8,
   -16, -18, -29, -31, -34, -35, inventory-7.
2. **Session and bookmark**: session-bookmark-8 with api-core-23 (both
   add `with_bookmark`), -11, -15, -24, -30, -47; api-core-33 with
   inventory-6's `unordered.rs` half (both reshape `Channel`).
3. **Link and conformance**: conformance-9, -11, -33, link-30;
   inventory-6's `router.rs` half after `p2-link` merges.
4. **Scaffolding and the rest**: testing-infra-10, -11,
   benches-envelope-23, tests-common-21.

Oracle: each row's own Acceptance grep, run in its commit; the union in
the closing commit: `grep -rn is_current src/peer tests/`,
`grep -rn 'fn dominance' src/tree/typed`, `grep -rn Persist src`
(prose only), `grep -rn private_bounds src`, `iter::from_fn` in
`node.rs`, `downcast` in `try_from_arc`,
`grep -c 'channel state present' src/rumors/`, all empty; one
`yield_once` in `src/`; `bounds_of` gone. Standing checks: the suites
each entry names and the `insta` snapshots (`just test`); a
behavior-preserving refactor has no negative control beyond them, so
the commit names, per entry, the test that fails if the refactor
changed behavior.

## Members

Heading entries quote Resolution and Acceptance verbatim from the topic document; nit rows quote their table row (site, issue, resolution). Each line opens with the entry's primary site at `9e5784fb`; the full record, with its related sites and evidence, is under `### <id>:` in the document the ledger's `doc` column names. Rulings are cited by number; T132 is the roster approval for every low and nit.

- **api-core-33** (medium; T125; T125 orders it with T74's observer registration (P6); if T74 has not landed at base, land it alone and report). `src/rumors/unordered.rs:197-226`. Resolution: Move the transitions onto the shared type: make `Channel<T>` own its optionality and give it `fn poll_receiver(&mut self, cx) -> Poll<Option<&mut watch::Receiver<Inner<T>>>>` (drives a `Waiting`, restores `Ready`, yields `None` once closed) and `fn wait(&mut self)` (moves `Ready(rx)` into the boxed `changed()` future); each observer's field becomes a plain `Channel<T>` and each `poll_next` becomes `let Some(rx) = ready!(this.channel.poll_receiver(cx)) else { return Poll::Ready(None) };`, the domain step, and `this.channel.wait()`. Delete `Changes::next_inner` and write its `try_next` as the siblings do. Host `Channel` in rumors.rs or a `channel` submodule rather than `pub(super)` inside unordered.rs. Acceptance: `grep -c 'channel state present' src/rumors/` and `grep -c 'matched Ready above' src/rumors/` both return 0; `next_inner` is gone; the observer suites and the doctests in unordered.rs and changes.rs still pass.
- **conformance-9** (medium; T128; its acceptance is the four probes and the two `Stalled` fixtures, all landed by T6 (`p1-conformance`, merged): re-anchor). `src/conformance/link.rs:465-678`. Resolution: Collapse to `async fn probe_independence<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A, stalled: usize)`. Sender: open `stalled` tagged streams into a `Vec`, `join_all` the pressure loops over them (a one-element `join_all` is the single case), open `STREAM_COUNT - stalled` live streams, one `select` with a single never-completing arm. Receiver: hold stalled receivers in a `Vec`, assert `held.len() == stalled` and `live_seen == STREAM_COUNT - stalled` (subsuming the single case's exactly-one assertion). `check_independence` calls it with `1` and `STALLED_COMPLEMENT` per direction; move `STALLED_COMPLEMENT` into the constant block; carry the two shapes' rationale as the parameter's doc plus one comment per call site. Acceptance: `check_independence` still runs four probes; `memory_link_conforms`, `one_byte_windows_conform`, `reordering_acceptor_passes_independence`, `never_binding_pooled_budget_conforms` pass; `shared_mux_coupling_is_caught` and `pooled_budget_below_the_bound_is_caught` still report `Err(Quiescence::Stalled)`.
- **api-core-23** (nit; T132). ``src/rumors.rs:62-77``: `Rumors` re-spells `Peer`'s field list and `Debug` body Resolution: Add a `pub(crate) fn with_bookmark` on `Peer` used by `Rumors::clone` and both `bookmark_inner` sites; share one `debug_fields` body between the two `Debug` impls
- **benches-envelope-23** (nit; T132). ``benches/support/latency.rs:405``: DelayedWire selects its cost model by a bool Resolution: Introduce `enum Clock { Paused, Running }`, store it, and match on it in `round_trip` and `round_trip_virtual`'s assert
- **conformance-11** (low; T132). `src/conformance/link.rs:680-696`. Resolution: One `pub(crate) async fn yield_once()` in a module both callers reach (src/link.rs is the natural host, gated `#[cfg(any(test, feature = "conformance", feature = "test-internals"))]`), with the doc stating the real constraint: `tokio::task::yield_now` needs tokio's `rt` feature, which the library build does not enable, and the deterministic driver is not a tokio runtime, so the helper self-wakes. Import it at both sites; `LossyAcceptor` at link/tests.rs:172 follows. Acceptance: one definition of `yield_once` in src/; both suites compile under `--no-default-features --features conformance` and under `test-internals` alone; no doc claims `yield_now` is runtime-bound.
- **conformance-33** (low; T132). `src/conformance/backend/tests.rs:260-263`. Resolution: Delete `bounds_of`; call `super::bound_bytes(&node)` at 255, 335, 351. In backend.rs, name the repeated max once (`fn bound_version_bytes(node: &impl Node) -> usize`) and use it at 349-351 and 523-526. Acceptance: one bound-bytes helper and one bound-max helper in the module; `materializing_backend_conforms` and its controls unchanged.
- **inventory-6** (low; T132; this lane lands the `unordered.rs` and `router.rs` halves; the `streams.rs` half rides remote-adapter-streams-20 in `p4-remote-simplify`). `src/rumors/unordered.rs:95-109`. Resolution: unordered.rs: have `open_pass` return `&mut Pass` via `self.pass.get_or_insert_with(..)` and use it at 210. streams.rs: store the open state as `Option<(FrameWrite<..>, Done<..>)>` and write `let (write, _) = self.state.insert(opened);` at 215-227. router.rs: `Entry::Vacant(vacancy) => { vacancy.insert(sender); break token; }` with `sender` moved directly. The observers' `Channel` and `finish` are a design option, not part of this finding: either accept the `None` arm as the "ended" state (a fused stream) or transition by cloning the `watch::Receiver` before building the wait, so `channel` needs no `Option` at all. Acceptance: the three cited panic sites are gone; the observer, streams, and routed suites pass unchanged.
- **inventory-7** (low; T132). `src/tree/typed/untyped.rs:280-283`. Resolution: Either delete both asserts (the differential tests and ingress errors carry the property), or check once at the typed entry (node.rs) with an O(1) probe such as first-versus-last ascent, and state in the message which caller the check exists for (a non-`Local` `Backend::assemble` override). Acceptance: no O(n) `debug_assert` runs inside the recursion; any remaining assert names the caller class it guards.
- **link-30** (nit; T132). ``src/link/routed/router.rs:188-202``: `route` exists only to discard `deliver`'s error and echo an id Resolution: Inline `deliver(..).map(move \
- **session-bookmark-11** (low; T132). `src/peer/gossip.rs:681-694`. Resolution: Make `Bookmarked::reclaim` return `bool` (true when it recorded, i.e. when the token was stale), folding `is_current` in as private. Both sites become `if let Some(party) = inner.party.as_mut() { persist = bookmark.reclaim(self.network, party, inner.tree.latest()); }` with no clone. Update the two prose references in tests/bookmark_when.rs to state the rule ("own region advanced or party changed") without the private name. Acceptance: `grep -rn is_current src/peer tests/` is empty; one place in the crate decides suppression; the two `.clone()` lines are gone; bookmark suites pass.
- **session-bookmark-15** (low; T132). `src/peer/gossip.rs:1186-1231`. Resolution: Keep `Reconciliation` as the bundle and add an admission variant (`Claimant`, `Network { remote, local, local_min_events }`, `None`); `bootstrap_erased` builds it with `Root::default()`, a default `Recorder`, and the claimant admission, then performs the mutual-bail epilogue and returns `Ok(None)` itself. Delete `bootstrap_reconcile`, its `#[allow(clippy::type_complexity)]`, and the `Option` in its return. Drop the `::<_, _>` at 1143 either way. Fix the struct doc to say it bundles the erased inputs and that the boundary is what pins codegen. Acceptance: one `Handshaking::start` pair in gossip.rs; bootstrap and gossip snapshot suites unchanged (the wire is untouched); `just gate` clean.
- **session-bookmark-24** (low; T132). `src/bookmark.rs:144-190`. Resolution: Replace the trait with two free fns in bookmark.rs, `read_record<B: Bookmark>(&B) -> impl Future<Output = Result<Record, BookmarkIo<B::Error>>> + Send` and `write_record<B: Bookmark>(&B, &Record) -> impl Future<..> + Send`, keeping the combinator shape (the `B: Sync` comment at 158-160 still applies); bound `Bookmarked<B: Bookmark>`, `impl<T, B: Bookmark> Peer<T, B>`, and `bookmark_inner<B: Bookmark>`; delete the `#[allow(private_bounds)]` and its comment. If finding 23 lands, the write fn shrinks further. Acceptance: `grep -rn Persist src` hits only prose (the word at gossip.rs:876 and bootstrap.rs:185); `grep -rn private_bounds src` is empty; `just gate` clean.
- **session-bookmark-30** (low; T132). `src/bookmark/format.rs:359-363`. Resolution: `let declared = usize::try_from(declared).map_err(|_| FormatError::Truncated { len: bytes.len() })?; let payload = reader.take(declared)?;`. `const INTEGRITY_HEAD: [u8; 2] = [0x58, HASH_LEN as u8];` with a `const { assert!(HASH_LEN < 256) }` beside it. Use `cbor::MAX_HEAD_LEN` in the capacity arithmetic. Acceptance: no `expect` in `unframe`; `truncation_at_every_prefix_is_rejected` and `framing_round_trips` pass; `grep -n '0x20\|9 + 2 + 9' src/bookmark/format.rs` empty.
- **session-bookmark-47** (low; T132). `src/message.rs:375-392`. Resolution: In `try_from_arc`: `let decoded: T = match decode_exact::<T>(&serialized, limit) { Ok(v) => v, Err(PayloadDecodeError::Depth(limit)) => return Err(EncodeError::Depth { limit }), Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Roundtrip(source)) }; if decoded != *arc { return Err(EncodeError::Unfaithful) }`, and reword the comment: `decode_exact` is the one parse ingress runs (the deserializer wraps it in the `Arc`). Move `deserializer`'s inner fn into `PayloadCodec::new` beside `serialize_payload` and delete `Message::deserializer`. Make `from_slice` call `Self::from_bytes::<T>(Bytes::copy_from_slice(bytes), limit)`. Acceptance: no `downcast` in `try_from_arc`; `Message::deserializer` is gone; message/tests.rs (including `from_bytes_matches_from_slice`, `try_new_admits_exactly_the_limit`, `try_new_prices_an_enums_own_decode`, `a_type_that_cannot_read_its_own_output_fails_admission`, `codec_serializes_through_the_carried_limit`) pass unchanged. Optional meter in the style of tests/encode_alloc.rs: allocations of `Message::try_new(())` drop by one.
- **session-bookmark-8** (low; T132; shares `with_bookmark` with api-core-23: one commit). `src/peer/gossip.rs:344-399`. Resolution: Add `with_bookmark`; `bookmark_inner` becomes `let peer = self.with_bookmark(bookmark); if pristine { return Ok(peer) } match peer.bookmark_record().await { Ok(()) => Ok(peer), Err(error) => Err(Unbookmarked { peer: peer.with_bookmark(NoBookmark), error }) }`. Acceptance: `bookmark_inner` contains no field list; gate clean.
- **testing-infra-10** (nit; T132). ``src/testing/transport.rs:328-328``: A `debug_assert` that restates tokio's `ReadBuf` contract, and an assertion of unit against unit Resolution: Delete line 328 and the `before` binding at 309
- **testing-infra-11** (low; T132; `src/testing/transport.rs` is edited by `p2-vanish-liveness` (unmerged): after its merge). `src/testing/transport.rs:451-475`. Resolution: Add `State::new(side, plan)` and a `wrap_read` twin of `wrap_write`; have `wrap_link` and the acceptor use them. Either route `wrap_link`'s control halves through `wrap_io` (sharing the state `Arc` with the connector and acceptor wrappers) or narrow `wrap_io` to `pub(super)` and drop it from the facade, re-pointing its two tests at `wrap_link` over `link::memory()`. Add `AdversarialLink` to the `pub use transport::{...}` list. Acceptance: one `State` constructor; `wrap_io` has a non-test caller or is not exported; `rumors::testing::AdversarialLink` is importable.
- **tests-common-21** (nit; T132). ``tests/common/schedule/events.rs:3-6``: `EventIdx` is a synonym for the same `usize` that indexes peers Resolution: Make `EventIdx` a `Copy` newtype over `usize` (and `PeerIdx` if wanted) with `Display` so counterexamples still read
- **tree-core-7** (low; T132). `src/tree.rs:273-278`. Resolution: `Node::root_hash(&self.root.root).into()`; delete the `From<Root> for Option<typed::node::Root>` impl. Acceptance: no `.into()` on a `tree::Root` remains in the crate; `hash()` performs no clone; `empty_tree_hash_matches_reference` and the `root_hash_read_meter_is_live` pins in `crate::tests` still pass (the meter call precedes the read).
- **tree-typed-16** (low; T132). `src/tree/typed/node.rs:301-308`. Resolution: replace lines 304-305 with `let path = <[u8; 32]>::from(path);`, and spell the impl directly (`value.hash.into_inner()`) rather than routing through `Path`. The second `Vec` is worth removing only if a run-heavy bench moves; it is small next to the run's node allocations. Acceptance: no `try_from` in `from_sorted_leaves`; `From<Prefix> for [u8; 32]` has a caller; `every_virtual_level_hashes_canonically` and the backend/local suites pass unchanged.
- **tree-typed-18** (nit; T132). ``src/tree/typed/node.rs:409-417``: `root_hash` takes `&Option<Root>` where its siblings take `Option<&Self>` Resolution: `pub fn root_hash(node: Option<&Root>) -> Hash`
- **tree-typed-29** (low; T132). `src/tree/typed/untyped.rs:535-557`. Resolution: delete both `dominance` methods; change unknown.rs:48 to `match node.span().dominance(known)`. The one rumors-specific sentence (routing through `span` pays a leaf one decode per stream) moves onto `span`'s doc. Acceptance: `grep -rn 'fn dominance' src/tree/typed` is empty; both `unknown` modules call `span().dominance`.
- **tree-typed-31** (nit; T132). ``src/tree/typed/untyped/fan.rs:203-226``: `Fan::iter` hand-rolls a named `Iter<'a>` nothing outside fan.rs names, while `values` returns `impl Trait` Resolution: Return `impl DoubleEndedIterator + ExactSizeIterator` from `Fan::iter` and delete the named `Iter<'a>` and its three impls
- **tree-typed-34** (nit; T132). ``src/tree/typed/untyped/iter.rs:311-312``: The owned walk's cursor is a `u16` with a 256 sentinel and two `as` casts where `Option<u8>` carries the same state Resolution: `next: Option<u8>` initialised `Some(0)`
- **tree-typed-35** (low; T132). `src/tree/typed/untyped/iter.rs:393-395`. Resolution: `impl<P: Polarity> Iterator for RangeOwned<P> { type Item = ([u8; 32], Leaf); fn next(&mut self) -> Option<Self::Item> { /* body */ } }`, delete the inherent method, and reduce `Node::leaves` to `RangeOwned::within(...).map(|(key, leaf)| (Prefix::from(key), Node::from_untyped(leaf.into_node())))`. Acceptance: `iter::from_fn` is gone from node.rs; causal.rs and unordered.rs compile unchanged (method syntax resolves to the trait); `range_and_freeze_match_the_naive_filter` passes.
- **tree-typed-8** (nit; T132). ``src/tree/typed/hash.rs:224-240``: `Hash`'s bytes are reachable three ways, and `From<[u8; MERKLE_HASH_LEN]> for Hash` has no caller Resolution: Either make `Hash`'s field private and construct through `Hash::from`, or keep the field and delete the unused `From` impl

## Hazards and stops

- `p2-commit-path` (unmerged) rewrites `tree.rs`, `traverse/*`,
  `node.rs`, `batch.rs`, `gossip.rs`; `p2-peer` (brief exists, not
  launched) rewrites `peer.rs`, `gossip.rs`, `bookmark.rs`, `batch.rs`;
  `p2-link` (unmerged) edits `router.rs`. Launch after all three merge.
- inventory-6's `streams.rs` half is `p4-remote-simplify`'s
  (remote-adapter-streams-20 reshapes the same state).
- conformance-9 must keep `check_independence` at four probes and the
  two known-bad fixtures `Stalled` (its Acceptance); T6 landed the
  suite's repairs, so re-anchor.
- session-bookmark-11's `is_current` fold changes when a bookmark
  persists: the bookmark suites are the check; any observable change
  is a stop.
