# Simplification, modularity, and vestigial code

This document collects every finalized finding whose primary class is simplification, modularity, vestigial, or idiom: what in the `rumors` crate at commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764 could be simpler, and how. It covers `src/`, `tests/`, `benches/`, `examples/`, `Cargo.toml`, and the verification recipes (`justfile`, `tools/`); the sibling crates under `crates/` appear only where rumors' use of them is the issue. Ids are `<partition or sweep key>-<n>`; the full record for each, including the lens reports it was distilled from, is in `evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md` beside this file. Severity follows the finalizers' scale: high (a defect that must be fixed before release), medium (a structural cost worth scheduling), low (a local improvement with a named payoff), nit (a legibility or consistency point). Provenance is stated per finding: *verified* means the finalizer ran a grep, a script, or a read-only git query, or read the cited lines against the tree; *assessed* means the claim rests on reading alone; *demonstrated* means a constructed test. One constructed demonstration in `witness/results.md` bears on these four classes: the second witness pass ran mirror-common-3's arm swap, and that entry is marked demonstrated inline; no other entry here is. The four classes hold 358 entries (37 medium, 154 low, 167 nit, no high; the finalizers filed 363, and five cross-lens duplicates stand as cross-references under their entries of record), so the per-module sections reproduce the medium and low entries in the finalizers' template and list the nits in a compact table per module, pointing at the evidence file for the full record. Where two finalizers reached the same defect from different partitions, both entries stand and each carries a "See also" line, except where one carried no evidence the other lacked: inventory-2, mirror-common-22, materialized-10, inventory-18, and suite-economics-11 are reduced to a cross-reference under their own heading or table row and are not counted; where finalizers disagree on the fix, a synthesis note says how. An id cited inside an entry that has no heading or table row here (for example api-core-25 or remote-proxy-tests-24) belongs to another class and is documented in its own file of this note. Four anchors were corrected against the tree at this commit and are listed under Notes in the structured return; every other anchor and quotation was mechanically checked against the file it names (a script compared 2,747 quoted evidence lines with the tree, and the residue was read by hand); the anchor pass that followed corrected no line in this document.

## Highest-value items

1. Close the streaming core's four-module cycle by moving two misplaced items and making `channel.rs` export a stream-typed receiver: `children_of` belongs beside its only consumer in `erased::ops`, `SupplyLedger` is neutral session state, and the `cfg(test)` receiver-as-stream forks in `erased.rs` and `materialized/common.rs` collapse to one definition in the module that owns the channel swap, after which `common.rs` dissolves and `tokio-stream` can leave the manifest (module-graph-2).
2. Give `tests/common` the drivers every suite re-implements: generalize `gossip_pair_async`, `bootstrap_fork_configured`, and `sim::quiesce` over `B: Bookmark`, add bootstrap-serving and retire-into drivers, and route the measurement suites through the closed-world poller; today ten copies of the bootstrap fork and four of the retire session run without the stall detector or the control-drain check the shared drivers enforce (tests-bookmark-3).
3. Sweep the height-erasure residue out of the adapter: `Encoded<Q>`, `Decoded<E, Q>`, `decode<.., Q, N>`, and `render<.., D>` have one instantiation each, and the scope-derivation rule is spelled four times; one `Level` argument and one derivation function replace them, and the same pass retires the erasure leftovers in `tree.rs`, `frame.rs`, `backend.rs`, and `height.rs` (remote-adapter-streams-5).
4. Replace `read_early`'s second copy of the frame grammar with `read_reply` over an empty root scope, which already yields every rejection the early stream pins, as the encode side already does for early supplies (remote-adapter-streams-3).
5. Let the proxy defer to the driver's election: `connected()` re-derives equality and the initiator role that `descend` already decided, then reconciles the two with an enum, two `unreachable!`s, and four `debug_assert_eq!`s; a plain `Connected` struct whose `initiator()`/`responder()` build the session deletes all of them, and the shared post-exchange tail of `complete_connect` and `accept` folds into one method (remote-proxy-7).
6. Make `gossip_inner`'s early returns `?`-able: nine of its eleven `(Intent, Err(..))` tuples are `Intent::Remain` by construction, and a module-private `Aborted<B>` with `From` impls leaves the two post-hand-off sites as the only places a non-`Remain` outcome can arise, which is where the identity-safety argument lives (session-bookmark-10).
7. Reduce `Extant` to one counter and the three observers to one `Channel` state machine: tokio's `watch::Sender` close is already the quiescence event `try_into_peer` waits for, and the `Waiting`/`Ready` transitions copied into three `poll_next` bodies plus `Changes::next_inner` are one pair of methods on the shared type (api-core-22, with api-core-33).
8. Drop `static_assertions` from `[dependencies]` and make `forbid(unsafe_code)` unconditional: its eight uses sit in one `cfg(test)` module, only `assert_eq_size_val!` expands to `unsafe`, and that test restates what `zero_size` asserts; `const _: () = assert!(..)` already exists in the same file (deps-1).
9. Register `absorb` as a `Work` task so `complete_initiator` stops hand-rolling `tasks::complete`, and replace `internal_walk`'s two `Option<oneshot>` plus two `Option<BTreeMap>` hand-off with one `Opening` enum consumed at stream start (materialized-13, materialized-34).
10. Give the routing table a type: `Table<C>` with `claim`, `route`, and `revoke` spells the token-claim critical section once, deletes two `type_complexity` allows, and makes the "single map operation" premise of `entries`' doc true (link-26).
11. Break the top-level cycle into `error`: `party.rs` should define a local error lifted by `From`, as `handshake.rs` already does, and `link::begin` should return a unit `Poisoned` error mapped at its two gossip.rs callers (module-graph-1).
12. Rule once on the conventions that block mechanical sweeps: `unreachable_pub` (or a recorded pub-in-private convention), a `tools/` check for em-dashes in `//` comments, import grouping, the placement of inline `mod tests {}` blocks, and which layer of `type_complexity` allow to keep; each of these appears in six or more partitions and every finalizer declined to fix it piecemeal (inventory-10).

## Crate-wide patterns

These are the issues that recur across partitions. Each is stated once here with its full site list; the per-module entries below carry the details and do not repeat the pattern.

**Retirement and erasure residue.** Three removals left machinery and prose behind. The V1 protocol retirement (368da2a5) left a `Default` derive nothing calls (api-core-21), a `V2` series label that distinguishes one dialect and two unreachable latency-group arms (benches-envelope-2), two-protocol dispatch prose in `gossip.rs` (session-bookmark-12), a shapes-by-protocol grid with one column (tests-disruption-handshake-27), a vacuous `Protocol::V2` assertion (tests-observation-13), a dead `Protocol` import (tests-wire-format-28, tests-common-10), `LengthOverflow` housed in a module that no longer writes a length header (mirror-common-6), a `[u8; 6]` that was `LEGACY_MAGIC.len()` (mirror-common-11), the node-codec comment block in `node.rs` and the `Levels` chain named in `traverse.rs` and `tests/future_size.rs` (tree-typed-19, prose-hygiene-1), and a V1 skip branch in `tools/digestshare` (verification-infra-11). The payload and height erasures left manual impls that no longer avoid a bound (remote-codec-22, streaming-backend-window-5, tree-core-4, tree-typed-24), `Height` supertraits that make the hand-rolled impls' rationale false (tree-typed-11), two aliases for one erased type (remote-adapter-tests-19), twelve redundant `+ Sync` bounds (materialized-12), `#[repr(C)]` on phantom markers (tree-typed-10), and single-instantiation type parameters with four copies of one rule in the adapter (remote-adapter-streams-5). The audit that Principle 3 asks for at each phase boundary did not run after these commits; the entries above are that audit.

**The channel-type boundary leaks `cfg` forks into its consumers.** `channel.rs` swaps tokio's `Receiver` for the instrumented one under `cfg(test)` but exports no stream-typed receiver, so `erased.rs` and `materialized/common.rs` each re-derive the receiver-as-stream split, `assembly.rs` imports `tokio_stream::StreamExt` for methods `futures` provides, `decode.rs` builds two raw tokio channels outside the instrumented path, and `local.rs` carries four adversarial forks (materialized-16, mirror-common-25, deps-2, module-graph-3, module-graph-2). One `ReceiverStream<T>` alias and one `into_stream` in `channel.rs` remove every consumer-side fork; the fuller form wraps the production receiver so both variants implement `Stream` and `tokio-stream` leaves the manifest.

**Test-harness helpers re-spelled per binary.** The bootstrap fork appears in ten binaries and `send_random` in seven (tests-resource-link-window-22, tests-disruption-handshake-31); the bootstrap-serving and retire-into sessions in ten (tests-lifecycle-3, tests-observation-32); bookmarked drivers in four suites because `common::wire` is fixed at `NoBookmark` (tests-bookmark-3); the `(hash, latest)` fingerprint at twelve sites and the quiesce loop at three (tests-common-18, tests-lifecycle-14); `seeded` in four to ten copies (tests-lifecycle-8, tests-wire-format-2); `pair()`/`seeded_pair` in three (tests-lifecycle-6, tests-disruption-handshake-12); observer `step`/`drain`/`live_map` in two suites plus `sim.rs` (tests-observation-2); `corpora` and `gossip_pair` copied verbatim (tests-observation-12); `LINK_BUF` declared ten times under two conventions, 8 KiB in `common::wire` and 64 KiB per suite, with rationales that point at each other (tests-bookmark-2, tests-observation-32); three capture-line parsers (tests-resource-link-window-15); two allocator scaffolds (tests-resource-link-window-5); three observer recorders (tests-wire-format-23); two `FixtureTree`s (tests-wire-format-21); a hand-rolled child-plan codec (tests-disruption-handshake-2). In-crate test tiers show the same shape: the two-proxy topology seven times and the error projection seven times (remote-proxy-tests-5, remote-proxy-tests-16), the two-`Local` session nine times behind a six-function ladder (streaming-tests-3), fixture machinery and the three ingress premises repeated across the adapter suites (remote-adapter-tests-10, remote-adapter-tests-2, remote-adapter-tests-20), a third byte-budget fuse in `src/tests.rs` (testing-infra-20, tests-common-3), and `party_of`/`with_messages` twins (testing-infra-17). Two root causes recur in the finalizers' history notes: the shared drivers landed after the suites and were never folded in, and the measurement suites avoid `mod common;` because its blanket allow and the sim engine's compile weight come with it (suite-economics-8, tests-common-8, tests-disruption-handshake-28, benches-envelope-22).

**`pub` inside the private `tree` module carries no information.** `mod tree;` is private, so `pub` and `pub(crate)` mean the same reach for roughly a hundred items, and the subtree mixes the two spellings with no rule (inventory-10, module-graph-12, tree-typed-1, streaming-backend-window-2, materialized-6, mirror-common-20, remote-codec-21). Smaller instances: `pub(crate)` fields on a module-private struct (session-bookmark-17, inventory-19), six `pub(crate)` items referenced only from their own file (inventory-14), `pub` items with in-file callers (mirror-common-20, tree-core-25), and `pub(super)` on a constant with no sibling user (link-22). One ruling settles all of them: enable `#![warn(unreachable_pub)]` and follow the diagnostics, or record the pub-in-private convention in AGENTS.md and reconcile typed.rs:19-22 with it.

**Inline module bodies against the sibling-file convention.** Six `#[cfg(test)] mod tests {` blocks remain, one named `test` (module-graph-6, suite-economics-11, mirror-common-22, streaming-backend-window-14, testing-infra-5, tree-core-25), and five inline production modules of 22 to 137 lines sit in implementation files (module-graph-14: `tree::meter`, `tree::panic_injection`, `erased::ops`, `untyped::census`, `decode::fan_probe`, `conformance::backend::ledger`). Nothing mechanical checks placement. Four of the six test blocks are in test-only scaffolding files, which is the one open question here (whether AGENTS.md should exempt them); the other two are pure moves.

**Import hygiene left by three mechanical sweeps.** The serde sweep (c6fe4018) left a trailing `use serde::..` pair glued to the next item's doc comment at 22 sites (tests-wire-format-10; also session-bookmark-2, tests-common-1, tests-lifecycle-12, tests-observation-21, tests-resource-link-window-17, tests-disruption-handshake-29, streaming-tests-21, remote-capture-atlas-30); the codec-threading commit (4356e197) inserted `use crate::message::PayloadCodec` ahead of the `std` group in the adapter, proxy, and codec test files (remote-adapter-tests-5, remote-proxy-tests-3, remote-proxy-11); and qualified paths stand beside existing imports throughout (api-core-31, materialized-9, mirror-common-7, mirror-common-18, remote-adapter-streams-18, remote-capture-atlas-26, remote-codec-13, session-bookmark-41, swarm-example-25, testing-infra-3, tests-common-26, tests-wire-format-24, tree-core-19, tree-typed-4, tree-typed-15, clippy-pedantic-5, clippy-pedantic-15, inventory-15, module-graph-10). rustfmt sorts within a group and never merges groups, so these persist until someone merges them by hand; a `rustfmt.toml` with `group_imports = "StdExternalCrate"` would keep them merged, at the cost that the option is unstable and `fmt-check` would then run on the pinned nightly.

**Em-dashes in `//` comments.** About 116 line comments under `src/` and 43 under `tests/` use a true em-dash where the owner's doctrine asks for a colon, a semicolon, or a spaced double-hyphen (remote-proxy-5, streaming-tests-4, testing-infra-14, tests-bookmark-10, tests-disruption-handshake-13, tests-observation-33, tests-resource-link-window-23, tests-wire-format-13; two assert messages carry one as well). Every finalizer reached the same disposition: one mechanical sweep plus a `tools/` check, never a per-partition edit.

**Hand-maintained widths and counts.** The 32-byte path width is a literal at every arithmetic site under `typed/` and is named independently three times elsewhere (`KEY_DEPTH`, `HASH_LEN`, `MAX_ARBITRARY_SUFFIX_LEN`), and `Root` is a hand-counted 32-deep `S<` chain with no tie to `Root::HEIGHT` (tree-typed-12, inventory-16, streaming-backend-window-25). The stream count 17 is a literal in two layers held equal by a pin test that compares literals (link-3, remote-codec-3), and appears bare in tests (remote-proxy-tests-17). The fan 256 recurs as a literal beside `window::FAN` and as `MAX_QUERY_CHILDREN` (streaming-tests-15, remote-codec-3, tests-resource-link-window-21, remote-capture-atlas-23). `62_500` is defined three times (testing-infra-2); `SCOPE_FIXED_BYTES` and `LEAF_REQUEST_BYTES` are hand-counted beside siblings derived from `size_of` (streaming-backend-window-26); `BERNSTEIN_TAIL` is a literal `tail_exponent(0)` computes (streaming-backend-window-34); `[u8; 6]` and seven literal `16`s spell widths that have names (mirror-common-11). Smaller: conformance-13, materialized-40, remote-capture-atlas-11, remote-proxy-21, swarm-example-12, tests-disruption-handshake-19.

**Guards and asserts that recompute what types or committed tests already establish.** A `TryFrom` that can never fail leaves a public error variant no session can produce (remote-adapter-streams-19); `Preamble::decode` takes a slice where every caller holds the fixed array and pays with two defensive `PreambleDefect` variants (mirror-common-10); a four-point monotonicity `debug_assert` duplicates the conformance sweep (streaming-backend-window-29); `Window::capacity` clamps a height only programmer error can produce (streaming-backend-window-31); `unframe` pre-checks the bound `Reader::take` enforces (session-bookmark-30); `try_from(..).expect(..)` converts a `Prefix<Z>` an infallible `From` already converts (tree-typed-16, remote-adapter-streams-9); a `>=` whose `>` half `LeafOrder` fires first (remote-adapter-streams-10); a `capacity.max(1)` that silences an assert no caller reaches (remote-proxy-tests-23); a `TerminalQuery` arm the placement grammar excludes (remote-proxy-29); a straggler drain the function's own proof makes unreachable (swarm-example-14); a `debug_assert` restating tokio's `ReadBuf` contract and an `assert_eq!(sent, ())` (testing-infra-10); asserts implied by the checks before them (tests-bookmark-14, streaming-tests-14); O(n) `debug_assert`s re-run at every recursion level of `from_sorted_leaves` (inventory-7); a greeting parser that routes a fixed roster through six `Option`s and eight panic sites (inventory-5, remote-codec-27); take-and-restore `Option`s whose panics have panic-free std spellings (inventory-6, remote-adapter-streams-20, remote-adapter-streams-24, api-core-33); and a second election reconciled with the first by assertion (remote-proxy-7).

**Duplicated halves of one mechanism in production.** `accept` rebuilds what `connect` followed by `complete_connect` produces, and `initiator`/`responder` share an eleven-line prelude (materialized-11); `complete_connect` and `accept` duplicate the post-exchange tail in the proxy (remote-proxy-4); `read_early` copies `read_reply` (remote-adapter-streams-3); three decode pumps repeat one prelude and the leaf and terminal pumps are one loop (remote-proxy-24, remote-proxy-28); `render` repeats its flush block four times (remote-adapter-streams-12); the two independence probes are one probe over the stalled count (conformance-9); `yield_once` exists twice with a false rationale (conformance-11); `bounds_of` duplicates `bound_bytes` (conformance-33); the reclaim-if-stale block and the two reconciliation drivers are copied in `gossip.rs` (session-bookmark-11, session-bookmark-15); `Peer` is rebuilt field by field at three sites (session-bookmark-8, api-core-23); the async reader and the sync oracle share fragments by copy (remote-codec-8); the erase-and-box of a request stream is written ten times under two aliases (remote-proxy-23); the poll-delay scheduler exists in two copies and its clamp in three (streaming-backend-window-13, streaming-tests-12); the frame-to-signal projection is written three times (remote-capture-atlas-29); the boxing rationale is pasted three times (mirror-common-32); `node_bytes` has an inherent and a trait definition (streaming-backend-window-8); and the walk's and the proxy's `pump` diverge on a closed receiver without saying why (module-graph-13).

**One-caller wrappers, bypassed impls, and shims.** `From<Root> for Option<..>` serves one line that already holds the field (tree-core-7, tree-typed-18); `From<[u8; MERKLE_HASH_LEN]> for Hash` has no caller (tree-typed-8); `dominance` is two delegates for one call site (tree-typed-29); `Fan::iter` hand-rolls a named iterator beside an `impl Trait` sibling (tree-typed-31); `RangeOwned` has `next` but no `Iterator` impl, so `Node::leaves` wraps it in `iter::from_fn` (tree-typed-35); `render_embedded` fixes one argument of `render_embedded_as` (remote-capture-atlas-16); `route` discards `deliver`'s error and echoes an id (link-30); the `Persist` trait has one blanket implementor and exists to be allowed past `private_bounds` (session-bookmark-24); `try_from_arc` erases and downcasts a value it could keep typed (session-bookmark-47); a pass-through generator relays `fold_parents` (streaming-backend-window-20); `materialized/channel.rs` re-exports six names from one module and `remote.rs` carries the crate's only glob re-export (module-graph-8, remote-capture-atlas-5); `codec_stream_count` wraps a constant reachable by name (remote-capture-atlas-4); `wrap_io` is exported with no consumer (testing-infra-11); `Handshaken` keeps a cloned `Greeting` to read two scalars and the descent future is boxed twice (mirror-common-16, mirror-common-17); `futures-util` is fully shadowed by `futures` (deps-4).

**Same-typed positional runs where a named bundle or newtype would carry the meaning.** The peer's decode premises travel as three loose parameters through six signatures and two structs and are re-spelled forty times in the adapter tests (remote-adapter-streams-4, remote-adapter-tests-2, remote-proxy-24); the proxy threads nine to eleven positional arguments through four lists under four `too_many_arguments` allows (remote-proxy-4); `from_budget` takes four `u64`s where a length and a version-bytes bound can be swapped silently (streaming-backend-window-28); the election key is spelled three ways (mirror-common-16); `Resolver::react` returns an undocumented four-tuple (materialized-36); `SupplyLedger::charge` reports its overdraw as a bare `u64` (materialized-4); `Entry::Complete` carries three unlabeled fields (remote-codec-21); a `bool` selects the clock model and the adjustment direction (benches-envelope-23, swarm-example-24); a `u16` with a 256 sentinel stands in for `Option<u8>` (tree-typed-34); `EventIdx` is a synonym for the `usize` that also indexes peers (tests-common-21); `PipeId` keys by `char` and `bool` (tests-disruption-handshake-29); the supply-failure slot is spelled twice with two lock sites (remote-adapter-streams-25).

**Lint attributes that guard nothing.** Twelve per-item `type_complexity` allows and `protocol.rs`'s inner allow sit under the module-wide allow at streaming.rs:43 (inventory-3, materialized-8, mirror-common-29); six `#[cfg(test)]` gates sit under parents already gated (module-graph-11, materialized-8); thirteen identical `missing_const_for_thread_local` allows carry the same four-line comment in seven files (inventory-13); a `too_many_arguments` allow sits on a seven-parameter function the lint does not flag (inventory-12); four item-level `dead_code` allows in `latency.rs` exist because one includer omits the module-level allow (benches-envelope-22, tests-disruption-handshake-28). The blanket `#![allow(dead_code, unused_imports)]` on `tests/common` hides two dead imports and a no-op statement, and its stated reason justifies only the `dead_code` half (tests-common-10, tests-wire-format-28).

**Braces left by the `send_all` rewrite.** Commit 212c6914 replaced `.batch(|batch| ..)` closures with single `send_all` statements inside the braces that once scoped a `Batch` guard, and the braces now scope nothing at conformance/link/tests.rs, peer/gossip/tests.rs, tests/disruption.rs, tests/hop_trace.rs, tests/common/sim.rs, tests/sanity.rs, tests/listen.rs, tests/session_stats.rs, tests/stale_floor.rs, tests/gossip_snapshot.rs, and tests/routed_link.rs (conformance-21, session-bookmark-19, tests-disruption-handshake-6, tests-lifecycle-13, tests-observation-16, tests-wire-format-8, tests-common-10, tests-resource-link-window-13). One mechanical pass.

**Manual impls identical to what `derive` emits.** conformance-15, link-19, tests-common-5, and the erasure cases above (remote-codec-22, streaming-backend-window-5, tree-core-4, tree-typed-24). The crate's deliberate manual impls carry their reason in place (`bootstrap.rs:81-83`, `path.rs:61`, `prefix.rs:202`); a manual impl without one sends the reader looking for a bound that is not there. tree-typed-11 shows that two of those recorded reasons are themselves false, since `Height` already implies `Debug + Clone + Default`.

**Missing common-trait impls on public types.** Twelve public types have no `Debug` (api-audit-12, link-5); `Gossiped` lacks `PartialEq`; `NoBookmark` derives only `Debug` (session-bookmark-3); the observer identity types lack `Hash`/`Ord` (session-bookmark-39); `Bootstrap`'s `Debug` omits `observe` (api-core-19); `EndpointError` and `LinkError` lack `#[non_exhaustive]` and no crate-wide rule says which enums are open (link-17). `#![warn(missing_debug_implementations)]` would keep the first class closed.

**Clippy pedantic and nursery stragglers with zero false positives in the run.** Range-checked `as` narrowings where `try_from` carries the check (clippy-pedantic-1, inventory-18), `&mut` receivers that never mutate (clippy-pedantic-2), single-use named lifetimes (clippy-pedantic-3), closures wrapping one method call (clippy-pedantic-4), two `match` spellings of `let ... else` against 107 sites of the idiom (clippy-pedantic-9), a wildcard arm over a two-variant crate enum (clippy-pedantic-10), and small test-side items (clippy-pedantic-6, -7, -12, -13, -14, -15). The sweep's open question is whether to adopt four of these lints in a `[lints.clippy]` table so they ride the existing `-D warnings` gate.

## Crate root and public surface

Files: src/lib.rs, peer.rs, rumors.rs and its observers, batch.rs, snapshot.rs, network.rs, tags.rs, protocol.rs, error.rs, tutorial.rs. Entries: 18 (3 medium, 8 low, 7 nit).

### api-core-22: `Extant` keeps two counters of one quantity; the `watch::Sender` clone count alone carries the protocol
- Where: src/rumors.rs:39-60 (related: src/rumors.rs:93-105, src/rumors.rs:108-131)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read the vendored tokio 1.52.3 watch.rs at the version Cargo.lock pins: `impl<T> Clone for Sender<T>` bumps `ref_count_tx` (201-209); `Drop for Sender` sets closed and notifies waiters when the last clone drops (1458-1465); `changed_impl` registers `notified()` before checking state (989-999), so no wake is lost; `maybe_changed` returns `Err` iff the version is unchanged and the closed bit is set (961-981); `subscribe` marks the current version seen (1387-1394). The redesign was not compiled or run.)
- Seen by: structure; refutation: confirmed; history: no rationale found (the two-counter design arrived whole in cb69fc951; its predecessor 36df73797 used the watch channel's own close as the quiescence signal, so the single-counter form has precedent in this tree)
- Owner-gated: no

`Extant` counts extant handles with an `Arc<()>` strong count and separately wakes reuniters through a cloned `watch::Sender<()>`, whose clone count is also the number of extant handles (each `Rumors` holds one `Extant`, and `#[derive(Clone)]` clones both fields). The redundancy is what forces the `Option` around the token (so `Drop` can shed it before the wake), the hand-written `Drop`, and the strong-count re-check loop in `try_into_peer_inner`. tokio's `watch` already emits the needed event: `Receiver::changed()` returns `Err` exactly when every `Sender` clone has dropped and the current value is seen, and nothing here ever sends a value, so the close is the one event a receiver can observe. tokio's `Sender::drop` decrements its count and closes the channel atomically with respect to receivers, so the ordering hazard the field docs describe cannot arise. Every piece exists to reconcile two counters that must agree; with one counter they dissolve.

Evidence:

    39	#[derive(Clone)]
    40	struct Extant {
    41	    /// The extancy token. An `Option` only so [`Drop`] can shed it *before*
    42	    /// waking waiters on `drops`: a reuniter woken by that send must already
    43	    /// observe the decremented strong count. Always `Some` outside `Drop`.
    44	    token: Option<Arc<()>>,

    54	impl Drop for Extant {
    55	    fn drop(&mut self) {
    56	        // Shed the token first, then wake: see the field docs above.
    57	        self.token = None;
    58	        self.drops.send_replace(());
    59	    }
    60	}

Resolution: `struct Extant { alive: watch::Sender<()>, claimed: Arc<AtomicBool> }` with `#[derive(Clone)]` and no `Drop`; `Rumors::new` builds `alive: watch::Sender::new(())`. `try_into_peer_inner` becomes: subscribe a receiver, clone `claimed`, drop the `Extant`, then `while alive.changed().await.is_ok() {}` (the `Err` means every sender is gone, and the closed state is monotone because a new `Sender` needs a live `Rumors` to clone from), then the existing `compare_exchange` claim. Keep the monotonicity and exactly-once prose; drop the token and wake-ordering prose. Land api-core-25's tests first so the redesign is exercised. Acceptance: `Extant` has two fields and no `Drop` impl; `Arc<()>` and `strong_count` no longer appear in rumors.rs; the reunion tests, including the concurrent-reuniter cases api-core-25 adds, pass.

### api-core-33: Three observers re-spell one `Channel` state machine; `Changes` implements it twice
- Where: src/rumors/unordered.rs:197-226 (related: src/rumors/unordered.rs:40, 69-74; src/rumors/causal.rs:41, 165-174, 187-193; src/rumors/changes.rs:48, 75-104, 112-122, 133-145, 156-162)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read all three `poll_next` bodies; grep: `channel state present` at four sites, `matched Ready above` at three; `next_inner` is defined at changes.rs:76 and called only at changes.rs:117)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: no rationale found for the duplication (`Channel` was shared at aa321d2ba but its transitions stayed inline; cd7c09db3 rewrote both message observers without consolidating and did not touch changes.rs); `next_inner` is deliberate-but-expired (it was the engine of a `Changes<T, Blocking>: Iterator` impl deleted at 83edcd944)
- Owner-gated: no

The `Waiting` arm (poll the boxed wait, restore `Ready`, end on `closed`) and the enter-owned-wait block (`take()` the channel, `unreachable!` on the impossible variant, `Box::pin` the `changed()` future) are copied into all three `poll_next` bodies, each with its own `expect("channel state present")` and `unreachable!("matched Ready above")`. `Changes` additionally carries a second, `async fn` implementation of its whole state machine (`next_inner`) whose only caller is `try_next`, whose doc claims to share the `Stream` state machine, and which awaits `rx.changed()` by borrow where `poll_next` materializes the owned wait; its siblings implement `try_next` as `self.next().now_or_never()`. A fix to the wait discipline must land in three places plus `next_inner`; the seven `expect`/`unreachable!` sites exist only because each observer re-implements the transition around an `Option` it owns solely to `take()` from; and each `poll_next` should read as its domain step, not channel plumbing.

Evidence:

   197	            match this.channel.as_mut().expect("channel state present") {
   198	                Channel::Waiting(wait) => match wait.as_mut().poll(cx) {
   199	                    Poll::Pending => return Poll::Pending,
   200	                    Poll::Ready((closed, rx)) => {
   201	                        this.channel = Some(Channel::Ready(rx));
   202	                        if closed {
   203	                            return Poll::Ready(None);
   204	                        }
   205	                    }
   206	                },

   220	                    let Some(Channel::Ready(mut rx)) = this.channel.take() else {
   221	                        unreachable!("matched Ready above");
   222	                    };
   223	                    this.channel = Some(Channel::Waiting(Box::pin(async move {
   224	                        let closed = rx.changed().await.is_err();
   225	                        (closed, rx)
   226	                    })));

    (changes.rs)
    75	    /// Await the next coalesced change, sharing the [`Stream`] state machine.
    76	    pub(crate) async fn next_inner(&mut self) -> Option<()>

Resolution: Move the transitions onto the shared type: make `Channel<T>` own its optionality and give it `fn poll_receiver(&mut self, cx) -> Poll<Option<&mut watch::Receiver<Inner<T>>>>` (drives a `Waiting`, restores `Ready`, yields `None` once closed) and `fn wait(&mut self)` (moves `Ready(rx)` into the boxed `changed()` future); each observer's field becomes a plain `Channel<T>` and each `poll_next` becomes `let Some(rx) = ready!(this.channel.poll_receiver(cx)) else { return Poll::Ready(None) };`, the domain step, and `this.channel.wait()`. Delete `Changes::next_inner` and write its `try_next` as the siblings do. Host `Channel` in rumors.rs or a `channel` submodule rather than `pub(super)` inside unordered.rs. Acceptance: `grep -c 'channel state present' src/rumors/` and `grep -c 'matched Ready above' src/rumors/` both return 0; `next_inner` is gone; the observer suites and the doctests in unordered.rs and changes.rs still pass.

See also: inventory-6.

### module-graph-1: Two single-site upward imports into `error` close the top-level cycle
- Where: src/tree/mirror/party.rs:6-11 (related: src/tree/mirror/party.rs:76-77, :102, :119-120, :131-137, :153-159; src/link.rs:385-388; src/peer/gossip.rs:959, :1021-1026, :1246; src/tree/mirror/handshake.rs:172; src/error.rs:32-50, :292-314)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (read every site; grep of `LinkPoisoned`, `HandOff`, `party::`, `handshake::` across `src/peer/`)
- Verification: confirmed; history: no-rationale-found (the neighbouring `handshake` module shows the intended shape and `error.rs:292` lifts it)
- Owner-gated: no

The only production edges from the `tree` and `link` subtrees into the top-level `error` module are `party.rs` constructing `Error::HandOffMalformed`/`HandOffTruncated`/`Io` and `link.rs`'s `begin` returning `crate::Error::LinkPoisoned`. The sibling parser `handshake.rs` defines a local `pub(crate) enum Error` that `error.rs` lifts by a `From` impl; these two sites are the asymmetry that folds `batch, bookmark, error, link, peer, rumors, snapshot, tree` into one SCC. Beyond the graph, `link.rs` naming the public error sum makes the transport contract recompile on every variant that sum aggregates (`error.rs:32-50` imports from `bookmark`, `peer`, and the whole mirror tree).

Evidence:

         6	use crate::{
         7	    Error,
         8	    observe::{CaptureRead, SessionHandle},
         9	    tags::PARTY_TAG,
        10	    tree::mirror::cbor::{self, HeadError, MAJOR_BSTR},
        11	};
    ...
       102	    let malformed = |defect| Error::HandOffMalformed { defect };
    ...
       119	            std::io::ErrorKind::UnexpectedEof => Error::HandOffTruncated,
       120	            _ => Error::Io(e),

    src/link.rs:
       385	    pub(crate) fn begin(&mut self) -> Result<u8, crate::Error> {
       386	        if self.poisoned {
       387	            return Err(crate::Error::LinkPoisoned);
       388	        }

    src/tree/mirror/handshake.rs (the template):
       170	/// A malformed, incompatible, or truncated preamble.
       171	#[derive(Debug, thiserror::Error)]
       172	pub(crate) enum Error {

    src/peer/gossip.rs (the funnel already owns the poison semantics):
       959	                        return Some((Err(Error::LinkPoisoned.widen()), drive));
    ...
      1021	                    let epoch = match drive.state.begin() {
      1022	                        Ok(epoch) => epoch,
      1023	                        Err(e) => {
      1024	                            drive.done = true;
      1025	                            return Some((Err(e.widen()), drive));
    ...
      1246	    let epoch = link.session.begin()?;

Resolution: In `party.rs` define `pub(crate) enum Error { Io(io::Error), Malformed(HandOffDefect), Truncated }` mirroring `handshake::Error`, and add `impl From<party::Error> for Error<NoBookmark>` beside the existing `From<handshake::Error>` at `error.rs:292`; the three call sites in `gossip.rs` (`:320`, `:761`, `:782`) already use `?` or `match`. In `link.rs` have `begin` return `Result<u8, Poisoned>` (a unit struct) and map it at the two `gossip.rs` sites (`:1021-1026`, `:1246`), which sit beside the site (`:959`) that constructs `LinkPoisoned` directly today. Acceptance: `analyze.py` reports no edge from `tree::*` or `link::*` into `error`; the top-level SCC dissolves into layers with `error` above `tree` and `link`.

### api-core-7: `Error::widen` is a variant-by-variant identity map forced by the flat generic shape
- Where: src/error.rs:316-366 (related: src/error.rs:292-314, src/error.rs:57-60)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed (with a correction: the finding's variant tally was wrong, and no tally belongs in the write-up); history: no rationale found (`widen` arrived in a WIP commit; the 2026-08-19 error-taxonomy ruling concerns naming, not shape)
- Owner-gated: yes: the alternative reshapes the public error type

`Error<B>` is generic in `B` solely for the `Bookmark(BookmarkIo<B::Error>)` arm (its own doc at 57-60 says so), so re-tagging an `Error<NoBookmark>` as `Error<B>` must enumerate every variant, all but one of them a verbatim copy, plus the `match never {}` elimination. `From<handshake::Error>` (292-314) is a second variant-by-variant map into the same enum. The compiler keeps both exhaustive, so this is maintenance weight, not a hazard; the steelman for the flat shape holds (users match one level deep, and the module's remedy table is the payoff).

Evidence:

    322	    pub(crate) fn widen<B: BookmarkError>(self) -> Error<B> {
    323	        match self {
    324	            Error::Io(error) => Error::Io(error),
    325	            Error::MagicMismatch { remote_magic } => Error::MagicMismatch { remote_magic },

    360	            Error::Bookmark(error) => match error {
    361	                BookmarkIo::Io(never) => match never {},
    362	                BookmarkIo::Format(error) => Error::Bookmark(BookmarkIo::Format(error)),
    363	            },

Resolution: Design proposal for the owner: give the wire layer a non-generic session error (the bookmark-independent variants) with `Error<B>` wrapping it beside `Bookmark`, or a `From<SessionError> for Error<B>`; `widen` then collapses to two arms or disappears and `From<handshake::Error>` targets the non-generic type. If the flat public shape is preferred, record that decision in `widen`'s doc and close the question. Acceptance: either `widen` is gone or reduced to a match over at most two arms with the public error-matching examples still compiling, or `widen`'s doc records the flat-shape decision.

### api-core-9: `static_assertions` is a runtime dependency used only under `cfg(test)`, and the crate-attribute comments misstate their mechanisms
- Where: src/lib.rs:295-302 (related: Cargo.toml:128, Cargo.toml:144-145, src/tree/typed/height.rs:175-176, src/tree/typed/height/tests.rs:8-10)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep: the crate's only `static_assertions` use is in src/tree/typed/height/tests.rs, declared behind `#[cfg(test)] mod tests;`; Cargo.toml:128 lists it under `[dependencies]`, not `[dev-dependencies]`; the vendored static_assertions 1.1.0 `assert_eq_size!` expands to an `#[allow(... unsafe_code ...)] unsafe { .. }` block)
- Seen by: prose (the comment wording; the dependency placement was its open question); refutation: confirmed; history: no rationale found (both comments date from fceb55f98 with no message body)
- Owner-gated: no

A released library pulls `static_assertions` into every downstream build for macros only its own test module expands. The comment above the `forbid(unsafe_code)` gate gestures at the right mechanism (the macro's `unsafe` block would trip `forbid`) but does not name the crate, has a number disagreement, and says "allow it only in tests" where the attribute lifts a `forbid`. The `large_futures` comment describes denying a lint as "check to make sure it's not an issue".

Evidence:

    295	// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
    296	#![cfg_attr(not(test), forbid(unsafe_code))]

    300	// Programmer error in recursive async traits can create large futures, so we
    301	// check to make sure it's not an issue
    302	#![deny(clippy::large_futures)]

    128	static_assertions = { workspace = true }

Resolution: Move `static_assertions` to `[dev-dependencies]`. Reword 295 to name the mechanism: "`static_assertions`' size macros expand to an `unsafe` block, and only the tests use them, so the `forbid` is lifted under `cfg(test)`." Reword 300-301: "Recursive async traits can produce very large futures without warning; deny the lint so a size regression is a build error." Acceptance: `cargo tree -e normal` (or `just` equivalent) shows no `static_assertions` in the normal dependency graph; `just check` and `just test` still pass; both comments name the crate or lint action they govern.

See also: deps-1 (the entry of record for the dependency placement; this entry is counted for the two crate-attribute comments at lib.rs:295-302), inventory-2 (a cross-reference to deps-1).

### api-core-12: Hidden test-only surface is gated inconsistently, and `Snapshot::warm_caches` has no caller
- Where: src/peer.rs:210-213 (related: src/peer.rs:710-716, src/rumors.rs:410-416, src/snapshot.rs:163-169, src/peer.rs:480-483, src/peer.rs:726-728, Cargo.toml:142, Cargo.toml:145)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep: every `seed_rng` caller is under tests/; every `warm_caches` caller is a bench on `Rumors` or a `Tree` in src/tree/tests.rs, none on a `Snapshot`; Cargo.toml:145 gives tests and benches the `test-internals` feature through the self dev-dependency)
- Seen by: structure; refutation: confirmed (raising severity: the ungated `seed_rng` puts `rand::RngCore` in a shipped public signature, so a `rand` major bump becomes semver-visible); history: no rationale found (both predate the cfg-gating convention first applied in cb69fc951 and stated in db2718d46)
- Owner-gated: no

`Peer::seed_rng` and the three `warm_caches` are `#[doc(hidden)] pub` but compiled into every build, while the equally test-only `sync_window_floor` and `dangerously_alias_party` carry `#[cfg(any(test, feature = "test-internals"))]`. Hidden-but-shipped serves neither reader, and `seed_rng` makes `rand` (Cargo.toml:142) a public dependency. `Snapshot::warm_caches` has no caller at all: the benches warm through `Rumors::warm_caches`.

Evidence:

    210	    /// Like [`seed`](Self::seed), but draws the universe's [`Network`]
    211	    /// identifier from a caller-supplied RNG instead of [`OsRng`].
    212	    #[doc(hidden)]
    213	    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {

    480	    #[cfg(any(test, feature = "test-internals"))]
    481	    #[doc(hidden)]
    482	    #[must_use]
    483	    pub fn sync_window_floor(mut self) -> Self {

Resolution: Gate `seed_rng`, `Peer::warm_caches`, and `Rumors::warm_caches` with the same `#[cfg(any(test, feature = "test-internals"))]` as their neighbours, and delete `Snapshot::warm_caches`. If deterministic seeding is meant for downstream users, un-hide `seed_rng` and document it instead. Acceptance: `grep -rn 'fn warm_caches' src/` lists peer.rs and rumors.rs under cfg gates; a default-features `cargo doc` exposes neither `seed_rng` nor `warm_caches`; `just bench-build` and `just gate` stay clean.

Synthesis note: inventory-4 corrects the gating step for `seed_rng`: `Peer::seed` calls it, so the gate needs a private inner function that `seed` calls, with the gated `pub fn seed_rng` as a wrapper, or the decision to un-hide and document it.

See also: inventory-4.

### api-core-16: The `Peer` forwarding layer has three asymmetries, and `Batch::commit`'s doc describes one caller of five
- Where: src/peer.rs:627-708 (related: src/rumors.rs:406-408, src/rumors.rs:372-377, src/batch.rs:121-127, src/tests.rs:629, src/tests.rs:648)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: `fn changes` exists only in rumors.rs; `Peer::messages_since` at peer.rs:689 is the target of `Rumors::unordered_messages_since`; `Batch::new` plus `commit` appears five times in peer.rs, four of them closure-free)
- Seen by: structure; refutation: confirmed; history: no rationale found (586911cfd renamed the public and one internal method but left `messages_since`; ce27df86 wrote "Runs iff the caller's closure returned `Ok`" in the same commit that shaped the closure-free callers)
- Owner-gated: no

`peer.rs` owns the local API as undocumented `pub(crate)` methods that `Rumors` forwards to one-for-one. Three exceptions: `Rumors::changes` constructs `Changes::subscribe(&self.peer.inner)` itself instead of going through `Peer` like its five sibling observers; `Peer::messages_since` is the target of the public `unordered_messages_since`, so the internal name diverges from the public one it implements; and `Peer::send`/`send_all` re-spell `Batch::new; op; commit` instead of `self.batch(|b| b.send(m))`, while `Batch::commit`'s doc says it runs iff the caller's closure returned `Ok` and that `Rumors::batch` owns that decision, which is untrue of the four direct callers. A forwarding layer is justified by being uniform; the `commit` doc is a present-tense claim the code contradicts.

Evidence:

    627	    pub(crate) fn send(&self, message: T) -> Result<(), EncodeError>
    628	    where
    629	        T: Send + Sync + 'static,
    630	    {
    631	        let mut batch = Batch::new(&self.inner, self.codec);
    632	        batch.send(message)?;
    633	        batch.commit();
    634	        Ok(())
    635	    }

    689	    pub(crate) fn messages_since(&self, since: Version) -> UnorderedMessages<T>

    123	    /// Observers and concurrent gossip sessions see all of it land at
    124	    /// once, in at most one observer wakeup. Runs iff the caller's
    125	    /// closure returned `Ok`
    126	    /// ([`Rumors::batch`](crate::Rumors::batch) owns that decision).

Resolution: Route `Rumors::changes` through a `Peer::changes`; rename `Peer::messages_since` to `unordered_messages_since`; express `Peer::send` as `self.batch(|batch| batch.send(message))` and `send_all` likewise (either leave the infallible `redact`/`redact_all` explicit or add a private `commit_with`); restate `Batch::commit`'s doc as: commits everything queued in one critical section, reached from `Rumors::batch` on `Ok` and from the one-shot `send`/`redact` families. Acceptance: `grep -n 'Batch::new' src/peer.rs` shows one site or a single private helper; `grep -rn 'fn messages_since' src/` is empty; `Rumors::changes` forwards to `self.peer`; batch.rs:121-127 no longer claims a single caller.

### api-core-34: `Snapshot`'s `IntoIterator` names an `Iter` no public path reaches, while `iter()` hides the same type
- Where: src/snapshot.rs:4-7 (related: src/snapshot.rs:105-112, src/snapshot.rs:172-179, src/lib.rs:317, src/lib.rs:322, src/lib.rs:347, src/tree.rs:167-176)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (lib.rs:317 `mod snapshot;` and 322 `mod tree;` are private; the re-export block at 328-349 exports only `Snapshot` from snapshot and `MERKLE_HASH_LEN`/`SessionStats` from tree; no external use of `snapshot::Iter` or `rumors::Iter` in tests, examples, or benches)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: deliberate-but-expired (cb69fc951 re-exported `Iter` at the root; 8dc0596ed removed that re-export and made `Snapshot::iter` return an opaque type "to move engine internals out of the user-facing prose", leaving this `pub use`, its doc, and the `IntoIterator` impl as unswept remainder)
- Owner-gated: yes: either adds a public type name or removes a public trait impl

`snapshot.rs` re-exports `crate::tree::Iter` with a doc calling it "re-exported from the tree internals", but no public path reaches it, so the doc describes a re-export nobody outside the crate can see and names internals. Meanwhile `impl IntoIterator for &Snapshot<T>` sets `type IntoIter = Iter<'a, T>` (a type users can spell only as `<&Snapshot<T> as IntoIterator>::IntoIter`) while `Snapshot::iter` returns `impl DoubleEndedIterator + ExactSizeIterator + Send + Sync`: one iterator presented two ways, and a caller who wants to store it in a struct field can do so through neither.

Evidence:

    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;

   172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
   173	    type Item = (&'a Version, Arc<T>);
   174	    type IntoIter = Iter<'a, T>;

Resolution: History already chose opacity, so the consistent completion is to delete the `pub use` with its doc and the `IntoIterator` impl (callers write `snapshot.iter()`). The alternative, exporting `Iter` at the crate root and returning it from `iter()` per std's naming convention, reverses 8dc0596ed and is the owner's to weigh. Acceptance: either `IntoIterator for &Snapshot` and snapshot.rs:4-7 are gone and the doctests using `snapshot.iter()` still compile, or `rumors::Iter` appears in the public index and `Snapshot::iter` names it; in both cases no public doc says "tree internals".

### inventory-4: Bench and test hooks stay in the shipped library while their siblings are gated
- Where: src/peer.rs:710-716 (related: src/peer.rs:206-213, src/snapshot.rs:163-169, src/rumors.rs:410-416, src/tree.rs:306-313, src/peer.rs:480-486, src/peer.rs:726-734, src/lib.rs:319-321, Cargo.toml:145)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `warm_caches` and `seed_rng` across src, benches, examples, tests, crates, and the `before` fuzz directories: consumers are benches/, tests/, src/tree/tests.rs, and the internal delegation chain; Cargo.toml:145 enables `test-internals` for every dev target)
- Verification: reframed: `Peer::seed` calls `seed_rng` (peer.rs:207), so the gate cannot be applied to `seed_rng` directly; history: no-rationale-found (the mutants note records `warm_caches` as a performance-genre missed mutant, which is consistent with a calibration-only hook)
- Owner-gated: yes: removes hidden-but-public API, and `seed_rng` may deserve un-hiding instead

`Peer::warm_caches`, `Rumors::warm_caches`, `Snapshot::warm_caches`, `Tree::warm_caches`, and `Peer::seed_rng` are `#[doc(hidden)] pub` with no cfg gate, while `sync_window_floor` and `dangerously_alias_party` beside them carry `#[cfg(any(test, feature = "test-internals"))]`, the crate's stated convention for test scaffolding. Every external consumer builds with `test-internals` through the self-referential dev-dependency. `seed_rng` differs from the rest: production `seed` is implemented through it, so gating it needs a private inner function, or a decision that deterministic seeding is documented API.

Evidence:

    206	    pub fn seed() -> Self {
    207	        Self::seed_rng(&mut OsRng)
    208	    }
    ...
    212	    #[doc(hidden)]
    213	    pub fn seed_rng<R: RngCore + ?Sized>(rng: &mut R) -> Self {

    710	    /// Force this set's tree to compute its lazy structural memos (observable
    711	    /// hash and ceiling/floor version bounds), so a subsequent operation is
    712	    /// timed against its own work. For benchmark and test calibration only.
    713	    #[doc(hidden)]
    714	    pub fn warm_caches(&self) {
    715	        self.inner.borrow().tree.warm_caches();
    716	    }

    480	    #[cfg(any(test, feature = "test-internals"))]
    481	    #[doc(hidden)]
    482	    #[must_use]
    483	    pub fn sync_window_floor(mut self) -> Self {

Resolution: Add `#[cfg(any(test, feature = "test-internals"))]` to the four `warm_caches` sites (tree.rs:306-313, snapshot.rs:166-169, peer.rs:713-716, rumors.rs:413-416). For `seed_rng`, either keep a private `fn seed_from(rng)` that `seed` calls and gate a `pub fn seed_rng` wrapper, or drop `#[doc(hidden)]` and document it as the deterministic-seeding entry for downstream tests. Acceptance: `cargo doc` and `cargo clippy -p rumors --lib` under default features show no `warm_caches`; `seed_rng` is either gated or documented; `just test-all` and `cargo bench --no-run` still compile.

See also: api-core-12.

### inventory-6: Three take-and-restore sites have panic-free std spellings
- Where: src/rumors/unordered.rs:95-109 (related: src/rumors/unordered.rs:208-210, src/tree/mirror/streaming/remote/streams.rs:215-226, src/link/routed/router.rs:105-113; design-option sites: unordered.rs:197, 218-222, causal.rs:165, 187-189, changes.rs:81, 133, 156-158, streams.rs:184-188)
- Class / severity / confidence: idiom / low / high
- Provenance: assessed (read; the std methods `Option::get_or_insert_with` and `Option::insert` return `&mut T`, and a move followed unconditionally by `break` is accepted by the borrow checker)
- Verification: reframed: the sweep's claim holds at three sites; for the observers' `channel` and `StreamSender::finish`, moving the payload out of an `Option` still leaves a `None` arm, so those panics relocate rather than vanish unless `None` is given a meaning; `StreamReceiver`'s pair of `Option`s is dropped (see Dropped); history: no-rationale-found
- Owner-gated: no

Three sites hold a value in an `Option` only to move it once, and pay with a panic whose message restates rather than proves: `open_pass` stores into `pass` and the caller re-fetches with `expect("opened above")`; `StreamSender::write` assigns `SendState::Open(..)` then re-matches it with `unreachable!("the open state was just stored")`; `router::register` wraps `sender` in an `Option` to move it inside a loop. `get_or_insert_with`, `Option::insert`, and a direct move followed by `break` each give the same behavior with no panic site.

Evidence:

    102	        if pass.is_none() {
    103	            let inner = rx.borrow_and_update();
    104	            *pass = Some(Pass {
    ...
    208	                    Self::open_pass(&mut this.pass, rx, &this.checkpoint);
    209	
    210	                    let pass = this.pass.as_mut().expect("opened above");

    215	                *state = SendState::Open(
    ...
    224	                let SendState::Open(write, _) = state else {
    225	                    unreachable!("the open state was just stored");
    226	                };

    106	    let mut sender = Some(sender);
    107	    let token = loop {
    108	        let token = Token::new();
    109	        if let Entry::Vacant(vacancy) = entries(table).entry(token) {
    110	            vacancy.insert(sender.take().expect("the loop ends at the first vacancy"));
    111	            break token;

Resolution: unordered.rs: have `open_pass` return `&mut Pass` via `self.pass.get_or_insert_with(..)` and use it at 210. streams.rs: store the open state as `Option<(FrameWrite<..>, Done<..>)>` and write `let (write, _) = self.state.insert(opened);` at 215-227. router.rs: `Entry::Vacant(vacancy) => { vacancy.insert(sender); break token; }` with `sender` moved directly. The observers' `Channel` and `finish` are a design option, not part of this finding: either accept the `None` arm as the "ended" state (a fused stream) or transition by cloning the `watch::Receiver` before building the wait, so `channel` needs no `Option` at all. Acceptance: the three cited panic sites are gone; the observer, streams, and routed suites pass unchanged.

See also: remote-adapter-streams-20, api-core-33, link-26.

### inventory-10: Pub-in-private is the pervasive convention under `tree/` and `message.rs`, so `pub` no longer marks API
- Where: src/message.rs:47-51 (related: src/tree/typed.rs:19-22, src/message.rs:233-238, src/tree/mirror/streaming.rs:50-60, and the section "pub items in non-public modules" of scratchpad/sweeps/inventory/visibility.txt)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (my own count: 218 column-0 `pub struct|enum|trait|type|const|fn|static` items in non-`tests.rs` files under src/tree/ and src/message.rs, before subtracting the names error.rs and lib.rs re-export and the `cfg(test)`-only files; lib.rs:322 declares `mod tree;` private and lib.rs:310 `mod message;`)
- Verification: confirmed as an order of magnitude; history: no-rationale-found (neither AGENTS.md nor any note records the convention)
- Owner-gated: yes: a crate-wide convention choice, though the resolution changes no public API

On the order of a hundred `pub` types, traits, consts, and free functions live in modules unreachable from the crate root without a re-export. Two costs: a reader cannot tell library API from internals by reading `pub`, and rustdoc written for external readers accumulates on types no external reader can reach (`Message`'s `# Panics` sections address "every caller"). The crate states a `pub(crate)`-for-rustdoc-links rationale at typed.rs:19-22 for `untyped` but leaves the sibling `hash`, `height`, `node`, `path`, `prefix` modules `pub`.

Evidence:

    47	#[derive(Clone)]
    48	pub struct Message {
    49	    message: Arc<dyn Any + Send + Sync>,
    50	    serialized: Bytes,
    51	}

    19	// `pub(crate)` so rustdoc elsewhere can link to `typed::untyped::Range`: a
    20	// private `mod` is unnameable from outside `typed`, so the links would not
    21	// resolve. The items below are still re-exported as the canonical paths.
    22	pub(crate) mod untyped;

Resolution: Decide once. Either add `#![warn(unreachable_pub)]` to lib.rs and convert the flagged items to `pub(crate)` (the rustdoc-link rationale holds under `pub(crate)`), rewriting `Message`'s rustdoc at maintainer altitude as part of it; or record the pub-in-private convention in AGENTS.md so it is deliberate and reconcile typed.rs:19-22 with it. Acceptance: `unreachable_pub` is enabled and clean, or AGENTS.md states the convention.

See also: module-graph-12, tree-typed-1, streaming-backend-window-2, materialized-6, mirror-common-20, inventory-14.

### Nits (7)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| api-core-19 | `src/peer/bootstrap.rs:96-104` | `Bootstrap`'s `Debug` omits the `observe` setting | Add `.field("observe", &self.observe)` |
| api-core-21 | `src/protocol.rs:13-18` | `Protocol` derives `Default` for a call site that no longer exists | Remove `Default` from the derive list and the `#[default]` attribute |
| api-core-23 | `src/rumors.rs:62-77` | `Rumors` re-spells `Peer`'s field list and `Debug` body | Add a `pub(crate) fn with_bookmark` on `Peer` used by `Rumors::clone` and both `bookmark_inner` sites; share one `debug_fields` body between the two `Debug` impls |
| api-core-31 | `src/rumors/unordered.rs:63-71` | Import hygiene: `crate::Inner` spelled at seven sites, a qualified constant in a return type, scattered import groups | `use crate::Inner;` in the three observer files and `use crate::MERKLE_HASH_LEN;` in snapshot.rs |
| api-core-32 | `src/rumors/unordered.rs:139` | Five doc examples discard `send`'s `Result`, and rustdoc's injected `#![allow(unused)]` hides it | Use `?` with an `Ok::<(), EncodeError>(())` tail at the five doc examples; add `#![doc(test(attr(deny(unused_must_use))))]` to lib.rs |
| inventory-15 | `src/lib.rs:339-339` | The crate root aliases a private struct with `pub(crate) use peer::Inner` | Remove the root alias; import `crate::peer::Inner` at the nine use sites and the bookmark.rs doc link |
| module-graph-10 | `src/lib.rs:339` | `pub(crate) use peer::Inner;` at the crate root, and seven inline `crate::Inner<T>` spellings | Add `use crate::peer::Inner;` to `causal.rs`, `changes.rs`, and `unordered.rs` and drop the inline qualifications |

## Session and bookmark

Files: src/peer/gossip.rs, bookmark.rs and bookmark/format.rs, reconciliation.rs, observe.rs, message.rs. Entries: 22 (1 medium, 9 low, 12 nit).

### session-bookmark-10: `gossip_inner` hand-builds eleven `(Intent, Err(..))` returns because its tuple return type defeats `?`
- Where: src/peer/gossip.rs:598-905 (related: src/peer/gossip.rs:443-455, src/peer/gossip.rs:472-487, src/peer/gossip.rs:1028-1064)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep: nine `return (Intent::Remain, Err(` sites at 627, 639, 679, 714, 746, 762, 775, 873, 885 and two `return (outcome, Err(` at 793, 898, both after `party::send` at 782)
- Seen by: structure; refutation: confirmed (with one implementation detail: the error type must not offer both `From<Error>` and `From<Error<B>>`, which overlap at `B = NoBookmark`); history: no rationale found (the tuple shape is original to the first split of the gossip module and was only ever justified by its semantics)
- Owner-gated: no

The session transaction returns `(Intent, Result<..>)` so that a retiree whose party crossed the wire is never handed back. That property matters at exactly two sites (793 and 898, which carry `outcome`), but the shape forces nine more sites to spell `return (Intent::Remain, Err(..))` by hand, so a reviewer must check every early return in a 300-line function to confirm the Intent is right, rather than seeing the post-hand-off region as the only place a non-Remain outcome can arise. Finished code should be obviously, reviewably correct; the safety argument (identity never duplicated) should be checkable at the two sites where a party is in flight.

Evidence:

    603	    ) -> (Intent, Result<(Version, SessionStats), Error<B>>)

    627	                Err(error) => return (Intent::Remain, Err(Error::from(error).widen())),

    679	                return (Intent::Remain, Err(Error::Bookmark(e)));

    793	                    return (outcome, Err(e.widen()));

Resolution: Introduce a module-private `struct Aborted<B: BookmarkError> { outcome: Intent, error: Error<B> }` with `From` impls that fix `outcome: Intent::Remain`: `From<Error>` (the widening source, since `Error = Error<NoBookmark>`; sites that `.widen()` today keep it before `?`), `From<BookmarkIo<B::Error>>`, and `From<handshake::Error>`; do not also implement `From<Error<B>>`, which overlaps at `B = NoBookmark`, and route the `Error::PartyOverlap` site (873) through the widening impl. Return `Result<(Intent, Version, SessionStats), Aborted<B>>`; the nine pre-hand-off sites become `?`; only 793 and 898 construct `Aborted { outcome, error }` explicitly. `retire_inner`, `gossip`, and `gossip_when` destructure at their match. Alternative with the same payoff: split at the reconciliation boundary into an `open` phase returning a plain `Result` and a `commit` phase that owns the Intent. Acceptance: `grep -c 'return (Intent::Remain, Err(' src/peer/gossip.rs` is 0; exactly two sites construct a non-Remain outcome and both sit after `party::send`; `tests/retire.rs` and `tests/lifecycle.rs` pass unchanged; `just gate` clean.

### session-bookmark-1: The bookmark-attach path lives in the wire-session module, which its own doc does not admit
- Where: src/peer/gossip.rs:1-6 (related: src/peer/gossip.rs:136-157, src/peer/gossip.rs:344-399, src/peer/gossip.rs:501-511, src/peer.rs:32, src/peer.rs:272-277)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep: `peer.rs:32` re-exports `Unbookmarked` from `gossip`; `peer.rs:272-277` is a one-line delegate to `bookmark_inner`)
- Seen by: structure; refutation: confirmed; history: no rationale found (historical accretion; the module doc was written later around the drivers)
- Owner-gated: no

The module doc scopes the file to the wire-session drivers, the preamble constants, and `PartyGuard`. `Unbookmarked`, `bookmark_inner`, and `bookmark_record` are the attach path: no session, no wire. A reader following `Peer::bookmark` from `peer.rs` is bounced into the gossip module for semantics unrelated to gossip. Modules have a single clear responsibility (Principle 5 doctrine).

Evidence:

    1	//! The wire-session drivers for [`Peer`]: [`bootstrap`](Bootstrap::join),
    2	//! [`gossip`](crate::Rumors::gossip), and [`retire`](Peer::retire).
    3	//!
    4	//! Also here: the preamble constants every session leads with, and the
    5	//! [`PartyGuard`] that snaps a speculatively donated party back in place
    6	//! on failure.

Resolution: Move `Unbookmarked`, `bookmark_inner`, and `bookmark_record` beside `Peer::bookmark` in `peer.rs` (or a `peer/bookmark.rs` sibling) and drop the `pub use gossip::Unbookmarked` re-export; the public path `rumors::Unbookmarked` is unchanged. `bookmark_update` and `bookmark_donate` are session-time and stay. At minimum, make the module doc list what the file holds. Acceptance: the module doc enumerates the file's contents accurately; `Peer::bookmark`'s implementation is declared in the same file as its public method; `just gate` clean.

### session-bookmark-8: `bookmark_inner` rebuilds `Peer` field-by-field twice to swap the bookmark type parameter
- Where: src/peer/gossip.rs:344-399 (related: src/peer/gossip.rs:328-339, src/peer.rs:216-225)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed; history: no rationale found (three fields when written, accreted to seven by extending both lists)
- Owner-gated: no

Attaching a bookmark destructures `self` and rebuilds a `Peer<T, B>` (349-366), then on a failed record rebuilds a `Peer<T, NoBookmark>` (387-395), copying seven fields each time. Struct-update syntax cannot change the type parameter, so a helper must destructure too, but once: a private `fn with_bookmark<B2: BookmarkError>(self, bookmark: B2) -> Peer<T, B2>` collapses both and states the intent ("same peer, different bookmark") that the field copies only imply. A field added to `Peer` is then threaded in one place instead of three.

Evidence:

    386	            Err(error) => Err(Unbookmarked {
    387	                peer: Peer {
    388	                    network: peer.network,
    389	                    window: peer.window,
    390	                    run_budget: peer.run_budget,
    391	                    inner: peer.inner,
    392	                    bookmark: Arc::new(Mutex::new(Bookmarked::new(NoBookmark))),
    393	                    codec: peer.codec,
    394	                    observe: peer.observe,
    395	                },

Resolution: Add `with_bookmark`; `bookmark_inner` becomes `let peer = self.with_bookmark(bookmark); if pristine { return Ok(peer) } match peer.bookmark_record().await { Ok(()) => Ok(peer), Err(error) => Err(Unbookmarked { peer: peer.with_bookmark(NoBookmark), error }) }`. Acceptance: `bookmark_inner` contains no field list; gate clean.

See also: api-core-23.

### session-bookmark-11: `bookmark_update`'s reclaim-if-stale block is duplicated inline in `gossip_inner`, with a needless `Version` clone at both sites
- Where: src/peer/gossip.rs:681-694 (related: src/peer/gossip.rs:537-553, src/peer/gossip.rs:884, src/bookmark.rs:282-298, src/bookmark.rs:436-479, tests/bookmark_when.rs:26 and 49)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `bookmark_update()` has one caller, line 884; `is_current` is called at 541 and 685 with the identical `let version = inner.tree.latest().clone();` above each; `Tree::latest` returns `&Version` at src/tree.rs:220; `reclaim` takes `&Version` and clones internally at bookmark.rs:469, 478)
- Seen by: structure; refutation: confirmed (adds that tests/bookmark_when.rs:26 and 49 name `Bookmarked::is_current` in prose and must move with it); history: deliberate and holds for placement (the single-critical-section design, 3f33afab, stated inline at 645-670), not for the duplicated decision
- Owner-gated: no

The block `if let Some(party) = inner.party.as_mut() { let version = inner.tree.latest().clone(); if !bookmark.is_current(..) { bookmark.reclaim(..); persist = true } }` appears in `bookmark_update` (539-549) and again inside `gossip_inner`'s combined critical section (683-694). The inline copy is a deliberate consequence of the one-critical-section design, which is a reason to factor the decision, not to copy it: a future change to the suppression rule must be made twice. Both copies clone `latest()` although `inner.party` and `inner.tree` are disjoint field borrows and `reclaim` clones internally, so the call-site clone is a third copy of the version. The decision "does this identity need re-recording" belongs to `Bookmarked`, whose `is_current` doc already calls itself the suppression test.

Evidence:

    683	                if let Some(party) = inner.party.as_mut() {
    684	                    let version = inner.tree.latest().clone();
    685	                    if !bookmark.is_current(party, &version) {

    539	            if let Some(party) = inner.party.as_mut() {
    540	                let version = inner.tree.latest().clone();
    541	                if !bookmark.is_current(party, &version) {

Resolution: Make `Bookmarked::reclaim` return `bool` (true when it recorded, i.e. when the token was stale), folding `is_current` in as private. Both sites become `if let Some(party) = inner.party.as_mut() { persist = bookmark.reclaim(self.network, party, inner.tree.latest()); }` with no clone. Update the two prose references in tests/bookmark_when.rs to state the rule ("own region advanced or party changed") without the private name. Acceptance: `grep -rn is_current src/peer tests/` is empty; one place in the crate decides suppression; the two `.clone()` lines are gone; bookmark suites pass.

### session-bookmark-12: Two-protocol dispatch prose survives the V1 retirement
- Where: src/peer/gossip.rs:723-729 (related: src/peer/gossip.rs:63, 258, 473, 615-617, 1264; src/peer/gossip/tests.rs:6, 184, 247, 261, 316, 326; src/observe.rs:236-264)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `V2|dialect|selected protocol|Both branches|neither concrete|both towers|whichever protocol` over the partition; read `Attachment::begin` at observe.rs:236-264, which has no dialect branch; `.agent-notes/2026-09-01-v1-retirement/README.md:225-226` records "drop the dialect guard in `Attachment::begin`"; README:263-265 records the sweep pattern `alternating\|protocol-v1\|\bv1\b`, which explains why V2-side qualifiers survived)
- Seen by: structure, prose, perfapi (open question); refutation: reframed (dispatch and contrast prose is stale; bare names of the live `Protocol::V2` variant are accurate); history: deliberate but expired (every phrase was accurate under two dialects; 368da2a5 removed the second; the `Protocol` enum itself stays public as wire vocabulary by recorded ruling, README:155-158)
- Owner-gated: no for the dispatch prose and the inert `v2_` test-name prefixes; yes for dropping "V2" as the name of the current dialect (tied to the `Protocol` enum, protocol.rs, outside this partition)

With one protocol, prose that describes a choice, a contrast, or a gate no longer present is a ghost reference (Principle 5). Line 616 conditions the observation handle on "the dialect is observable", a guard `Attachment::begin` does not have (the sibling field doc at 1099-1101 was corrected by the retirement; this comment was not). Lines 723-729 describe "this peer's selected protocol", "Both branches", and "neither concrete protocol state machine" over a single `Reconciliation`. Line 63 "re-instantiate both towers" dates to when `mirror/alternating` existed; 258 and 473 "under V2" imply a contrast partner; 1264 "whichever protocol carries it" has one carrier. In tests.rs the `v2_` prefixes on the two test names (261, 326) distinguished them from `v1_` twins the retirement deleted. Sites that merely name the live `Protocol::V2` (gossip.rs:51, 1276; observe/tests.rs:58, which asserts `protocol: Protocol::V2`; reconciliation.rs:246, fresh authorship by 368da2a5) are accurate today.

Evidence:

    615	        // The session's observation handle: inert unless a handler is
    616	        // attached and the dialect is observable, and shared, like the
    617	        // recorder, by every layer that moves a wire item.

    723	        // Reconcile using this peer's selected protocol. Both branches meet at
    724	        // the lifecycle boundary the surrounding transaction needs: a local
    725	        // root plus raw transport halves positioned after reconciliation.
    726	        // The protocol bodies live behind the non-generic [`Reconciliation`],
    727	        // whose methods return their futures boxed: neither concrete
    728	        // protocol state machine becomes part of this outer session future,
    729	        // or of the consumer crate that instantiates it.

Resolution: 616: match the field doc at 1099-1101 ("inert unless a handler is attached"). 723-729: single-path prose ("The reconciliation runs behind the non-generic [`Reconciliation`], whose future is boxed so the protocol state machine stays out of this session future and out of consumer crates"). 63: "the protocol tower". Delete "under V2" at 258 and 473 and "whichever protocol carries it" at 1264. In tests.rs drop the `v2_` prefixes and the "under V2" qualifiers at 247 and 316. Whether "V2" survives as the dialect's name in prose (51, 1276, tests.rs:6, 184) follows the owner's ruling on the `Protocol` enum. Acceptance: `grep -n 'dialect\|selected protocol\|Both branches\|neither concrete\|both towers\|whichever protocol\|fn v2_' src/peer/gossip.rs src/peer/gossip/tests.rs` is empty.

See also: api-core-21, tests-disruption-handshake-27, tests-observation-13, benches-envelope-2.

### session-bookmark-15: The two erased reconciliation drivers assemble identical protocol towers
- Where: src/peer/gossip.rs:1186-1231 (related: src/peer/gossip.rs:1071-1079, 1127-1168, 1143, 302-327; src/tree/mirror/streaming/materialized.rs:427-441; src/tree/mirror/streaming/remote/proxy/start.rs:69-79)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified for the equivalence premises (read both `start` constructors: materialized defaults `stats: Recorder::default()` at materialized.rs:439 and the proxy defaults `stats: Recorder::default()` and `observe: SessionHandle::default()` at start.rs:75, 77; `Handshaking<B, V>` is constructed only by `impl<B: ..> Handshaking<B, Start>::start` at materialized.rs:427, so the `::<_, _>` turbofish at 1143 asserts only an arity); the duplication itself assessed by reading both bodies
- Seen by: structure, perfapi (turbofish); refutation: confirmed (merging is a judgment call; the duplication and the doc imprecision are real); history: no rationale found (the split mirrors the pre-existing gossip_inner/bootstrap_erased split and was carried through the sealing commit as a rename; the turbofish dates to that commit while the sibling body used the plain form)
- Owner-gated: no

`Reconciliation::reconcile` (1143-1151) and `bootstrap_reconcile` (1203-1209) build the same two `Handshaking` towers with the same builder chains and `Link::for_session`, differing only in the post-greeting check (`Claimant` newborn check versus `NetworkMismatch`), the bootstrap's mutual-bail epilogue (which `bootstrap_erased` could perform itself; it already calls `epilogue` at 327), and a `.stats(..)` call present in one and absent in the other (equivalent, since both `start`s default it). A builder added to the tower must be added in two places, and the asymmetry has already begun. The `Reconciliation` struct doc (1071-1079) says the struct "exists so that body is non-generic", but non-genericity comes from the boundary (free fn plus boxed future plus `inline(never)`), as `bootstrap_reconcile` itself demonstrates; the struct is a parameter bundle, which is fine but not the stated reason.

Evidence:

    1143	            let local = materialized::Handshaking::<_, _>::start(Local, root.into())
    1144	                .window(window)
    1145	                .target_message_size(run_budget.bytes() as u64)
    1146	                .stats(stats.clone());

    1203	        let local = materialized::Handshaking::start(Local, local_root)
    1204	            .window(window)
    1205	            .target_message_size(run_budget.bytes() as u64);

Resolution: Keep `Reconciliation` as the bundle and add an admission variant (`Claimant`, `Network { remote, local, local_min_events }`, `None`); `bootstrap_erased` builds it with `Root::default()`, a default `Recorder`, and the claimant admission, then performs the mutual-bail epilogue and returns `Ok(None)` itself. Delete `bootstrap_reconcile`, its `#[allow(clippy::type_complexity)]`, and the `Option` in its return. Drop the `::<_, _>` at 1143 either way. Fix the struct doc to say it bundles the erased inputs and that the boundary is what pins codegen. Acceptance: one `Handshaking::start` pair in gossip.rs; bootstrap and gossip snapshot suites unchanged (the wire is untouched); `just gate` clean.

See also: session-bookmark-10.

### session-bookmark-24: The `Persist` trait has one implementor (the blanket) and exists only to be allowed past `private_bounds`
- Where: src/bookmark.rs:144-190 (related: src/bookmark.rs:301, src/peer/gossip.rs:345, 402-406)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep across src/, tests/, benches/, examples/: `Persist` appears only as the trait, its blanket impl, and bounds at bookmark.rs:301, gossip.rs:345, 406; the three test bookmarks implement the public `Bookmark`)
- Seen by: structure; refutation: confirmed; history: deliberate but expired (the trait dispatched over two I/O faces via a `Mode` type parameter with two non-overlapping blanket impls; 83edcd94 deleted the blocking face and the `Mode` parameter, leaving one implementor and a doc that states what the layer does, not why a trait is needed)
- Owner-gated: no

`Persist` is implemented exactly once, by `impl<B: Bookmark> Persist for B`. No test double implements it, so its only observable effect is forcing `#[allow(private_bounds)]` with an apology comment on `impl<T, B: Persist> Peer<T, B>` and a `B: Persist` bound on `bookmark_inner`. Steelman: it names the decoded layer and would admit a byte-free test double; neither payoff is realized. What a thing does is not why it should exist (Principle 3).

Evidence:

    144	pub(crate) trait Persist: BookmarkError {

    161	impl<B: Bookmark> Persist for B {

    405	#[allow(private_bounds)]
    406	impl<T, B: Persist> Peer<T, B> {

Resolution: Replace the trait with two free fns in bookmark.rs, `read_record<B: Bookmark>(&B) -> impl Future<Output = Result<Record, BookmarkIo<B::Error>>> + Send` and `write_record<B: Bookmark>(&B, &Record) -> impl Future<..> + Send`, keeping the combinator shape (the `B: Sync` comment at 158-160 still applies); bound `Bookmarked<B: Bookmark>`, `impl<T, B: Bookmark> Peer<T, B>`, and `bookmark_inner<B: Bookmark>`; delete the `#[allow(private_bounds)]` and its comment. If finding 23 lands, the write fn shrinks further. Acceptance: `grep -rn Persist src` hits only prose (the word at gossip.rs:876 and bootstrap.rs:185); `grep -rn private_bounds src` is empty; `just gate` clean.

### session-bookmark-25: `Bookmarked`'s loaded state is a three-field invariant proven by three `expect`s instead of one `Option<Loaded>`
- Where: src/bookmark.rs:238-269 (related: src/bookmark.rs:343-361, 372, 419, 437; src/peer/gossip.rs:677-715)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `expect("loaded before mutation")` at 372, 419, 437; `write`'s `Err` arm resets `inner`, `staged`, and `last` separately at 354-358)
- Seen by: structure; refutation: confirmed (the `Loaded` shape compiles by reading: `ensure_loaded(&mut self) -> Result<&mut Loaded, _>`, the `send_if_modified` closure captures the `&mut Loaded`, and the borrow ends before `bookmark.write().await`); history: no rationale found (`last` and `staged` arrived at different times for stated reasons; nothing records why the three `Option`s stay separate)
- Owner-gated: no

`inner: Option<Record>` carries the unloaded/loaded distinction while `staged` and `last` must be reset in lockstep with it on a failed write; `slice`, `record`, and `reclaim` each re-prove "loaded before mutation" with an `expect`, and every method doc restates the call order ("after `ensure_loaded`"). Moving `record`, `staged`, and `last` into one `Loaded` struct behind a single `Option`, with `ensure_loaded` returning `&mut Loaded` and the mutators (plus `is_current`) living on it, makes the order compiler-checked, deletes the three panics, and turns the three-way reset into one assignment that cannot be done partially. Types-first: the compiler catches errors, not humans reading carefully.

Evidence:

    247	    inner: Option<BTreeMap<Network, Vec<Clock>>>,

    259	    staged: Option<(Party, Version)>,

    268	    last: Option<(Party, Version)>,

    354	            Err(_) => {
    355	                self.inner = None;
    356	                self.staged = None;
    357	                self.last = None;
    358	            }

Resolution: `struct Loaded { record: BTreeMap<Network, Vec<Clock>>, staged: Option<(Party, Version)>, last: Option<(Party, Version)> }`; `Bookmarked<B> { persist: B, state: Option<Loaded> }`; `ensure_loaded(&mut self) -> Result<&mut Loaded, ..>`; `reclaim`, `slice`, `record`, `is_current` become `Loaded` methods; `write(&mut self)` matches `self.state` once (`None` stays `Ok(())`) and sets `self.state = None` on `Err`. Callers: `let loaded = bookmark.ensure_loaded().await?; ...; bookmark.write().await`. Acceptance: `grep -c 'expect("loaded before mutation")' src/bookmark.rs` is 0; `write`'s `Err` arm is a single assignment; the "after `ensure_loaded`" sentences are deleted as redundant; `tests/bookmark_when.rs` and `tests/bookmark_transmit_window.rs` pass.

### session-bookmark-30: `unframe` pre-checks the payload bound that `Reader::take` already enforces, then `expect`s it; two magic numbers are untied from their constants
- Where: src/bookmark/format.rs:359-363 (related: src/bookmark/format.rs:282-293, 75, 78, 241; src/tree/mirror/cbor.rs:77)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read `Reader::take` at 282-293: `checked_add(n).filter(|&end| end <= self.bytes.len())` reports `Truncated { len: self.bytes.len() }` on any overrun; `MAX_HEAD_LEN: usize = 9` at cbor.rs:77)
- Seen by: structure; refutation: confirmed (equivalent on every pointer width: a `declared` above `usize::MAX` exceeds any slice length); history: no rationale found (introduced together in 35572cfc)
- Owner-gated: no

`unframe` re-implements the truncation check by hand to justify an `as` cast and an `expect("length checked")`. `usize::try_from(declared).map_err(|_| FormatError::Truncated { len: bytes.len() })?` followed by `reader.take(declared)?` is behaviorally identical, including on 32-bit targets, with no panic site; the `Reader` type exists so every byte is "compared, parsed, hashed, or payload" under one truncation rule, and duplicating that rule at one call site undercuts the totality argument the type carries. Nearby, `INTEGRITY_HEAD = [0x58, 0x20]` spells `HASH_LEN` (32) as `0x20` with nothing tying the two, and `Vec::with_capacity(9 + 2 + 9 + payload.len())` spells `cbor::MAX_HEAD_LEN` twice as `9`.

Evidence:

    359	    let declared = reader.head(MAJOR_BSTR, FrameDefect::PayloadByteString)?;
    360	    if declared > (bytes.len() - reader.at) as u64 {
    361	        return Err(FormatError::Truncated { len: bytes.len() });
    362	    }
    363	    let payload = reader.take(declared as usize).expect("length checked");

    75	const INTEGRITY_HEAD: [u8; 2] = [0x58, 0x20];

    241	    let mut covered = Vec::with_capacity(9 + 2 + 9 + payload.len());

Resolution: `let declared = usize::try_from(declared).map_err(|_| FormatError::Truncated { len: bytes.len() })?; let payload = reader.take(declared)?;`. `const INTEGRITY_HEAD: [u8; 2] = [0x58, HASH_LEN as u8];` with a `const { assert!(HASH_LEN < 256) }` beside it. Use `cbor::MAX_HEAD_LEN` in the capacity arithmetic. Acceptance: no `expect` in `unframe`; `truncation_at_every_prefix_is_rejected` and `framing_round_trips` pass; `grep -n '0x20\|9 + 2 + 9' src/bookmark/format.rs` empty.

### session-bookmark-47: `try_from_arc` erases a `T` to `Arc<dyn Any>` and downcasts it back (with a panic arm) to route through the fn pointer; `from_slice` duplicates `from_bytes`
- Where: src/message.rs:375-392 (related: src/message.rs:300-318, 440-463, 411-420, 472-482, 209-227)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: `Message::deserializer`'s inner fn at 455-461 is `decode_exact::<T>(bytes, limit)?` then `Ok(Arc::new(message))`; `deserializer` has two callers, 224 and 380; `from_slice` at 411-420 is `from_bytes` at 472-482 plus `Bytes::copy_from_slice`)
- Seen by: structure, perfapi; refutation: confirmed (both); history: deliberate and holds for the intent ("admission is the ingress computation", 6e4b6eea), which calling `decode_exact::<T>` directly preserves since the deserializer only wraps it in `Arc::new`; nothing pins fn-pointer identity
- Owner-gated: no

Send-side admission calls the fn pointer, receives `Arc<dyn Any + Send + Sync>`, downcasts to `Arc<T>` with `unwrap_or_else(|_| panic!(..))`, dereferences for `Eq`, and drops it. The pointer is `decode_exact::<T>` plus `Arc::new`, so calling `decode_exact::<T>` directly is the same computation and yields a typed `T`, removing one `ArcInner<T>` allocation, one `TypeId` check, and one unreachable panic per admitted send (denominated per `Rumors::send`/`Batch::send`; strict deletion of redundant work, fixed sign). The codec's two halves then become symmetric (both inner fns of `PayloadCodec::new`) instead of one living on `Message`. Separately, `from_slice` can delegate to `from_bytes`.

Evidence:

    380	        let decoded = match Self::deserializer::<T>()(&serialized, limit) {
    381	            Ok(decoded) => decoded,
    382	            Err(PayloadDecodeError::Depth(limit)) => return Err(EncodeError::Depth { limit }),
    383	            Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Roundtrip(source)),
    384	        };
    385	        // Faithfulness: what a receiver reads must be the value that was
    386	        // sent, judged by the payload type's own equality.
    387	        let decoded: Arc<T> = decoded
    388	            .downcast()
    389	            .unwrap_or_else(|_| panic!("a payload decodes to its own type"));

Resolution: In `try_from_arc`: `let decoded: T = match decode_exact::<T>(&serialized, limit) { Ok(v) => v, Err(PayloadDecodeError::Depth(limit)) => return Err(EncodeError::Depth { limit }), Err(PayloadDecodeError::Io(source)) => return Err(EncodeError::Roundtrip(source)) }; if decoded != *arc { return Err(EncodeError::Unfaithful) }`, and reword the comment: `decode_exact` is the one parse ingress runs (the deserializer wraps it in the `Arc`). Move `deserializer`'s inner fn into `PayloadCodec::new` beside `serialize_payload` and delete `Message::deserializer`. Make `from_slice` call `Self::from_bytes::<T>(Bytes::copy_from_slice(bytes), limit)`. Acceptance: no `downcast` in `try_from_arc`; `Message::deserializer` is gone; message/tests.rs (including `from_bytes_matches_from_slice`, `try_new_admits_exactly_the_limit`, `try_new_prices_an_enums_own_decode`, `a_type_that_cannot_read_its_own_output_fails_admission`, `codec_serializes_through_the_carried_limit`) pass unchanged. Optional meter in the style of tests/encode_alloc.rs: allocations of `Message::try_new(())` drop by one.

### Nits (12)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| clippy-pedantic-9 | `src/bookmark.rs:167-170` | `match` spelling of `let ... else` in two library sites | Write both bindings as `let .. else`, the crate's idiom at 107 sites; the example and test sites likewise |
| inventory-14 | `src/message.rs:91-93` | Six `pub(crate)` items are referenced only from their own file | Drop `pub(crate)` on message.rs:91 `recursion_limit`, 175 `PayloadSerializer`, 181 `PayloadDeserializer`, 368 `try_from_arc` |
| inventory-19 | `src/peer/gossip.rs:1379-1382` | `pub(crate)` fields on a module-private struct | Drop the `pub(crate)` on both fields |
| session-bookmark-2 | `src/peer/gossip.rs:43-45` | Stray trailing `use serde::..` lines jammed against the next item's doc comment | Fold each into the main import block (`use serde::{Serialize, de::DeserializeOwned};`) and restore the blank line before the doc comment |
| session-bookmark-3 | `src/peer/gossip.rs:165-167` | `Gossiped` lacks `PartialEq`/`Eq` although every field is `Eq`; `NoBookmark` derives only `Debug` | `#[derive(Debug, Clone, PartialEq, Eq)]` on `Gossiped` |
| session-bookmark-7 | `src/peer/gossip.rs:297-300` | `let _ = remote.intent;` is a no-op standing in for a comment | Delete line 300 |
| session-bookmark-17 | `src/peer/gossip.rs:1375-1382` | `PartyGuard` has `pub(crate)` fields on a module-private struct and a `//` comment where the module doc links a rustdoc item | Drop the `pub(crate)` |
| session-bookmark-19 | `src/peer/gossip/tests.rs:162-166` | Test helpers carry braces left over from removed `.batch(..)` closures, and one helper doc promises a value the function does not return | Remove the three brace pairs |
| session-bookmark-29 | `src/bookmark/format.rs:241-256` | `frame_as` builds the hash-covered region in a scratch `Vec` and copies it into the output, where `unframe` hashes slices in place | Write `out` once with a reserved digest slot and hash two slices of `out`, the way `unframe` reads them |
| session-bookmark-32 | `src/bookmark/format/tests.rs:189-214` | Two format tests rebuild the frame by hand, each a copy of `frame_as` with one spelling widened | Add the helper in tests.rs |
| session-bookmark-39 | `src/observe.rs:129-131` | Observer identity types derive `PartialEq, Eq` but not `Hash`/`Ord`, so they cannot key a per-stream table | Derive `Hash, PartialOrd, Ord` on the five identity types now; on `SessionInfo` once `Protocol` derives them |
| session-bookmark-41 | `src/observe.rs:368-380` | Long qualified paths at use sites where an import (or an already-in-scope name) would do | Import `Pin`, `Context`, `Poll`, `AsyncRead`, `ReadBuf`, `fmt`, `io`, `Ordering`, `Infallible`; use the in-scope `Value` and `CLOCK_TAG` in the test |

## Link

Files: src/link.rs, link/erased.rs, link/routed/. Entries: 14 (1 medium, 5 low, 8 nit).

### link-26: The routing table has no type of its own: the token claim is spelled twice, two clippy allowances stand in for a newtype, and a maintainer comment's premise is false as written
- Where: src/link/routed/router.rs:102-115 (related: src/link/routed/router.rs:46-69, src/link/routed/router.rs:249-260, src/link/routed/router.rs:58-59, src/link/routed/endpoint.rs:214, src/link/routed/tests.rs:188-232)
- Class / severity / confidence: modularity / medium / high
- Provenance: assessed (read both spellings, the alias, `entries()`, both `#[allow(clippy::type_complexity)]`, and the constructor at endpoint.rs:214)
- Seen by: structure (0), prose (23), perfapi (44); refutation: confirmed 0 (severity to low; noted the untested outbound-queue capacity), confirmed 23 (subsumed), reframed 44 (subsumed: a second alias to dodge the lint is the coining doctrine discourages); history: no rationale (both spellings from b16a800b; the allowances arrived with 4be4b830)
- Owner-gated: no

Claiming a token (create the `STREAM_COUNT`-bounded channel, insert the sender if the token is vacant, wrap in a `Registration`) is written once for outbound links in `register`, with an `Option`/`take()`/`expect` dance because the channel is created outside the loop, and once more, differently, in `deliver`'s `Link` arm, which holds the guard across `contains_key` then `insert`. That second spelling falsifies the premise of `entries`' doc ("Every critical section is a single map operation"); neither operation panics, so the conclusion holds, but the proof obligation as written is unmet. `Table<C>` is a semantic alias, `entries()` and `register` each carry `#[allow(clippy::type_complexity)]`, and `Endpoint::new` spells `Arc::new(Mutex::new(HashMap::new()))`, all symptoms of the table having no type. Newtypes over type synonyms; one spelling of one invariant. Drift exposure: `queue_overflow_evicts_the_link` floods only the inbound-created queue, so a capacity divergence in `register`'s copy would pass the suite today. I hold this at medium rather than the refutation's low because the false comment premise is a standing maintainer-facing inaccuracy, not only taste, and the same change fixes it.

Evidence:

    58	/// Every critical section is a single map operation, so a panic
    59	/// elsewhere cannot leave the map torn; continuing lets the surviving

    105	    let (sender, receiver) = mpsc::channel(STREAM_COUNT);
    106	    let mut sender = Some(sender);
    107	    let token = loop {
    108	        let token = Token::new();
    109	        if let Entry::Vacant(vacancy) = entries(table).entry(token) {
    110	            vacancy.insert(sender.take().expect("the loop ends at the first vacancy"));
    111	            break token;
    112	        }
    113	    };

    249	            let (sender, receiver) = mpsc::channel(STREAM_COUNT);
    250	            {
    251	                let mut entries = entries(table);
    252	                if entries.contains_key(&token) {
    253	                    // A duplicate establishment is a peer bug (tokens
    254	                    // are drawn fresh per link); dropping it leaves
    255	                    // the live link undisturbed.
    256	                    return Ok(());
    257	                }
    258	                entries.insert(token, sender);
    259	            }

Resolution: introduce `pub(super) struct Table<C>(Arc<Mutex<HashMap<Token, mpsc::Sender<(C, Done<C>)>>>>)` with `Default`, `Clone`, and methods: `fn claim(&self, token: Token) -> Option<(Registration<C>, mpsc::Receiver<(C, Done<C>)>)>` (creates the channel under one `Entry` critical section, `None` if occupied), `fn route(&self, token: &Token) -> Option<mpsc::Sender<..>>`, and `fn revoke(&self, token: &Token)` used by `Registration::drop`. `register` becomes `loop { let token = Token::new(); if let Some(claimed) = table.claim(token) { return (token, claimed); } }`; `deliver`'s `Link` arm becomes `let Some((registration, receiver)) = table.claim(token) else { return Ok(()); };`. `entries()` moves inside as a private method; both allowances go; `Endpoint::new` calls `Table::default()`; the `entries` doc premise becomes literally true. Add an overflow witness for the outbound (`register`-created) queue. Acceptance: `mpsc::channel(STREAM_COUNT)` and `Registration::new` each appear once in router.rs; no `#[allow(clippy::type_complexity)]` remains; endpoint.rs no longer names `HashMap`/`Mutex`; every guard scopes exactly one map call; `just gate` clean.

See also: inventory-6.

### api-audit-12: Twelve public types have no `Debug` impl
- Where: src/link.rs:322-329 (related: src/link.rs:482, src/link.rs:589-590, src/link.rs:610, src/batch.rs:27, src/rumors/unordered.rs:33, src/rumors/causal.rs:37, src/rumors/changes.rs:44, src/link/routed/endpoint.rs:141, src/link/routed/endpoint.rs:287, src/link/routed/stream.rs:24, src/link/routed/stream.rs:66, src/link.rs:345-346, src/link.rs:201-205)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rn -E 'impl(<[^>]*>)? (std::fmt::|fmt::)?Debug for|derive\([^)]*Debug' src` lists no impl in batch.rs, the three observer files, endpoint.rs, or stream.rs, and in link.rs only `Done` (201) and `SessionState` (345))
- Verification: confirmed; history: no-rationale-found
- Owner-gated: yes: adds impls

`Link`, `LinkParts`, `MemoryConnector`, `MemoryAcceptor`, `Batch`,
`UnorderedMessages`, `CausalMessages`, `Changes`, `routed::Endpoint`,
`routed::Incoming`, `routed::StreamConnector`, and `routed::StreamAcceptor`
implement no `Debug`, so none can sit in a `#[derive(Debug)]` struct or an
`expect` message without a wrapper. The crate knows how to do this for
opaque types (`Done`, `Bootstrap`, `Peer`, `Rumors` print summaries), and
`SessionState` is already the summary a `Link` would print.

Evidence:

    src/link.rs
    322	pub struct Link<CR, CW, C, A> {
    323	    pub(crate) control_read: CR,
    324	    pub(crate) control_write: CW,
    325	    pub(crate) connector: C,
    326	    pub(crate) acceptor: A,
    327	    /// This link's session counter and poison latch.
    328	    pub(crate) session: SessionState,
    329	}
    ...
    345	#[derive(Clone, Copy, Debug)]
    346	pub struct SessionState {

    src/link.rs
    589	#[derive(Clone)]
    590	pub struct MemoryConnector {

Resolution: add manual `Debug` impls printing a summary independent of the
type parameters (`Link { session: .. }`, `UnorderedMessages { checkpoint:
.. }`, `Changes { seen: .. }`, `Batch { queued: .. }`), and enable
`#![warn(missing_debug_implementations)]` so the class stays closed.
Acceptance: `cargo clippy -p rumors --lib -- -D warnings` passes with
`#![warn(missing_debug_implementations)]` in lib.rs.

See also: link-5.

### link-3: `STREAM_COUNT` is a literal 17 in two layers, held equal by a pin test, and the stated derivation is never asserted
- Where: src/link.rs:161-169 (related: src/link/tests.rs:11-19, src/tree/mirror/streaming/remote/codec/signal.rs:15-32, src/tree/mirror/streaming/remote/streams.rs:71, src/tree/mirror/streaming/remote.rs:87-91, src/tree/mirror/streaming/window.rs:124)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `= 17` appears at link.rs:169 and signal.rs:32 only; `codec_stream_count` is a `#[cfg(test)]` accessor whose sole caller is link/tests.rs:17; `crate::link::STREAM_COUNT` is imported by window.rs:124 and window/tests.rs:9, so the existing layering arrow runs tree to link)
- Seen by: structure (1), correctness (33); refutation: confirmed both, merged; history: deliberate-and-holds for the pin mechanism (b3b877d9), no rationale for literal over derivation
- Owner-gated: no (crate-internal; the resolution reaches into the codec partition)

The doc derives `ceil(32 / 2) + 1 = 17` and says the value is "pinned against the wire codec by test", but the pin compares two literals: `Stream::COUNT` is `17` at signal.rs:32, not computed from `STREAMED_HEIGHT_COUNT` and `STREAM_HEIGHT_STRIDE` beside it. The derivation is enforced only indirectly (the codec's height arithmetic would misbehave in its own tests at 16 or 18). No hand-maintained counts: a quantity computable from constants already in scope should be computed, and the pin test plus its test-only accessor then dissolve.

Evidence:

    165	/// protocol's own, fixed by its wire schedule (the descent's 32 tree
    166	/// heights at a two-height stride per stream, plus the shared opening
    167	/// stream: `ceil(32 / 2) + 1 = 17`) and pinned against the wire codec by
    168	/// test, so it cannot drift silently.
    169	pub const STREAM_COUNT: usize = 17;

    (src/link/tests.rs)
    13	#[test]
    14	fn stream_count_matches_the_codec() {
    15	    assert_eq!(
    16	        STREAM_COUNT,
    17	        usize::from(crate::tree::mirror::streaming::remote::codec_stream_count()),
    18	    );
    19	}

    (signal.rs)
    32	    pub const COUNT: u8 = 17;

Resolution: keep ownership in `link` (the direction the existing imports already run) and have the codec cite it: `pub const COUNT: u8 = crate::link::STREAM_COUNT as u8;` with `const _: () = assert!(STREAMED_HEIGHT_COUNT.div_ceil(STREAM_HEIGHT_STRIDE) + 1 == crate::link::STREAM_COUNT);` beside the schedule constants in signal.rs, so the compiler checks the arithmetic the doc states. Delete `stream_count_matches_the_codec` and the `#[cfg(test)] codec_stream_count()` accessor; consider retiring the private `STREAM_COUNT` alias at streams.rs:71. Flag to the codec partition's reviewer. Acceptance: `grep -rn '= 17' src/link.rs src/tree/mirror/streaming/remote/codec/signal.rs` returns exactly one line; a compile-time assertion ties the value to the stride and height constants; `codec_stream_count` is gone; `just gate` clean.

Synthesis note: remote-codec-3 (under Remote codec) reaches the same literal from the codec side and proposes the opposite ownership (the codec derives `COUNT` from its stride constants and `link` keeps its own literal plus the cross-layer pin). Both agree on the essential step, a compile-time assertion tying the value to `STREAMED_HEIGHT_COUNT` and `STREAM_HEIGHT_STRIDE`; whichever side owns the literal, the other should cite it by name rather than repeat it. This entry is the entry of record for the stream-count literal; remote-codec-3 is counted for the four other quantities it bundles.

### link-5: No `Debug` on `Link`, `LinkParts`, `Endpoint`, `Incoming`, or the four supply types; smaller common-trait gaps
- Where: src/link.rs:322-329 (related: src/link.rs:201-205, src/link.rs:482-500, src/link.rs:589-593, src/link.rs:610-612, src/link/routed/endpoint.rs:141-143, src/link/routed/endpoint.rs:287-289, src/link/routed/stream.rs:24-28, src/link/routed/stream.rs:66-70, src/link/routed/header.rs:97-98, src/link/routed/header.rs:129)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read every `derive` and `impl ... for` in the seven production files; no `Debug` impl exists for the listed types; `Token` derives `Clone, Copy, PartialEq, Eq, Hash` without `Ord`; `SessionState` and `Config` derive without `PartialEq`)
- Seen by: perfapi (42); refutation: confirmed; history: no rationale (no `missing_debug_implementations` lint; never ruled)
- Owner-gated: yes: public trait impls

`Done` has a hand-written `Debug` (link.rs:201-205), so the crate wants its transport types debuggable, yet `Link`, `LinkParts`, `MemoryConnector`, `MemoryAcceptor`, `Endpoint`, `Incoming`, `StreamConnector`, and `StreamAcceptor` implement none; a user struct holding a `RoutedLink<TcpDial>` cannot `#[derive(Debug)]`. Smaller: `Token` lacks `Ord` (cannot key a `BTreeMap`), `SessionState`, `Config`, and `LinkInfo` lack `PartialEq`, and `LinkInfo<A>: Debug` is conditional on an `A: Debug` bound `Addr` never asks for. The crate's own convention for opaque types is a manual `Debug` with `finish_non_exhaustive()` (message.rs, rumors.rs, peer.rs), which needs no bounds on the type parameters.

Evidence:

    322	pub struct Link<CR, CW, C, A> {
    323	    pub(crate) control_read: CR,
    324	    pub(crate) control_write: CW,
    325	    pub(crate) connector: C,
    326	    pub(crate) acceptor: A,
    327	    /// This link's session counter and poison latch.
    328	    pub(crate) session: SessionState,
    329	}

Resolution: manual `Debug` impls printing what is type-agnostic (`Link { session, .. }`, `LinkParts { session, .. }`, `MemoryConnector { capacity, .. }`, `Endpoint { local_addr, .. }` bounded on `D::Addr: Debug`, unit-style for `MemoryAcceptor`, `Incoming`, `StreamAcceptor`, `StreamConnector { token, .. }`), all via `finish_non_exhaustive()`. Derive `PartialEq, Eq` on `SessionState`, `Config`, `LinkInfo` and `PartialOrd, Ord` on `Token`. Consider `Debug` as an `Addr` supertrait. Acceptance: a test-only `#[derive(Debug)] struct Holder(RoutedLink<MemoryDial>)` compiles; `just gate` clean.

See also: api-audit-12.

### link-8: `Link` doubles as the one-session carrier, so `for_session` builds a `Link` whose poison flag is admittedly inert
- Where: src/link.rs:441-461 (related: src/peer/gossip.rs:85, src/peer/gossip.rs:1147, src/peer/gossip.rs:1206, src/peer/gossip/tests.rs:209, src/tree/mirror/streaming/remote/proxy/start.rs:69, src/tree/mirror/streaming/remote/proxy/start.rs:420-427)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep: three `for_session` callers; proxy/start.rs:420-427 destructures the carrier and reads only `session.epoch()`; `DynLinkParts` is a positional 5-tuple at gossip.rs:85)
- Seen by: structure (4); refutation: confirmed; history: no rationale (the inert-flag comment has admitted the mismatch since the flag existed)
- Owner-gated: no (crate-internal)

`Handshaking::start` takes a `Link`, so the funnels rebuild one from the erased 5-tuple and `open` reads only the epoch. The type means two things: the long-lived link with a live `SessionState`, and a per-session carrier where half of that state is meaningless, as the comment says. A field meaningless in one of a type's two uses is two types sharing one name; a carrier struct also gives `DynLinkParts` field names.

Evidence:

    453	            // The carrier's own poison flag is inert: it lives for one
    454	            // session and is discarded; the long-lived link's state is the
    455	            // one the funnels consult and clear.
    456	            session: SessionState {
    457	                epoch,
    458	                poisoned: false,
    459	            },

Resolution: design proposal, crate-internal: a `pub(crate) struct Carrier<CR, CW, C, A> { control_read, control_write, connector, acceptor, epoch: u8 }` in `link` (or `link::erased`), with `Link::carrier(self)` for tests that hand a fresh `memory()` link to the proxy; `Handshaking::start` takes the `Carrier`; `DynLinkParts<'a>` becomes `Carrier<DynRead<'a>, DynWrite<'a>, DynConnector, DynAcceptor<'a>>`; `for_session` and its comment go. Steelman for the status quo: 25 lines, zero test ceremony; the change touches proxy/start.rs, the proxy harness, gossip.rs, and gossip/tests.rs. Acceptance: no `Link` is constructed with a `SessionState` no session consults; the proxy's entry point names the epoch as a field.

### link-9: `LinkParts` is a field-for-field twin of `Link` that exists to publish the fields
- Where: src/link.rs:481-500 (related: src/link.rs:322-329, src/link.rs:463-479, src/link.rs:502-519, src/link.rs:346-361)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (grep: `into_parts()` appears on 63 lines across 26 files in src, tests, and examples, every one a decorate-and-rebuild; `SessionState`'s fields are private at link.rs:352,360)
- Seen by: structure (5); refutation: confirmed (migration larger than "about twenty sites"); history: deliberate-and-holds (created in b3b877d9 "for wrapper-building"; R40 sealed `SessionState` while keeping the parts type; why a twin rather than `pub` fields was never argued)
- Owner-gated: yes: public API change

`Link`'s fields are `pub(crate)`; `LinkParts` has the same five fields `pub`, and `into_parts`/`into_link` move them across. With `Link`'s fields public, the same decoration is a destructure-and-rebuild of one type, and the twin, the two methods, and their docs dissolve. Nothing new becomes forgeable: `SessionState` stays sealed, so a `Link` literal can only carry a `SessionState` obtained from a real link, exactly as `LinkParts` permits today. The one reason to decline is representation freedom for `Link`; if that is the ruling, the reason belongs in `Link`'s doc so the twin has a stated justification.

Evidence:

    481	/// The dismantled pieces of a [`Link`]; see [`Link::into_parts`].
    482	pub struct LinkParts<CR, CW, C, A> {
    483	    /// The control stream's read half.
    484	    pub control_read: CR,
    485	    /// The control stream's write half.
    486	    pub control_write: CW,
    487	    /// The outgoing stream supply.
    488	    pub connector: C,
    489	    /// The incoming stream supply.
    490	    pub acceptor: A,

Resolution: either make `Link`'s five fields `pub`, move `LinkParts`' field docs (especially the `session` preservation warning at 491-498) onto them under a `# Decorating a link` section, delete `LinkParts`/`into_parts`/`into_link`, and update the 63 sites mechanically; or keep `LinkParts` and state in `Link`'s doc why the fields stay private. Acceptance: `grep -rn LinkParts src tests examples` is empty, or `Link`'s doc states the reason the parts type exists beyond publishing the fields.

### Nits (7, and one cross-reference)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| inventory-18 | `src/link/routed/header.rs:247-257` | See link-25 (correctness), the entry of record for the truncating `as u8` behind a `debug_assert` at header.rs:254; not counted here | carried by link-25 |
| link-10 | `src/link/erased.rs:5-10` | The erasure funnel's vocabulary and rationale are split between `erased.rs` and `gossip.rs` | Move `DynRead`/`DynWrite` into `link::erased` as `pub(crate)`; state the erasure argument once there; gossip.rs imports them |
| link-17 | `src/link/routed/endpoint.rs:82-83` | `EndpointError` and `LinkError` lack `#[non_exhaustive]`, as do many other public enums; a crate-wide ruling is missing | rule crate-wide which public enums are open taxonomies |
| link-19 | `src/link/routed/endpoint.rs:145-151` | Hand-written `Clone` impls where `#[derive(Clone)]` produces the same impl | replace both with `#[derive(Clone)]` on the struct definitions (endpoint.rs:141, stream.rs:24) |
| link-20 | `src/link/routed/endpoint.rs:208-213` | `Config`'s two zero-bound errors are what `NonZeroUsize` fields make unrepresentable | owner call |
| link-22 | `src/link/routed/header.rs:59` | `PREFIX_LEN` is `pub(super)` but used only inside `header`, and the layout it heads is spelled three times by offset arithmetic | drop `pub(super)` |
| link-24 | `src/link/routed/header.rs:211-215` | `SocketAddr::decode` hand-checks the length and then `expect`s twice | Split with `split_first_chunk::<16>()` and a fallible two-byte tail so the length check is structural and both `expect`s go |
| link-30 | `src/link/routed/router.rs:188-202` | `route` exists only to discard `deliver`'s error and echo an id | Inline `deliver(..).map(move \|_\| id)` at the push site with the discarded-error comment; delete `route` |

## Conformance

Files: src/conformance/. Entries: 13 (1 medium, 6 low, 6 nit).

### conformance-9: The two independence probes are one probe parameterized by the stalled count
- Where: src/conformance/link.rs:465-678 (related: src/conformance/link.rs:521-524, src/conformance/link.rs:573-575, src/conformance/link.rs:631-636, src/conformance/link/tests.rs:889-896, src/conformance/link/tests.rs:993-1000)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read both probes in full; the receive loops at 529-560 and 641-668 differ only in `Option` versus `Vec` and the final count, the send halves only in how many stalled streams the pressure `join_all`s)
- Seen by: structure; refutation: confirmed; history: no rationale found (32813f55 added the pooled shape whole beside the existing probe and argued why the shape exists, not why it is a second function)
- Owner-gated: no

`probe_independence` (one stalled stream, `STREAM_COUNT - 1` live) and `probe_independence_pooled` (`STREAM_COUNT - 1` stalled, one live) are near-clones: about ninety lines kept in sync by hand, and already drifted. The never-completing `select` arm is `Either::Left((never, _)) => never` at 522 (a no-op, since the pressure block's output unifies with `()`) and `unreachable!(...)` at 632-634; `STALLED_COMPLEMENT` sits mid-file at 575 while every sibling constant lives in the 69-145 block. The suite is public production code and the deadlock-freedom argument rests on this clause; one probe whose only free variable is the stalled count makes the two coupling classes read as two points on one axis.

Evidence:

       529	        for _ in 0..STREAM_COUNT {
       530	            let (mut rx, _) = acceptor
       531	                .accept()
       532	                .await
       533	                .expect("contract: later streams are accepted beside a stalled one");
       534	            let mut tag = [0u8; 1];
       535	            rx.read_exact(&mut tag)
       536	                .await
       537	                .expect("contract: every stream's first byte is delivered");
       538	            match tag[0] {

       641	        for _ in 0..STREAM_COUNT {
       642	            let (mut rx, _) = acceptor
       643	                .accept()
       644	                .await
       645	                .expect("contract: later streams are accepted beside stalled ones");
       646	            let mut tag = [0u8; 1];
       647	            rx.read_exact(&mut tag)
       648	                .await
       649	                .expect("contract: every stream's first byte is delivered");
       650	            match tag[0] {

Resolution: Collapse to `async fn probe_independence<C: Connector, A: Acceptor>(connector: &C, acceptor: &mut A, stalled: usize)`. Sender: open `stalled` tagged streams into a `Vec`, `join_all` the pressure loops over them (a one-element `join_all` is the single case), open `STREAM_COUNT - stalled` live streams, one `select` with a single never-completing arm. Receiver: hold stalled receivers in a `Vec`, assert `held.len() == stalled` and `live_seen == STREAM_COUNT - stalled` (subsuming the single case's exactly-one assertion). `check_independence` calls it with `1` and `STALLED_COMPLEMENT` per direction; move `STALLED_COMPLEMENT` into the constant block; carry the two shapes' rationale as the parameter's doc plus one comment per call site. Acceptance: `check_independence` still runs four probes; `memory_link_conforms`, `one_byte_windows_conform`, `reordering_acceptor_passes_independence`, `never_binding_pooled_budget_conforms` pass; `shared_mux_coupling_is_caught` and `pooled_budget_below_the_bound_is_caught` still report `Err(Quiescence::Stalled)`.

### conformance-7: Eight public signatures restate the same eight type parameters and eight bounds; the recorded reason lives only in a commit message
- Where: src/conformance/link.rs:158-169 (related: src/conformance/link.rs:191-203, src/conformance/link.rs:254-266, src/conformance/link.rs:332-344, src/conformance/link.rs:435-447, src/conformance/link.rs:709-721, src/conformance/link.rs:816-828, src/conformance/link.rs:1019-1031, src/link.rs:322-329, src/link.rs:435-478, src/link.rs:502-518)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (eight `pub async fn check*` signatures each carry the identical eight-line where clause; `pub struct Link<CR, CW, C, A>` at src/link.rs:322 carries no bounds and `into_parts` sits in the unbounded `impl<CR, CW, C, A> Link<...>` at 435-478, while `LinkParts::into_link` at 502-518 requires the full set)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate and holds (f5039abb: "Left alone deliberately: the five check_* functions' repeated 8-type-parameter signatures (the honest shape of Link's four parameters per end; a bounds-alias trait would add public API without shrinking the lists, and a macro would hide the rustdoc-visible signatures deployments read)")
- Owner-gated: yes: reopens a recorded ruling, and one alternative adds public API

The rendered rustdoc of every check is a screen of bounds that carry no per-check information; the decision to accept that was made and argued in f5039abb, but nothing at the site says so, and the sealed-helper-trait alternative (one impl, for `Link<CR, CW, C, A>`, exposing the four associated types and `into_parts`) is a different mechanism from the bounds alias that ruling rejected, since it does shrink the lists. Independently of that ruling, the bounds overstate what the focused checks need: they reach the halves through the unbounded `into_parts`, so `check_control` and `check_control_duplex` need only `AsyncRead`/`AsyncWrite + Unpin` on the halves, the four stream-only checks need no control-half bounds at all, and only `check_sessions` (through `counting` and `into_link`) needs the full set.

Evidence:

       158	pub async fn check<CRa, CWa, Ca, Aa, CRb, CWb, Cb, Ab>(
       159	    mut pair: impl AsyncFnMut() -> (Link<CRa, CWa, Ca, Aa>, Link<CRb, CWb, Cb, Ab>),
       160	) where
       161	    CRa: AsyncRead + Unpin + Send,
       162	    CWa: AsyncWrite + Unpin + Send,
       163	    Ca: Connector,
       164	    Aa: Acceptor,
       165	    CRb: AsyncRead + Unpin + Send,
       166	    CWb: AsyncWrite + Unpin + Send,
       167	    Cb: Connector,
       168	    Ab: Acceptor,

Resolution: Owner's call among three: (1) state f5039abb's rationale in a maintainer comment above `check` and leave the shape; (2) a sealed `LinkEnd` trait implemented once for `Link<CR, CW, C, A>` under the existing bounds, so each check reads `check_x<A: LinkEnd, B: LinkEnd>(a: A, b: B)`; (3) the non-breaking loosening: drop `Send` and the unused control-half bounds from the focused checks, keeping the full set on `check` and `check_sessions`. Acceptance: the shape is either argued at the site or changed; if (3), each focused check demands only the bounds its body uses and the in-tree consumers compile unchanged.

### conformance-11: `yield_once` is byte-identical to `testing::transport`'s copy, and both give a reason for existing that does not survive checking
- Where: src/conformance/link.rs:680-696 (related: src/testing/transport.rs:723-741, src/conformance/link/tests.rs:172, Cargo.toml:112, Cargo.toml:137)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (bodies at link.rs:685-696 and transport.rs:730-741 are identical; in tokio 1.52.3 from Cargo.lock, `mod yield_now; pub use yield_now::yield_now;` sits inside `cfg_rt! { ... }` at task/mod.rs:277-291 and `context::defer` at runtime/context.rs:168-176 falls back to `waker.wake_by_ref()` outside a runtime; Cargo.toml:137 gives the library tokio `io-util, macros, sync` and Cargo.toml:112 `test-internals = ["tokio/rt"]`)
- Seen by: structure; refutation: confirmed; history: no rationale found beyond the doc sentences themselves
- Owner-gated: no

Both docs justify the hand-roll as "Runtime-agnostic ... unlike `tokio::task::yield_now`", but tokio's `yield_now` is runtime-agnostic too. What excludes it from link.rs is that the library's tokio feature set omits `rt`, which gates `yield_now`; in transport.rs, where `test-internals` lights `rt`, tokio's own function is available. A reader who trusts the doc believes tokio's yield is unusable here for a reason that is false and never learns the true constraint that decides where a shared helper may live.

Evidence:

       680	/// Yield to the executor exactly once: `Pending` with an immediate
       681	/// self-wake.
       682	///
       683	/// Runtime-agnostic (the suite runs on the caller's executor, which may be
       684	/// no runtime at all), unlike `tokio::task::yield_now`.
       685	async fn yield_once() {

    src/testing/transport.rs:
       726	/// Runtime-agnostic (the deterministic driver is no runtime at all), unlike
       727	/// `tokio::task::yield_now`; a copy of `conformance`'s helper, on the same

Resolution: One `pub(crate) async fn yield_once()` in a module both callers reach (src/link.rs is the natural host, gated `#[cfg(any(test, feature = "conformance", feature = "test-internals"))]`), with the doc stating the real constraint: `tokio::task::yield_now` needs tokio's `rt` feature, which the library build does not enable, and the deterministic driver is not a tokio runtime, so the helper self-wakes. Import it at both sites; `LossyAcceptor` at link/tests.rs:172 follows. Acceptance: one definition of `yield_once` in src/; both suites compile under `--no-default-features --features conformance` and under `test-internals` alone; no doc claims `yield_now` is runtime-bound.

See also: conformance-18.

### conformance-14: Hand-rolled poll-once where `now_or_never` and `futures::poll!` are the crate's idiom
- Where: src/conformance/link.rs:847-853 (related: src/conformance/link.rs:61, src/conformance/link.rs:908-923, src/conformance/link/tests.rs:72-77, src/link/routed/tests.rs:430, src/tree/mirror/streaming/remote/proxy/work.rs:231, src/testing/transport.rs:828, Cargo.toml:52)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (grep locates `now_or_never` at src/link/routed/tests.rs:430 and across src/rumors/{changes,unordered,causal}.rs, and `futures::poll!` at proxy/work.rs:231 and transport.rs:828; Cargo.toml:52 enables futures' `async-await`, which `poll!` needs; equivalence of the rewrites assessed by reading, not compiled)
- Seen by: structure; refutation: confirmed; history: no rationale found; one constraint on record: d263a91d and the comment at link.rs:908-909 require the poll-drop cycle to use the real waker, which `futures::poll!` preserves and `now_or_never` would not
- Owner-gated: no

Three sites build a noop waker or a `poll_fn` to poll a future exactly once: link.rs:847-853 (assert a fresh accept is pending), link.rs:916-923 (poll a fresh accept once with the real waker), and link/tests.rs:72-77 (`ReversingAcceptor` draining what is `Ready`). `FutureExt::now_or_never` is the first and third shape; `futures::poll!` is the second. A reader who knows those idioms from elsewhere in rumors has to re-derive that these seven lines mean the same thing.

Evidence:

       847	        let mut pending = pin!(acceptor.accept());
       848	        let waker = futures::task::noop_waker();
       849	        let mut cx = Context::from_waker(&waker);
       850	        assert!(
       851	            pending.as_mut().poll(&mut cx).is_pending(),
       852	            "no stream was opened yet",
       853	        );

       916	            let polled_once = std::future::poll_fn(|cx| {
       917	                let mut accept = pin!(acceptor.accept());
       918	                Poll::Ready(match accept.as_mut().poll(cx) {
       919	                    Poll::Ready(rx) => Some(rx),
       920	                    Poll::Pending => None,
       921	                })
       922	            })
       923	            .await;

Resolution: 847-853: `assert!(acceptor.accept().now_or_never().is_none(), "no stream was opened yet");`. 916-923: `let polled_once = futures::poll!(pin!(acceptor.accept()));` as its own statement (so the fresh accept future drops at the statement's end, before the yield, preserving the current drop timing), then match `Poll::Ready`/`Poll::Pending`. link/tests.rs:72-77 dissolves with conformance-18, else `match self.inner.accept().now_or_never() { Some(Ok(rx)) => ..., _ => break }`. Prune the then-unused `Context` import at link.rs:61. Acceptance: no `noop_waker` or `poll_fn` remains in src/conformance/link.rs; `memory_link_conforms`, `lossy_accept_cancellation_is_caught`, and `asymmetric_lossiness_is_caught` keep their verdicts.

### conformance-18: `ReversingAcceptor` duplicates `testing::ReorderingAcceptor`, which this file already imports from
- Where: src/conformance/link/tests.rs:41-124 (related: src/conformance/link/tests.rs:24, src/conformance/link/tests.rs:134-150, src/conformance/link/tests.rs:907-920, src/testing.rs:7-12, src/testing/transport.rs:646-720, src/testing/transport.rs:667-669, src/lib.rs:319-321)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (src/testing.rs:10 re-exports `ReorderingAcceptor, reorder_accepts`; src/lib.rs:319-321 compiles `testing` under `cfg(any(test, feature = "test-internals"))`; link/tests.rs:24 already imports `crate::testing::{Quiescence, run_to_quiescence}`; transport.rs:646-720 read in full. The suite's behavior under the patience-waiting variant is assessed, not run.)
- Seen by: structure; refutation: confirmed; history: deliberate and holds (f5039abb lists the duplication under "Left alone deliberately", citing transport.rs's feature-isolation rationale; cbc4a0aa made `ReorderingAcceptor` wait because the Ready-only drain "was empirically pass-through" in the proxy topology)
- Owner-gated: yes: reopens a recorded ruling

The recorded rationale is one-directional: it keeps `testing` from depending on the public `conformance` feature. The direction the conformance tests need runs the other way, and this file already takes it at line 24. The one behavioral difference is that `ReorderingAcceptor` waits up to `REORDER_PATIENCE` yields for company while `ReversingAcceptor` takes only what is `Ready`; the patient variant subsumes the Ready-only one (it forms batches wherever the Ready-only one does), both are legal acceptors, and the `reordered > 0` assertions at 146-149 and 916-919 already guard against degeneration. Two reordering adversities with subtly different semantics is one more than the crate needs, and if the two are kept, the transport.rs comment should carry the actual reason they differ.

Evidence:

        50	struct ReversingAcceptor<A: Acceptor> {
        51	    inner: A,
        52	    held: VecDeque<(A::Rx, Done<A::Rx>)>,
        53	    /// Arrivals buffered before each reversed release.
        54	    batch: usize,
        55	    /// Batches of two or more released: genuine inversions.
        56	    reordered: Arc<AtomicUsize>,
        57	}

    src/testing/transport.rs:
       667	/// A sibling of the conformance suite's `ReversingAcceptor`
       668	/// (`src/conformance/link/tests.rs`), duplicated so this crate-internal seam
       669	/// does not depend on the public `conformance` feature.

Resolution: Delete `ReversingAcceptor` and `reversing`; in `reordered_accepts_conform` and `reordering_acceptor_passes_independence` call `crate::testing::reorder_accepts(a, 3, counter.clone())`, confirming `reordered > 0` still holds and the negative controls keep their verdicts under the patient wait. Then restate the transport.rs sibling comment without the reference (outside this partition). If the owner keeps two, the comment at transport.rs:667-669 should say why they differ (Ready-only drain for concurrently connected probes, patient wait for the proxy topology). Acceptance: `grep -rn ReversingAcceptor src` is empty and the two tests pass with nonzero `reordered`, or the rationale for two fixtures is stated where the duplicate is declared.

See also: conformance-11.

### conformance-33: `bounds_of` in the tests duplicates `bound_bytes` in the suite
- Where: src/conformance/backend/tests.rs:260-263 (related: src/conformance/backend/tests.rs:255, src/conformance/backend/tests.rs:335, src/conformance/backend/tests.rs:351, src/conformance/backend.rs:268-275, src/conformance/backend.rs:348-351, src/conformance/backend.rs:522-526, src/tree/mirror/streaming/backend/local.rs:34-57, src/tree/typed/node.rs:178-182)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`impl<H: Height> Node for typed::Node<H>` at local.rs:34-57 delegates `span()` to the inherent method, documented at node.rs:178-179 as "the memoized `[floor, ceiling]` pair", so `span().hi()`/`lo()` are `ceiling()`/`floor()` and the two functions compute one number)
- Seen by: structure; refutation: confirmed; history: no rationale found (both born in 922db57a; 68194d19 rewrote `bound_bytes` onto `span()` and left `bounds_of`)
- Owner-gated: no

The reference backend's row pricing should visibly use the same bound arithmetic the suite checks it against, so a reader cannot wonder whether the two differ; a child module reaches the parent's private fn as `super::bound_bytes`. backend.rs also spells "max of the two bound encodings" twice (349-351, 523-526).

Evidence:

       260	/// The two encoded bounds of a freshly built node, in bytes.
       261	fn bounds_of<H: Height>(node: &typed::Node<H>) -> usize {
       262	    node.ceiling().as_bytes().len() + node.floor().as_bytes().len()
       263	}

    src/conformance/backend.rs:
       268	/// The two encoded version bounds a node keeps resident, in bytes.
       269	fn bound_bytes<N>(node: &N) -> usize
       270	where
       271	    N: Node,
       272	{
       273	    let bounds = node.span();
       274	    bounds.hi().as_bytes().len() + bounds.lo().as_bytes().len()
       275	}

Resolution: Delete `bounds_of`; call `super::bound_bytes(&node)` at 255, 335, 351. In backend.rs, name the repeated max once (`fn bound_version_bytes(node: &impl Node) -> usize`) and use it at 349-351 and 523-526. Acceptance: one bound-bytes helper and one bound-max helper in the module; `materializing_backend_conforms` and its controls unchanged.

### conformance-36: Dead `Charged` value in the ledger test, and a long path where `Z` is already imported
- Where: src/conformance/backend/tests.rs:570-573 (related: src/conformance/backend/tests.rs:14, src/conformance/backend/tests.rs:24, src/conformance/backend/tests.rs:555, src/conformance/backend.rs:145-149)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git blame -L 570,573` attributes 572-573 to 922db57a, and `git show 922db57a:src/conformance/backend/tests.rs` has the same dead pair at its lines 244-245; `Z` is imported at line 24 and used bare at 555; `Charged::new` at backend.rs:145-149 charges nothing)
- Seen by: structure, correctness, perfapi; refutation: confirmed; history: no rationale found (dead from the suite's first commit)
- Owner-gated: no

`charged` is constructed and then only borrowed into `let _`; nothing reads it, and its presence makes a reader check whether `Charged::new` charges the ledger (it does not). Removing it retires the `Charged` import at line 14. The line above spells `crate::tree::typed::height::Z` where `Z` is imported and used bare fifteen lines earlier.

Evidence:

       570	    let node: typed::Node<crate::tree::typed::height::Z> =
       571	        typed::Node::leaf(Version::new(), Message::new(7));
       572	    let charged = Charged::<Local>::new(Local);
       573	    let _ = &charged;

Resolution: Delete lines 572-573; drop `Charged` from the `use super::{...}` at line 14; write `typed::Node<Z>` at 570. Acceptance: `ledger_settles_over_clone_and_drop` compiles without the `Charged` import and passes; no `let _ = &` remains in the file.

### Nits (6)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| conformance-13 | `src/conformance/link.rs:745-745` | `index as u8` ties the in-band index to `STREAM_COUNT <= 256` with nothing at compile time | `u8::try_from(index).expect("STREAM_COUNT fits an index byte")`, or a `const _: () = assert!(STREAM_COUNT <= u8::MAX as usize + 1);` beside the probe |
| conformance-15 | `src/conformance/link.rs:964-971` | Manual `Clone` impls identical to the derive, in a file whose other connectors derive it | Replace both with `#[derive(Clone)]` |
| conformance-16 | `src/conformance/link.rs:985-1007` | Five `LinkParts` rebuilds spell the same five fields | Local option: a `with_connector` twin of `with_acceptor` |
| conformance-21 | `src/conformance/link/tests.rs:1049-1057` | Vestigial braces around `send_all`, and qualified paths where the production sibling imports | Remove the braces |
| conformance-27 | `src/conformance/backend.rs:584-588` | Redundant `+ Clone` on `B: Measure` at five sites; `node_bytes_monotone` demands a `Debug` bound it never uses | Drop `+ Clone` at 586, 648, 675, 731, 752 |
| module-graph-11 | `src/conformance.rs:18-19` | Redundant `#[cfg(test)]` gates under test-only parents, and a `pub(crate)` that widens nothing | Either delete the six inner gates or record once, in AGENTS.md's testing section, that every `mod tests;` carries `#[cfg(test)]` regardless of context |

## Tree core

Files: src/tree.rs, tree/traverse/, tree/arb.rs, tree/tests.rs. Entries: 15 (1 medium, 4 low, 10 nit).

### tree-core-22: `arb_divergent_pair` and `arb_wide_divergent_pair` duplicate a 30-line generator body, and `ESCAPE_MARGIN` is declared twice
- Where: src/tree/arb.rs:193-230 (related: src/tree/arb.rs:132-172, 427-431, 534-541; src/tree/mirror/streaming/remote/proxy/tests.rs:26, 435, 539)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (read both bodies: 142-171 and 203-229 are identical apart from the comment at 147-149; only the five tuple-strategy literals at 136-140 versus 197-201 differ; `grep ESCAPE_MARGIN src/` finds two local `const ESCAPE_MARGIN: usize = 64;` at 431 and 541 with near-identical docs; `arb_wide_divergent_pair` has two callers in the proxy suite)
- Seen by: structure (4); refutation: confirmed; history: no-rationale-found (b3b877d9b added the wide variant as a verbatim copy; the two constants came in separate same-day commits, d29485fd0 and 7c9175a0b)
- Owner-gated: no

The `side` closure encodes the redaction semantics every merge property and every streaming differential suite depends on, and it exists twice; one edit to it must be made twice or the two generators diverge without anyone noticing. Named constants over repeated literals applies to the two `ESCAPE_MARGIN`s, whose docs already say the same thing.

Evidence:

    193    pub fn arb_wide_divergent_pair() -> BoxedStrategy<(crate::tree::Root, crate::tree::Root)> {
    194        use crate::tree::{Action, Tree};
    195
    196        (
    197            0usize..12,                // shared inserts (the common base)
    198            0usize..40,                // a-only inserts
    199            0usize..40,                // b-only inserts
    200            vec(any::<bool>(), 0..12), // which shared keys side a redacts
    201            vec(any::<bool>(), 0..12), // which shared keys side b redacts
    202        )
    203            .prop_map(|(n_shared, n_a, n_b, a_redact, b_redact)| {
    ...
    215                let side = |party: &Party, n: usize, redact: &[bool]| {
    216                    let mut t = base.clone();
    217                    t.act(party, (0..n).map(|_| Action::Insert(Message::new(()))));

Resolution: Introduce one private builder, e.g. `fn divergent_pair(shared: Range<usize>, per_side: Range<usize>) -> BoxedStrategy<(Root, Root)>` whose redact-mask length tracks `shared.end`, and make `arb_divergent_pair()` and `arb_wide_divergent_pair()` one-line wrappers keeping their current docs (the wide doc's budget-versus-bias argument is the valuable part). Hoist `ESCAPE_MARGIN` to one module-level constant with one doc. Acceptance: one `side` closure in the file; both public names keep their signatures and docs; the proxy suite compiles unchanged; one `ESCAPE_MARGIN`.

### tree-core-4: Erasure residue in `tree.rs`: a derivable hand-written `PartialEq for Root` and three unused `T: Send + Sync` where-clauses
- Where: src/tree.rs:131-135 (related: src/tree.rs:107-111, 419-423, 477-481, 567-570; src/tree/traverse/act.rs:39-43; src/tree/traverse/join.rs:57-63)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the three method bodies, 424-452, 483-540, 571-612: none names `T`; both callees `traverse::act` and `traverse::join` are non-generic in `T`; `Root` is non-generic, so the manual `eq` is field-for-field what the derive produces)
- Seen by: structure (1), perfapi (53), correctness (41); refutation: confirmed; history: deliberate-but-expired (the bounds made the then-async traversal futures `Send`, edea11ac and 329d891b; 262568f9e made the traversals synchronous and b524e406 erased `T` from storage, both leaving the bounds; the manual `PartialEq` was written in 300e2298 for the then-generic `Root<P, T>` and b524e406 made `Root` bare while keeping the impl)
- Owner-gated: no

`Tree::act`, `Tree::react`, and `Tree::join` each carry `where T: Send + Sync` that nothing in their bodies uses: the messages they move were constructed already, and the traversal entries take `Option<Node<Root>>` and `Message` with no `T` in sight. `Root` gained `#[derive(Clone, Debug, Eq)]` when it lost its type parameter, but the hand-written `PartialEq` beside it stayed, and a manual impl beside a derive line invites the reader to look for a subtlety that is not there (contrast `Tree<T>`'s manual impls, which exist to avoid `T` bounds). Machinery outlives the constraint that justified it; the audit recurs at each phase boundary.

Evidence:

    107    #[derive(Clone, Debug, Eq)]
    108    pub struct Root {
    ...
    131    impl PartialEq for Root {
    132        fn eq(&self, other: &Self) -> bool {
    133            self.ceiling == other.ceiling && self.root == other.root
    134        }
    135    }
    ...
    419        pub fn act<I>(&mut self, party: &before::Party, actions: I) -> bool
    420        where
    421            T: Send + Sync,
    422            I: IntoIterator<Item = Action>,
    ...
    477        fn react<M, I>(&mut self, reactions: I) -> bool
    478        where
    479            T: Send + Sync,
    ...
    567        pub fn join(&mut self, other: Tree<T>) -> bool
    568        where
    569            T: Send + Sync,
    570        {

Resolution: Change `Root`'s attribute to `#[derive(Clone, Debug, PartialEq, Eq)]` and delete lines 131-135; delete the three `where T: Send + Sync` clauses. Leave `Tree<T>`'s manual `Clone`/`PartialEq`/`Default` (137-156), which the phantom justifies. Acceptance: the crate compiles; the production callers `batch.rs:143` and `peer/gossip.rs:869` are untouched; no method in `tree.rs` names a bound its body does not use.

See also: tree-core-14.

### tree-core-7: `Tree::hash` clones the whole `Root` through a one-caller `From` impl to borrow a field
- Where: src/tree.rs:273-278 (related: src/tree.rs:113-117, src/tree/typed/node.rs:409-417)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep for `Option<typed::node::Root>` and `root.clone().into()` over `src/`: the `From<Root> for Option<typed::node::Root>` impl has exactly one consumer, tree.rs:277; the other root `.into()` calls target `StreamingRoot<Local>`; `Node::root_hash` takes `&Option<Root>`, node.rs:410)
- Seen by: structure (2), correctness (40), perfapi (46); refutation: confirmed; history: no-rationale-found (both the line and the impl date to 60a3b0569, whose message does not mention the conversion)
- Owner-gated: no

`hash()` clones the `Root` (a `Version` handle bump and a node `Arc` bump, then two drops), converts it by value through the `From` impl, and borrows the result, when `&self.root.root` is already exactly the `&Option<typed::node::Root>` that `Node::root_hash` takes. The `From` impl exists only to serve this line. `hash()` is the read the crate meters inside commit critical sections (616-623), so gratuitous work here runs in the wrong place, and the conversion implies work it does not do.

Evidence:

    113    impl From<Root> for Option<typed::node::Root> {
    114        fn from(value: Root) -> Self {
    115            value.root
    116        }
    117    }
    ...
    273        /// Returns the root hash for the tree.
    274        pub fn hash(&self) -> [u8; MERKLE_HASH_LEN] {
    275            #[cfg(test)]
    276            meter::record_root_hash_read();
    277            Node::root_hash(&self.root.clone().into()).into()
    278        }

Resolution: `Node::root_hash(&self.root.root).into()`; delete the `From<Root> for Option<typed::node::Root>` impl. Acceptance: no `.into()` on a `tree::Root` remains in the crate; `hash()` performs no clone; `empty_tree_hash_matches_reference` and the `root_hash_read_meter_is_live` pins in `crate::tests` still pass (the meter call precedes the read).

See also: tree-typed-18.

### tree-core-14: Insert-or-forget has three spellings on one call path, and `react`'s `M: Into<Option<Message>>` serves only test call sites
- Where: src/tree.rs:477-497 (related: src/tree.rs:158-165, 436-452; src/tree/traverse/act.rs:8-15; src/tree/arb.rs:5, 133, 194, 307; src/tree/tests.rs:98-105)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (`grep -rn '\.react(' src tests` outside `tree/tests.rs`: `Tree::react`'s only production caller is `act`, tree.rs:436; the `levels.rs` hits are `resolver.react`, a different method; `Into<Option<` appears once in the crate, tree.rs:480; `arb.rs` imports `traverse::{Action, act}` at line 5 and re-imports `crate::tree::{Action, Tree}` inside three bodies)
- Seen by: structure (5), perfapi (54); refutation: confirmed; history: deliberate-but-expired (47c9c9013 added the generic so callers of the then-public `react` could pass `unknown()`'s output directly; `react` went private in 8e348feff)
- Owner-gated: no

`tree::Action { Insert(Message), Forget(Path) }` is converted by `act` into `Option<Message>` (447-451), which `react` accepts through a generic `M: Into<Option<Message>>` and converts back into `traverse::Action { Insert(Message), Forget }` (493-496) before the walk. Production has one caller of `react` (`act`, with `M = Option<Message>`); the generic exists so tests can pass a bare `Message`. The middle representation and the type parameter exist only to bridge the other two, and the two enums sharing the name `Action` force qualified paths and function-local imports wherever both are in scope.

Evidence:

    477        fn react<M, I>(&mut self, reactions: I) -> bool
    478        where
    479            T: Send + Sync,
    480            M: Into<Option<Message>>,
    481            I: IntoIterator<Item = (typed::Path, Version, M)>,
    ...
    491            let actions: Vec<_> = reactions
    492                .into_iter()
    493                .map(|(path, version, message)| match message.into() {
    494                    None => (path, version, traverse::Action::Forget),
    495                    Some(value) => (path, version, traverse::Action::Insert(value)),
    496                })
    497                .collect();

    src/tree/arb.rs
    5     use crate::tree::traverse::{Action, act};
    ...
    133        use crate::tree::{Action, Tree};

Resolution: Have `react` take `I: IntoIterator<Item = (typed::Path, Version, traverse::Action)>` and drop `M`; build `traverse::Action::Insert(value)` / `traverse::Action::Forget` directly in `act` at 447-450 so the `match` at 493-496 becomes a plain `collect()`. Tests then construct `traverse::Action` in the `insert_at`/`event` helpers (the dozen inline `(path, version, msg)` tuples at tests.rs:1701, 1712, 1810, 1813, 1839, 1841, 1865, 1867, 1882, 1883, 1895, 1896 and the `event` closures at 258-264 and 280-284 change). The reverse direction (drop `traverse::Action` for `Option<Message>`, since they are isomorphic) also removes one spelling. Whichever enum survives, give the internal one a distinct name so `arb.rs` needs no function-local imports. Acceptance: `react` has one type parameter; no `Into<Option<Message>>` in the crate; `arb.rs` has no `use crate::tree::{Action, Tree};` inside function bodies.

See also: tree-core-4.

### tree-core-31: `join`'s divergent arm clones the fan three times over and hand-rolls the two-way merge that `itertools::merge_join_by` spells in a `match`
- Where: src/tree/traverse/join.rs:150-159 (related: src/tree/traverse/join.rs:160-195; src/tree/typed/node.rs:41-86; src/tree/typed/untyped/fan.rs:14-20 and 154-160; src/tree/mirror/streaming/materialized/work/answer.rs:1, 58-86, 129-146; src/tree/traverse/unknown.rs:65-73)
- Class / severity / confidence: simplification / low / high
- Provenance: verified for the mechanism (`merged = ours.clone()` bumps k handles; `Children::iter` clones every child, node.rs:81-85, on both fans, including pairs then pruned at 179-183; `merged.insert`/`remove` are binary-search edits; `Children` exposes `insert`/`remove`/`iter` but no `push`, while `Fan::push` exists, fan.rs:154-160; `merge_join_by`/`EitherOrBoth` is already the crate's spelling of this shape in `answer.rs`); assessed for magnitude
- Seen by: structure (3), perfapi (47); refutation: confirmed (one rewrite satisfies both: a consuming `merge_join_by` feeding `push`); history: no-rationale-found for the hand-rolled loop (f5426c9d6 replaced imbl's `OrdMap::diff` after jneem/imbl#161 without considering itertools); deliberate-but-expired for the comment (written for a persistent `OrdMap` where `merged = ours.clone()` was O(1); f70f9559a replaced `OrdMap` with the non-persistent `Fan` whose doc states the premise the comment now contradicts)
- Owner-gated: no

For every divergent branch the walk clones `ours`' whole fan into `merged`, iterates both fans through `Children::iter` (which clones every child again, including pairs it then prunes as equal and drops), and patches `merged` by `insert`/`remove` with element shifts. The comment justifies starting from `ours` by "structural sharing", but sharing lives on the node handles, not the fan: `fan.rs:14-16` says `Fan` "is deliberately *not* a persistent map" and cloning it is "one refcount bump per child". The loop itself is two `Peekable`s, a four-arm `match` on `(peek, peek)`, `min`, two `next_if` calls, and `ours`/`theirs` shadowed from fan to iterator: the one place in the file where a reader simulates iterator state to trust the walk. Both fans are ascending by construction, which is exactly `merge_join_by`'s precondition, and itertools is already a dependency this crate uses for the same shape. A fresh fan built by ascending `push` from consuming iterators holds the identical handles with no extra bumps and no `insert`/`remove`, and reads as evidently correct.

Evidence:

    150                    // The walk reads both fans directly rather than diffing the
    151                    // two maps against each other: the merged map starts from
    152                    // *ours* and only divergent radixes are rewritten, so every
    153                    // shared child persists by structural sharing.
    154                    let ours = ours.into_children();
    155                    let theirs = theirs.into_children();
    156
    157                    let mut merged = ours.clone();
    158                    let mut ours = ours.iter().peekable();
    159                    let mut theirs = theirs.iter().peekable();
    160                    loop {
    161                        let (radix, our_child, their_child) = match (ours.peek(), theirs.peek()) {
    162                            (None, None) => break,
    ...
    169                            (Some((ours_radix, _)), Some((theirs_radix, _))) => {
    170                                let radix = (*ours_radix).min(*theirs_radix);

    src/tree/typed/untyped/fan.rs
    14    //! [`Fan`] is deliberately *not* a persistent map. Structural sharing lives
    15    //! one level up, on the node handles the fan stores (each entry is one
    16    //! `Arc` reference): cloning a fan is one refcount bump per child, and

Resolution: Expose `push` on `Children<H>` (delegating to `Fan::push`), start `merged` as `Children::default()`, and drive the loop as `for pair in ours.into_iter().merge_join_by(theirs.into_iter(), |(r, _), (s, _)| r.cmp(s))` with a three-arm `match` on `EitherOrBoth::{Both, Left, Right}`: push `our_child` for an equal pair, push `Join::join(..)`'s `Some` result otherwise. Rewrite the comment to state the invariant (both fans ascending; merged built ascending). Apply `push` (or `collect`) to `unknown.rs:65-73` too. Acceptance: `join_idempotent`, `join_commutative`, `join_associative`, both changed-flag properties, and `join_unwind_leaves_tree_byte_identical` (fuse count per branch level, unaffected) still pass; the loop-with-peekables is gone; a `testing::node_census` reading around one `Tree::join` of a wide divergent pair shows the peak drop by about two handles per divergent child.

### Nits (9, and one cross-reference)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| clippy-pedantic-3 | `src/tree.rs:190-196` | Named lifetimes used once, where the crate writes `'_` | `impl<T: Send + Sync + 'static> DoubleEndedIterator for Iter<'_, T>` and likewise at 196 |
| clippy-pedantic-5 | `src/tree/traverse.rs:7` | Glob imports whose source is not a curated surface | explicit lists at traverse.rs:7 and materialized.rs:185 (`use common::children_of;`) |
| clippy-pedantic-14 | `src/tree/tests.rs:1819-1823` | Three test clones whose result is discarded or whose original is never used again | Rewrite the three redundant clones by hand; examine the remaining `redundant_clone` hits individually, never with `--fix` |
| inventory-13 | `src/tree.rs:632-639` | Thirteen identical `missing_const_for_thread_local` allows with the rationale copied into seven files | One crate-level `#![allow(clippy::missing_const_for_thread_local)]` with the rationale once; delete the thirteen per-static allows and seven comments |
| module-graph-14 | `src/tree.rs:628-712` | Inline bodied production modules in implementation files | Move each inline module body to a sibling file (`tree/meter.rs`, `tree/panic_injection.rs`, `erased/ops.rs`, `untyped/census.rs`, `decode/fan_probe.rs`, `backend/ledger.rs`) |
| suite-economics-11 | `src/tree/arb.rs:672-673` | See module-graph-6, the entry of record for the six inline test modules; not counted here | carried by module-graph-6 |
| tree-core-3 | `src/tree.rs:108-111` | `Root.root` makes every read-path accessor spell `self.root.root` among three other things named `Root` | Rename the field to `node`, updating `tree.rs`, `arb.rs` (113-116, 518-521), and `backend/local.rs` (234-242) |
| tree-core-19 | `src/tree/tests.rs:98-105` | Test-helper idiom nits in `tests.rs`: `insert_at` carries two parameters that only re-derive the first, a wrapper that only renames, qualified `arb` paths, and a stray import | Give `insert_at` two parameters; inline or re-document `naive_max_version_bytes`; `use super::arb;` once; move the serde import into the block |
| tree-core-25 | `src/tree/arb.rs:672-673` | `arb.rs` and `act.rs` open without a module doc; `arb.rs` ends in an inline `mod test` inside an already `cfg(test)` module; `arb_root_node` is `pub` with one same-file caller | Move `distinct_indices_are_pairwise_disjoint` to `src/tree/arb/tests.rs` behind `mod tests;` and drop the inner cfg |
| tree-core-28 | `src/tree/traverse/act.rs:95-95` | Comments that narrate the next line, and a mutable-accumulator fan build where `collect` reads directly | Delete the pure narrations (keep act.rs:114, the short-circuit's why, and 142-144, the tie-break rule) |

## Tree typed

Files: src/tree/typed/. Entries: 24 (2 medium, 8 low, 14 nit).

### prose-hygiene-1: Comments and a test module doc describe the V1 node wire encoding and `Levels` chain by names the retirement deleted
- Where: src/tree/typed/node.rs:428-447 (related: src/tree/traverse.rs:9-12; tests/future_size.rs:3, 7-9, 37-38)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`git show 368da2a5 --stat` lists `src/tree/wire.rs | 267 ---------`, `src/tree/typed/levels.rs | 189 ------`, `src/tree/typed/levels/level.rs | 152 -----`; `grep -rnE '(struct|trait|type|enum) +(Levels|Below)\b' src crates` returns nothing; `find src -name wire.rs` returns nothing; `grep -rnE 'serialize_to|DecodeNode|tree::wire' src tests` returns only the three prose lines in node.rs)
- Verification: confirmed and extended: the sweep's name-based grep missed tests/future_size.rs, whose module doc names the deleted `Levels<Below<…>>` chain and attributes the boxing to `mirror()`, which is `#[cfg(test)]` and unboxed (src/tree/mirror/streaming.rs:142-153); the `BoxFuture` lives in `Handshaken::reconcile` (streaming.rs:110-116) and `Reconciliation` (src/peer/gossip.rs:1128); history: deliberate-but-expired (368da2a5 deleted the identifiers; the prose survived the retirement's prose pass c13c21b4)
- Owner-gated: no

A twenty-line maintainer comment documents a node serialization through
`crate::tree::wire`, `untyped::Node::serialize_to`, and `DecodeNode`, none of
which exist. The `pub(crate)` rationale in traverse.rs names the deleted
`Levels` docs as its example linker. tests/future_size.rs explains its budget
by a `Levels<Below<…>>` chain that no longer exists and credits the erasure
to a test-only function that does not box.

Evidence:

    src/tree/typed/node.rs
    428	// Wire format (see [`crate::tree::wire`]). Serialization is
    429	// height-uniform: every typed `Node<H>` delegates to
    430	// [`untyped::Node::serialize_to`], which emits the in-memory
    ...
    436	// Deserialization at typed height `H` ([`DecodeNode`]) reads `prefix_len`, then either

    src/tree/traverse.rs
    9	// `act` and `unknown` are `pub(crate)` so rustdoc elsewhere (e.g. the
    10	// `Levels` docs) can link to the traversal traits inside them: a private

    tests/future_size.rs
    3	//! The mirror protocol's `Levels<Below<…, Below<…, …>>>` chain is ~30 deep,
    ...
    37	/// The erasure is `mirror()`'s internal `Pin<Box<dyn
    38	/// Future>>`, so the protocol's `Levels` chain doesn't appear in the

Resolution: Delete node.rs:428-447; if the path-compression sentence at
444-446 is wanted, restate it against `Node::beneath` without the wire
framing (the V2 node vocabulary lives in src/tree/mirror/streaming/message.rs
and the codec under remote/codec/). In traverse.rs:9-12 replace the `Levels`
example with a link that exists today (src/tree.rs:49 and :475,
src/tree/typed/untyped/iter.rs:172, src/tree/mirror/streaming/materialized/
unknown.rs:5 all link into `traverse::act` or `traverse::unknown`). In
tests/future_size.rs re-denominate against the streaming protocol's
type-level phase schedule (the `protocol` module's stage traits) and name the
boxing sites that exist: `Handshaken::reconcile` and `Reconciliation`.
Acceptance: `grep -rnwE 'Levels|Below|DecodeNode|serialize_to|tree::wire' src
tests` returns nothing, and future_size.rs's module doc names only items
that resolve.

See also: tree-typed-19.

### tree-typed-19: Prose describing the retired node codec survives at four sites
- Where: src/tree/typed/node.rs:428-447 (related: src/tree/typed/hash.rs:16-17, src/tree/typed/hash.rs:91-92, src/tree/typed/hash.rs:128-129; the live framing at src/tree/mirror/streaming/remote/codec/frame.rs:469 and 521-522; out of partition: src/tree.rs:36 "the node serializer relies on")
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`grep -rn 'tree::wire\|serialize_to\|DecodeNode\|count_minus_two\|Read::chain' src tests benches examples` matches only node.rs:428-440; `git log --diff-filter=D --name-only -- src/tree/wire.rs` names 368da2a5 "retire Protocol::V1 and the alternating mirror"; `git blame -L 428,447` gives 7d95be5b, f2b74a97, b524e406, all before the retirement; `git log -S'never length-prefixed'` names f2b74a97, whose `impl Encode for Hash` in the deleted `src/tree/wire.rs` wrote raw bytes; frame.rs:521-522 writes `cbor::write_head(out, MAJOR_BSTR, MERKLE_HASH_LEN as u64)` and frame.rs:469 rejects any other head; `grep -rn serializer src` finds no node serializer, only the payload one in message.rs)
- Seen by: structure ([0]), prose ([20]), correctness ([34], [35]), perfapi ([47]); refutation: confirmed, plus its new item (hash.rs:92, 129); history: contradicts-hard-rule; the V1 retirement plan (.agent-notes/2026-09-01-v1-retirement/README.md:199-201) scoped node.rs to "the gated codec/ingress items and their malformed-input tests" and did not list these comments, so this is a sweep blind spot, not a ruling
- Owner-gated: no

node.rs ends with a twenty-line comment documenting node serialization through `crate::tree::wire`, `untyped::Node::serialize_to`, `DecodeNode`, a `count_minus_two` field, and `std::io::Read::chain` recursion; none exists, and the design it describes (whole nodes travel, untagged) contradicts the live protocol, which ships leaf runs and listings through the CBOR codec. `Hash`'s type doc says the digest travels as raw bytes "never length-prefixed", but the codec writes every hash as a CBOR byte string with a 24-length head and the decoder rejects any other head. `Hash::leaf`'s and `Hash::branch`'s docs describe path order "as the node serializer emits it"; no node serializer exists. AGENTS.md hard rule: nothing refers to code that no longer exists; the block is a plain `//` comment, so `just doclint` cannot see its dangling links.

Evidence:

       428	// Wire format (see [`crate::tree::wire`]). Serialization is
       429	// height-uniform: every typed `Node<H>` delegates to
       430	// [`untyped::Node::serialize_to`], which emits the in-memory
       431	// representation directly (prefix length, head bytes, then either a leaf
       432	// body or a `count_minus_two` + children list). No leaf-vs-branch tag is
    ...
       436	// Deserialization at typed height `H` ([`DecodeNode`]) reads `prefix_len`, then either

    src/tree/typed/hash.rs:
        16	/// A newtype over a fixed-size byte array; on the wire it travels as its
        17	/// raw bytes, never length-prefixed (the width is pinned by the type).

        91	    /// `suffix` is the leaf's path-compressed span in **path order** —
        92	    /// shallowest byte first, as the node serializer emits it — and

    src/tree/mirror/streaming/remote/codec/frame.rs:
       521	        cbor::write_head(out, MAJOR_BSTR, MERKLE_HASH_LEN as u64);
       522	        out.extend_from_slice(hash.as_bytes());

Resolution: delete node.rs:428-447 (the file then ends after the `PartialEq` impl; the one surviving fact, that singletons never materialize and reconstruct through `beneath`, already lives at `Hash::branch`'s canonicity section and `untyped::Node::branch`). At hash.rs:16-17 either drop the wire sentence (the codec module owns framing) or restate it: the digest travels as a 24-byte CBOR byte string whose head the decoder checks against `MERKLE_HASH_LEN`. At hash.rs:92 and 129 cut "as the node serializer emits it"; the sentence already says "shallowest byte first". Hand tree.rs:36 to the tree-root partition. Acceptance: `grep -rn 'tree::wire\|serialize_to\|DecodeNode\|count_minus_two\|Read::chain\|node serializer\|never length-prefixed' src` returns nothing; `just doclint` clean.

See also: prose-hygiene-1.

### inventory-7: `from_sorted_leaves` re-checks O(n) preconditions at every recursion level
- Where: src/tree/typed/untyped.rs:280-283 (related: src/tree/typed/untyped.rs:325, src/tree/typed/node.rs:296-300, src/tree/mirror/streaming/remote/adapter/decode.rs:508-527, src/tree/mirror/streaming/backend/local.rs:201-221, src/tree/typed/untyped/tests.rs:470-481)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (grep for `from_sorted_leaves`: callers are local.rs:213, 219 and node.rs:309, plus tests; the ingress checks were read at decode.rs:497-540; the recursive call is untyped.rs:325)
- Verification: confirmed and sharpened: the untyped function recurses on each radix group, so the `windows(2)` scan re-runs on every sub-run, making the debug cost O(n · levels); history: no-rationale-found (the link-transport review discusses the bulk path's equivalence tests, not these asserts)
- Owner-gated: no

The untyped `from_sorted_leaves` asserts strict path ascent with a full pass over the run, and its typed wrapper asserts prefix containment with another; both properties are established before any run arrives (`DecodeError::LeafOrder` and `LeafOutsideScope` at ingress; `Local::assemble` groups by height-H prefix), and the function has committed differential coverage (`every_virtual_level_hashes_canonically`). A sorted run's sub-slices are sorted, so the recursive re-checks add nothing even as a precondition check. The messages state the property, not the caller class the check guards.

Evidence:

    280	        debug_assert!(
    281	            leaves.windows(2).all(|pair| pair[0].0 < pair[1].0),
    282	            "a leaf run is strictly ascending by path",
    283	        );

    296	        debug_assert!(
    297	            run.iter()
    298	                .all(|(path, _)| path.as_bytes().starts_with(prefix.as_bytes())),
    299	            "every leaf in a run falls under the run's prefix",
    300	        );

    325	            children.push(radix, Self::from_sorted_leaves(branch_at + 1, group));

Resolution: Either delete both asserts (the differential tests and ingress errors carry the property), or check once at the typed entry (node.rs) with an O(1) probe such as first-versus-last ascent, and state in the message which caller the check exists for (a non-`Local` `Backend::assemble` override). Acceptance: no O(n) `debug_assert` runs inside the recursion; any remaining assert names the caller class it guards.

### tree-typed-1: Mixed `pub`/`pub(crate)` spelling inside a subtree nothing outside the crate can reach
- Where: src/tree/typed.rs:27-28 (related: src/lib.rs:322, src/tree.rs:67, src/tree/typed/node.rs:155-166, src/tree/typed/node.rs:268-294, src/tree/typed/untyped/iter.rs:184-191, src/tree/typed/untyped/iter.rs:368-395, src/tree/typed/prefix.rs:32, src/tree/typed/prefix.rs:166)
- Class / severity / confidence: idiom / low / medium
- Provenance: verified (read `mod tree;` at lib.rs:322 and `pub(crate) mod typed;` at tree.rs:67; conformance.rs:14-19 exposes only `pub mod link`, with `backend` `pub(crate)` and `#[cfg(test)]`; testing.rs:6-16 re-exports nothing under `typed`; spellings counted per file by grep). The dead-code-lint consequence is assessed, not compiled.
- Seen by: structure; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`mod tree` is private and `typed` is `pub(crate)`, so every item under `typed` has crate-only reach, yet the subtree splits items between `pub` and `pub(crate)` with no rule: `ErasedPrefix` and `Prefix::erase` are `pub(crate)` beside a `pub` `Prefix`; `Node::from_untyped`, `into_untyped`, `leaves`, `from_sorted_leaves` are `pub(crate)` beside `pub` `ceiling`, `hash`, `branch`; every constructor in iter.rs is `pub(crate)` while the walk types are `pub`. The modifier suggests an API boundary that does not exist (working default: visibility no wider than use, and one convention so a modifier carries information).

Evidence:

        27	pub(crate) use prefix::ErasedPrefix;
        28	pub use prefix::Prefix;

    src/lib.rs:
       322	mod tree;

    src/tree.rs:
        67	pub(crate) mod typed;

Resolution: pick one convention for src/tree/typed: either `#![warn(unreachable_pub)]` at the crate root with every item under `tree` spelled `pub(crate)`, or plain `pub` throughout with the `pub(crate)` modifiers dropped. The typed.rs:19-21 comment about rustdoc link resolution concerns `mod untyped` and stays valid under either. Acceptance: one spelling per item kind across the subtree; if `unreachable_pub` is adopted, `just clippy` is clean under it.

See also: inventory-10, module-graph-12.

### tree-typed-11: `Height`'s `Debug + Clone + Default` supertraits and the hand-rolled impls that "avoid bounds on `H`" are two mechanisms for one goal
- Where: src/tree/typed/height.rs:79 (related: src/tree/typed/path.rs:61-69, src/tree/typed/prefix.rs:202-212, src/tree/typed/node.rs:23-39, src/tree/typed/node.rs:130-137)
- Class / severity / confidence: vestigial / low / medium
- Provenance: assessed by grep, not compiled (no `#[derive` within six lines above any `H`-generic header anywhere in src; no `H::default()`, `H: Debug`, `H: Clone`, or `H: Default` bound outside typed; every height marker is `PhantomData<fn() -> H>`; height/tests.rs:18 calls `S::<Z>::default()` on the concrete type)
- Seen by: structure; refutation: confirmed (with the `Copy` nuance); history: deliberate-but-expired (at 847f772e0 `#[derive(Clone, Debug)] pub struct Node<.., H: Height>` consumed the supertraits; the derives became manual impls at 8231541a0/fe3612311 and the bounds stayed)
- Owner-gated: no

`Height` requires `Debug + Clone + Default`, yet `Node<H>`, `Children<H>`, `Path<H>`, and `Prefix<H>` hand-write `Clone`/`Default`/`Debug` with the stated reason "so we don't require unnecessary bounds on `H`". Because `H: Height` already implies those bounds, the manual impls avoid nothing they claim to, and nothing I can find consumes the supertraits. One of the two is dead weight, and the rationale comments are false either way. Only `Copy`, which `Height` does not imply, needs the manual impls.

Evidence:

        79	pub trait Height: Debug + Clone + Default + sealed::Sealed + 'static {

    src/tree/typed/path.rs:
        61	// Manual copy/clone impls so we don't require unnecessary bounds on `H`:

    src/tree/typed/prefix.rs:
       202	// Manual clone/comparison impls so we don't require unnecessary bounds on `H`.

Resolution: reduce the trait to `pub trait Height: sealed::Sealed + 'static { const HEIGHT: usize; }` and run `just check`; the manual impls then do what their comments say. If the compile reveals a consumer, keep the bounds and rewrite the three comments to say the manual impls exist for `Copy` (and, for `S<T>`, the reason at height.rs:16-21). Acceptance: either `Height` names no `Debug + Clone + Default` and `just check` is clean, or the comments at path.rs:61, prefix.rs:202, and node.rs:23-39 state a true reason.

See also: tree-typed-24, conformance-15, link-19.

### tree-typed-12: The number 32 is hand-maintained: three enumerations in height.rs, a bare literal at every path-width site, and no `Root::HEIGHT` tie
- Where: src/tree/typed/height.rs:125-133 (related: src/tree/typed/height.rs:116-123, 154-166; literal sites: hash.rs:253, 273, 278; node.rs:301, 304; path.rs:17, 50, 76, 88, 102, 111; prefix.rs:20, 33, 47, 59, 119, 125, 175; untyped.rs:278, 310; iter.rs:301, 387, 473; independent local names for the same width: src/tree/mirror/streaming/window.rs:137 `KEY_DEPTH`, src/bookmark/format.rs:78 `HASH_LEN`, src/tree/mirror/streaming/remote/codec/tests.rs:66 `MAX_ARBITRARY_SUFFIX_LEN`)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (the only compile-time pin is height.rs:166 over `H0`/`H32`; `grep -rn 'Root::HEIGHT' src` finds only runtime uses; `grep -rn 'const .*: usize = 32;' src` finds the three local names and no shared constant; erased.rs:215-238 splices `H~N` aliases and never names `Root`)
- Seen by: structure ([3], [4]), perfapi ([51]); refutation: confirmed (all three); history: no-rationale-found (bf1a5b4bc added `alias_heights!` and the endpoint assert but left `Root` a literal; its doc at 135 asserts "H32 (= [`Root`])" in prose only)
- Owner-gated: no

Heights are enumerated by `impl_heights!` over 32 `_` tokens, by `alias_heights!` over 33 names, and by `Root` as a hand-counted 32-deep `S<` chain; the `const _` ties the aliases to their numbers but `Root` to nothing, so a 31-deep `Root` compiles (H31 has a `Height` impl) and is caught only by runtime tests. Separately, `MERKLE_HASH_LEN` names the 24-byte width, but the 32-byte path width, which is both SHA3-256's output and `Root::HEIGHT`, is a literal at every site: `[u8; 32]` types, `32 - H::HEIGHT` depth arithmetic, `(depth..32)`, `Vec::with_capacity(32)`, and the depth assert; three modules elsewhere have each coined a local name for the same fact. Principle 5 (no hand-maintained counts) and the working default (named constants over magic numbers); under Principle 6 the cheapest artifact passing today's check is a miscounted `Root`.

Evidence:

       116	// 32 `_` tokens => heights 0..=32 inclusive (33 impls).
    ...
       127	#[rustfmt::skip]
       128	pub type Root =
       129	// Laid out for your counting convenience in two rows of 16:
       130	    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 0
       131	    S<S<S<S<S<S<S<S<S<S<S<S<S<S<S<S< // 1
       132	//  0 1 2 3 4 5 6 7 8 9 a b c d e f
       133	    Z>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>;
    ...
       166	const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);

    src/tree/typed/path.rs:
        50	        let byte = self.hash[32 - S::<H>::HEIGHT];

    src/tree/typed/prefix.rs:
        59	        32 - self.hash.len()

    src/tree/typed/untyped.rs:
       310	        let branch_at = (depth..32)

    src/tree/typed/untyped/iter.rs:
       387	            spine: Vec::with_capacity(32),

Resolution: (1) add `pub const PATH_LEN: usize = 32;` beside `MERKLE_HASH_LEN` in hash.rs with a one-line doc (the SHA3-256 output width and the root height), and use `[u8; PATH_LEN]` and `PATH_LEN - H::HEIGHT` at the listed sites; optionally give `Height` a derived `const DEPTH: usize = PATH_LEN - Self::HEIGHT;` so `Path::pop` and `Prefix` read `H::DEPTH`. (2) Merge the two macros into one `heights!(H0 H1 … H32)` that emits both the `Height` impl and the numbered alias per name, using the `$n + 1` accumulator `impl_heights!` already has, then `pub type Root = H32;` and `const _: () = assert!(Root::HEIGHT == PATH_LEN);`. If the two-row layout is wanted as pedagogy, keep it and add only the `Root::HEIGHT` assert. Acceptance: height.rs has one enumeration of the heights (or the literal plus an assert on `Root::HEIGHT`); `grep -n '\b32\b' src/tree/typed/*.rs src/tree/typed/untyped/*.rs` outside doc comments returns only the constant's definition; erased.rs's `at_parent_height!` compiles unchanged.

See also: inventory-16, streaming-backend-window-25.

### tree-typed-16: `from_sorted_leaves` re-checks a typed fact with `try_from(...).expect` while the infallible `From<Prefix> for [u8; 32]` sits unused
- Where: src/tree/typed/node.rs:301-308 (related: src/tree/typed/prefix.rs:119-123, src/tree/typed/untyped.rs:276-279)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (every `<[u8; 32]>::from(` call in src takes a `Path`; prefix.rs:119-123 provides the `Prefix`-to-array impl with default `H = Z`; `git show 9c73d7b46 -- src/tree/typed/prefix.rs` shows the impl retargeted from a retired `Key` type to `[u8; 32]` without revisiting this call site)
- Seen by: structure ([5]), perfapi ([48]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

A `Prefix<Z>` is 32 bytes by its type (`32 - Z::HEIGHT`), and the crate's one site converting `Prefix<Z>` to an array bypasses the `From` impl for a fallible `try_from` with an `expect` asserting what the type already fixes (doctrine on guards: a runtime check must name a failure the types cannot prevent; a trait impl bypassed at its one natural site is the circular-justification tell). The same loop also re-collects the incoming `Vec<(Prefix<Z>, Node<Z>)>` into a second `Vec<([u8; 32], Option<untyped::Node>)>`, one allocation per supplied run, because the untyped entry takes exactly that slice shape.

Evidence:

       301	        let mut entries: Vec<([u8; 32], Option<untyped::Node>)> = run
       302	            .into_iter()
       303	            .map(|(path, leaf)| {
       304	                let path = <[u8; 32]>::try_from(path.as_bytes())
       305	                    .expect("a leaf prefix is a full 32-byte path");
       306	                (path, Some(leaf.into_untyped()))
       307	            })
       308	            .collect();

    src/tree/typed/prefix.rs:
       119	impl From<Prefix> for [u8; 32] {
       120	    fn from(value: Prefix) -> Self {
       121	        Path::from(value).into()
       122	    }
       123	}

Resolution: replace lines 304-305 with `let path = <[u8; 32]>::from(path);`, and spell the impl directly (`value.hash.into_inner()`) rather than routing through `Path`. The second `Vec` is worth removing only if a run-heavy bench moves; it is small next to the run's node allocations. Acceptance: no `try_from` in `from_sorted_leaves`; `From<Prefix> for [u8; 32]` has a caller; `every_virtual_level_hashes_canonically` and the backend/local suites pass unchanged.

### tree-typed-24: Manual `Clone`/`Default` impls on `NodeInner`, untyped `Children`, and `Fan` are what `#[derive]` generates
- Where: src/tree/typed/untyped.rs:103-111 (related: src/tree/typed/untyped.rs:164-187, src/tree/typed/untyped/fan.rs:58-72)
- Class / severity / confidence: idiom / low / high
- Provenance: verified by reading the field types' impls in the pinned dependencies and siblings (smallvec 1.15.1 lib.rs:2125 `impl<A: Array> Default for SmallVec<A>`, 2160 `impl<A: Array> Clone for SmallVec<A>`; `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Span` at crates/before/src/span.rs:104; `#[derive(Clone)] pub struct Message` at src/message.rs:47; std's `OnceLock<T: Clone>: Clone`); not compiled
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (at b524e406^ the types were generic over the payload `T` and `impl<T> Clone` avoided a `T: Clone` bound; item erasure dropped `T` and kept the bodies)
- Owner-gated: no

Three non-generic types hand-write impls whose bodies are field-wise clones or defaults, about thirty-five lines a reader must check for a deviation that is not there. Every field type derives or implements the trait already. The untyped `Node`'s manual `Clone` is different and stays: it is the census funnel. Machinery outlived the constraint (payload genericity) that justified it.

Evidence:

       103	impl Clone for NodeInner {
       104	    fn clone(&self) -> Self {
       105	        Self {
       106	            prefix: self.prefix.clone(),
       107	            hash: self.hash.clone(),
       108	            children: self.children.clone(),
       109	        }
       110	    }
       111	}

    src/tree/typed/untyped/fan.rs:
        58	impl Default for Fan {
        59	    fn default() -> Self {
        60	        Self {
        61	            entries: SmallVec::new(),
        62	        }
        63	    }
        64	}

Resolution: `#[derive(Clone)] struct NodeInner`; `#[derive(Debug, Clone)] enum Children`, moving the memo-carrying comment at untyped.rs:171-173 above the derive; `#[derive(Default, Clone)] pub struct Fan`. Acceptance: the three manual blocks are gone; `just check` and the untyped and fan proptests pass unchanged.

### tree-typed-29: `dominance` is a one-line delegate at both layers for one caller, and the sibling module already inlines it
- Where: src/tree/typed/untyped.rs:535-557 (related: src/tree/typed/node.rs:184-195, src/tree/traverse/unknown.rs:48, src/tree/mirror/streaming/materialized/unknown.rs:66)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn '\.dominance(' src` finds two production callers, unknown.rs:48 through the wrapper and materialized/unknown.rs:66 as `node.span().dominance(known)`; `git show 4874f527 -- src/tree/typed/untyped.rs` shows the leaf arm collapsing to the delegate once `before` gained the coincident-span rung)
- Seen by: structure; refutation: confirmed; history: deliberate-but-expired (the wrapper had a body of its own, a leaf arm saving a decode, until 4874f527)
- Owner-gated: no

The untyped method is `self.span().dominance(probe)` under twenty lines documenting `Span::dominance`'s internals; the typed method delegates to it under twelve more. Two wrappers and thirty lines of prose about a dependency's method, for one call site, while the streaming counterpart spells the same classification directly.

Evidence:

       555	    pub fn dominance(&self, probe: &Version) -> Dominance {
       556	        self.span().dominance(probe)
       557	    }

    src/tree/traverse/unknown.rs:
        48	        match node.dominance(known) {

    src/tree/mirror/streaming/materialized/unknown.rs:
        66	    node.span().dominance(known)

Resolution: delete both `dominance` methods; change unknown.rs:48 to `match node.span().dominance(known)`. The one rumors-specific sentence (routing through `span` pays a leaf one decode per stream) moves onto `span`'s doc. Acceptance: `grep -rn 'fn dominance' src/tree/typed` is empty; both `unknown` modules call `span().dominance`.

### tree-typed-35: `RangeOwned` has an inherent `next` but no `Iterator` impl, so `Node::leaves` wraps it in `iter::from_fn` and observers hand-drive it
- Where: src/tree/typed/untyped/iter.rs:393-395 (related: src/tree/typed/node.rs:272-281, src/rumors/causal.rs:99, src/rumors/unordered.rs:211)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (callers by grep; `git log -S'Iterator for RangeOwned'` and for its former names is empty; the one nearby rationale for an inherent `next`, ef4239f87's "no standard lending-iterator trait", is about the `Messages` observer that lends borrows, not this walk, whose item is owned)
- Seen by: structure ([6]), correctness ([38]), perfapi ([50]); refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The method has `Iterator::next`'s exact shape and an owned item; the borrowing `Range` in the same file implements `Iterator`. Without the trait, `Node::leaves` builds a `std::iter::from_fn` closure to get `.map`, and the subscription drains call `.next()` in `while let`/`if let` by hand. Implementing the trait removes the shim and gives every caller the adaptors; `Send` and the constant-size state are unaffected, and `size_hint` can report `(0, None)`.

Evidence:

       393	    /// Advance to the next passing leaf. The same classification as the
       394	    /// borrowing walk, with the leaf handed out by value.
       395	    pub(crate) fn next(&mut self) -> Option<([u8; 32], Leaf)> {

    src/tree/typed/node.rs:
       274	        std::iter::from_fn(move || {
       275	            walk.next().map(|(key, leaf)| {
       276	                (
       277	                    super::Prefix::from(key),
       278	                    Node::from_untyped(leaf.into_node()),
       279	                )
       280	            })
       281	        })

Resolution: `impl<P: Polarity> Iterator for RangeOwned<P> { type Item = ([u8; 32], Leaf); fn next(&mut self) -> Option<Self::Item> { /* body */ } }`, delete the inherent method, and reduce `Node::leaves` to `RangeOwned::within(...).map(|(key, leaf)| (Prefix::from(key), Node::from_untyped(leaf.into_node())))`. Acceptance: `iter::from_fn` is gone from node.rs; causal.rs and unordered.rs compile unchanged (method syntax resolves to the trait); `range_and_freeze_match_the_naive_filter` passes.

### Nits (14)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| clippy-pedantic-4 | `src/tree/typed/node.rs:414-416` | Closures wrapping one method call where the crate passes the method path | `.map(Root::hash)` |
| clippy-pedantic-10 | `src/tree/typed/untyped.rs:356-359` | Wildcard arm over the crate's two-variant `Children` | `Children::Branch { .. } => None` in place of the wildcard arm; the three test sites over crate enums likewise |
| inventory-16 | `src/tree/typed/prefix.rs:44-49` | The 32-byte path width is a repeated literal with no shared name | Define one `pub(crate) const PATH_LEN: usize = 32;` under typed/ (or use `Root::HEIGHT`); replace the arithmetic literals; alias window.rs's `KEY_DEPTH` to it |
| inventory-20 | `src/tree/typed/node.rs:361-366` | Expect messages that restate the call instead of proving it | node.rs:365: `expect("a Node<Z> is only re-tagged from a node erased at leaf height")` |
| tree-typed-4 | `src/tree/typed/hash.rs:52-56` | `std::fmt::Formatter`, `std::fmt::Result`, and `std::cmp::Ordering` spelled in full at every manual impl | `use std::fmt;` (and `use std::cmp::Ordering;` where needed) per file, then `fmt::Formatter<'_>`, `fmt::Result`, `Ordering::Equal` |
| tree-typed-8 | `src/tree/typed/hash.rs:224-240` | `Hash`'s bytes are reachable three ways, and `From<[u8; MERKLE_HASH_LEN]> for Hash` has no caller | Either make `Hash`'s field private and construct through `Hash::from`, or keep the field and delete the unused `From` impl |
| tree-typed-10 | `src/tree/typed/height.rs:13-14` | `#[repr(C)]` on the zero-sized height markers | remove both attributes |
| tree-typed-15 | `src/tree/typed/node.rs:268-294` | `super::Prefix` qualified at five use sites instead of imported | `use super::prefix::Prefix;` beside the other `super::` imports |
| tree-typed-18 | `src/tree/typed/node.rs:409-417` | `root_hash` takes `&Option<Root>` where its siblings take `Option<&Self>` | `pub fn root_hash(node: Option<&Root>) -> Hash` |
| tree-typed-20 | `src/tree/typed/path.rs:94-98` | `Debug` renderings disagree with what `Eq` compares and with the documented path order | `Path`: format `&self.hash[32 - H::HEIGHT..]` |
| tree-typed-21 | `src/tree/typed/prefix.rs:172-180` | `Prefix::containing` builds its bytes by slice-iterate-collect where `ArrayVec::from_array_len` says it in O(1) | `ArrayVec::from_array_len(<[u8; 32]>::from(*path), 32 - H::HEIGHT)` in place of the slice-iterate-collect |
| tree-typed-25 | `src/tree/typed/untyped.rs:122-134` | Untyped `Children` (a leaf-or-branch body) and typed `Children<H>` (a radix fan) share a name one module apart | rename the untyped enum (`Body` or `Content`), a mechanical change confined to untyped.rs and iter.rs |
| tree-typed-31 | `src/tree/typed/untyped/fan.rs:203-226` | `Fan::iter` hand-rolls a named `Iter<'a>` nothing outside fan.rs names, while `values` returns `impl Trait` | Return `impl DoubleEndedIterator + ExactSizeIterator` from `Fan::iter` and delete the named `Iter<'a>` and its three impls |
| tree-typed-34 | `src/tree/typed/untyped/iter.rs:311-312` | The owned walk's cursor is a `u16` with a 256 sentinel and two `as` casts where `Option<u8>` carries the same state | `next: Option<u8>` initialised `Some(0)` |

## Mirror common

Files: src/tree/mirror.rs, cbor, framing, handshake, party, the streaming module root, protocol, message, driver, erased, tasks, stats, and the streaming test suites under streaming/tests/. Entries: 32 (3 medium, 13 low, 16 nit).

### mirror-common-10: `Preamble::decode` takes `&[u8]` where every caller holds `[u8; V2_PREAMBLE_LEN]`, and pays for it with two public `PreambleDefect` variants that guard programmer error
- Where: src/tree/mirror/handshake.rs:111-159 (related: src/tree/mirror/handshake.rs:99-108, src/tree/mirror/handshake.rs:219-241, src/tree/mirror/handshake.rs:246, src/tree/mirror/handshake.rs:285-288, src/tree/mirror/handshake/tests.rs:283-290, src/error.rs:39, .cargo/mutants.toml:15-24, .agent-notes/2026-08-20-cbor-wire-review/REVIEW.md:72)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read: the width arithmetic by hand against `read_head`'s shortest-form check at cbor.rs:191-194 and `V2_PREAMBLE_LEN = 11 + 1 + 17 + 1` at :53; the call sites verified by grep: `Staged::validate` at :287 passes `&self.buf: [u8; V2_PREAMBLE_LEN]`, the two proptests pass full-width inputs; no `.cargo/mutants.toml` entry covers handshake.rs)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed (adopting [39]'s mechanism: keep `read_head` for the version so a wide canonical version head still yields `VersionMismatch { remote_version }`); history: already known, reopens R2 ("keep-and-document; do not delete `NetworkTruncated`", REVIEW.md:72, executed in 39290c4af) on one new fact: R2's premise was a `buf: [u8; PREAMBLE_MAX]` sliced per dialect, and 368da2a5 made it `[u8; V2_PREAMBLE_LEN]`
- Owner-gated: yes: removes two variants of the public `#[non_exhaustive] PreambleDefect` and reopens a recorded ruling

`decode` accepts an arbitrary slice although its only production caller passes the fixed array, so it re-derives the fixed width at runtime: two `// Defensive:` checks (:135-140, :148-153) are the sole constructors of `PreambleDefect::NetworkTruncated` and `PreambleDefect::TrailingBytes`, whose own docs say they are "Defensively reachable only" and guard "the decoder's width arithmetic against layout drift, not any input the current dialect admits"; the test file carries a standing exemption for them (tests.rs:283-290); and the slice indexings at :112 and :114 panic on any input shorter than the width the type could carry. After a canonical one-byte version head (`value == 2`) and a canonical one-byte network head (`value == 16`), exactly 17 of the 30 bytes remain, so neither check can fire. Layout drift is programmer error, and `prefix_matches_the_writers` plus `encode`'s `debug_assert_eq!` already catch it. A guard must name a constructible failure the committed tests cannot catch; never document what the types prevent; and the project's own mutants policy (mutants.toml:15-24) orders this disposition: "(1) refactor, so the mutated codepoint does not structurally exist (... a stronger type, a dissolved dead arm)". Both checks are value-equivalent mutant sites (`<` → `<=` at :138; `!input.is_empty()` → `false` at :151) with no roster entry. Since R2 was ruled, the typed signature became free, which is why the question is worth putting again.

Evidence:

    111	    fn decode(bytes: &[u8]) -> Result<Self, Error> {
    112	        if bytes[..V2_PREFIX.len()] != V2_PREFIX {
    ...
    135	        // Defensive: a validated version and network head leave 17 of the
    136	        // fixed item's 30 bytes here, so the 16 network bytes always fit;
    137	        // the bound keeps `split_at` in range under any layout drift.
    138	        if input.len() < NETWORK_LEN {
    139	            return Err(malformed(PreambleDefect::NetworkTruncated));
    140	        }
    ...
    148	        // Defensive: the one-byte intent item consumes the fixed item's
    149	        // last byte, so nothing can trail; the check guards any caller
    150	        // handing the decoder non-fixed input.
    151	        if !input.is_empty() {
    152	            return Err(malformed(PreambleDefect::TrailingBytes));
    153	        }
    ...
    246	    buf: [u8; V2_PREAMBLE_LEN],

Resolution: Take `bytes: &[u8; V2_PREAMBLE_LEN]` and split it with `split_first_chunk::<{ V2_PREFIX.len() }>()`, which types the remainder as `&[u8; 19]`. Keep the version diagnosis as it is (`cbor::read_head` over the remainder, `VersionMismatch` carrying `remote_version` for any canonical value other than 2, `Malformed(Version)` for a widened or non-uint head): once `value == 2` passes, canonicality means the head was exactly the remainder's first byte, so destructure the remainder as `[_version, network_head, network @ .., intent]` and both `network: &[u8; NETWORK_LEN]` and `intent: &u8` are type facts. Validate `network_head` and `intent` each as a one-byte head via `cbor::read_head(&mut &[*b][..])` with the same filters as today; a widened head on a one-byte slice reports `Truncated` and lands in the same `Malformed(Network)`/`Malformed(Intent)` class the tests pin (`intent_byte_space_is_exhaustive` classifies `0x18..` as `Malformed(Intent)`). Delete `NetworkTruncated`, `TrailingBytes`, both `// Defensive:` comments, the `expect("network width")`, the intent-width `expect` (a one-byte uint head's value is at most 23, so `intent.value as u8` with that sentence, or `Intent::from_byte(*intent & 0x1f)` after the major check), and the tests.rs exemption block. Dually, `encode(self) -> [u8; V2_PREAMBLE_LEN]` states the width in its type and drops the per-session `Vec` and its `debug_assert_eq!` (this half is not owner-gated). Acceptance: `PreambleDefect` has exactly `Version`, `Network`, `Intent`, each with a construction test; `decode` contains no `Defensive` comment and no `expect`; `intent_byte_space_is_exhaustive`, `widened_version_spelling_is_the_version_defect`, `version_mismatch_is_diagnosed_before_intent`, `arbitrary_preamble_decodes_by_the_oracle`, and `arbitrary_bytes_never_panic` pass with their classifications unchanged; a peer sending a canonical two-byte version head (`0x18 0x18`) still yields `VersionMismatch { remote_version: 24 }`.

See also: mirror-common-11.

### module-graph-2: The streaming core 4-cycle is closed by single-item imports, two of them misplaced items
- Where: src/tree/mirror/streaming/erased.rs:203-206 (related: src/tree/mirror/streaming/materialized.rs:114, :186-188, :197-251, :438, :487; src/tree/mirror/streaming/materialized/common.rs:17-36; src/tree/mirror/streaming/remote/proxy/work.rs:19; src/tree/mirror/streaming/remote/adapter/decode.rs:15, :166, :373; src/tree/mirror/streaming/remote/proxy/work/pump.rs:40; src/tree/mirror/streaming/remote/codec/budget.rs:64-71; src/tree/mirror/streaming/window.rs:123, :154-156; src/tree/mirror/streaming.rs:10-24)
- Class / severity / confidence: modularity / medium / high on the facts, medium on the shape of the fix
- Provenance: verified (read every site; grep of every `children_of`, `SupplyLedger`, `.absorb(`, `.charge(`, and `DEFAULT_TARGET_MESSAGE_SIZE` use)
- Verification: reframed: the facts hold, but two of the sweep's three moves do not work as written (see below); history: no-rationale-found
- Owner-gated: no

Among the direct children of `tree::mirror::streaming`, `erased → materialized` (`children_of`), `materialized → remote` (`DEFAULT_TARGET_MESSAGE_SIZE`), `remote → materialized` (`SupplyLedger`, three files), and `window → materialized` (`Resolve`) form the only non-trivial cycle in the protocol core, and it defeats the layer order `streaming.rs:10-24` draws. Two of the four closing items live in the wrong module: `children_of` is a generic `Backend` helper whose single consumer is `erased::ops` (the re-export comment at `materialized.rs:186-187` credits the remote proxy, which in fact reaches it through `erased::ops::children_of` at `pump.rs:211`); `SupplyLedger` is greeting-derived session state that its own doc says is shared by the walk and the wire decoder. The other two edges are cheaper than the sweep's resolution assumed: `DEFAULT_TARGET_MESSAGE_SIZE` is derived from codec frame constants and cannot leave `codec`, and the `window → materialized` edge prices `REFERENCE_SLOT_BYTES` by `size_of` on the `Resolve` slot type itself, which is deliberate.

Evidence:

    src/tree/mirror/streaming/erased.rs:
       203	    use crate::tree::{
       204	        mirror::streaming::{
       205	            backend::BoxNodeStream, materialized::children_of as children_of_typed,
       206	        },

    src/tree/mirror/streaming/materialized.rs:
       186	// The remote proxy explodes early-supplied whole root children into the
       187	// same per-child shape the walks consume, with the walks' own helper.
       188	pub(crate) use common::children_of;
    ...
       197	/// The session-total supplied-leaf ledger: the ingestion-side counterpart
       198	/// of the greeting's declared set length, shared by every stage that
       199	/// absorbs supplies.
    ...
       245	    pub(crate) fn absorb<E>(&self, leaves: u64) -> Result<(), Error<E>> {
       246	        match self.charge(leaves) {
       247	            Ok(()) => Ok(()),
       248	            Err(_) => violation(Violation::OverdrawnSupply),

    src/tree/mirror/streaming/remote/codec/budget.rs:
        71	pub const DEFAULT_TARGET_MESSAGE_SIZE: usize = FAN * FULL_FAN_QUERY_FRAME_LEN;

    src/tree/mirror/streaming/materialized.rs (the walk's only use of it):
       437	            window: WindowConfig::default(),
       438	            target_message_size: DEFAULT_TARGET_MESSAGE_SIZE as u64,

    src/tree/mirror/streaming/window.rs:
       123	use super::{Backend, Local, materialized::Resolve};
    ...
       154	const REFERENCE_SLOT_BYTES: usize = std::mem::size_of::<(u8, typed::Node<Z>)>()
       155	    + std::mem::size_of::<(u8, Resolve<<Local as Backend>::Erased>)>()
       156	    + std::mem::size_of::<(u8, typed::Hash)>();

Resolution: (a) Move `children_of` from `materialized/common.rs:17-36` into `erased::ops` (its only consumer) or `backend.rs` beside `Backend::children`, and delete the re-export and its comment at `materialized.rs:186-188`. (b) Move the `SupplyLedger` struct with `new` and `charge` (`materialized.rs:197-241`) to a neutral module (`streaming/ledger.rs`, or `message.rs` beside the `Greeting` whose `set_len` it enforces); `absorb` depends on `materialized::Error`/`Violation`, so it stays in `materialized.rs` as an inherent impl on the moved type, which Rust permits within one crate. (c) `DEFAULT_TARGET_MESSAGE_SIZE` stays in `codec::budget` (its derivation is codec vocabulary); either thread the target through `materialized::Handshaking::start` so the walk stops defaulting it, or state at `materialized.rs:114` that the walk adopts the codec's default. (d) State at `window.rs:123` that the `Resolve` import exists for `REFERENCE_SLOT_BYTES`'s `size_of` pricing. Acceptance: `analyze.py`'s sibling-subtree section under `tree::mirror::streaming` lists at most the two documented edges (`window → materialized`, and `materialized → remote` if (c) takes the documenting option), and `common.rs` holds only `ok_channel` or dissolves with module-graph-3.

Carried from materialized-10 (the `materialized → remote` edge, now a cross-reference here): the walk's `Handshaking::start` default at materialized.rs:438 casts the codec's `usize` constant to the greeting's `u64` (`Greeting.target_message_size` at src/tree/mirror/streaming/message.rs:103), the only `remote::` import in materialized.rs; its proposed fix was to define the constant beside `Greeting` in `message.rs`, keeping the `FAN * FULL_FAN_QUERY_FRAME_LEN` derivation, with `remote::codec::budget` importing it and converting once at `RunBudget::from_bytes`, which option (c) above declines because the derivation is codec vocabulary. Retyping the constant to `u64` changes a public item (re-exported through src/peer.rs:18 and src/lib.rs:342) and is owner-gated; relocating or threading it is not.

See also: materialized-3, materialized-10, materialized-16, mirror-common-25, deps-2, module-graph-3.

### streaming-tests-3: The two-Local-endpoints session is constructed at nine sites behind a six-function ladder
- Where: src/tree/mirror/streaming/tests.rs:62-150 (related: tests.rs:105-116, 234-240; capacity.rs:26-36, 198-218; stats.rs:42-61; faults.rs:72-75, 87-90, 135-145, 211-215)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep: `Handshaking::start(Local` on 18 lines, nine client/server pairs, across tests.rs (6), faults.rs (6), capacity.rs (4), stats.rs (2); ladder call sites: `streaming_mirror_sides_with_schedule` is called only at tests.rs:65 and :128, both wrappers; `streaming_mirror` only at tests.rs:185; `awk 'length > 100'` over the partition prints exactly faults.rs:72, 73, 74, 87, 88, 89, 97, 211, 212)
- Seen by: structure-prose ([6], [7], [18]), api-economics ([40]); refutation: confirmed; history: no rationale found (seven creating commits between a8b130e6 and 7626543e, each adding a copy for a new instrument)
- Owner-gated: no

The sequence "convert to `StreamingRoot<Local>`, `Handshaking::start(Local, ..).window(WindowConfig::FLOOR)` twice, `run_to_quiescence(drive_streaming(client, server))`, unwrap quiescence and violations" is spelled out at nine sites with different instruments attached, and tests.rs stacks `streaming_mirror_sides` -> `_with_schedule` -> `_with_schedules` plus three convergence-asserting twins that differ only in their message string; two rungs have a single caller. A change to the session's construction touches nine sites, and each suite's distinctive claim is buried under identical setup. The nine lines of 102 to 124 characters in `faults.rs` (unformatted because rustfmt leaves `proptest!` bodies alone) are the same construction spelled inline.

Evidence:

        64	fn streaming_mirror_sides(a: Root, b: Root) -> (Root, Root) {
        65	    streaming_mirror_sides_with_schedule(a, b, Vec::new())
        66	}
        67	
        68	/// Reconcile under an explicit, shrinkable channel-poll schedule.
        69	fn streaming_mirror_sides_with_schedule(a: Root, b: Root, schedule: Vec<u8>) -> (Root, Root) {
        70	    streaming_mirror_sides_with_schedules(a, b, schedule, Vec::new())
        71	}

       106	    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
       107	    let client = Handshaking::start(Local, a).window(WindowConfig::FLOOR);
       108	    let server = Handshaking::start(Local, b).window(WindowConfig::FLOOR);
       109	    let ((result, trace), transcript) =
       110	        with_transcript(|| with_trace(|| run_to_quiescence(drive_streaming(client, server))));

    stats.rs:
        43	    let (a, b): (StreamingRoot<Local>, StreamingRoot<Local>) = (a.into(), b.into());
        44	    let a_recorder = Recorder::default();
        45	    let b_recorder = Recorder::default();
        46	    let client = Handshaking::start(Local, a)
        47	        .window(WindowConfig::FLOOR)
        48	        .stats(a_recorder.clone());

Resolution: Give tests.rs one `LocalSession` builder: `LocalSession::new(a, b)` with `.channel_schedule(..)`, `.backend_schedule(..)`, `.kind_capacity(kind, n)`, `.stats()`, `.trace()`, `.transcript()`, and `run(self) -> Outcome`, where `Outcome` carries `Result<Result<(Root, Root), MirrorError<..>>, Quiescence>` plus the requested instruments, with `converged(self) -> Root` and `sides(self) -> (Root, Root)` owning the two `expect`s. Express `transcribed_mirror_sides`, `mirror_with_stats`, `shape_stalls`, `underbuffered_mirror_stalls`, and the inline body of `uncontained_supply_is_rejected_by_streaming` through it; keep `faults.rs`'s own construction only where a `Faulting` or `Failing` wrapper replaces `Local`, via a `floor_start(root)` helper that also wraps the long lines. The same `Outcome` type resolves streaming-tests-11. Acceptance: `grep -c 'Handshaking::start(Local'` over the partition drops to the builder plus the fault-wrapper sites; `streaming_mirror_sides_with_schedule` and `scheduled_streaming_mirror` are gone; `awk 'length > 100' faults.rs` prints nothing.

### inventory-3: Twelve per-item `type_complexity` allows under `streaming/` are shadowed by the module-wide allow
- Where: src/tree/mirror/streaming.rs:42-43 (related: src/tree/mirror/streaming/protocol.rs:10; materialized/work/levels.rs:67, 174, 296, 341, 539, 644; materialized/work/answer.rs:31, 113; materialized/work/resolver.rs:61; materialized.rs:388; remote/adapter/encode.rs:57; and in test files materialized/tests.rs:48, materialized/work/tests/violations.rs:100, remote/adapter/tests/malformed.rs:620)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read; lint attributes on a module apply to every nested item, out-of-line files included, so the claim rests on documented semantics; clippy was not run)
- Verification: confirmed; history: no-rationale-found (the module-wide allow is da4234ba, 2026-07-10; protocol.rs's inner allow predates it, bdfdf252, 2026-07-01; the levels.rs per-item allows postdate it, bf1a5b4b, 2026-08-19)
- Owner-gated: no

`#![allow(clippy::type_complexity)]` at the top of `mod streaming` already covers every nested module. The twelve per-item allows and protocol.rs's second inner allow suppress nothing, and a reader cannot tell which layer does the suppressing.

Evidence:

    42	// Where we're going, we need to write some Complex Types.
    43	#![allow(clippy::type_complexity)]

    10	#![allow(clippy::type_complexity)]

Resolution: Keep one layer. Either delete the twelve production per-item allows (and the `clippy::type_complexity` half of levels.rs:341) plus the three test-file allows under `streaming/`, or drop the module-wide allow and keep per-item allows where each complex type lives. Acceptance: `just clippy` and `just clippy-default` stay clean with a single layer of `type_complexity` allow under `streaming/`.

See also: materialized-8, mirror-common-29.

### mirror-common-3: `read_head` spells its width as `1 + (a - b - 1)`, and the info-to-width table is written twice with the second copy's error arms unreached by any test
- Where: src/tree/mirror/cbor.rs:191-194 (related: src/tree/mirror/cbor.rs:170-190, src/tree/mirror/cbor.rs:270-280, src/tree/mirror/cbor/tests.rs:46-59, src/tree/mirror/cbor/tests.rs:91-105, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:302)
- Class / severity / confidence: simplification / low / high
- Provenance: demonstrated (second witness pass: with `extension_len`'s two error arms swapped, the 135 existing tests selected from the mutated function's callers all passed and only an added witness test failed, `initial byte 0x1c: async reader classified Indefinite, slice reader says Reserved`; the whole suite was not run; before the pass, verified by grep: the only assertions on `HeadError::Reserved` and `HeadError::Indefinite` are cbor/tests.rs:49-52, which call the slice `read_head`; `extension_len` is reached only from `read_head_async`, `read_head_io`, and the codec's `partial_head`)
- Seen by: structure, correctness; refutation: confirmed, plus a new verification-gap note; history: no rationale found (birth forms from 4dd2053c9)
- Owner-gated: no

Line 191 computes the consumed width as `1 + (input.len() - rest.len() - 1)`, which is `input.len() - rest.len()`; a self-cancelling pair makes a reader stop to check an identity. The additional-information-to-width mapping (24 → 1, 25 → 2, 26 → 4, 27 → 8, 28..=30 reserved, 31 indefinite) is written once in `read_head`'s match and again in `extension_len`. `async_heads_match_the_slice_reader` ties the two copies together only on canonical heads; no committed test feeds an initial byte with info 28..=31 to `read_head_async`, `read_head_io`, or the codec, so swapping `extension_len`'s two error arms would survive every suite. Legibility (finished code reads as evidently right) and adequacy (a criterion needs a committed demonstration that the bad mechanism fails it).

Evidence:

    191	    let width = 1 + (input.len() - rest.len() - 1);
    192	    if width != head_len(value) {
    193	        return Err(HeadError::NotShortest);
    194	    }
    ...
    277	        28..=30 => Err(HeadReadError::Malformed(HeadError::Reserved)),
    278	        _ => Err(HeadReadError::Malformed(HeadError::Indefinite)),

Resolution: Write `let width = input.len() - rest.len();`. Either restructure `read_head` to call `extension_len(initial)` and decode the argument by the returned width (one classification table), or add one row to `async_heads_match_the_slice_reader` (or a sibling point test) feeding `[(major << 5) | info]` for `info in 28..=31` to `read_head_async` and asserting the same `HeadError` the slice reader returns. Factoring the shared twelve lines of `read_head_async`/`read_head_io` into `assemble_head(initial, argument)` is optional. Acceptance: the four head proptests pass unchanged; a deliberate swap of the two arms at cbor.rs:277-278 fails a committed test.
Construction (run in the second witness pass): swap the two arms at cbor.rs:277-278 and run the suites that reach `extension_len`: every existing test passes at this commit (135 of 135 in the pass's selection; the whole suite was not run), because cbor/tests.rs:46-59 exercises the slice reader's own copy of the table (:188-189).

Witness: the second witness pass ran this construction (`witness/results.md`, `## mirror-common-3`). The two error arms of `extension_len` at cbor.rs:277-278 were swapped, a witness test feeding `[(major << 5) | info]` for `info in 28..=31` to `read_head_async` was appended to cbor/tests.rs, and every unit module that reaches `extension_len` (the `cbor`, every `remote::codec` submodule, `remote::streams`, `party`, and `proxy::tests::malformed`) plus the `gossip_snapshot` and `decode_alloc` binaries were run: 136 tests. Decisive output:

        Starting 136 tests across 60 binaries (703 tests skipped)
            FAIL [   0.017s] ( 11/136) rumors tree::mirror::cbor::tests::zz_witness_async_reader_classifies_reserved_and_indefinite
        assertion `left == right` failed: initial byte 0x1c: async reader classified Indefinite, slice reader says Reserved
         Summary [   8.722s] 136 tests run: 135 passed, 1 failed, 703 skipped

With the arms swapped, all 135 existing tests in the selection passed and only the added witness failed; the whole suite was not run (by instruction), but the selection covers every caller of the mutated function found by grep (src/tree/mirror/party.rs:151, src/tree/mirror/streaming/remote/streams.rs:761, src/tree/mirror/streaming/remote/codec/greeting.rs:240 and :254, src/tree/mirror/streaming/remote/codec/decode/async_io.rs:302 and :503, and the test-only src/tree/mirror/streaming/remote/codec/decode.rs:87 and :199). The `1 + (input.len() - rest.len() - 1)` reading at cbor.rs:191 is assessed by reading only. The edits were restored afterwards.

See also: inventory-17.

### mirror-common-6: `LengthOverflow` is a codec-side error housed in the reader-side framing module
- Where: src/tree/mirror/framing.rs:55-65 (related: src/tree/mirror/streaming/remote/codec/frame.rs:306-313, src/tree/mirror/streaming/remote/codec/error.rs:75, src/tree/mirror/streaming/remote/adapter/error.rs:53, src/tree/mirror/streaming/remote/error.rs:19, src/error.rs:47)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: the only constructor is `checked_run_len` at frame.rs:311; nothing in framing.rs names a `u32` or a header; the public path is `rumors::error::LengthOverflow` via remote/error.rs:19 and error.rs:47)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate but expired (framing owned `length_header` and `LENGTH_HEADER_LEN` until 368da2a5 deleted them; the V1-retirement note kept the type for survival, not placement)
- Owner-gated: no

`framing` documents itself as "the memory policy beneath every variable-length body read"; both its functions take `len: usize` and nothing in the file converts to `u32` or writes a header. `LengthOverflow` describes a length "which cannot be represented by a `u32` wire length header" and is produced only by the encoder's `checked_run_len`. A reader of `framing` meets a type none of its code can produce; the encoder imports its own error from a module about reading. Modules have one responsibility.

Evidence:

    55	/// A payload length which cannot be represented by a `u32` wire length
    56	/// header.
    57	#[derive(Debug, thiserror::Error)]
    58	#[error("payload length {len} exceeds the u32 framing limit")]
    59	pub struct LengthOverflow {

Resolution: Move `LengthOverflow` beside `checked_run_len` (or into `remote/codec/error.rs`); repoint the `pub use` at remote/error.rs:19 so `rumors::error::LengthOverflow` is unchanged. Acceptance: framing.rs defines only `PAYLOAD_CHUNK_LEN`, `chunk_boundary_cuts`, `read_payload`, `resume_payload`; the public path still resolves.

### mirror-common-7: The framing readers demand a `Sized` reader, forcing a double reborrow and a fully qualified path in `party.rs`
- Where: src/tree/mirror/framing.rs:77-80 (related: src/tree/mirror/framing.rs:95-99, src/tree/mirror/party.rs:116, src/tree/mirror/cbor.rs:225, src/tree/mirror/handshake.rs:267)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep of every caller; tokio 1.52.3 `io/util/read_buf.rs:11-14` takes `R: AsyncRead + Unpin + ?Sized`, and `resume_payload`'s body calls only `read.read_buf`)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found
- Owner-gated: no

`read_payload` and `resume_payload` bound `R: AsyncRead + Unpin` (implicitly `Sized`), unlike their neighbours `cbor::read_head_async` and `Staged::fill`, which take `?Sized`. The party hand-off is generic over `R: ?Sized`, so its call has to reborrow `&mut &mut *reader` and, alone among the file's uses, spells the full crate path. Bounds no wider than needed; imports over long qualified paths.

Evidence:

    77	pub(crate) async fn read_payload<R: AsyncRead + Unpin>(
    78	    read: &mut R,
    79	    len: usize,
    80	) -> std::io::Result<Vec<u8>> {

    party.rs:
    116	    let bytes = crate::tree::mirror::framing::read_payload(&mut &mut *reader, len)

Resolution: Add `?Sized` to both bounds; `use crate::tree::mirror::framing::read_payload;` in party.rs and call `read_payload(reader, len)`. Acceptance: party.rs:116 reads `read_payload(reader, len)`; the other callers (greeting.rs:273, decode/async_io.rs:430 and :460, testing.rs:111) are unchanged.

### mirror-common-16: `Handshaken` keeps a cloned `Greeting` to read two scalars, and the `(len, version)` election key is spelled three ways
- Where: src/tree/mirror/streaming.rs:88-92 (related: src/tree/mirror/streaming.rs:101-104, src/tree/mirror/streaming.rs:117-131, src/tree/mirror/streaming.rs:165-180, src/tree/mirror/streaming.rs:184-191, src/tree/mirror/streaming/message.rs:141-146, src/peer/gossip.rs:1156, src/peer/gossip.rs:1160, src/peer/gossip.rs:1221)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: every `.peer()` reader outside the file reads `.version` only; `reconcile` reads `peer.version` and `peer.set_len`; both `complete_connect` implementations take `Greeting` by value, materialized.rs:544 and remote/proxy/start.rs:168)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found (the clone dates from 0aa29ed94, the commit that made the greeting carry the root listing; the two length scalars were added by 872aaaa19)
- Owner-gated: no

`handshake` clones the peer's `Greeting` (its `listing` is up to a full root fan, plus a `Version`) to hand one copy to `complete_connect` and retain the other in `Handshaken.peer`, from which only `version` and `set_len` are ever read. `descend` then takes `local_version, local_len, remote_version, remote_len` as four adjacent scalars that `reconcile` unpacks and re-pairs in a different order for `message::initiates(len, version, peer_len, peer_version)`: one concept, the role-election key, spelled three ways, with same-typed adjacent parameters the compiler cannot keep in order. `peer()` destructures `self` to return one field. Clone where a move would do (sign fixed, per session, negligible bytes); types-first legibility for the election key.

Evidence:

    88	    our_version: Version,
    89	    /// Our advertised live message count: our half of the role election's
    90	    /// primary key ([`message::initiates`]).
    91	    our_len: u64,
    92	    peer: message::Greeting,
    ...
    101	    pub(crate) fn peer(&self) -> &message::Greeting {
    102	        let Handshaken { peer, .. } = self;
    103	        peer
    104	    }
    ...
    169	    let client = client
    170	        .complete_connect(peer.clone())

Resolution: In `handshake`, take `peer_version` and `peer_len` before `complete_connect(peer)` and store them in place of `peer: Greeting`, symmetric with `our_version`/`our_len`; rename `peer()` to `peer_version()` (three gossip.rs readers). Optionally name the pair (`ElectionKey { set_len: u64, version: Version }`) so `descend(local, remote, ours, theirs)` and `initiates(ours, theirs)` speak one vocabulary. Acceptance: no `peer.clone()` in streaming.rs; `Handshaken` holds no `Greeting`; gossip.rs compiles against `peer_version()`; the streaming suites pass unchanged.

### mirror-common-17: The descent future is boxed twice: `reconcile` returns a `BoxFuture` and every caller `Box::pin`s it again
- Where: src/tree/mirror/streaming.rs:110-116 (related: src/peer/gossip.rs:1108-1126, src/peer/gossip.rs:1164, src/peer/gossip.rs:1223, src/peer/gossip/tests.rs:218, src/lib.rs:300-302, tests/future_size.rs:1-15)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the three call sites, each `let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());`; `#![deny(clippy::large_futures)]` at lib.rs:302)
- Seen by: structure, perfapi; refutation: confirmed; history: deliberate but expired (before 83edcd944 the callers' box carried the comment "Boxed: the descent state machine is a large future, and the codec buffers inflate it past the crate-wide `large_futures` ceiling"; 83edcd944 moved the box into `reconcile` and rewrote every caller with a second box, deleting the comment; neither box has a rationale today)
- Owner-gated: no

`Handshaken::reconcile` already returns `Pin<Box<dyn Future>>`; all three callers wrap it in another `Box::pin`, producing `Pin<Box<Pin<Box<dyn Future>>>>` coerced back to `BoxFuture`: one redundant allocation per session, one extra vtable hop per poll of the descent, and an annotation that appears to perform the erasure. Legibility: a reader asking why the future is boxed twice finds no answer at either site; the codegen-boundary rationale lives on the outer `Reconciliation::reconcile` shell (gossip.rs:1118-1126), which owns its own box.

Evidence:

    110	    pub(crate) fn reconcile<'a>(
    111	        self,
    112	    ) -> BoxFuture<'a, Result<(C::Output, S::Output), Error<C::Error, S::Error>>>
    113	    where
    114	        Self: 'a,
    115	    {
    116	        Box::pin(async move {

    gossip.rs:
    1164	            let descent: BoxFuture<'_, _> = Box::pin(handshaken.reconcile());

Resolution: Keep the inner box (it is what satisfies `clippy::large_futures` at the await sites) and delete the callers' re-boxing: `let (root, (read, write)) = handshaken.reconcile().await.map_err(streaming_error)?;` at gossip.rs:1164-1165, :1223-1224, and gossip/tests.rs:218-219. State the reason at `reconcile`'s doc ("Boxed: the descent state machine is a large future; the box keeps every await of it under the crate's `large_futures` ceiling."). Acceptance: exactly one `Box::pin` stands between `reconcile`'s body and each `.await`; `grep -rn 'Box::pin(handshaken.reconcile())'` is empty; gossip, bootstrap, and `tests/future_size.rs` pass unchanged.

### mirror-common-19: Two unrelated types named `ErrorRoute`, both "one-slot first-error routes", in sibling modules
- Where: src/tree/mirror/streaming/driver.rs:19-23 (related: src/tree/mirror/streaming/driver.rs:34-44, src/tree/mirror/streaming/remote/streams.rs:496-520, src/tree/mirror/streaming/remote/streams.rs:528)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep `struct ErrorRoute`: driver.rs:20 and streams.rs:498; read both)
- Seen by: structure; refutation: confirmed (the streams one carries a `supply_failure` slot and `supply_failed`, so the unification is not trivial reuse; the rename is the essential part); history: no rationale found
- Owner-gated: no

`driver::ErrorRoute<E, S>` (a `try_send` reporter with a `wrap: fn(E) -> S`) and `remote::streams::ErrorRoute` (a `try_send` reporter over `StreamError` with a deposited supply failure) share a name, a one-capacity `mpsc` channel, and a `report` method with the same "first wins, the rest are cascade" semantics, but are distinct types with distinct receivers (`FirstError` versus `FirstStreamError`). A reader following `ErrorRoute` from the driver lands in `streams.rs` and vice versa. One name, one thing.

Evidence:

    19	/// One endpoint's typed route into the shared session error.
    20	pub struct ErrorRoute<E, S> {
    21	    send: mpsc::Sender<S>,
    22	    wrap: fn(E) -> S,
    23	}

    streams.rs:
    496	/// The reporting half of the session's one-slot first-error route.
    498	pub struct ErrorRoute {

Resolution: Minimum: rename the streams one `StreamErrorRoute`, matching its `FirstStreamError` twin. Optional: build the streams reporter on `driver::ErrorRoute<StreamError, StreamError>` (identity wrap) and keep only the `supply_failure` slot local, so the first-wins semantics are stated once. Acceptance: `grep -rn 'struct ErrorRoute' src` returns one definition, or two with distinct names.

### mirror-common-22: `driver.rs` carries an inline test module against the sibling-file convention

See module-graph-6, the entry of record for the six inline `#[cfg(test)] mod tests { ... }` blocks; src/tree/mirror/streaming/driver.rs:203-238 is one of them, and this partition's history note (the block predates the AGENTS.md rule and was never swept) is carried there. The partition's full entry is in `evidence/partitions/mirror-common.md`.

### mirror-common-25: The channel module's test/production stream swap is re-implemented in two consumers
- Where: src/tree/mirror/streaming/erased.rs:141-159 (related: src/tree/mirror/streaming/erased.rs:46-47, src/tree/mirror/streaming/materialized/common.rs:47-65, src/tree/mirror/streaming/channel.rs:75-90, src/tree/mirror/streaming/channel/instrumented.rs:176, src/tree/mirror/streaming/remote/adapter/decode.rs:84)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read channel.rs:75-90, the cfg split of `Receiver`/`Sender`/`channel`; instrumented.rs:176 `impl<T> Stream for Receiver<T>`; common.rs:47-65 `ok_channel_with` and `OkReceiverStream` re-derive the same split; the only other `ReceiverStream` use, adapter/decode.rs, wraps a locally created tokio channel)
- Seen by: structure; refutation: confirmed; history: no rationale found
- Owner-gated: no

`channel.rs` owns the cfg split between tokio's `Receiver` (production, not a `Stream`) and the instrumented `Receiver` (test, a `Stream`), but never exports a stream-typed receiver, so `erased.rs` defines a cfg'd `ReceiverStreamOf` alias plus a cfg'd `receiver_stream` function, and `materialized/common.rs` defines the same split again as `OkReceiverStream` plus `ok_channel_with`. Two consumers re-derive a fact that belongs to the module that created it, and the two derivations already differ in name and shape.

Evidence:

    143	#[cfg(test)]
    144	type ReceiverStreamOf<E> = Receiver<E>;
    ...
    147	#[cfg(not(test))]
    148	type ReceiverStreamOf<E> = ReceiverStream<E>;
    149	
    150	fn receiver_stream<E: Send>(receiver: Receiver<E>) -> ReceiverStreamOf<E> {
    151	    #[cfg(test)]
    152	    {
    153	        receiver
    154	    }
    155	    #[cfg(not(test))]
    156	    {
    157	        ReceiverStream::new(receiver)
    158	    }
    159	}

Resolution: In `channel.rs`, export one stream-typed receiver under both cfgs (`pub type ReceiverStream<T> = tokio_stream::wrappers::ReceiverStream<T>;` / `= instrumented::Receiver<T>;`) and a `stream_channel(role, capacity) -> (Sender<T>, ReceiverStream<T>)`. Then `erased.rs` drops `ReceiverStreamOf`, `receiver_stream`, and its `tokio_stream` import, and `common.rs` reduces `OkReceiverStream` to one alias over `channel::ReceiverStream<T>` and `ok_channel_with` to `(tx, rx.map(Ok))`. Acceptance: no `cfg(test)` on a channel type outside `channel.rs`; `tokio_stream::wrappers::ReceiverStream` is named only in `channel.rs` and `adapter/decode.rs`.

See also: materialized-16, deps-2, module-graph-3.

### module-graph-6: Six inline `#[cfg(test)] mod tests { ... }` blocks breach the sibling `tests.rs` convention
- Where: src/tree/mirror/streaming/driver.rs:203-204 (related: src/testing.rs:396-397; src/testing/transport.rs:801-802; src/tree/mirror/streaming/backend/local/adversarial.rs:127-128; src/tree/mirror/streaming/testing/failing.rs:267-268; src/tree/arb.rs:672-673)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rn -E '^\s*(pub(\([^)]*\))?\s+)?mod\s+tests?\s*\{' src`; `tools/` and `justfile` grepped for any placement check: none)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

AGENTS.md's testing section states the convention ("Unit and protocol tests live in a sibling file: `mod tests;` in the source, `tests.rs` next to it") and its reason (brevity of the implementation file). Six files carry a brace-bodied test module instead, one named `test` in the singular. Nothing mechanical enforces placement.

Evidence:

    src/tree/mirror/streaming/driver.rs:
       203	#[cfg(test)]
       204	mod tests {

    src/tree/arb.rs:
       672	#[cfg(test)]
       673	mod test {

    (the same two-line shape at testing.rs:396-397, testing/transport.rs:801-802,
     backend/local/adversarial.rs:127-128, streaming/testing/failing.rs:267-268)

Resolution: Move each body to a sibling file (`driver/tests.rs`, `testing/tests.rs`, `testing/transport/tests.rs`, `backend/local/adversarial/tests.rs`, `streaming/testing/failing/tests.rs`, `tree/arb/tests.rs`), leaving `#[cfg(test)] mod tests;`; rename `arb`'s `test` to `tests`. Pure moves. Acceptance: the grep above returns nothing.

Carried from mirror-common-22 (its history note): the `driver.rs` block, cbfe1aff3 on 2026-07-15, predates the AGENTS.md rule, dfd19c447 on 2026-07-24, and was never swept; `driver.rs` is the only production module among the six, the other five being test scaffolding. mirror-common-22 and suite-economics-11 refer here; testing-infra-5 (the two `src/testing` blocks, plus a `pin_mut!` idiom) and streaming-backend-window-14 (which recommends exempting test-only modules instead) keep their own entries.

See also: suite-economics-11, mirror-common-22, streaming-backend-window-14, testing-infra-5, tree-core-25, module-graph-14.

### streaming-tests-12: Poll-schedule literals are copied across files with their clamp and length unexplained
- Where: src/tree/mirror/streaming/tests/capacity.rs:42-52 (related: capacity.rs:118, 120, 177, 221-235; tests.rs:198-203; src/tree/mirror/streaming/channel/instrumented.rs:308-318; src/tree/mirror/streaming/backend/local/adversarial.rs:115-125)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep: `16_384` on 13 lines of capacity.rs and `2_048` on two lines of tests.rs; read both consumers: `delay.min(2)` at instrumented.rs:316 and adversarial.rs:123, and `.get(current.step).copied().unwrap_or(0)` at instrumented.rs:314 and adversarial.rs:121)
- Seen by: structure-prose ([10]), blind-spots ([34]), api-economics ([45]); refutation: confirmed (corrected the count from eleven to 13 lines); history: no rationale found (the `% 5` row was written in 0ad9077a against a clamp that already existed)
- Owner-gated: no

`assert_capacity_case` inlines the first three rows of `probe_schedules()`; tests.rs carries a fourth copy at length 2_048; `vec![2; 16_384]` recurs at 118, 120, and 177. Both consumers clamp every delay to 2, so the `% 5` row at 232 runs as 0,1,2,2,2 (a reader infers five delays and gets three), and an exhausted schedule runs the rest of the session undelayed with no signal, so whether 16_384 covers the largest fixtures is unmeasured. Named constants over magic numbers; one provider over four copies.

Evidence:

        42	    let schedules = [
        43	        (Vec::new(), Vec::new()),
        44	        (
        45	            vec![2; 16_384],
        46	            (0..16_384).map(|step| (step % 3) as u8).collect(),
        47	        ),
        48	        (
        49	            (0..16_384).map(|step| (step % 3) as u8).collect(),
        50	            vec![2; 16_384],
        51	        ),
        52	    ];

       232	        ((0..16_384).map(|s| (s % 5) as u8).collect(), Vec::new()),

Resolution: In tests.rs define `const MAX_DELAY: u8 = 2` (asserted equal to the consumers' clamp, or exported from them), `const SCHEDULE_LEN: usize = 16_384` with a one-line derivation, a `fn round_robin(modulus: u8) -> Vec<u8>`, and one `fn standard_schedules()` used by `assert_capacity_case`, `probe_schedules`, and `honors_redaction_under_leaf_parent_dispute`; rewrite or delete the `% 5` row. Optionally have `with_schedule` return the steps consumed so a test can assert `consumed <= SCHEDULE_LEN`. Acceptance: `grep -rn '16_384\|2_048' src/tree/mirror/streaming/tests*` hits only the constant definitions; the `% 5` row is gone or stated in terms of `MAX_DELAY`.

See also: streaming-backend-window-13.

### streaming-tests-22: fixtures.rs hand-rolls what its own `grown` and `rooted` express; the ceiling-of-node expression is written three times
- Where: src/tree/mirror/streaming/tests/fixtures.rs:336-380 (related: fixtures.rs:61-96, 424-471; src/tree/arb.rs:509-522; local_eq.rs:278-282; wedge.rs:99-100)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: the two version-chain loops in `one_sided_pair` (342-351, 360-369) and `divergent_cells_pair` (431-440, 446-459) are one tick per path with `Action::Insert(Message::new(()))`, exactly `grown(node, party, 1, &(), &paths)`; the `root` closures at 372-379 and 463-470 are `rooted` verbatim; `ceiling_of` repeats the expression; `arb.rs:517 root_with_ceiling` has the same body as `rooted_at`, and `arb.rs:509 leaf_sibling_path` is private)
- Seen by: structure-prose ([8]), api-economics ([44]); refutation: confirmed; history: no rationale found (the older builders date from ddc9a2d8; `grown`/`rooted` arrived a week later in 97dfbcdf and the older ones were never folded)
- Owner-gated: no

`one_sided_pair` and `divergent_cells_pair` each contain two loops that are `grown` and a `root` closure that is `rooted`; `ceiling_of` is `rooted`'s ceiling expression a third time; `rooted_at` and `arb::root_with_ceiling` are one constructor under two names; and the "leaf whose path differs only in byte 31" builder exists as `arb::leaf_sibling_path`, `local_eq::leaf`, and inline at wedge.rs:99-100. The fixture file's contract is "a shared base grown per party", and it should read that way once.

Evidence:

       372	    let root = |node: Option<TreeNode<height::Root>>| Root {
       373	        ceiling: node
       374	            .as_ref()
       375	            .map(TreeNode::ceiling)
       376	            .cloned()
       377	            .unwrap_or_default(),
       378	        root: node,
       379	    };
       380	    (root(a_node), root(b_node))

Resolution: Rewrite `one_sided_pair` and `divergent_cells_pair` as `grown` calls over path lists (parties 0/1 and 0/2/1 as today) and `rooted`; define `rooted(node)` as `rooted_at(node, ceiling_of(&node))`; make `leaf_sibling_path` `pub(crate)` in `tree::arb` (or move it to fixtures) and use it at local_eq.rs:278 and wedge.rs:99; drop one of `root_with_ceiling`/`rooted_at` (the arb.rs side is outside this partition and optional). Acceptance: fixtures.rs contains one `Version::tick` loop (inside `grown`) and one `map(TreeNode::ceiling)` expression; `bytes[31] =` appears once in the streaming test tree.

### streaming-tests-24: The role election and the advertised length are mirrored twice in the suite
- Where: src/tree/mirror/streaming/tests/skeleton.rs:148-168 (related: stats.rs:63-75; src/tree/mirror/streaming/backend.rs:385-394)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read: `advertised_len` (163-168) is byte-for-byte the body of `pub(crate) fn len` on `Root<B>` (backend.rs:389-394); `initiates` is called at skeleton.rs:149 and stats.rs:74, each wrapped in a role mirror)
- Seen by: structure-prose ([9]), api-economics ([47]); refutation: reframed ([47]'s premise that private fields force the duplication is wrong: `tree::Root`'s fields are visible to descendants of `tree`, which is how skeleton.rs reads `root.root` and fixtures.rs constructs `Root { ceiling, root }`; the `live()` detour through `StreamingRoot<Local>` is a choice); history: no rationale found (`client_role` was re-rigged in 872aaaa1 as "the test mirror" of `message::initiates`; `a_initiates` and `live` were added the next day without reference to it)
- Owner-gated: no (an optional `tree::Root::len()`/`ceiling()` accessor would be an API decision; the in-suite consolidation needs none)

`skeleton::client_role` (returning `Party`) and `stats::a_initiates` (returning `bool`) both mirror `message::initiates` over two `tree::Root`s; `skeleton::advertised_len` reimplements `Root<B>::len`, which `stats::live` reaches by cloning into `StreamingRoot<Local>`. Two election mirrors can drift from each other and from `initiates` independently, and the inline `crate::tree::mirror::streaming::message::initiates(` path at 149 hides that stats.rs imports the same function.

Evidence:

       148	pub(super) fn client_role(client: &TreeRoot, server: &TreeRoot) -> Party {
       149	    if crate::tree::mirror::streaming::message::initiates(
       150	        advertised_len(client),
       151	        &client.ceiling,
       152	        advertised_len(server),
       153	        &server.ceiling,
       154	    ) {

       163	fn advertised_len(root: &TreeRoot) -> u64 {
       164	    root.root
       165	        .as_ref()
       166	        .map(|node| node.len() as u64)
       167	        .unwrap_or_default()
       168	}

Resolution: Keep one `client_role(&Root, &Root) -> Party` (in skeleton.rs or fixtures.rs) importing `initiates`, compute lengths through `Root<Local>::len` or one shared `advertised_len`, and define `a_initiates` as `client_role(a, b) == Party::I` (or use `client_role` directly) and `live` as the shared length helper. Acceptance: one call to `initiates` and one definition of the advertised length remain in the partition.

### Nits (16)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| inventory-17 | `src/tree/mirror/cbor.rs:191-194` | Head width computed as `1 + (n - 1)` | `let width = input.len() - rest.len();` |
| mirror-common-11 | `src/tree/mirror/handshake.rs:176-178` | Two widths restated as literals beside the constants that name them | Write `[u8; MISMATCH_PREVIEW_LEN]` at both variants; add `Network::LEN` and replace `NETWORK_LEN` and network.rs's literal 16s with it |
| mirror-common-18 | `src/tree/mirror/streaming/driver.rs:3-3` | Redundant `std::future::Future` imports under edition 2024 | `use std::pin::pin;` in driver.rs and tasks.rs (six more redundant `Future` imports sit outside the partition) |
| mirror-common-20 | `src/tree/mirror/streaming/driver.rs:65-72` | Visibility wider than use: `pub` items with in-file-only callers | Make the four driver items private and `descend` private |
| mirror-common-24 | `src/tree/mirror/streaming/erased.rs:116-139` | `ReplyResultStream` stores a fn pointer that its own type parameters already determine | Replace the fn-pointer field with `PhantomData<fn() -> H>` and call `assume_reply::<B, H>` directly in `poll_next` |
| mirror-common-28 | `src/tree/mirror/streaming/message.rs:158-162` | `Reply`'s field of `Reaction`s is named `replies` | Rename the field to `reactions` in `message::Reply` and `erased::Reply` (mechanical) |
| mirror-common-29 | `src/tree/mirror/streaming/protocol.rs:10-10` | `protocol.rs` re-states the module-wide `type_complexity` allow it already inherits | Delete protocol.rs:10 |
| mirror-common-31 | `src/tree/mirror/streaming/protocol.rs:24-24` | The typestate trait is named `Protocol`, colliding with the public `Protocol` enum one directory up | Rename the trait (`Phase` reads naturally against `Reply`, `CompleteEqual`, and the rest) and give it a one-line doc (mirror-common-30) |
| mirror-common-32 | `src/tree/mirror/streaming/protocol.rs:116-118` | The "rustc explodes" boxing rationale is pasted three times instead of stated once at `BoxResponses` | State the boxing rationale once on `BoxResponses`'s doc and delete the three pasted comments |
| module-graph-9 | `src/tree/mirror.rs:3-5` | `streaming` is the one path segment whose contrast retired with V1 | Owner call |
| module-graph-12 | `src/tree/mirror/streaming.rs:45-58` | `pub` versus `pub(crate)` on module declarations under the private `tree` encodes no visibility difference | Normalize the ten `pub mod` sites to `pub(crate)`, or adopt `#![warn(unreachable_pub)]` and follow every diagnostic |
| streaming-tests-4 | `src/tree/mirror/streaming/tests.rs:92-94` | Em-dashes in `//` code comments | Replace the eight em-dashes with a colon, semicolon, or ` -- `; add a `tools/` check for the character on `//` lines |
| streaming-tests-14 | `src/tree/mirror/streaming/tests/capacity.rs:116-123` | Statements that carry nothing: a root returned to be dropped, a bound implied by the line above it, a conditional implied by reflexivity | Return `()` from the closure and delete `drop(pair)` |
| streaming-tests-15 | `src/tree/mirror/streaming/tests/capacity.rs:136-143` | Magic numbers where a named constant exists or is owed | Import `window::FAN` and write `FAN`, `FAN - 2`, `FAN - 3` with the derivation stated once at 183-192 |
| streaming-tests-21 | `src/tree/mirror/streaming/tests/fixtures.rs:17-18` | Inline qualified paths and split import groups where an import would do | Add the imports at file top, merge the split groups, and put a blank line after fixtures.rs:17 |
| streaming-tests-25 | `src/tree/mirror/streaming/tests/skeleton.rs:351-365` | skeleton.rs re-derives height parity beside its own `asks`, and keys channel projections by `&'static str` | Define `asker_at`/`answerer_at` through `asks`; key `trace_channels` by a small `Ord` enum rather than `&'static str` |

## Streaming backend and window

Files: src/tree/mirror/streaming/backend/, convert.rs, window.rs, channel.rs, testing/failing.rs, testing/faulting.rs. Entries: 16 (1 medium, 8 low, 7 nit).

### deps-2: tokio-stream patches a Stream gap channel.rs leaves open, and two consumers carry duplicated cfg(test) duals because of it
- Where: src/tree/mirror/streaming/channel.rs:75-76 (related: src/tree/mirror/streaming/erased.rs:46-47, src/tree/mirror/streaming/erased.rs:141-159, src/tree/mirror/streaming/materialized/common.rs:4-5, src/tree/mirror/streaming/materialized/common.rs:47-65, src/tree/mirror/streaming/materialized/work/assembly.rs:6-7, src/tree/mirror/streaming/materialized/work/assembly.rs:73, src/tree/mirror/streaming/materialized/work/assembly.rs:84, src/tree/mirror/streaming/remote/adapter/decode.rs:7-8, src/tree/mirror/streaming/remote/adapter/decode.rs:83-84, src/tree/mirror/streaming/remote/adapter/decode.rs:399-404, src/tree/mirror/streaming/channel/instrumented.rs:152-182, Cargo.toml:138)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (grep census of `tokio_stream` over src/; read channel.rs in full, instrumented.rs:140-190, erased.rs:105-165, common.rs:1-70, assembly.rs:1-20 and 60-90, decode.rs:1-30 with its `ReceiverStream`/`mpsc` sites; grep of `mpsc::channel|mpsc::Receiver|mpsc::Sender` and of `.recv()|.try_recv(` over the streaming module; read futures-util 0.3.32 `stream/stream/mod.rs` for `fn next` (273) and `fn fuse` (1245))
- Verification: reframed in two places (below); history: no-rationale-found (tokio-stream arrived with 61ddf3a5 "WIP: streaming protocol, mostly reorg"; the 2c73d032 table migration moved it without pruning, its message stating `cargo tree` was byte-identical)
- Owner-gated: no

channel.rs exists to make the test and production channel types interchangeable, but its production side re-exports tokio's `Receiver` bare, which does not implement `Stream`, while the instrumented `Receiver` does (instrumented.rs:176). Two consumers therefore carry `#[cfg(test)]`/`#[cfg(not(test))]` duals of a type alias plus a function body each, wrapping the production receiver in `tokio_stream::wrappers::ReceiverStream` (erased.rs:141-159, common.rs:47-65), and assembly.rs imports `tokio_stream::StreamExt` for `.fuse()` and `.next()`, methods `futures::StreamExt` (imported in the same crate everywhere else, and `futures::{Stream, stream}` is imported in that file) provides identically. Two corrections to the sweep: the crate's tokio-stream footprint is five sites in four files, not four, because decode.rs wraps two raw `tokio::sync::mpsc` receivers (lines 84 and 404) that never pass through channel.rs; and the `try_recv` sites the sweep listed as forwarding surface (driver.rs:80, streams.rs:570) are on raw `tokio::sync::mpsc` receivers (`FirstError(mpsc::Receiver<E>)` at driver.rs:66, `receive: mpsc::Receiver<StreamError>` at streams.rs:529), not channel.rs's type, and the instrumented `Receiver` itself exposes only `recv` and `Stream`, so the production wrapper's forwarding surface is `recv` alone.

Evidence:

    src/tree/mirror/streaming/channel.rs
        75	#[cfg(not(test))]
        76	pub use tokio::sync::mpsc::{Receiver, Sender};

    src/tree/mirror/streaming/erased.rs
       141	/// The channel receiver as a stream, uniform across the test and
       142	/// production channel types.
       143	#[cfg(test)]
       144	type ReceiverStreamOf<E> = Receiver<E>;
       145	/// The channel receiver as a stream, uniform across the test and
       146	/// production channel types.
       147	#[cfg(not(test))]
       148	type ReceiverStreamOf<E> = ReceiverStream<E>;
       149	
       150	fn receiver_stream<E: Send>(receiver: Receiver<E>) -> ReceiverStreamOf<E> {
       151	    #[cfg(test)]
       152	    {
       153	        receiver
       154	    }
       155	    #[cfg(not(test))]
       156	    {
       157	        ReceiverStream::new(receiver)
       158	    }
       159	}

    src/tree/mirror/streaming/materialized/common.rs
        60	/// The type of a receiver stream wrapping items in `Ok`.
        61	#[cfg(test)]
        62	pub type OkReceiverStream<T, E> = stream::Map<Receiver<T>, fn(T) -> Result<T, E>>;
        63	/// The type of a receiver stream wrapping items in `Ok`.
        64	#[cfg(not(test))]
        65	pub type OkReceiverStream<T, E> = stream::Map<ReceiverStream<T>, fn(T) -> Result<T, E>>;

    src/tree/mirror/streaming/materialized/work/assembly.rs
         6	use futures::{Stream, stream};
         7	use tokio_stream::StreamExt;
        73	        let mut level = pin!(level.fuse());
        84	                        next_or_cancelled(level.next()).await?

    src/tree/mirror/streaming/remote/adapter/decode.rs
         7	use tokio::sync::mpsc;
         8	use tokio_stream::wrappers::ReceiverStream;
        83	        let (tx, rx) = mpsc::channel::<Result<(Prefix<Z>, B::Node<Z>), B::Error>>(FAN);
        84	        let leaves = ReceiverStream::new(rx);
       404	    let leaves = ReceiverStream::new(leaves);

    src/tree/mirror/streaming/channel/instrumented.rs
       152	impl<T> Receiver<T> {
       153	    /// Receive one item after the scheduled suspension points.
       154	    pub async fn recv(&mut self) -> Option<T> {
       176	impl<T> Stream for Receiver<T> {

Resolution: Move the boundary into channel.rs. (a) Minimal: channel.rs exports under both cfgs a `pub type ReceiverStream<T>` (tokio-stream's wrapper in production, `Receiver<T>` under test) and `pub fn into_stream<T>(rx: Receiver<T>) -> ReceiverStream<T>`; erased.rs:141-159 and common.rs:47-65 collapse to one line each and lose their duals; assembly.rs:7 becomes `futures::StreamExt`. (b) Full: in production channel.rs defines `pub struct Receiver<T>(tokio::sync::mpsc::Receiver<T>)` implementing `Stream` via `poll_recv` and forwarding `recv` (the only method channel.rs consumers call: materialized.rs:874, levels.rs:140/381/565/662, encode.rs:59/95, pump.rs:191/289/383), which makes both variants `Stream`; decode.rs's two raw channels then either route through channel.rs (which also brings the adapter's leaf channel under the instrumented channel's test observation) or take a local `poll_recv`-based wrapper, after which `tokio-stream` leaves rumors' `[dependencies]` and the workspace table. Acceptance: erased.rs and common.rs contain no `#[cfg(test)]`/`#[cfg(not(test))]` pair around the receiver-stream type; `grep -rn 'tokio_stream::StreamExt' src/` is empty; under (b), `grep -rn tokio_stream src/` is empty and `cargo tree -p rumors -e normal --depth 1` omits tokio-stream.

See also: materialized-16, mirror-common-25, module-graph-3.

### module-graph-3: `channel.rs` swaps channel types by `cfg(test)`, and the split leaks `cfg` forks into three other files, two of them duplicating one adapter
- Where: src/tree/mirror/streaming/channel.rs:75-90 (related: src/tree/mirror/streaming/erased.rs:46-47, :141-159; src/tree/mirror/streaming/materialized/common.rs:4-5, :47-65; src/tree/mirror/streaming/backend/local.rs:142-145, :165-173, :185-188, :222-225; Cargo.toml:145)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (all sites read; `grep -rn mpsc src` for channel bypasses)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`Sender`/`Receiver`/`channel` are Tokio's types under `cfg(not(test))` and the instrumented wrapper under `cfg(test)`. Tokio's `Receiver` is not a `Stream` and the wrapper is, so `erased.rs` and `materialized/common.rs` each carry their own cfg-forked receiver-as-stream alias and constructor, and `backend/local.rs` carries four `#[cfg(test)] return adversarial::...; #[cfg(not(test))] value` forks. The integration-test build compiles the production arms (the self dev-dependency at `Cargo.toml:145` enables `test-internals` without `cfg(test)`), so this is a legibility and duplication cost, not a coverage gap.

Evidence:

    src/tree/mirror/streaming/channel.rs:
        75	#[cfg(not(test))]
        76	pub use tokio::sync::mpsc::{Receiver, Sender};
    ...
        84	#[cfg(test)]
        85	pub use instrumented::{
        86	    ChannelReport, Receiver, Sender, channel, with_kind_capacity, with_observation, with_schedule,
        87	};

    src/tree/mirror/streaming/erased.rs:
       143	#[cfg(test)]
       144	type ReceiverStreamOf<E> = Receiver<E>;
       145	/// The channel receiver as a stream, uniform across the test and
       146	/// production channel types.
       147	#[cfg(not(test))]
       148	type ReceiverStreamOf<E> = ReceiverStream<E>;

    src/tree/mirror/streaming/materialized/common.rs:
        61	#[cfg(test)]
        62	pub type OkReceiverStream<T, E> = stream::Map<Receiver<T>, fn(T) -> Result<T, E>>;
        63	/// The type of a receiver stream wrapping items in `Ok`.
        64	#[cfg(not(test))]
        65	pub type OkReceiverStream<T, E> = stream::Map<ReceiverStream<T>, fn(T) -> Result<T, E>>;

    src/tree/mirror/streaming/backend/local.rs:
       142	        #[cfg(test)]
       143	        return adversarial::stream(adversarial::Role::Children { height: H::HEIGHT }, children);
       144	        #[cfg(not(test))]
       145	        children

    Cargo.toml:
       145	rumors = { workspace = true, features = ["test-internals", "conformance"] }

Resolution: Export from `channel.rs`, under both cfgs, one `ReceiverStream<T>` type and `fn into_stream(rx: Receiver<T>) -> ReceiverStream<T>` (production wraps `tokio_stream::wrappers::ReceiverStream`; the test arm is the identity because the instrumented `Receiver` implements `Stream`), then delete `erased.rs:141-159` and `common.rs:47-65`'s forks in its favour. Leave `local.rs`'s four adversarial forks unless a no-op `adversarial` shim reads better than four visible forks. Acceptance: `grep -rn 'cfg(not(test))' src/tree/mirror/streaming` returns only `channel.rs` and `local.rs`.

See also: materialized-16, mirror-common-25, deps-2, streaming-backend-window-12.

### streaming-backend-window-5: Manual `Clone` for `Root<B>` justified by a `T: Clone` bound that erasure removed; `T` also named at line 51
- Where: src/tree/mirror/streaming/backend.rs:374-383 (related: src/tree/mirror/streaming/backend.rs:50-52, 367-372; src/message.rs:48)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 2fd590d57:src/tree/mirror/streaming/backend.rs` has `pub struct Root<B, T>` at line 142 with the identical comment at 152; `git show 48bc31df -- backend.rs` retypes `impl<B, T> Clone for Root<B, T>` to `impl<B: Backend<Node<Z>: Leaf>> Clone for Root<B>` and leaves the comment; the derive-compiles claim is assessed from the GAT bound at line 52, not compiled)
- Seen by: structure, prose; refutation: confirmed; history: deliberate-but-expired (true at 2fd590d5, expired at 48bc31df)
- Owner-gated: no

`Root<B>` has one type parameter, but the comment says the derive "would demand `T: Clone`". `Backend: Clone` (46) and `Node<H>: ... + Clone` (52) supply every bound `#[derive(Clone)]` needs for `ceiling: Version` and `root: Option<B::Node<height::Root>>`, so the manual impl and its rationale are leftovers. Line 51's "decode as `T` at the wire boundary" names the same erased parameter; `Message` is not generic. AGENTS.md hard rule: nothing in the tree refers to code that no longer exists.

Evidence:

       374	// Manual because the derive would demand `T: Clone`; nodes are cloneable
       375	// handles regardless of the message type they carry.
       376	impl<B: Backend<Node<Z>: Leaf>> Clone for Root<B> {
    ...
        50	    /// The type of nodes, indexed by height `H`; leaf payloads are
        51	    /// erased in storage and decode as `T` at the wire boundary.

Resolution: Replace the manual impl with `#[derive(Debug, Clone)]` on `Root<B>` and delete the comment; if the derive fails for a reason other than `T`, state that reason at the impl. At line 51, drop the clause or name what the wire produces ("decode as the peer's message type at the wire boundary"). Acceptance: no bare `T` in backend.rs prose; `Root<B>` is `Clone` by derive, or by a manual impl whose comment is true of today's code.

### streaming-backend-window-12: `adversarial::Role` is threaded through three layers and never read; its recorded heights disagree between `children` and `leaves`
- Where: src/tree/mirror/streaming/backend/local/adversarial.rs:10-15 (related: adversarial.rs:47-62, 101-125; backend/local.rs:142-145, 165-173, 185-188, 222-225; testing/failing.rs:28-33, 251-253)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (the only consumer is `next_delay(_role: Role)`, which binds the parameter with an underscore and reads only the thread-local; `Role` appears nowhere outside adversarial.rs and local.rs)
- Seen by: structure, correctness; refutation: confirmed; history: no-rationale-found (`_role` has been unread since the file's creation at 065356abe, a one-line commit; nothing records what it was reserved for)
- Owner-gated: no

`Role` is constructed at four sites in local.rs, stored on `DelayedFuture`/`DelayedStream`, passed to `delay()`, and discarded by `next_delay`. Meanwhile `Local::children<H>` records `Role::Children { height: H::HEIGHT }` where `H` is the child height, while `Local::leaves<H>` records the same variant where `H` is the source height, and `failing::Operation::Children` documents the source height. The inconsistency is harmless only because nothing reads the field. Circular justification: the type exists to be passed to a function that ignores it, and each `Local` method carries a four-line `cfg` split whose only test-specific content is the unused label.

Evidence:

        10	/// One typed asynchronous surface of [`Local`](super::Local).
        11	#[derive(Clone, Copy, Debug, Eq, PartialEq)]
        12	pub(super) enum Role {
        13	    Children { height: usize },
        14	    Parent { height: usize },
        15	}
    ...
       115	fn next_delay(_role: Role) -> u8 {

Resolution: Delete `Role` and the parameter from `future`/`stream`/`delay`/`next_delay`; optionally compile `adversarial` unconditionally with `#[cfg(not(test))]` identity versions of `future`/`stream` so each `Local` method ends in one line. If a role-keyed schedule is wanted instead, settle the height convention (source height, matching `failing::Operation`) and read the field. Acceptance: `Role` is read by the scheduler or is gone; local.rs's four `Backend` methods no longer construct a label.

### streaming-backend-window-13: The poll-delay scheduler exists in two copies, the countdown in three, the `.min(2)` cap in three files unnamed, and the `RoleStats` merge twice
- Where: src/tree/mirror/streaming/backend/local/adversarial.rs:17-44 (related: adversarial.rs:101-125; channel/instrumented.rs:34-45, 120-129, 159-167, 207-238, 288-294, 308-318; src/testing/transport.rs:180; src/tree/mirror/streaming/tests.rs:84-85)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep `min(2)`: adversarial.rs:123, instrumented.rs:316, transport.rs:180, none commented; the `Schedule`/`with_schedule`/`next_delay` blocks read identical modulo the ignored `Role`; streaming/tests.rs:84-85 nests `with_schedule` inside `with_local_schedule`)
- Seen by: structure, prose, perfapi; refutation: confirmed (adds the third cap at transport.rs:180); history: no-rationale-found (the scheduler entered inline at c9b8f38b0 and was copied at 065356abe, both one-line commits)
- Owner-gated: no

adversarial.rs and instrumented.rs each define an identical `Schedule { delays, step }`, an identical `with_schedule` with a `Restore` drop guard, and an identical `next_delay` including an unexplained `delay.min(2)` cap; the take-a-delay-then-self-wake countdown is written three times (adversarial.rs `delay()`, `Sender::send`, `Receiver::poll_recv`); a third uncommented copy of the cap sits in `src/testing/transport.rs`. Separately, `ChannelReport::kind` and `with_observation` fold `RoleStats` field by field with the same six rules. The two schedule cells are deliberately independent (streaming/tests.rs nests them), but the mechanism need not be duplicated to keep two cells. Duplicated logic drifts; the cap encodes a decision about schedule expressiveness that nothing states.

Evidence:

        17	struct Schedule {
        18	    delays: Vec<u8>,
        19	    step: usize,
        20	}
    ...
       121	        let delay = current.delays.get(current.step).copied().unwrap_or(0);
       122	        current.step += 1;
       123	        delay.min(2)
    (instrumented.rs)
       316	        delay.min(2)
    ...
        37	                total.channels += stats.channels;
        38	                total.effective_capacity = total.effective_capacity.max(stats.effective_capacity);

Resolution: Extract one test-support module (for example `streaming/testing/schedule.rs`) with a `ScheduleCell` around a `thread_local!` `RefCell<Option<Schedule>>` exposing `with` and `next`, plus a `Countdown(Option<u8>)` whose one method performs the wake-and-`Pending` step; instantiate two cells (backend, channel). Name the cap (`MAX_SCHEDULED_DELAY: u8 = 2`) with one sentence on why delays are bounded, and use it at transport.rs:180 too. Add `RoleStats::absorb(&mut self, other: RoleStats)` and call it from both folds. Acceptance: one definition each of the schedule struct, the restore guard, the next-delay read, the countdown step, and the `RoleStats` merge; both cells still independently settable; no bare `.min(2)`.

See also: streaming-tests-12.

### streaming-backend-window-26: `SCOPE_FIXED_BYTES` and `LEAF_REQUEST_BYTES` are hand-counted, name no types, and sit beside siblings whose docs state the rule against hand counting
- Where: src/tree/mirror/streaming/window.rs:158-163 (related: window.rs:139-156, 165-176, 243, 265; src/tree/mirror/streaming/materialized.rs:265-279; src/tree/mirror/streaming/remote/adapter/scope.rs:12-16; src/tree/typed/prefix.rs:17-34; src/tree/mirror/streaming/materialized/work/queues.rs:254; src/peer.rs:431, 452; window/tests.rs:24)
- Class / severity / confidence: idiom / low / medium
- Provenance: assessed (layout arithmetic from source, not compiled: `Prefix`/`ErasedPrefix` wrap `tinyvec::ArrayVec<[u8; 32]>`, which in the locked tinyvec 1.11.0 is `#[repr(C)] { len: u16, data: A }`, 34 bytes at alignment 2; `Query<E> { prefix: ErasedPrefix, ours: Vec<(u8, E)> }` and `Resolution<E> { prefix: ErasedPrefix, resolved: Vec<(u8, Resolve<E>)> }` each come to 64; the proxy `Scope { parent: ErasedPrefix, children: Vec<u8>, next: usize }` to 72; the leaf-request channel item at queues.rs:254 is a bare `Prefix<Z>`)
- Seen by: structure, prose, correctness; refutation: confirmed (adds `Resolution` as a third prefix-carrying container); history: already-known (the un-executed remainder of prior review R16, whose fix derived `REFERENCE_SLOT_BYTES` only; R16's padding argument lives only in the review note)
- Owner-gated: yes: deriving may move the pinned `SCOPE_ENVELOPE_BYTES` (5_431) and the "5431" figure at peer.rs:431

`REFERENCE_SLOT_BYTES` (139-156) and `FAN_SLOT_BYTES` (165-176) derive from `size_of` with the rationale that a hand-counted total goes stale. The two constants between them are literals whose docs name no containers, so the count cannot be audited against the code. Reading the candidates: if the two prefixes are `Query`'s and `Resolution`'s, 128 is exact today; if the second is the proxy `Scope` (whose `next: usize` the count omits), the fixed cost is 136; and a buffered scope has three prefix-carrying containers whose coexistence the doc does not settle. The 40-byte leaf request is the pointer-padded footprint of a 34-byte prefix, safe but not what the doc says. The 62,500 design-corpus figure is likewise restated by hand at window.rs:243 and peer.rs:452 beside its defining constant in window/tests.rs:24.

Evidence:

       158	/// Fixed in-memory bytes per buffered scope beyond its per-child slots:
       159	/// two inline prefixes (40 B each) and the container `Vec` headers.
       160	const SCOPE_FIXED_BYTES: usize = 2 * 40 + 2 * 24;
       161	
       162	/// In-memory bytes of one buffered leaf request: an inline leaf prefix.
       163	const LEAF_REQUEST_BYTES: usize = 40;
    ...
       142	/// Derived from `size_of` of the real slot types under the in-memory
       143	/// backend, so a layout change moves the price with it instead of
       144	/// leaving a hand-counted byte total stale: the pointer-aligned

Resolution: Name the containers and derive: `SCOPE_FIXED_BYTES = size_of::<Query<E>>() + size_of::<Resolution<E>>()` (or whichever pair is meant, at the `Local` erased instantiation) and `LEAF_REQUEST_BYTES = size_of::<Prefix<Z>>()` or its padded form with the padding intent stated; keep the pointer-class caveat the sibling docs carry. If the derived value differs from 128, re-pin `SCOPE_ENVELOPE_BYTES`, `tradeoff.md`, and the peer.rs figures in one deliberate commit naming the layout attribution. Cite `DESIGN_SESSION_MESSAGES` by name for the 62,500 restatements, or move that constant out of the test module. Acceptance: all four slot constants are `size_of` expressions over named types; `scope_envelope_matches_the_derivation` passes; if the pin moved, the commit states the attribution.

### streaming-backend-window-28: `from_budget`/`resolve` take four positional `u64`s where a length and a version-bytes bound can be swapped silently
- Where: src/tree/mirror/streaming/window.rs:338-345 (related: window.rs:524-531; src/tree/mirror/streaming/materialized.rs:624-630; src/tree/mirror/streaming/remote/proxy/start.rs:177-183; window/tests.rs:76, 88, 99)
- Class / severity / confidence: idiom / low / medium
- Provenance: assessed (read: the body at 346-358 uses only `max`, product, and sum of each pair, so local/remote order is immaterial; both production callers interleave fields from two greetings)
- Seen by: structure, perfapi; refutation: confirmed and reframed (the hazard is a length/version-bytes confusion, not side order); history: no-rationale-found (the signature accreted one argument at a time across d27cb5aa, 1988de6d, aabdcea0)
- Owner-gated: no

A `len` swapped with a `version_bytes` type-checks and mis-sizes the window, and an under-priced window is the one failure the docs call a memory breach rather than latency. Types-first: newtypes over same-typed positional runs. The test suite repeats the `(X, X, 0, 0, budget, f)` shape about fifteen times with no field names.

Evidence:

       338	    pub(crate) fn from_budget(
       339	        local_messages: u64,
       340	        remote_messages: u64,
       341	        local_version_bytes: u64,
       342	        remote_version_bytes: u64,
       343	        budget_bytes: usize,
       344	        node_bytes: impl Fn(usize, usize) -> usize,
       345	    ) -> Self {

Resolution: Introduce a small `Copy` struct (`Corpus { messages: u64, version_bytes: u64 }`, or reuse the greeting's pair) and take `local: Corpus, remote: Corpus`; derive it from `Root<B>` on the walk side and from the two greetings on the proxy side; shrink the test helpers accordingly. Acceptance: no call site passes four bare `u64`s to the solve; window/tests.rs compiles against the struct form and still passes.

### streaming-backend-window-29: Four-point monotonicity `debug_assert` in `from_budget` duplicates the conformance suite's grid sweep over a crate-internal boundary
- Where: src/tree/mirror/streaming/window.rs:360-370 (related: src/tree/mirror/streaming/backend.rs:110-117; window.rs:334-337; src/conformance/backend.rs:571-615, 651; window/tests.rs:316-318, 330-332)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (read: `node_bytes_monotone::<B>()` at conformance/backend.rs:584-615 sweeps every adjacent fan pair up to `FAN` crossed with a bound grid and runs at :651 for every backend the suite exercises; its doc at 580-581 says it exists because the derivation's check is "a four-point `debug_assert`, compiled out of release"; lib.rs:322 keeps `tree` private so no external `node_bytes` reaches the assert)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (prior review R22 asked for the sweep; 8d959048 added it and kept the assert, naming both in the trait doc without saying why)
- Owner-gated: no

A guard is justified by naming a constructible failure the committed tests cannot catch; for a deterministic pure function, a runtime spot check samples the same space the sweep samples deliberately. The one space the sweep does not reach is the ad-hoc closures window/tests.rs hands to `from_budget` directly (`materializing_node_bytes`, `|_, _| usize::MAX`), which are test inputs rather than backends. Three prose sites cite the assert as a safeguard on the production contract, which overstates what it does.

Evidence:

       360	        #[cfg(debug_assertions)]
       361	        for window in [0usize, 1, 16, FAN].windows(2) {
       362	            debug_assert!(
       363	                node_bytes(window[0], version_bound) <= node_bytes(window[1], version_bound),
       364	                "node_bytes must be monotone in the child count",
       365	            );

Resolution: Delete the assert and re-state the three prose references (backend.rs:113 "debug-asserted when a session derives its window", window.rs:337 "spot-checked here in debug builds", conformance/backend.rs:580-581) to name the conformance sweep alone; or keep it and say at the site that it guards test-supplied pricing closures, which is the only space the sweep misses. Acceptance: either no `debug_assert` on `node_bytes` in `from_budget` and the three sites cite `node_bytes_monotone`, or the assert's site names what it catches.

### streaming-backend-window-34: `BERNSTEIN_TAIL` is a hand-derived literal with a maintenance instruction where `tail_exponent(0)` computes the same number
- Where: src/tree/mirror/streaming/window.rs:601-606 (related: window.rs:622-624, 685)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (replica: `tail_exponent(0) = ceil(7 * 48 / 10) = 34 = BERNSTEIN_TAIL`; `jointly_occupied`'s doc at 685 already calls it the flat level)
- Seen by: prose; refutation: confirmed; history: no-rationale-found (prior review R75 asked that the tail literals be named so a change cannot strand a companion; b0304e71 named them as literals with a reminder rather than deriving)
- Owner-gated: no

"A changed tail level must move this with it" asks a human to do what the compiler can. R75's own goal favors the derivation.

Evidence:

       601	/// The Bernstein exponent delivering the union tail: `e⁻ᵗ ≤ 2⁻⁴⁸` needs
       602	/// `t ≥ UNION_TAIL_BITS × ln 2 ≈ 33.3`, rounded up.
       603	///
       604	/// Derived from [`UNION_TAIL_BITS`]; a changed tail level must move
       605	/// this with it.
       606	const BERNSTEIN_TAIL: u128 = 34;

Resolution: Make `tail_exponent` a `const fn` (replace `j.min(TAIL_DEPTH_CAP)` with an `if`; `u128::div_ceil` is const) and define `const BERNSTEIN_TAIL: u128 = tail_exponent(0);` with the doc reading "the flat union level: `tail_exponent` at zero sharpening". Drop the maintenance sentence. Acceptance: `BERNSTEIN_TAIL` has no literal initializer; `envelopes_are_consistent` and the window tests pass unchanged.

### Nits (7)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| streaming-backend-window-2 | `src/tree/mirror/streaming/backend.rs:46-46` | Bare `pub` on crate-internal items beside `pub(crate)` in the same layer | Enable `#![warn(unreachable_pub)]`, or record in AGENTS.md that bare `pub` inside private modules is the crate's style |
| streaming-backend-window-8 | `src/tree/mirror/streaming/backend/local.rs:103-129` | `Local::node_bytes` exists twice: an inherent fn and a trait impl that delegates to it | Move the doc onto the `Backend` impl's `node_bytes`, delete the inherent fn, and have testing.rs write `<Local as Backend>::node_bytes` |
| streaming-backend-window-14 | `src/tree/mirror/streaming/backend/local/adversarial.rs:127-128` | Inline `mod tests {}` blocks against the sibling-file convention | Move each block to a `tests.rs` sibling, or amend the convention to exempt test-only modules explicitly (my recommendation |
| streaming-backend-window-20 | `src/tree/mirror/streaming/convert.rs:83-91` | `S<H>::assemble` relays `fold_parents` through a pass-through generator left over from watermark stripping | `Box::pin(fold_parents(backend, H::assemble(backend.clone(), leaves)))`, the form the parent of 748325407 compiled |
| streaming-backend-window-23 | `src/tree/mirror/streaming/testing/faulting.rs:227-240` | faulting.rs: a bare `unreachable!()`, dated "current fixture" language over an unasserted premise, a type-admitted rejected `Fault`, and a duplicated corruption check | Give line 240 its proof ("handled by the early returns above") |
| streaming-backend-window-25 | `src/tree/mirror/streaming/window.rs:134-137` | `KEY_DEPTH` restates `height::Root::HEIGHT` as a fresh literal | `const KEY_DEPTH: usize = <height::Root as Height>::HEIGHT;`, or at minimum a `const _` assertion tying the two |
| streaming-backend-window-31 | `src/tree/mirror/streaming/window.rs:478-480` | `Window::capacity` clamps an out-of-range height that only programmer error can produce; two `unwrap_or`s guard conversions that cannot fail | Index directly with a one-line comment that typed heights bound the argument (or `debug_assert!(height <= KEY_DEPTH)` and index) |

## Materialized

Files: src/tree/mirror/streaming/materialized/. Entries: 20 (2 medium, 11 low, 7 nit).

### materialized-13: `complete_initiator` hand-rolls `tasks::complete` because `absorb` is the one walk not registered as a `Work` task
- Where: src/tree/mirror/streaming/materialized.rs:823-853 (related: src/tree/mirror/streaming/tasks.rs:19-37; src/tree/mirror/streaming/materialized/work.rs:81-83; src/tree/mirror/streaming/materialized.rs:862-911; src/tree/mirror/streaming/tests/announced.rs:27-28; src/tree/mirror/streaming/tests/skeleton.rs:646-647)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`diff` of the `select!` bodies at tasks.rs:26-36 and materialized.rs:841-851 after stripping indentation: the only differences are the binding names; grep `.stats()` shows materialized.rs:827 as `Work::stats()`'s only caller; `Requests<B, H>` is `Send + 'static` at protocol.rs:34-36, so `absorb`'s future can be boxed into `tasks`)
- Seen by: structure; refutation: confirmed; history: no rationale found (the `select!` and the terminal loop entered together in 7126e489, the commit that also introduced the shared primitive `Work::execute` wraps; `Work::stats()` was added by 487e17ea for this one caller)
- Owner-gated: no

The initiator's terminal state races `absorb` against `self.work.execute(self.finish)` with a `select!` that is, arm for arm, the body of `tasks::complete` ("Race registered work against its terminal operation, failing on either", tasks.rs:19). Every other walk publishes into `Work.tasks` and lets `complete` race it; `absorb` alone is driven by hand, which is why the initiator's terminal walk lives here instead of beside the other walk bodies and why `Work::stats()` exists. The comment at 838-840 restates `complete`'s contract rather than naming anything the primitive misses. Equivalence: `complete` over `tasks ∪ {absorb}` fails on absorb's error first (`try_run_all` over `FuturesUnordered` returns the first `Err`), awaits `finish` after all tasks, and awaits the remaining tasks (including absorb's trailing `UnaskedReply` check at 906) when `finish` resolves first, which is exactly the three arms here. One recorded fact bears on the change: 97dfbcdf found that this unbiased `select!` reorders the tail of back-to-back runs and stated the payload-independence bridge per channel because of it. `tasks::complete` uses the same unbiased `select!`, so the property is preserved, but two test-module docs cite the `select!` by its current location and must be re-pointed.

Evidence:

    838	        // Race rather than join: a violation in `absorb` must surface even
    839	        // though the session's remaining work, which includes streams the
    840	        // now-misbehaving counterparty feeds, may never complete.
    841	        tokio::select! {
    842	            absorbed = &mut absorb => {
    843	                absorbed?;
    844	                finish.await
    845	            }
    846	            finished = &mut finish => {
    847	                let root = finished?;
    848	                absorb.await?;
    849	                Ok(root)
    850	            }
    851	        }

Resolution: Add `Work::absorb_leaves(&mut self, their_version, ledger, requests, queries, returns)` in `levels.rs` beside `leaf_level` (or `assembly.rs` beside `assemble_leaves`) that pushes `Box::pin(absorb(...))` onto `self.tasks`, moving `absorb` there; `complete_initiator` becomes: erase the requests, call `absorb_leaves`, `work.execute(self.finish).await`. Delete the `select!`, the `pin!`s, the comment, and `Work::stats()`. Re-point announced.rs:27-28 and skeleton.rs:646-647 at `tasks::complete`. Acceptance: `complete_initiator` contains no `select!`; `absorb` is registered through `Work`; `Work::stats` is gone; `just gate` clean, including the `terminal_absorb_*` tests in `materialized/tests.rs`.

See also: materialized-9.

### materialized-34: `internal_walk`'s opening hand-off is two `Option<oneshot>`s plus two `Option<BTreeMap>`s awaited lazily inside the reaction loop; one enum consumed once at stream start says the same thing
- Where: src/tree/mirror/streaming/materialized/work/levels.rs:377-400 (related: work/levels.rs:118-120, 232-234, 301-302, 346-347, 443-449; src/tree/mirror/streaming/materialized.rs:378-389, 644-645, 696-697, 732-733, 747-748, 767-770; work/tests/violations.rs:290-291)
- Class / severity / confidence: simplification / medium / medium
- Provenance: assessed (read; the liveness argument is by reading the hand-off order: the initiator sends `early_tx` at 120 before its yield at 123 and its query at 134, the responder at 234 before its `yield_resolve_query!` at 239-247; the constructors set exactly one field `Some` at 644-645 and 696-697 and both `None` at 747-748, with only the `debug_assert!` at 767-770 stating the exclusion)
- Seen by: structure; refutation: confirmed (both halves); history: no rationale found (55d76d5c introduced the state and recorded the premise that makes an eager await safe, "oneshots filled before their consumer's first query can exist, never awaited across wire progress", not a reason for laziness; the two-`Option` shape has no recorded reason)
- Owner-gated: no

`Descending` carries `early_survivors` and `early_supplies`, two `Option<oneshot::Receiver<..>>` of which at most one is `Some` (a fact stated only in prose and a `debug_assert!`), and `internal_walk` receives both as separate parameters, awaits each on first need inside the reaction loop, and guards every consult with `early_x.is_some() || x.is_some()` followed by `if x.is_none() && let Some(early) = early_x.take()`. The senders are filled before the opening yields, and this stage's first query is sent only after that yield, so awaiting the oneshot at the top of the stream body waits for nothing the first `queries.recv()` did not already transitively wait for, and adds no edge to the wait graph. With that, both maps are plain `BTreeMap`s (empty below the first stage) and every guard collapses to `map.remove(&radix)`. This is the function the deadlock-freedom argument most depends on, and the lazy state doubles the branches a reader must hold for the root-level special cases; the change is a strict deletion of state with a fixed sign. The `unwrap_or_default()` also needs a stated reason in either design: a dropped sender means the opening stream failed before handing off, so it never sent this stage a query and the empty default never reaches a committed root.

Evidence:

    377	            let mut early_survivors = early_survivors;
    378	            let mut survivors: Option<BTreeMap<u8, Option<B::Erased>>> = None;
    379	            let mut early_supplies = early_supplies;
    380	            let mut supplied: Option<BTreeMap<u8, Vec<(u8, B::Erased)>>> = None;

    396	                    if supplied.is_none()
    397	                        && let Some(early) = early_supplies.take()
    398	                    {
    399	                        supplied = Some(early.await.unwrap_or_default().into_iter().collect());
    400	                    }

    381	    early_survivors: Option<oneshot::Receiver<Vec<(u8, Option<B::Erased>)>>>,
    ...
    389	    early_supplies: Option<oneshot::Receiver<Vec<(u8, Vec<(u8, B::Erased)>)>>>,

Resolution: Introduce `enum Opening<E> { Survivors(oneshot::Receiver<Vec<(u8, Option<E>)>>), Supplies(oneshot::Receiver<Vec<(u8, Vec<(u8, E)>)>>), None }` with the two field docs as variant docs; `Descending.opening: Opening<B::Erased>`; `initiator()`/`responder()` construct their arm; `reply()` passes `Opening::None`; the `debug_assert!` becomes `matches!(self.opening, Opening::None)`. `internal_level`/`internal_walk` take one `Opening` parameter and, at the top of the `try_stream!` body, match it once into `survivors: BTreeMap<..>` and `supplied: BTreeMap<..>` with a one-line comment ("filled before the opening yields; our first query follows that yield, so this await never crosses wire progress; a dropped sender means the opening never sent a query"). Replace the clause at 393 and block at 396-400 with `&& let Some(children) = supplied.remove(&radix)`; replace 443-449 with `if let Some(survivor) = survivors.remove(&radix)`. Acceptance: `internal_walk` has no `Option<BTreeMap>` and no `.take()` on the hand-off; no `Option<oneshot::Receiver<..>>` field on `Descending`; violations.rs passes `Opening::None` where it passed `None, None`; `Trace::assert_valid` on the real-session traces (streaming/tests.rs:95, 114), the capacity suite, and the early-supply tests under `remote/adapter/tests/opening.rs` still pass.

See also: materialized-8.

### materialized-1: `yield_resolve_query!` is defined in `materialized.rs` but every expansion is in `work/levels.rs`
- Where: src/tree/mirror/streaming/materialized.rs:127-171 (related: src/tree/mirror/streaming/materialized/work/levels.rs:239, 453, 468, 486, 586, 597, 677)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep `yield_resolve_query` across src/: definition at materialized.rs:133, seven expansions all in levels.rs, two prose mentions at progress.rs:203 and remote/proxy.rs:24)
- Seen by: structure; refutation: confirmed; history: no rationale found (the definition landed at 065356ab when the walks still lived in `work.rs`; `macro_rules!` textual scoping only requires the definition to precede `mod work;`)
- Owner-gated: no

The macro that fixes the three-phase publication order is defined 45 lines into the parent file while all seven expansions sit in `levels.rs`, so a reader of `internal_walk` leaves the file to learn what the macro yields, sends, and traces, and a reader of `materialized.rs` meets `progress::` calls whose module only the walks import. A helper belongs beside its only consumer.

Evidence:

    133	macro_rules! yield_resolve_query {

Resolution: Move the macro and its doc comment to the top of `work/levels.rs` (which already imports `progress` under `#[cfg(test)]` at lines 19-20). Acceptance: grep `yield_resolve_query` outside `levels.rs` matches only the `proxy.rs` prose mention.

### materialized-3: The typed `children_of` lives in `materialized/common.rs`, its only consumer is `erased::ops`, and the re-export comment names a consumer that no longer exists
- Where: src/tree/mirror/streaming/materialized.rs:186-188 (related: src/tree/mirror/streaming/materialized/common.rs:17-36; src/tree/mirror/streaming/erased.rs:205, 256; src/tree/mirror/streaming/remote/proxy/work/pump.rs:211)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep `children_of` across src/ excluding `ops::children_of`: the definition at common.rs:18, the re-export at materialized.rs:188, and erased.rs:205/256 importing it as `children_of_typed`; the proxy at pump.rs:211 calls `ops::children_of`)
- Seen by: structure; refutation: confirmed; history: deliberate but expired (55d76d5c added the re-export when pump.rs:30/186 called the typed helper directly; bf1a5b4b switched the proxy to `ops::children_of` and made `erased::ops` the typed helper's sole consumer)
- Owner-gated: no

Nothing under `materialized` calls `common::children_of`; every walk uses the erased `ops::children_of`, which wraps this one. The comment justifying the `pub(crate) use` says the remote proxy uses "the walks' own helper", but the proxy calls the erased wrapper, not this. That is a ghost reference under the hard rule, and a typed primitive that exists to be erased belongs beside its eraser.

Evidence:

    186	// The remote proxy explodes early-supplied whole root children into the
    187	// same per-child shape the walks consume, with the walks' own helper.
    188	pub(crate) use common::children_of;

Resolution: Move `children_of` into `erased::ops` as a private `fn children_of_typed` (or into `backend.rs` beside `Backend::children`); delete the `pub(crate) use` and its comment. Acceptance: `grep children_of src/tree/mirror/streaming/materialized` matches only `ops::children_of` call sites, and `erased.rs` imports nothing from `materialized`.

See also: module-graph-2.

### materialized-10: The walk imports `DEFAULT_TARGET_MESSAGE_SIZE` from `remote` and casts it to the greeting's type

See module-graph-2, the entry of record for the streaming core's four-module cycle, whose `materialized → remote` edge is this import (src/tree/mirror/streaming/materialized.rs:114 and :438); this entry's alternative resolution (define the constant beside `Greeting` in `message.rs`) and its owner-gating note (retyping the constant to `u64` changes a public item) are carried there. The partition's full entry is in `evidence/partitions/materialized.md`.

### materialized-11: The handshake phases duplicate the greeting derivation, the role-opening prelude, and the local version
- Where: src/tree/mirror/streaming/materialized.rs:563-602 (related: src/tree/mirror/streaming/materialized.rs:430-441, 507-538, 541-560, 614-633, 661-680; src/tree/mirror/streaming/protocol.rs:56-87)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`diff` of lines 516-528 against 572-584: identical; `diff` of 614-633 against 661-680: one line differs, `their_listing` bound versus `their_listing: _`; `Start.our_version` is initialised from `root.ceiling.clone()` at 434 and `root` rides every `Handshaking` phase unmutated; the trait shapes at protocol.rs:56-87 make `Accept::Next` and `CompleteConnect::Next` identical bounds)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: no rationale found (every greeting-field commit since 7126e489 grew both literals; 1988de6d, 487e17ea, and 50c8b0a3 each added a line to both preludes)
- Owner-gated: no

Three duplications in one 250-line stretch. (1) `accept` rebuilds the fan, the `Greeting` literal with both of its multi-line comments, and a `Connected`; `connect` followed by `complete_connect` produces the same values, and the trait shapes permit the composition. (2) `initiator` and `responder` share an identical eleven-line prelude (destructure, join ceilings, `window.resolve` with five arguments, `window_granted`, `SupplyLedger::new`, `Work::new`), so a change to the window solve's inputs must be made twice and a reviewer must diff the two to confirm the roles are priced identically. (3) `Start`, `Connecting`, and `Connected` each carry `our_version`, a copy of `self.root.ceiling`, which is present on the same struct in every phase. Two copies of a contract-bearing greeting literal are two places for `set_len`/`max_version_bytes` sourcing to drift in the one message the wire pairs positionally.

Evidence:

    566	    async fn accept(self, request: Greeting) -> Result<(Greeting, Self::Next), Self::Error> {
    567	        let Start { our_version } = self.versions;
    568	
    569	        let fan = greeting_fan(&self.backend, self.root.root.clone())
    570	            .await
    571	            .map_err(Error::Backend)?;
    572	        let greeting = Greeting {
    573	            version: our_version.clone(),

    622	        let ceiling = our_version | &their_version;
    623	
    624	        let window = self.window.resolve(
    625	            self.root.len(),
    626	            their_len,
    627	            self.root.max_version_bytes(),
    628	            their_version_bytes,
    629	            B::node_bytes,
    630	        );
    631	        self.stats.window_granted(window.widest());
    632	        let ledger = SupplyLedger::new(their_len);
    633	        let mut work = Work::new(self.backend, window, self.stats);

    433	            versions: Start {
    434	                our_version: root.ceiling.clone(),
    435	            },

Resolution: (1) `accept` becomes `let (greeting, next) = Connect::connect(self).await?; Ok((greeting, CompleteConnect::complete_connect(next, request).await?))`; if the owner prefers to keep the two impls independent, hoist the literal into `fn greeting(&self, fan: &[(u8, B::Erased)]) -> Greeting` carrying the two comments. (2) `fn open(self) -> Opened<B>` on `Handshaking<B, Connected<B>>` returning `{ ceiling, their_version, their_listing, fan, ledger, work }`, called by both roles. (3) Remove `our_version` from the three version structs (`Start` becomes a unit struct) and read `self.root.ceiling` at the greeting and the join. Acceptance: one `Greeting { .. }` literal and one `window.resolve(` in `materialized.rs`; `our_version` absent; the greeting snapshots (`tests/gossip_snapshot.rs`) unchanged, since the bytes derive from the same fields.

### materialized-12: Twelve `+ Sync` bounds restate a `Backend` supertrait
- Where: src/tree/mirror/streaming/materialized.rs:610 (related: src/tree/mirror/streaming/materialized.rs:654, 717, 759; src/tree/mirror/streaming/materialized/unknown.rs:85, 140; work/answer.rs:48; work/levels.rs:82, 190, 312, 358, 530, 553; src/tree/mirror/streaming/backend.rs:46)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep `+ Sync|B: Sync` across the partition: twelve sites; backend.rs:46 reads `pub trait Backend: Clone + Send + Sync + 'static`; `git show 7126e489:src/tree/mirror/streaming/backend.rs` shows the trait already carried `Clone + Send + Sync + 'static` at the rebuild that wrote these impls, so the bounds were redundant from birth. This corrects the refutation pass's dating, which placed the supertrait at 48bc31df.)
- Seen by: refutation pass; refutation: raised as new; history: no rationale found
- Owner-gated: no

`B: Backend<..>` already implies `B: Sync`, so every `+ Sync` and `where B: Sync` here is a bound the compiler discharges from the supertrait. A redundant bound is a reader's question ("what here needs `Sync` that `Backend` does not give?") with no answer.

Evidence:

    610	impl<B: Backend<Node<Z>: Leaf> + Sync> protocol::Initiator<B> for Handshaking<B, Connected<B>> {

    46	pub trait Backend: Clone + Send + Sync + 'static

Resolution: Delete the twelve bounds (and the sibling sites elsewhere under `streaming/` in the same pass). Acceptance: `grep -rn '+ Sync\|B: Sync' src/tree/mirror/streaming/materialized*` returns nothing; `just check` clean.

### materialized-16: The receiver-as-stream `cfg(test)` adapter is duplicated in `common.rs` and `erased.rs`, and belongs in `channel.rs`, which owns the channel-type swap; with `children_of` relocated, `common.rs` dissolves
- Where: src/tree/mirror/streaming/materialized/common.rs:38-65 (related: src/tree/mirror/streaming/erased.rs:141-159; src/tree/mirror/streaming/channel.rs:75-90; src/tree/mirror/streaming/channel/instrumented.rs:176-182; src/tree/mirror/streaming/materialized.rs:185; work/queues.rs:29-32)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (read channel.rs:75-90, which swaps `Receiver`/`Sender`/`channel` under the same cfg; instrumented.rs:176 `impl<T> Stream for Receiver<T>`; erased.rs:141-159 is a second `ReceiverStreamOf`/`receiver_stream` copy of the same split; grep shows `ok_channel`/`OkReceiverStream` imported only by `queues.rs` and `levels.rs` through the `use common::*` glob)
- Seen by: structure; refutation: reframed (stronger: two copies, not one); history: no rationale found (`ok_channel_with` dates to c9b8f38b; `common.rs` has had no module doc since 7126e489)
- Owner-gated: no

The instrumented test `Receiver` implements `Stream` while tokio's needs `ReceiverStream`; that difference is the channel module's concern, and channel.rs already swaps the channel types under the same cfg. Two other modules each re-derive the stream adapter from that swap. A module named `common` with no module doc, holding one typed tree helper and one channel adapter, is the grab-bag smell; after materialized-3 it has one occupant with a better home.

Evidence:

    47	fn ok_channel_with<T: Send, E>(
    48	    (tx, rx): (Sender<T>, Receiver<T>),
    49	) -> (Sender<T>, OkReceiverStream<T, E>) {
    50	    #[cfg(test)]
    51	    {
    52	        (tx, rx.map(Ok))
    53	    }
    54	    #[cfg(not(test))]
    55	    {
    56	        (tx, ReceiverStream::new(rx).map(Ok))
    57	    }
    58	}

Resolution: In `channel.rs` add one `ReceiverStreamOf<T>` alias and `pub fn into_stream<T>(rx: Receiver<T>) -> ReceiverStreamOf<T>` beside the existing cfg swaps; in `queues.rs` define `ok_channel(role, buffer) = { let (tx, rx) = channel(role, buffer); (tx, into_stream(rx).map(Ok)) }` and one `OkReceiverStream` alias; have `erased::reply_channel` use the same `into_stream`; delete `common.rs` and the `use common::*` glob. Acceptance: no `common.rs` under `materialized`; `#[cfg(test)]`/`#[cfg(not(test))]` pairs concerning channel types appear only in `channel.rs`.

See also: mirror-common-25, deps-2, module-graph-3, materialized-3.

### materialized-26: `unknown::known` and `mirror::contained` spell one predicate two ways on the two sides of one contract
- Where: src/tree/mirror/streaming/materialized/unknown.rs:43-45 (related: src/tree/mirror.rs:29-39; src/tree/mirror/streaming/materialized/work/answer.rs:138, 182; work/resolver.rs:88; work/levels.rs:224; src/tree/mirror/streaming/materialized.rs:887; crates/before/src/causally/forms.rs:301-309; crates/before/src/oracle/version.rs:421-433)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`Ceiling::contains` is `le(version, &self.at)`, forms.rs:307-309, inclusive per the doc example at 75-77; `contained(bound, declared)` is `bound <= declared`, mirror.rs:37-39, with `PartialOrd for Version` the causal order at version.rs:421-433; so `known(node, v) == contained(node.span().hi(), v)`; grep lists three prune-side and three ingestion-side call sites)
- Seen by: structure, correctness; refutation: confirmed; history: no rationale found (`known` dates to a8b130e6; 0116a081 named `contained` "so the predicate is named once" across the ingestion sites only)
- Owner-gated: no

The prune side (`unknown.rs:92`, `answer.rs:138`, `answer.rs:182`) and the ingestion side (`materialized.rs:887`, `levels.rs:224`, `resolver.rs:88`) evaluate the same relation under two names and two spellings, and `mirror.rs:29-36` argues at length why the predicate deserves a single name. A reader checking that what one side prunes is exactly what the other side accepts has to prove the two spellings equal instead of seeing one predicate.

Evidence:

    43	pub(super) fn known(node: &impl ErasedNode, version: &Version) -> bool {
    44	    causally::before(version).contains(node.span().hi())
    45	}

Resolution: Define `known` as `contained(node.span().hi(), version)`, or move one named predicate to `mirror.rs` and use it at all six sites. The two names may stay (filter versus validation read differently) so long as one is defined by the other. Acceptance: one comparison expression underlies both names; the oracle and containment suites pass unchanged.

### materialized-31: `answer::internal` re-derives the listing `fan_listing` already computes
- Where: src/tree/mirror/streaming/materialized/work/answer.rs:69-73 (related: src/tree/mirror/streaming/materialized.rs:493-505; work/levels.rs:29)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (the closure at answer.rs:70-72 is the body of `fan_listing` at materialized.rs:502-504, read side by side; grep `.hash())` in non-test streaming code returns exactly those two sites; `fan_listing` is `pub(crate)` and `levels.rs` already imports it at line 29)
- Seen by: structure, perfapi; refutation: reframed (the `fan_listing` doc's "single derivation" claim is scoped to the two listings the proxy pairs positionally, the greeting's and the opening question's, and is accurate as written; this is a plain de-duplication, not a doc inaccuracy); history: no rationale found (the closure predates `fan_listing`, 0aa29ed9)
- Owner-gated: no

A `Query` listing is the same wire vocabulary as the greeting's listing; one derivation keeps them one, and the helper exists.

Evidence:

    69	                reactions.push(Reaction::Query(
    70	                    ours.iter()
    71	                        .map(|(radix, child)| (*radix, child.hash()))
    72	                        .collect(),
    73	                ));

Resolution: `reactions.push(Reaction::Query(fan_listing(&ours)));` with the import; optionally widen `fan_listing`'s doc from "both" to "every listing the walk emits". Acceptance: grep for `.hash()))` in non-test streaming code matches only `fan_listing`'s body.

### materialized-33: `initiator_level` hand-rolls a Left-only merge that the sibling module expresses with `merge_join_by`
- Where: src/tree/mirror/streaming/materialized/work/levels.rs:97-106 (related: work/answer.rs:56-59, 127-130; Cargo.toml:57)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read both merge sites; itertools 0.14 is a dependency at Cargo.toml:57 and ships `EitherOrBoth::just_left`)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found (55d76d5c wrote the loop and its "Left-arm-only merge" comment in the same commit)
- Owner-gated: no

The comment above already describes the loop in merge-join vocabulary; the `peekable`/`next_if` form requires the reader to re-derive the sortedness argument that `merge_join_by(...).filter_map(EitherOrBoth::just_left)` states.

Evidence:

    97	            let mut exclusive = Vec::new();
    98	            {
    99	                let mut theirs = their_listing.iter().map(|(radix, _)| *radix).peekable();
    100	                for (radix, node) in &fan {
    101	                    while theirs.next_if(|theirs| theirs < radix).is_some() {}
    102	                    if theirs.peek() != Some(radix) {
    103	                        exclusive.push((*radix, node.clone()));
    104	                    }
    105	                }
    106	            }

Resolution: `let exclusive: Vec<_> = fan.iter().merge_join_by(&their_listing, |(ours, _), (theirs, _)| ours.cmp(theirs)).filter_map(EitherOrBoth::just_left).map(|(radix, node)| (*radix, node.clone())).collect();`. Acceptance: no `peekable()`/`next_if` in levels.rs; the early-supply tests and the wire snapshots unchanged.

### materialized-36: `Resolver::react` returns an undocumented four-tuple that every caller immediately re-shapes
- Where: src/tree/mirror/streaming/materialized/work/resolver.rs:61-65 (related: work/levels.rs:432-435, 573-576, 670-678)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the three call sites: each destructures the tuple and computes `prefix.push(radix)` before using `radix` again for `ready`/`pending`; `react` carries no doc comment)
- Seen by: structure, prose; refutation: confirmed; history: no rationale found (the tuple dates to 7126e489/83db6b26; the callers' `prefix.push(radix)` recomputation entered with the erasure)
- Owner-gated: no

The maintainer must read `levels.rs` to learn that `Some` means "this child is disputed: the scope prefix, its radix, our node, and their listing; answer it, then call `pending` or `ready`", and that `Match` and `Supply` resolve in place and return `None`. A four-tuple under a (dead, see materialized-8) `type_complexity` allow is exactly where a named struct carries the meaning of each position.

Evidence:

    61	    #[allow(clippy::type_complexity)]
    62	    pub fn react(
    63	        &mut self,
    64	        reaction: Reaction<B::Erased>,
    65	    ) -> Result<Option<(ErasedPrefix, u8, B::Erased, Vec<(u8, Hash)>)>, Error<B::Error>> {

Resolution: Return `Option<Dispute<B::Erased>>` with `struct Dispute<E> { child_prefix: ErasedPrefix, radix: u8, node: E, listing: Vec<(u8, Hash)> }`, field docs stating each role, and a doc on `react` naming its three outcomes; drop the `allow`. Acceptance: no `prefix.push(radix)` following `react` in levels.rs; `react` documented.

### module-graph-8: Redundant re-export layers: a six-line shim over `streaming::channel`, and a re-export list that `remote.rs` glob-re-exports
- Where: src/tree/mirror/streaming/materialized/channel.rs:1-6 (related: src/tree/mirror/streaming/materialized.rs:173, :184; src/tree/mirror/streaming/materialized/common.rs:15; src/tree/mirror/streaming/materialized/work.rs:28-36; src/tree/mirror/streaming/materialized/work/queues.rs:30; src/tree/mirror/streaming/materialized/work/levels.rs:28; src/tree/mirror/streaming/materialized/work/assembly.rs:14; src/tree/mirror/streaming/remote.rs:85; src/tree/mirror/streaming/remote/error.rs:1-19; src/error.rs:44-50)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (files read in full; `grep -rn 'channel::' src/tree/mirror/streaming/materialized*` lists the six shim consumers; `git log --follow` shows the shim unchanged since `83edcd944 WIP: swap over to streaming`; `git blame` dates `remote.rs:85` to `cbfe1aff3`)
- Verification: confirmed; history: no-rationale-found (the cbor wire review's R3 ruling routes public error types by *named* re-export lists through `codec` → `remote` → `error.rs`, which the glob at `remote.rs:85` sits inside)
- Owner-gated: no

`materialized/channel.rs` exports nothing of its own: every name is `streaming::channel`'s, re-spelled so materialized's children write `super::channel::` while `work.rs:28-36` already imports `erased`, `protocol`, `tasks`, and `window` by crate path in the same block. `remote/error.rs` is `pub use` lines only, and `remote.rs` re-exports it with the crate's only glob re-export, so a codec error type reaches `crate::error` through three hops and `remote.rs`'s reader cannot see which names it exports.

Evidence:

    src/tree/mirror/streaming/materialized/channel.rs (whole file):
         1	//! Materialized protocol access to the shared named-channel infrastructure.
         2	
         3	pub use super::super::channel::{QueueKind, QueueRole, Receiver, Sender, channel};
         4	
         5	#[cfg(test)]
         6	pub use super::super::channel::{with_kind_capacity, with_observation, with_schedule};

    src/tree/mirror/streaming/materialized/work.rs:
        28	use crate::tree::{
        29	    mirror::streaming::{
        30	        Backend, Leaf, erased,
        31	        materialized::{Error, channel::Sender},
        32	        protocol::BoxResponses,

    src/tree/mirror/streaming/remote.rs:
        85	pub use error::*;

    src/tree/mirror/streaming/remote/error.rs:
        17	pub use super::proxy::Error as RemoteError;
        18	pub use super::streams::{AcceptError, ReplyFrameError, SendError, StreamError};
        19	pub use crate::tree::mirror::framing::LengthOverflow;

Resolution: Delete `materialized/channel.rs` and repoint its six consumers at `crate::tree::mirror::streaming::channel`; replace `remote.rs:85` with the explicit list (moving `remote/error.rs:1-6`'s doc onto it) or keep `error.rs` and re-export it by name. Acceptance: `grep -rn 'pub use .*\*;' src` returns nothing; `materialized.rs` declares no `channel` module.

See also: remote-capture-atlas-5, remote-capture-atlas-3.

### Nits (7)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| materialized-4 | `src/tree/mirror/streaming/materialized.rs:228-241` | `SupplyLedger::charge` reports its overdraw as a bare `u64` | Introduce `pub(crate) struct Overdrawn { pub declared: u64 }` as the `Err` type |
| materialized-6 | `src/tree/mirror/streaming/materialized.rs:265-288` | Field visibility on `Query`, `Resolution`, and `Resolve` is inconsistent inside a tree that is private at the crate root | Pick one spelling for the walk's three item types |
| materialized-8 | `src/tree/mirror/streaming/materialized.rs:388` | Attributes that guard nothing: twelve per-item `type_complexity` allows under a module-wide allow, and `#[cfg(test)]` inside a `#[cfg(test)]` module | Delete the twelve per-item `type_complexity` allows (or the module-wide one), the two inner `#[cfg(test)]`, and reword progress.rs:106 |
| materialized-9 | `src/tree/mirror/streaming/materialized.rs:400` | Qualified paths where imports exist, and clone helpers that half the call sites bypass | Import `PhantomData`, `Stream`, and `pin` |
| materialized-15 | `src/tree/mirror/streaming/materialized.rs:890-891` | Leaf counts cross the `usize`/`u64` boundary at every ingestion site, and `absorb` charges `len()` where it credits `1` | Bind `leaf.len() as u64` once and feed both `ledger.absorb` and `stats.gained`; consider `fn len(&self) -> u64` on `Node`/`ErasedNode` |
| materialized-40 | `src/tree/mirror/streaming/materialized/work/tests/violations.rs:345` | `violations.rs` hard-codes the height count and mirrors `Violation` with an identity enum | `for height in 0..Root::HEIGHT`; generate over `Violation` directly or derive the `Injection` mapping with a stated reason |
| module-graph-13 | `src/tree/mirror/streaming/materialized/work.rs:134-153` | Two near-identical `pump` helpers diverge on a closed receiver without saying why | Lift one `pump` into `tasks.rs` parameterized by the closed-receiver policy, or comment at each site why the walk returns where the proxy parks |

## Remote codec

Files: src/tree/mirror/streaming/remote/codec/ (production files). Entries: 10 (2 medium, 6 low, 2 nit).

### remote-codec-9: The frame decoder flattens typed head and listing defects into static strings; every sibling taxonomy keeps them typed
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:316-360 (related: src/tree/mirror/streaming/remote/codec/error.rs:137-143; src/tree/mirror/streaming/remote/codec/frame.rs:381-387 and :404-419; src/tree/mirror/streaming/remote/codec/greeting.rs:102-110; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:531-538; src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:105-110)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read)
- Seen by: structure (1), perfapi (open question); refutation: confirmed; history: `listing_issue` and `head_detail` arrived whole in 4dd2053c with no rationale for the shape; the atlas exemption states the collapse as a fact; R5 typed only the greeting side
- Owner-gated: yes (`DecodeErrorKind` reaches users as `rumors::error::CodecDecodeErrorKind`)

`listing_issue` and `head_detail` exist only to convert `ListingIssue` and `HeadError` into `DecodeErrorKind::Malformed { detail: &'static str }`, discarding which `HeadError` fired (`ListingIssue::Head(_)` becomes the fixed string "listing head is not canonical") and re-spelling `ListingIssue::Truncated`'s own message. `LeafRunError::Head { source: HeadError }`, `GreetingError::Head(HeadError)`, and `GreetingError::Listing(ListingIssue)` all keep the defect typed, so the two listing ingress surfaces (query frame and greeting) type the same failure differently, and the error atlas carries an exemption explaining the collapse. Types-first: a typed source carries strictly more than a string re-spelling of its `Display`, and the two mapping functions are machinery whose only job is to lose information.

Evidence:

    317	pub(super) fn listing_issue(issue: ListingIssue) -> DecodeErrorKind {
    318	    match issue {
    319	        ListingIssue::Order(order) => DecodeErrorKind::QueryOutOfOrder(order),
    320	        ListingIssue::Head(_) => DecodeErrorKind::Malformed {
    321	            part: FramePart::QueryChildren,
    322	            detail: "listing head is not canonical",
    323	        },
    ...
    353	fn head_detail(error: cbor::HeadError) -> &'static str {
    354	    match error {
    355	        cbor::HeadError::Truncated => "truncated head",

    error_atlas.rs:106	        "kind: Listing",
    error_atlas.rs:107	        "ListingIssue never surfaces from the frame decoders: they collapse \

Resolution: Add typed variants to `DecodeErrorKind`: `Head { part: FramePart, #[source] source: HeadError }` (dissolving `head_detail`) and `InvalidListing(#[from] ListingIssue)` (dissolving `listing_issue`; `QueryOutOfOrder` then duplicates `InvalidListing(ListingIssue::Order(_))` and can retire, or stay as the flat form if matchers rely on it). Keep `Malformed { part, detail }` for the shape-level cases ("frame item is not an array", "query body is not a listing map"). Re-accept the error-atlas snapshot deliberately and delete its `"kind: Listing"` exemption. This also dissolves remote-codec-10. Acceptance: `listing_issue` and `head_detail` no longer exist; a widened listing key head decodes to an error carrying `HeadError::NotShortest` as a typed source in both the frame and greeting paths; the atlas covers the new variants without an exemption; `tests/common/sim.rs` and the proxy tests that match `CodecDecodeErrorKind` compile.

See also: remote-codec-21.

### remote-codec-27: Greeting encode and parse dispatch on key strings inside a roster loop, forcing six `Option`s, six `expect`s, and two `unreachable!`s
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:60-88 (related: greeting.rs:128-133, :150-211; src/tree/mirror/streaming/remote/codec/greeting/tests.rs:56)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read)
- Seen by: structure (2), correctness (37), perfapi (53); refutation: confirmed; history: the roster-as-data shape is the original 4dd2053c design and its purpose (one spelling of the roster and its order shared by writer and parser) is stated inline and holds; nothing argues for the loop-plus-string-match as the mechanism, so this is a mechanism swap under the stated goal, not a reopen
- Owner-gated: no (the wire is untouched)

Both `greeting_map` and `parse_greeting` iterate `KEYS` and then `match key { "listing" => ..., _ => unreachable!(...) }`. The loop forces the parser to accumulate six `Option`s and unwrap each with `expect("the roster visits ...")`, and gives both functions an `unreachable!` arm; all eight panic sites exist only because a fixed six-step sequence is expressed as data-then-string-dispatch. Each site is programmer-error-only today and its message argues it, but the doctrine prefers removing the need for a proof over supplying one, and finished code should be obviously correct, not correct by an argument about a roster. Written straight-line in roster order, the same code has no `Option`, no `expect`, no `unreachable!`, and no stringly dispatch.

Evidence:

    63	    for key in KEYS {
    64	        cbor::write_head(&mut map, MAJOR_TEXT, key.len() as u64);
    65	        map.extend_from_slice(key.as_bytes());
    66	        match key {
    67	            "listing" => write_listing(&mut map, &greeting.listing),
    ...
    84	            _ => unreachable!("the key roster is exhaustive"),

    128	    let mut version = None;
    129	    let mut set_len = None;
    ...
    204	    Ok(Greeting {
    205	        version: version.expect("the roster visits version"),
    206	        set_len: set_len.expect("the roster visits set_len"),

Resolution: Factor `fn key(input: &mut &[u8], name: &'static str) -> Result<(), GreetingError>` for the text-key check and read the six entries in wire order as straight-line code (`key(&mut input, "listing")?; let listing = parse_listing_map(&mut input)...?; key(&mut input, "set_len")?; let set_len = uint(&mut input, ...)?; ...; Ok(Greeting { .. })`), mirroring `greeting_map` as six explicit writes. Keep `KEYS` as the single spelling of the roster and its order for the sortedness test of remote-codec-26 (and for the map count), so the roster's single-source role survives as a test rather than a runtime loop. Acceptance: `greeting.rs` contains no `unreachable!`, no `expect`, and no `match key`; `greetings_round_trip` and `greeting_key_roster_is_exact` pass unchanged; the greeting bytes in `tests/gossip_snapshot.rs` are byte-identical.

See also: inventory-5.

### clippy-pedantic-1: Range-checked `as` narrowings where `try_from` carries the check
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:455-458 (related: src/tree/mirror/streaming/remote/codec/frame.rs:439-443; src/bookmark/format.rs:360-363; src/tree/typed/untyped/iter.rs:411-413 and 429; src/link/routed/header.rs:248 and 254; house pattern at src/tree/mirror/streaming/remote/streams.rs:729-731; construction check at src/link/routed/endpoint.rs:205)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (lint hits in the run log; every guard and cast read at the cited lines; `MAX_ADDR_LEN` and `MAX_QUERY_CHILDREN` definitions read; `link_header`'s one non-test caller grepped)
- Verification: confirmed; history: deliberate-and-holds for the `try_from` idiom (the 2026-08-20 CBOR wire review prescribes it at codec sites; these five are stragglers), no-rationale-found for the `as` spellings
- Owner-gated: no

Five library sites narrow with `as` one line after a range check (or, at
`header.rs`, a `debug_assert!`) that makes the cast lossless; the reader
pairs check and cast by hand. `u8::try_from`/`usize::try_from` folds them
into one expression whose losslessness is local, which `streams.rs:729-731`
already writes. At `header.rs:254` the only guard is a `debug_assert!`, so a
violated precondition in a release build writes a wrong length byte instead
of failing; the decode side (`header.rs:307`) rejects only `len == 0`.

Evidence:

    frame.rs
    455        if head.major != MAJOR_UINT || head.value > u64::from(u8::MAX) {
    456            return Err(ListingIssue::Shape("listing key is not a radix"));
    457        }
    458        let radix = head.value as u8;

    439        if count > MAX_QUERY_CHILDREN as u64 {
    443            children: Vec::with_capacity(count as usize),

    bookmark/format.rs
    360    if declared > (bytes.len() - reader.at) as u64 {
    363    let payload = reader.take(declared as usize).expect("length checked");

    typed/untyped/iter.rs
    411                        Children::Branch { children, .. } if level.next <= u8::MAX as u16 => {
    413                                .successor(level.next as u8)
    429                            level.next = radix as u16 + 1;

    link/routed/header.rs
    248    debug_assert!((1..=MAX_ADDR_LEN).contains(&addr.len()));
    254    bytes.push(addr.len() as u8);

    house pattern, streams.rs
    729        let stream = u8::try_from(index)
    730            .ok()
    731            .and_then(|index| Stream::new(index).ok())

Resolution: `frame.rs:455-458`: keep the `major` check, then
`let radix = u8::try_from(head.value).map_err(|_| ListingIssue::Shape("listing key is not a radix"))?;`.
`frame.rs:438-446`: `usize::try_from(count).ok().filter(|c| *c <= MAX_QUERY_CHILDREN).ok_or(ListingIssue::Shape(..))?`.
`format.rs:360-363`: `usize::try_from(declared).ok().filter(|d| *d <= bytes.len() - reader.at).ok_or(FormatError::Truncated { len: bytes.len() })?`,
then `reader.take(len)` with its `expect` gone. `iter.rs:411-413`: one arm,
`Children::Branch { children, .. } => u8::try_from(level.next).ok().and_then(|next| children.successor(next)).map(..)`;
line 429 `u16::from(radix) + 1`. `header.rs:248-254`: the minimal change is
`bytes.push(u8::try_from(addr.len()).expect("advertised-name length is validated at endpoint construction"))`
with the `debug_assert!` kept for its lower bound; the types-first change is
a validated-length newtype for the encoded advertised name that
`Endpoint` constructs at line 205 and `link_header` accepts, which deletes
both the assert and the cast. All behavior-preserving on valid input.
Acceptance: no `as` narrowing remains at the five sites; `just gate` clean;
the wire and bookmark snapshot suites unchanged.

See also: inventory-18, tree-typed-34, session-bookmark-30.

### inventory-5: `parse_greeting` and `greeting_map` route a fixed-order roster through six `Option`s and eight panic sites
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:128-211 (related: greeting.rs:37-44, greeting.rs:60-88, greeting/tests.rs:56)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the cbor-wire review packet, REVIEW.md:985-987, verified the roster order and the exact-roster parse but did not consider the loop shape)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The parser walks `KEYS` in wire order and returns `GreetingError::Shape` on any deviation (lines 136-149), so every field is present by the time the loop ends. The code nevertheless accumulates six `Option`s, dispatches on the key string with an `unreachable!` arm, and finishes with six `expect`s; the encoder mirrors the string dispatch with its own `unreachable!`. A straight-line parse in `KEYS` order mirrors the grammar it implements and needs none of the eight panic sites. The wire bytes are unchanged, so the snapshot pins hold, and `greeting_key_roster_is_exact` (greeting/tests.rs:56) keeps guarding the roster.

Evidence:

    128	    let mut version = None;
    129	    let mut set_len = None;
    130	    let mut max_version_bytes = None;
    131	    let mut payload_depth_limit = None;
    132	    let mut target_message_size = None;
    133	    let mut listing = None;
    134	    for key in KEYS {
    ...
    198	            _ => unreachable!("the key roster is exhaustive"),
    ...
    204	    Ok(Greeting {
    205	        version: version.expect("the roster visits version"),
    206	        set_len: set_len.expect("the roster visits set_len"),

    84	            _ => unreachable!("the key roster is exhaustive"),

Resolution: Replace the loop with a small `expect_key(&mut input, "listing")?` helper (read the text head, compare bytes) followed by the field parse, one pair per key in roster order, and build `Greeting { .. }` directly; do the same in `greeting_map` with a `write_key` helper. Keep `KEYS` if a test wants to assert the order; otherwise the sequence is the roster. Acceptance: `parse_greeting` and `greeting_map` contain no `Option` fields, no `unreachable!`, and no `expect`; `tests/gossip_snapshot.rs` and the greeting tests pass unchanged.

See also: remote-codec-27.

### remote-codec-3: Wire quantities spelled several times: the opener length, the supply head, the record item length, the stream count, the radix fan
- Where: src/tree/mirror/streaming/remote/codec/budget.rs:86-89 (related: budget.rs:53-55; src/tree/mirror/streaming/remote/codec/encode.rs:24-31; src/tree/mirror/streaming/remote/codec/decode/async_io.rs:37-39; src/tree/mirror/streaming/remote/codec/frame.rs:155-160, :179-181, :325-328, :19; src/tree/mirror/streaming/remote/codec/frame/tests.rs:8-10; src/tree/mirror/streaming/remote/codec/signal.rs:31-32; outside the partition, src/link.rs:161-169 and src/tree/mirror/streaming/window.rs:132)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the stream-count derivation checked against `height.rs:128-130`, where `Root` is `S` applied 32 times to `Z`, so `32 / 2 + 1 = 17`)
- Seen by: structure (6, 7, 11), perfapi (52); refutation: confirmed all four; history: the opener spellings accreted across 3327a92b and 6f3792ab with no rationale; the `record_len`/`push` pair is deliberate with its rationale inline (frame.rs:148-151) and pinned by f2b74a97; `COUNT = 17` is from 2203c104 with no rationale; f94f2056's message treats `FAN` and `MAX_QUERY_CHILDREN` as distinct quantities that share the radix
- Owner-gated: no (the `FAN`/`MAX_QUERY_CHILDREN` unification is taste; ask first)

One quantity, several spellings. `cbor::head_len(3) + WireSignal::ENCODED_LEN` is the opener's length and appears as `FRAME_HEAD_LEN` (encode.rs:26), as `OPENER_LEN` (async_io.rs:39), and inline twice in budget.rs (:53-54 and :86-87); `head_len(TAG_CBOR_SEQUENCE) + head_len(u32::MAX as u64)` is `SUPPLY_HEAD_LEN` (encode.rs:31) and is spelled out again inside `SUPPLY_FRAME_OVERHEAD`. `RECORD_TAG_LEN + head_len(body) + body` is computed in `record_len`, again in `push`, and a third time in u64 in `lone_record_spans`. `Stream::COUNT` is the literal 17 six lines below the two constants it follows from, with the derivation living only in `link.rs` prose; `MAX_QUERY_CHILDREN = 256` and `window::FAN = 256` name the same radix fan and `budget.rs` imports both. A reader cannot see that the encoder's stack buffer, the reader's opener buffer, and the budget envelope are one quantity without expanding each by hand.

Evidence:

    86	pub const SUPPLY_FRAME_OVERHEAD: usize = cbor::head_len(3)
    87	    + WireSignal::ENCODED_LEN
    88	    + cbor::head_len(cbor::TAG_CBOR_SEQUENCE)
    89	    + cbor::head_len(u32::MAX as u64);

    encode.rs:26	const FRAME_HEAD_LEN: usize = cbor::head_len(3) + WireSignal::ENCODED_LEN;
    async_io.rs:39	const OPENER_LEN: usize = cbor::head_len(3) + WireSignal::ENCODED_LEN;
    frame.rs:157	        RECORD_TAG_LEN
    frame.rs:158	            .saturating_add(cbor::head_len(body as u64))
    frame.rs:159	            .saturating_add(body)
    frame.rs:179	        let item = RECORD_TAG_LEN
    frame.rs:180	            .saturating_add(cbor::head_len(body as u64))
    frame.rs:181	            .saturating_add(body);
    signal.rs:32	    pub const COUNT: u8 = 17;

Resolution: Define the opener length once (beside `WireSignal::ENCODED_LEN` in signal.rs, or in frame.rs) and the supply head length once (beside `RECORD_TAG_LEN`), then `SUPPLY_FRAME_OVERHEAD = OPENER_LEN + SUPPLY_HEAD_LEN`, `FULL_FAN_QUERY_FRAME_LEN` starts from `OPENER_LEN`, and `Heads<OPENER_LEN>` / `Heads<SUPPLY_HEAD_LEN>` use the shared names. Add `const fn record_item_len(content: usize) -> usize` (saturating) and have `record_len` and `push` call it; `lone_record_spans` compares through a u64 twin or a `usize::try_from`. The pin `record_len_matches_an_actual_push` stays meaningful because it compares the closed form against bytes actually written, not against a second arithmetic. Write `pub const COUNT: u8 = (STREAMED_HEIGHT_COUNT / STREAM_HEIGHT_STRIDE + 1) as u8;` with a `const _: () = assert!(...)` that it fits, keeping `link::STREAM_COUNT` as the transport's own literal and the existing cross-layer pin. Ask whether `MAX_QUERY_CHILDREN` should be defined as `FAN` (or both from one radix constant); if they are meant as distinct quantities, say so at one of the two declarations. Acceptance: exactly one definition each of the opener length, the supply head length, and the record item sum in the codec; no bare `17` in signal.rs; `default_budget_matches_its_derivation` (pinned 1_830_400), `full_fan_frame_len_matches_an_actual_encode`, `record_len_matches_an_actual_push`, and `stream_count_matches_the_codec` pass unchanged.

Synthesis note: link-3 (under Link) proposes the reverse ownership for the stream count (the codec cites `link::STREAM_COUNT` and the pin test dissolves). The compile-time derivation both propose is the substance; the ownership direction is one decision to take once. link-3 is the entry of record for the stream count; this entry is counted for the opener length, the supply head, the record item length, and the radix fan.

### remote-codec-7: Test-only code is interleaved through the production codec files
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:3-61 (related: decode.rs:63-215; src/tree/mirror/streaming/remote/codec/encode.rs:3-4, :17-22, :33-44, :159-180; src/tree/mirror/streaming/remote/codec.rs:105-240)
- Class / severity / confidence: modularity / low / medium
- Provenance: assessed (read)
- Seen by: structure (8); refutation: confirmed; history: the `#[cfg(test)]` gate on `FrameDecoder` was added by a WIP commit (83edcd94) when the async reader took over, never as a placement decision; the meter shims arrived with 90512e88 without saying why they live in codec.rs
- Owner-gated: no

`decode.rs` is half test oracle: the imports at 3-4, 12-15, 21-22, the module's headline `decode` and `decode_exact`, and the whole `FrameDecoder` (about 190 of 378 lines) are `#[cfg(test)]`, while the production reader lives in the child `async_io.rs`. `encode.rs` carries the `#[cfg(test)]` sync `encode`, `FrameEncoding::write`, and `write`; `codec.rs` spends lines 105-240 on `test-internals` meter scaffolding. A reader of the production path skips cfg gates to find it, and the parent module's headline function never ships. The project convention keeps tests in sibling files "for brevity of reading the implementation"; oracles and meter scaffolding are test support and deserve the same separation. The crate already has the shape: `capture` is a cfg-gated sibling module (codec.rs:66-67).

Evidence:

    54	/// Frame reader that adds protocol context as soon as the signal reveals it.
    55	#[cfg(test)]
    56	struct FrameDecoder<'a, R> {
    57	    speaker: Speaker,
    58	    /// The session's run budget, gating supply-body buffering.
    59	    budget: RunBudget,
    60	    read: &'a mut R,
    61	}

Resolution: Move the sync oracle into `#[cfg(test)] mod oracle;` under `decode/` (its re-exports are consumed by several sibling test modules, so `decode/tests.rs` is the wrong home), keeping the shared validators (`frame_arity`, `opener_item`, `check_arity`, `query_listing`, `run_head`, `head_error`, `listing_issue`, `decode_signal`) in `decode.rs`; give the struct a doc stating its role as the differential's independent implementation. Move the sync `encode` and `FrameEncoding::write` into a cfg-gated `encode/oracle.rs`; move the meter scaffolding into `#[cfg(any(test, feature = "test-internals"))] mod meters;` beside `capture`, re-exporting from `codec.rs` as today. Acceptance: `decode.rs`, `encode.rs`, and `codec.rs` contain no `#[cfg(test)]` items other than `mod` and `pub use` lines; the gate passes with the same test set.

### remote-codec-8: Decode fragments duplicated between the async reader and the sync oracle: the over-budget gate, `record_prefix`, the EOF classification, and a bare `RECORD_TAG_LEN + 1`
- Where: src/tree/mirror/streaming/remote/codec/decode.rs:147-163 (related: src/tree/mirror/streaming/remote/codec/decode/async_io.rs:444-457; decode.rs:183-196 and async_io.rs:473-486; decode.rs:207-213, :338-344, async_io.rs:204-212, :543-551)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; each pair compared line by line)
- Seen by: structure (9); refutation: confirmed; history: the mirroring is a recorded choice (ed0f1775: "the shared lone_record_spans predicate keeps the two decoders' boundary identical"), but nothing says why only that predicate was shared; the stated purpose is served better by sharing more
- Owner-gated: no

The over-budget gate (the `OverbatchedRun` closure, the `len < RECORD_TAG_LEN + 1` short-body check, the `lone_record_spans` test) is repeated verbatim in both decoders; `record_prefix` is repeated differing only by `.await`; the mapping "UnexpectedEof becomes Truncated, anything else Read" is spelled four times; and the `+ 1` (the smallest byte-string head) is an unnamed number at both sites. The oracle's documented independent contribution (decode.rs:169-172) is the whole-body read shape; none of these fragments is about read shape, so sharing them costs no oracle independence.

Evidence:

    147	        if !self.budget.covers(len) {
    148	            let budget = self.budget;
    149	            let overbatched = move || DecodeErrorKind::OverbatchedRun {
    150	                declared: super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
    151	                budget: budget.bytes(),
    152	            };
    153	            // A body too short to hold a record's heads cannot be a lone
    154	            // record: rejected on the declared length alone.
    155	            if len < super::frame::RECORD_TAG_LEN + 1 {
    156	                return Err(overbatched());
    157	            }

    async_io.rs:445	            let overbatched = move || DecodeErrorKind::OverbatchedRun {
    async_io.rs:446	                declared: super::super::budget::SUPPLY_FRAME_OVERHEAD.saturating_add(len),
    async_io.rs:449	            if len < super::super::frame::RECORD_TAG_LEN + 1 {

Resolution: In `budget.rs` add `pub(super) fn overbatched(self, body: usize) -> DecodeErrorKind` (or a free function in decode.rs), and in `frame.rs` a `pub(super) const MIN_RECORD_HEADS_LEN: usize = RECORD_TAG_LEN + 1` with a doc naming it as the tag head plus a one-byte byte-string head; have both decoders call them. Hoist `classify` to `decode.rs` as `pub(super)` and route `head_error`'s `Io` arm and the sync `read_exact` through it (`Arrived::short` can call it with a fresh `UnexpectedEof`). Acceptance: `OverbatchedRun { .. }` is constructed in exactly one place; `RECORD_TAG_LEN + 1` appears nowhere as a bare expression; `ErrorKind::UnexpectedEof => DecodeErrorKind::Truncated` appears once; the `decode_both` suites pass.

### remote-codec-22: `LeafRun`'s hand-written `Default`, `Clone`, `PartialEq`, and `Eq` are residue of the erased type parameter
- Where: src/tree/mirror/streaming/remote/codec/frame.rs:92-112 (related: frame.rs:114-121)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 48bc31df -- src/tree/mirror/streaming/remote/codec/frame.rs` shows `-pub struct LeafRun<T>`, `-    marker: PhantomData<fn() -> T>,`, and `-impl<T> Clone for LeafRun<T>` rewritten as `+impl Clone for LeafRun`, likewise for `Default`, `PartialEq`, `Eq`, with the bodies unchanged)
- Seen by: structure (4), perfapi (44); refutation: confirmed; history: deliberate-but-expired (the impls existed to avoid `T: Clone`/`T: PartialEq` bounds; 48bc31df erased `T` and the marker but only stripped the `<T>`)
- Owner-gated: no

`LeafRun` has one field, `bytes: Vec<u8>`, and four hand-written impls that are byte-for-byte what `#[derive(Default, Clone, PartialEq, Eq)]` produces. Machinery outliving the constraint that justified it: twenty lines a reader must verify are equivalent to derives. `Debug` is legitimately hand-written (it renders counts, not bytes).

Evidence:

    92	impl Default for LeafRun {
    93	    fn default() -> Self {
    94	        Self::new()
    95	    }
    96	}
    97	
    98	impl Clone for LeafRun {
    99	    fn clone(&self) -> Self {
    100	        Self {
    101	            bytes: self.bytes.clone(),
    102	        }
    103	    }
    104	}
    105	
    106	impl PartialEq for LeafRun {
    107	    fn eq(&self, other: &Self) -> bool {
    108	        self.bytes == other.bytes
    109	    }
    110	}
    111	
    112	impl Eq for LeafRun {}

Resolution: `#[derive(Default, Clone, PartialEq, Eq)]` on `LeafRun`, keeping the custom `Debug`. Acceptance: `frame.rs` has no `impl Default/Clone/PartialEq/Eq for LeafRun`; the codec tests pass.

### Nits (2)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| remote-codec-13 | `src/tree/mirror/streaming/remote/codec/decode/async_io.rs:446-455` | Long qualified paths at use sites where the file already imports the module | Extend the existing `use` blocks (`SUPPLY_FRAME_OVERHEAD`, `RECORD_TAG_LEN`, `lone_record_spans`, `VERSION_TAG`, `std::io`) and drop the qualified spellings |
| remote-codec-21 | `src/tree/mirror/streaming/remote/codec/frame.rs:18-29` | Visibility wider than reach, `thiserror::Error` derived for `Display` alone, and a positional three-field variant | Tighten the listed constants and helpers to their reach; replace the `thiserror::Error` derives on `StreamClass`/`FramePart` with `Display`; name `Entry::Complete`'s fields |

## Remote capture and codec tests

Files: src/tree/mirror/streaming/remote.rs, remote/error.rs, codec/capture.rs, codec/tests.rs, codec/tests/error_atlas.rs. Entries: 15 (4 low, 11 nit).

### remote-capture-atlas-4: `codec_stream_count` wraps a constant already reachable as `remote::Stream::COUNT`
- Where: src/tree/mirror/streaming/remote.rs:87-91 (related: src/link/tests.rs:17, src/tree/mirror/streaming/remote/error.rs:15, src/tree/mirror/streaming/remote/codec/signal.rs:32)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn codec_stream_count src tests` returns the definition and one caller; `Stream` is in remote/error.rs:15's re-export list and remote.rs:85 is `pub use error::*`; signal.rs:32 is `pub const COUNT: u8 = 17;`)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found (redundant from birth: b3b877d9 introduced the wrapper in the same commit that exported `Stream` at this level)
- Owner-gated: no

The `#[cfg(test)]` function exists for one caller to read `codec::Stream::COUNT`, but `Stream` is exported through the `error::*` glob at the same path the caller already spells, so `remote::Stream::COUNT` resolves directly. Principle: circular justification (the function's only reason to exist is to name a thing already named); it also removes one `cfg(test)` item from a production file.

Evidence:

    87	/// The codec's logical stream count, for cross-layer constant assertions.
    88	#[cfg(test)]
    89	pub(crate) fn codec_stream_count() -> u8 {
    90	    codec::Stream::COUNT
    91	}

    link/tests.rs:
    17	        usize::from(crate::tree::mirror::streaming::remote::codec_stream_count()),

Resolution: Delete lines 87-91; in src/link/tests.rs:17 write `usize::from(crate::tree::mirror::streaming::remote::Stream::COUNT)`. Acceptance: `grep -rn codec_stream_count src` is empty; `stream_count_matches_the_codec` compiles and passes.

### remote-capture-atlas-5: The proxy error carries two names in the `remote` namespace, and tests rename toward the one that already exists
- Where: src/tree/mirror/streaming/remote.rs:93 (related: src/tree/mirror/streaming/remote/error.rs:17, src/tree/mirror/streaming/remote.rs:85, src/peer/gossip.rs:1416, src/peer/gossip.rs:1423, src/tree/mirror/streaming/remote/proxy/tests.rs:32, src/tree/mirror/streaming/remote/proxy/tests/{harness,malformed,declarations,failures}.rs)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (grep for `Error as RemoteError` returns five proxy test files plus the alias site; grep for `streaming_remote::Error\b` returns gossip.rs:1416 and 1423; src/error.rs:48 fixes the public name as `RemoteError`)
- Seen by: structure, correctness, perfapi; refutation: reframed (the sub-claim that the glob also doubles the codec and adapter `DecodeError`s is wrong: `codec` is `pub(crate) mod` and `adapter` is private, neither glob-exported); history: no rationale found (both spellings arrived in one WIP commit; 474ddcc0 settled the public flattening on `RemoteError`)
- Owner-gated: no

Line 93 exports `proxy::Error` as `remote::Error`; remote/error.rs:17 exports the same type as `RemoteError`, which the glob at line 85 also places in `remote`. Five test modules then import `remote::{Error as RemoteError, ...}`, hand-renaming to a name the module already provides, while gossip.rs uses the bare `Error`. One type, one name per namespace; the renames are evidence the doubling makes readers re-derive a name that is already there.

Evidence:

    85	pub use error::*;
    ...
    93	pub use proxy::Error;

    error.rs:
    17	pub use super::proxy::Error as RemoteError;

Resolution: Delete `pub use proxy::Error;` at line 93 (`RemoteError` remains via the glob and is the name `crate::error` re-exports), change gossip.rs:1416 and 1423 to `streaming_remote::RemoteError`, and drop the `Error as RemoteError` renames in the five proxy test files. The `tree` module is crate-private, so this is not a public API change. Acceptance: `grep -rn 'Error as RemoteError' src` returns only remote/error.rs:17; `grep -rnE 'streaming_remote::Error\b' src` is empty.

See also: module-graph-8.

### remote-capture-atlas-23: Test-side `SIGNALS`, `SIGNAL_COUNT`, a bare `2`, and "All 340 placements" restate what the codec owns
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:42-63 (related: src/tree/mirror/streaming/remote/codec/tests.rs:246, src/tree/mirror/streaming/remote/codec/tests.rs:331-340, src/tree/mirror/streaming/remote/codec/tests.rs:347, src/tree/mirror/streaming/remote/codec/tests.rs:477-482, src/tree/mirror/streaming/remote/codec/signal.rs:232-246, src/tree/mirror/streaming/remote/codec/signal/tests.rs:12-20)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (signal.rs:232 `pub const STATE_COUNT`, :235 `const STATES` (private) lists the same ten signals in the same order; signal/tests.rs:14-17 proves `STATES[i].state() == i`, so the `position` lookup at 479-482 equals `usize::from(signal.state())`)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed (noting `STATES` must widen to `pub(super)` since `codec::tests` is a sibling of `signal`, not a child); history: deliberate but expired (the copy had a mechanical reason at birth, both constants were private to signal.rs; 3327a92b made `STATE_COUNT` public and did not sweep the copy)
- Owner-gated: no

`SIGNAL_COUNT = 10` hand-maintains `Signal::STATE_COUNT`; `SIGNALS` is a byte-for-byte copy of the private `Signal::STATES`; the corpus bucketing recovers a signal's index by linear search over the copy where `signal.state()` is that index; `accepted = [0; 2]` uses a bare 2 beside the named `SPEAKER_COUNT`; and the atlas testdoc pins "All 340 placements" where the body derives the set from `SPEAKER_COUNT * Stream::COUNT * SIGNAL_COUNT`. A new `Signal` variant fails compilation at `representative_frame` (331-340, exhaustive) but not at `SIGNALS`, so the atlas and manifest would enumerate a stale roster without noticing. No hand-maintained counts: a number that matters lives in a mechanically enforced place the code cites by name.

Evidence:

    43	const SPEAKER_COUNT: usize = 2;
    ...
    46	const SIGNAL_COUNT: usize = 10;
    ...
    52	const SIGNALS: [Signal; SIGNAL_COUNT] = [
    53	    Signal::Match(Flow::Continue),

    246	/// All 340 placements pin either their canonical frame bytes or typed rejection.

    347	    let mut accepted = [0; 2];

    479	    let signal_index = SIGNALS
    480	        .iter()
    481	        .position(|candidate| *candidate == signal)
    482	        .expect("every frame maps to a semantic signal state");

    signal.rs:
    232	    pub const STATE_COUNT: u8 = Flow::STATE_COUNT * Self::REACTION_COUNT + Self::END_COUNT;
    235	    const STATES: [Signal; Self::STATE_COUNT as usize] = [

Resolution: `const SIGNAL_COUNT: usize = Signal::STATE_COUNT as usize;`; widen `Signal::STATES` to `pub(super)` and iterate it in place of `SIGNALS`; replace the `position` lookup with `usize::from(signal.state())`; write `[0; SPEAKER_COUNT]`; reword line 246 to "Every (speaker, stream, signal) placement pins either its canonical frame bytes or its typed rejection." Acceptance: codec/tests.rs contains no literal signal roster and no literal `10` or `340`; both snapshot files are byte-identical; a new `Signal` variant fails compilation in codec/tests.rs without touching a literal.

### remote-capture-atlas-29: The frame-to-signal projection is written three times: inline in the encoder and twice verbatim in tests
- Where: src/tree/mirror/streaming/remote/codec/tests.rs:504-514 (related: src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:543-553, src/tree/mirror/streaming/remote/codec/encode.rs:107-124, src/tree/mirror/streaming/remote/codec/tests.rs:28)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn frame_signal' src` returns the two test sites, whose bodies are byte-identical by reading; encode.rs:107-124 computes the same five-arm projection inline in `FrameEncoding::new`; `mod error_atlas;` at codec/tests.rs:28 makes `use super::frame_signal` the zero-cost minimum)
- Seen by: structure, correctness, perfapi; refutation: confirmed (neither test copy acts as a differential oracle: both only filter placements and pick buckets); history: no rationale found (both copies date from d88d84e5; maintained in lockstep through every roster change)
- Owner-gated: no

A change to the signal roster must be made in three places, two of them test code where a stale copy narrows what the corpus enumerates without failing to compile. A `Frame::signal(&self) -> Signal` method would be the one home and would let `FrameEncoding::new` read as signal-then-body.

Evidence:

    504	fn frame_signal(frame: &Frame) -> Signal {
    505	    match frame {
    506	        Frame::Reaction(Reaction::Match, flow) => Signal::Match(*flow),
    507	        Frame::Reaction(Reaction::Query(children), flow) if children.is_empty() => {
    508	            Signal::QueryEmpty(*flow)
    509	        }
    510	        Frame::Reaction(Reaction::Query(_), flow) => Signal::Query(*flow),
    511	        Frame::Reaction(Reaction::Supply(_), flow) => Signal::Supply(*flow),
    512	        Frame::End(end) => Signal::End(*end),
    513	    }
    514	}

    encode.rs:
    107	        let (signal, body) = match frame {
    108	            Frame::Reaction(Reaction::Match, flow) => (Signal::Match(*flow), BodyEncoding::Empty),

Resolution: Add `pub(super) fn signal(&self) -> Signal` on `Frame` in frame.rs; have `FrameEncoding::new` compute `let signal = frame.signal();` and match only on the body; delete both test copies and call `frame.signal()`. Minimal alternative: delete error_atlas.rs:543-553 and `use super::frame_signal;`. Acceptance: exactly one `Frame -> Signal` match exists in the tree; codec and atlas snapshots are byte-identical.

### Nits (11)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| remote-capture-atlas-3 | `src/tree/mirror/streaming/remote.rs:75-84` | Five identical `cfg` attributes gate what two `use` statements would carry | Collapse to one `pub use codec::{...}` and one `pub(crate) use codec::{...}` under one attribute each |
| remote-capture-atlas-9 | `src/tree/mirror/streaming/remote/codec/capture.rs:214-220` | The `Role -> Speaker` inverse lives apart from `Speaker::role` | Add a `pub(super) fn from_role(role: Role) -> Speaker` beside `Speaker::role` in signal.rs, call it at capture.rs:195, delete lines 214-220 |
| remote-capture-atlas-11 | `src/tree/mirror/streaming/remote/codec/capture.rs:356-362` | Bare major numbers `7` and `1` beside named `MAJOR_*` constants | Define local `MAJOR_NINT` and `MAJOR_SIMPLE` constants in capture.rs and use them at 356 and 362 |
| remote-capture-atlas-14 | `src/tree/mirror/streaming/remote/codec/capture.rs:502-513` | Dead disjunct in the listing order check, and an order verdict for a key-shape violation | Delete the disjunct |
| remote-capture-atlas-15 | `src/tree/mirror/streaming/remote/codec/capture.rs:600-609` | `render_tag`'s scalar guard computes `scalar()` twice and re-proves itself with an `expect` | Merge the two arms into one `_ => match scalar(content) { Some(text) => inline, None => block }` |
| remote-capture-atlas-16 | `src/tree/mirror/streaming/remote/codec/capture.rs:616-625` | `render_embedded` is a one-production-caller wrapper over `render_embedded_as` | Rename `render_embedded_as` to `render_embedded`, pass `Naming::Plain` at the four call sites, delete the wrapper |
| remote-capture-atlas-25 | `src/tree/mirror/streaming/remote/codec/tests.rs:128-132` | Speaker drawn from a `bool` three times where an `arb_speaker()` strategy would do | Add `fn arb_speaker()` in the file's `prop_oneof!` style and draw `speaker in arb_speaker()` at the three sites |
| remote-capture-atlas-26 | `src/tree/mirror/streaming/remote/codec/tests.rs:265-301` | Fully qualified paths where the import already exists | In codec/tests.rs:265-301 use `cbor::write_head`/`cbor::MAJOR_UINT`/`cbor::MAJOR_ARRAY` |
| remote-capture-atlas-27 | `src/tree/mirror/streaming/remote/codec/tests.rs:325-329` | `write_hex` hand-rolls `hex::encode`, which the same file already uses | Replace `write_hex(&mut atlas, &encoded);` at line 282 with `atlas.push_str(&hex::encode(&encoded));` and delete `write_hex` |
| remote-capture-atlas-30 | `src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:25-47` | Import groups in the atlas are interleaved: crate, std, tokio, super, crate, serde | Regroup as std, external (`serde`, `tokio`), `crate`, `super` |
| remote-capture-atlas-34 | `src/tree/mirror/streaming/remote/codec/tests/error_atlas.rs:674-747` | Synchronous fail-after IO doubles are re-declared per codec test file | Move `FailAfterWriter`/`FailAfterReader` into codec/tests.rs as `pub(super)` and use them with N = 0 in the two sibling suites |

## Remote adapter and streams

Files: src/tree/mirror/streaming/remote/adapter/, remote/streams.rs, and the adapter test suites. Entries: 21 (6 medium, 7 low, 8 nit).

### remote-adapter-streams-3: `read_early` re-implements `read_reply`; `read_reply` over an empty root scope already produces its exact rejections
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:136-200 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:68-131, src/tree/mirror/streaming/remote/adapter/decode.rs:306-392, src/tree/mirror/streaming/remote/adapter/decode.rs:396-414, src/tree/mirror/streaming/remote/adapter/decode.rs:550-560, src/tree/mirror/streaming/remote/proxy/work/encode.rs:153-160, src/tree/mirror/streaming/erased.rs:319-323)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (traced both loops arm by arm against `Scope::new`/`Scope::next` at scope.rs:20-26, 40-44; not executed)
- Seen by: structure ([1]), correctness ([31]), perfapi ([36]), prose ([24], the decode.rs pair); refutation: confirmed ([1], [36]), reframed ([31]: the missing `debug_assert!` is not drift but a copy made without it: 8e1ed47a7 added the assert to `read_reply` five days before 55d76d5cf wrote `read_early`); history: no-rationale-found (55d76d5c explains why `early_supplies` yields incrementally, never why the frame reader is a second copy; 6bac3e1a noticed the twin shape and extended the fan probe to both copies instead of unifying; 08f2899b then threaded the ledger into both)
- Owner-gated: no

`read_early` (136-200) duplicates `read_reply`'s frame loop and its per-record admission (157-176 against 359-383: `records(codec)`, `observe`, `ledger.charge(1)`, `Leaf::leaf`, the test probe, the channel send), including the five-line "set-length half of the greeting's priced premises" comment verbatim at 160-164 and 367-371. With `Scope::new(parent, &[])` and the interior question closure, `read_reply` produces exactly `read_early`'s outcomes: `Match` fails `scope.next()` (empty children) as `UnpositionedMatch` (339 against 182-184); `Query` fails the closure's `scope.next()` as `UnpositionedQuery` (344, 222 against 185-187); `End(Reply)` with an empty skeleton breaks and after a reaction is `BareEndAfterReaction` (327, 329 against 188-189; the first supply record of a reply always starts a run, so `skeleton.is_empty()` and `!any` agree on every wire-reachable input); `End(Stream)` is `UnexpectedStreamEnd` (328 against 190); stream exhaustion is `TruncatedReply` (322-324 against 151-153). The encode side already does this: proxy/work/encode.rs:154-159 renders the early supplies with `encode_reply(..., Scope::opening(&[]), Reply { replies: supplies })`. The one behavioral difference is the in-process-only empty-run `debug_assert!` (352-355) that `read_early` lacks, which the unification closes. Separately, `early_supplies` (83-92) and `assemble_supplies` (404-408) both hand-build the `ReceiverStream` + `#[cfg(test)] inspect(fan_probe::on_recv)` + `Box::pin` + `ops::assemble` pipeline, and `ops::assemble` already returns `Pin<Box<dyn Stream + Send>>` (erased.rs:323), so both `pin!`s (88, 408) are redundant. Two copies of one trust-boundary step are two places every future premise must land; the encode/decode asymmetry is the tell that the bespoke reader is incidental.

Evidence:

       157	                for record in records.records(codec) {
       158	                    let (version, message) = record.map_err(DecodeError::Record)?;
       159	                    let (leaf_prefix, _) = supplies.observe::<B::Error>(parent, &version)?;
       160	                    // The set-length half of the greeting's priced
       161	                    // premises, charged per record before the payload
       162	                    // takes backend custody: a peer supplying past its
       163	                    // declaration fails at the offending record, while
       164	                    // the reply is still open.
       165	                    ledger
       166	                        .charge(1)
       167	                        .map_err(|declared| DecodeError::OverdrawnSupply { declared })?;
       168	                    let leaf = <B::Node<Z> as Leaf>::leaf(version, message)
       169	                        .await
       170	                        .map_err(DecodeError::Backend)?;
       171	                    #[cfg(test)]
       172	                    fan_probe::on_send();
       173	                    if leaves.send(Ok((leaf_prefix, leaf))).await.is_err() {
       174	                        return Ok(());
       175	                    }
       176	                }

    (decode.rs:359-383 is the same sequence with `read.supplies` for `supplies` and a skeleton push at 364-366; the comment at 367-371 is byte-identical to 160-164.)

Resolution: Replace `read_early`'s body with `let read = read_reply::<B, _, _, Scope>(version_bytes, ledger, Scope::new(parent, &[]), &mut frames, <the interior question closure>, leaves, codec).await?; if read.is_none() { return Ok(()); } if frames.next().await.is_some() { return Err(DecodeError::ExtraOpeningReply); } Ok(())`, discarding the skeleton and (necessarily empty) questions. Extract `fn assembly<B>(backend: B, height: usize, rx: mpsc::Receiver<..>) -> Pin<Box<dyn Stream<Item = Result<(ErasedPrefix, B::Erased), B::Error>> + Send>>` holding the `ReceiverStream`/probe/`ops::assemble` setup; `assemble_supplies` becomes `assembly(...).map_err(DecodeError::Backend).try_collect().await` and `early_supplies` pins the same helper; drop both `pin!`s. The `#[cfg(test)]` probe hooks fall from four sites to two, and the `fan_probe` module doc (553) no longer needs to say both paths "hook the same counter". Acceptance: `read_early` is gone or is a handful of lines delegating to `read_reply`; `ledger.charge(1)` and `<B::Node<Z> as Leaf>::leaf` each appear once in decode.rs; `adapter/tests/{opening,malformed,fan_occupancy}.rs` pass unchanged (they pin `UnpositionedMatch`/`UnpositionedQuery`/`BareEndAfterReaction`/`ExtraOpeningReply` on the early stream and the FAN + 1 occupancy ceiling on both channels).

See also: remote-proxy-25.

### remote-adapter-streams-5: `Encoded<Q>`, `Decoded<E, Q>`, `decode<.., Q, N>`, `render<.., D>` and four scope-derivation closures are height-erasure residue
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:261-273 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:28-31, src/tree/mirror/streaming/remote/adapter/decode.rs:221-224, src/tree/mirror/streaming/remote/adapter/decode.rs:249-255, src/tree/mirror/streaming/remote/adapter/encode.rs:21-24, src/tree/mirror/streaming/remote/adapter/encode.rs:45, src/tree/mirror/streaming/remote/adapter/encode.rs:94-106, src/tree/mirror/streaming/remote/adapter/encode.rs:125-140, src/tree/mirror/streaming/remote/adapter/encode.rs:144-156, src/tree/mirror/streaming/remote/adapter/encode.rs:183, src/tree/mirror/streaming/remote/proxy/work/encode.rs:176-190, src/tree/mirror/streaming/remote/proxy/work/encode.rs:217-230)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`git grep -n 'Encoded<\|Decoded<\|Frames<' -- src/` at HEAD: every instantiation is `Scope` or `Vec<Scope>` (encode.rs:85, 116, 150; decode.rs:210, 238), and proxy/work/encode.rs:176-190 and 217-230 carry a `Q` parameter that only ever binds `Scope`; the pre-erasure signatures are as the history pass quotes from `git show d8bef16b^`)
- Seen by: structure ([0]), prose ([24], the encode.rs comment pair); refutation: confirmed; history: deliberate-but-expired (the `Q`/`N`/`D` parameters existed so one body could map `Scope<S<H>>` to `Scope<H>` at every height; d8bef16b erased `Scope` to one type and carried the closure shape forward without mention)
- Owner-gated: no

Before erasure, `Scope<H>` was height-typed and the generic carriers were how one body served every height. `Scope` is now one type, so `Encoded<Q>`, `Decoded<E, Q>`, `Frames<E, Q>`, `N`, and the `D`/`Q` closure parameters have exactly one instantiation each, and the closures at decode.rs:221-224 and 249-255 and encode.rs:94-106 and 125-140 spell the same two rules four times (interior: `scope.next()` then `Scope::new(prefix, listing)`; leaf: reject nonempty, `scope.next()` then `Scope::leaf(prefix)`), with the encode pair also duplicating the `Match` arm and its "Symmetric with decode" comment (96-97, 127-128). The generics ripple into `write_reply<C, Q, E>` and `write_encoded<C, Q, E>` and force `Send + 'static` bounds on `D`. A type parameter is justified by varying; these vary nowhere. Circular justification is the tell, and the erasure commit is exactly the phase boundary where the audit recurs. Four copies of one rule are four places for the encode/decode symmetry the module doc leans on to drift.

Evidence:

       261	async fn decode<B, F, Q, N>(
       262	    backend: B,
       263	    version_bytes: u64,
       264	    ledger: SupplyLedger,
       265	    scope: Scope,
       266	    frames: &mut F,
       267	    question: Q,
       268	    codec: PayloadCodec,
       269	) -> Result<Decoded<B::Erased, Vec<N>>, DecodeError<B::Error>>
       270	where
       271	    B: Backend<Node<Z>: Leaf>,
       272	    F: Stream<Item = Frame> + Unpin,
       273	    Q: FnMut(&mut Scope, &[(u8, Hash)]) -> Result<N, ScopeError>,

        21	pub struct Encoded<Q> {
        22	    frame: Frame,
        23	    question: Option<Q>,
        24	}

Resolution: Introduce a two-variant `enum Level { Interior, Leaf }` (or two named functions with one signature) and one `fn derive_question(level: Level, scope: &mut Scope, listing: &[(u8, Hash)]) -> Result<Scope, ScopeError>` used by both `render` and `read_reply`; `render` handles `Match` and `Supply` itself as `read_reply` already does, so the `debug_assert!(question.is_none())` at encode.rs:183 dissolves (a `Supply` structurally derives no question). Make `Encoded { frame, question: Option<Scope> }`, `Decoded<E> { reply, questions: Vec<Scope> }`, `Frames<E>`; keep `encode_reply`/`encode_leaf_reply`/`decode_reply`/`decode_leaf_reply` as thin wrappers passing the level. Drop `Q` from `write_reply`/`write_encoded` in proxy/work/encode.rs. Acceptance: no type parameter in adapter/{encode,decode}.rs has a single instantiation; `git grep -n 'Symmetric with decode'` returns nothing; the scope-derivation rule appears once; `adapter/tests/{properties,malformed,opening}.rs` pass unchanged except for `into_parts` tuple types.

See also: remote-adapter-tests-19, remote-codec-22, streaming-backend-window-5, tree-core-4, tree-typed-11.

### remote-adapter-streams-19: `ReplyFrame`'s "static" exclusion is a runtime `TryFrom` on frames the adapter never produces, leaving a public error variant no session can fire
- Where: src/tree/mirror/streaming/remote/streams.rs:73-106 (related: src/tree/mirror/streaming/remote/streams.rs:169, src/tree/mirror/streaming/remote/proxy/work/encode.rs:224-229, src/tree/mirror/streaming/remote/proxy/error.rs:51-53, src/tree/mirror/streaming/remote/error.rs:18, src/error.rs:48, src/tree/mirror/streaming/remote/adapter/encode.rs:157-246, src/tree/mirror/streaming/remote/streams/tests.rs:562-571)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`git grep -n ReplyFrame -- src/ tests/`: the single production construction is `ReplyFrame::try_from(frame).map_err(Error::ReplyFrame)?` at proxy/work/encode.rs:226, applied to `render`'s output; read `render` end to end: it yields `Frame::Reaction(_, Flow::Continue)` at 166-169, 176-179, 215-218, 227-230, `Frame::Reaction(_, Flow::End)` at 237-240, and `Frame::End(End::Reply)` at 241-244, never `End::Stream`; `ReplyFrameError` is re-exported at remote/error.rs:18 and src/error.rs:48)
- Seen by: structure ([2]), prose ([26], the "smuggle" wording); refutation: confirmed (with the caveat that "statically excluding" is true at `StreamSender::frame`'s signature; the substance is the guard with no constructible failure and the public variant no session can produce); history: no-rationale-found (the type, its `TryFrom`, and the doc line arrived verbatim in 24b187a8 and moved in b3b877d9; the choice of a runtime `TryFrom` over infallible constructors is argued nowhere)
- Owner-gated: yes: deleting `ReplyFrameError` and `RemoteError::ReplyFrame` changes the public error taxonomy re-exported through `crate::error`

`ReplyFrame` does keep `End::Stream` out of `StreamSender::frame`'s signature (169), but the only way to construct one is the fallible `TryFrom<Frame>`, and its only production caller applies it to `Encoded.frame` values that `render` only ever builds as reactions or `End::Reply`. The check therefore never fails, and `ReplyFrameError` plus `RemoteError::ReplyFrame` (whose own doc, proxy/error.rs:51, calls it "A frame constructed by the adapter violated the reply-only boundary") are public taxonomy entries no session can produce. The exclusion is not static at the adapter because `ReplyFrame` lives in `streams`, which the adapter does not depend on, so the adapter cannot emit the typed frame. The cheapest artifact that satisfies the check is the one the code already produces, so the check catches nothing; a guard must name a concrete, constructible failure. The doc's "smuggle" also imports an adversary for what is an in-process programmer error (the model of record has no hostile party here).

Evidence:

        73	/// A protocol reply frame, statically excluding stream-end transport control.
        74	///
        75	/// Stream end is a lifecycle event owned by [`StreamSender::finish`]; a
        76	/// producer cannot smuggle one into the middle of its replies.
        77	#[derive(Debug, Clone, PartialEq, Eq)]
        78	pub struct ReplyFrame(Frame);
        79	
        80	impl TryFrom<Frame> for ReplyFrame {
        81	    type Error = ReplyFrameError;
        82	
        83	    /// Check that a general wire frame belongs to a protocol reply.
        84	    fn try_from(frame: Frame) -> Result<Self, Self::Error> {
        85	        if matches!(frame, Frame::End(End::Stream)) {
        86	            Err(ReplyFrameError::StreamEnd)
        87	        } else {
        88	            Ok(Self(frame))
        89	        }
        90	    }
        91	}

    proxy/work/encode.rs:
       226	            let frame = ReplyFrame::try_from(frame).map_err(Error::ReplyFrame)?;

Resolution: Move `ReplyFrame` beside `Frame`/`End` in `codec::frame` with infallible constructors (`ReplyFrame::reaction(Reaction, Flow)`, `ReplyFrame::reply_end()`) and `From<ReplyFrame> for Frame`; have `render` yield `Encoded { frame: ReplyFrame, .. }` so `write_encoded` passes it straight to `StreamSender::frame`. Delete `TryFrom<Frame>`, `ReplyFrameError`, `RemoteError::ReplyFrame` (owner decision), and the `stream_end_is_not_a_reply_frame` test that pins the runtime check; adapter tests using `into_parts()` compare via `Frame::from`. If the type is kept as is, at least reword 75-76 to "a producer cannot emit one mid-reply". Acceptance: `git grep ReplyFrameError` returns nothing; `Encoded`'s frame field is `ReplyFrame`; no `TryFrom<Frame>` exists; `cargo doc` links resolve; wire snapshots unchanged (no byte moves).

See also: remote-adapter-streams-20.

### remote-adapter-tests-2: the three ingress premises are re-spelled at every decode and encode call site
- Where: src/tree/mirror/streaming/remote/adapter/tests.rs:35-39 (related: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:42-55, src/tree/mirror/streaming/remote/adapter/tests/properties.rs:111-119, src/tree/mirror/streaming/remote/adapter/tests/opening.rs:153-165, src/tree/mirror/streaming/remote/adapter/tests/runs.rs:108-128)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -c 'PayloadCodec::new::<u64>(PayloadDepthLimit::default())'` per file: backend_errors 1, fan_occupancy 2, malformed 14, opening 5, parking 2, properties 12, runs 4, total 40; `u64::MAX,` 36 and `unbounded(),` 36 across the partition)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: codemod residue (165b0dd3 threaded `version_bytes`, 08f2899b the ledger with the `unbounded()` helper, 4356e197 the codec inline at every site)
- Owner-gated: no

`PayloadCodec::new::<u64>(PayloadDepthLimit::default())` appears 40 times and `u64::MAX, unbounded(),` 36 times, while the only helper in hand covers one premise of three. The handful of tests that vary a premise (malformed.rs:523-571 the version bound, malformed.rs:610-683 and opening.rs:187-222 the ledger, backend_errors.rs the backend) are visually indistinguishable from the forty that do not, and the varied parameter is the thing a reader came to see.

Evidence:

    35	/// A set-length allowance no fixture here can exhaust, for tests whose
    36	/// subject is not the ingress supply charge.
    37	fn unbounded() -> SupplyLedger {
    38	    SupplyLedger::new(u64::MAX)
    39	}

    42	    let error = runtime().block_on(async {
    43	        let mut frames = stream::iter(frames);
    44	        decode_leaf_reply(
    45	            Local,
    46	            u64::MAX,
    47	            unbounded(),
    48	            Scope::new(parent.erase(), &[(0, hash(0))]),
    49	            &mut frames,
    50	            PayloadCodec::new::<u64>(PayloadDepthLimit::default()),
    51	        )

Resolution: add to tests.rs a `fn codec() -> PayloadCodec` and thin wrappers that fix `Local, u64::MAX, unbounded(), codec()` for the common decode, leaf-decode, and early-supply shapes, leaving the raw six-argument calls only where a premise is varied so those sites stand out. Acceptance: the codec constructor appears in the helper and at the sites that vary it, nowhere else; the tests varying the version bound, ledger, or backend are the only ones spelling those arguments.

See also: remote-adapter-streams-4, remote-proxy-24.

### remote-adapter-tests-10: fixture and helper machinery is duplicated across the partition
- Where: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:321-335 (related: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:53-84; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:445-448, 576-582; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:965-975; src/tree/mirror/streaming/remote/adapter/tests/opening.rs:123-124, 137-151, 190-191; src/tree/mirror/streaming/remote/adapter/tests/fan_occupancy.rs:43-75; src/tree/mirror/streaming/remote/adapter/tests/runs.rs:87-104; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:727-739, 863-872; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:584-595; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:61-62, 98-101, 363-366; src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:28-48, 178-201; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:1003-1030; src/tree/mirror/streaming/remote/adapter/tests/properties.rs:834-840; src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:369-377; src/tree/mirror/streaming/remote/adapter/tests/runs.rs:284-294)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read every cited site; `grep -c 'a test record fits the run framing'` finds the expect string defined at tests.rs:46 and again at fan_occupancy.rs:65)
- Seen by: structure-prose, api-economics; refutation: confirmed (with the correction that the "ascending leaves" family spans three different fixture shapes, so the shared helper needs a parameter rather than being a pure hoist); history: pure accretion (each file's helpers landed with the commit that created the file; `OpeningNode::node` at opening.rs:43-56 builds a fixed `Version::new()`/`Message::new(())` leaf and is a different shape from the two `LeafCase`-derived node traits)
- Owner-gated: no

The same machinery is written several times with small variations: (a) `under_root_pair()` is `colliding_leaves(2)` with a tuple return consumed as `.0/.1/.2` at malformed.rs:343, 347, 369, 409-410, 432-433, 694, 699; (b) "scan versions until a path predicate holds" appears as runs.rs `separated_leaves`, malformed.rs:445-448, and properties.rs `foreign_parent`; (c) "n ascending leaves" appears as malformed.rs `ascending_leaves`, fan_occupancy.rs `leaves` (raw versions), and inline at opening.rs:123-124 and 190-191 (the last duplicated within its own file), each choosing `ticks` differently (`value as u8 % 4`, `0`, `1`); (d) fan_occupancy.rs:62-66 re-implements `leaf_run` with the identical expect string; (e) "last frame gets `Flow::End`" tail-flagging is spelled six times; (f) `AdapterHeight::node` and `BackendHeight::node` are the same one-leaf-subtree builder; (g) the `dispatch_height!`/`at_height!` recursive macro pair is defined twice here (and a third time in `materialized/work/tests/violations.rs`) where a `seq_macro::seq!` over a range would express each in six lines; (h) the `<Local as Backend>::leaves(Local, prefix, assume::<H>(node.clone()))` rebuild appears three times. Each duplicate is a place a fixture change must be repeated, and the tuple-indexed variant is less legible than the `LeafCase` it wraps.

Evidence:

    321	fn under_root_pair() -> [(Version, Message, Path); 2] {
    322	    let mut by_radix: BTreeMap<u8, Vec<(Version, Message, Path)>> = BTreeMap::new();
    323	    for value in 0..u64::MAX {
    324	        let leaf = LeafCase::new(value, value as u8 % 4);
    325	        let path = leaf.path();
    326	        let bytes: [u8; 32] = path.into();
    327	        let group = by_radix.entry(bytes[0]).or_default();
    328	        group.push((leaf.version, leaf.message, path));
    329	        if group.len() == 2 {

    53	fn colliding_leaves(count: usize) -> Vec<LeafCase> {
    54	    let mut by_radix: BTreeMap<u8, Vec<LeafCase>> = BTreeMap::new();
    55	    for value in 0..u64::MAX {
    56	        let leaf = LeafCase::new(value, value as u8 % 4);

Resolution: hoist into tests.rs: `colliding_leaves(count)`, a `leaf_outside::<H>(anchor)` search, an `ascending_leaves(count, ticks)` builder, a `reply_frames(Vec<WireReaction>) -> Vec<Frame>` that owns tail-flagging, one `NodeAt: Height { fn node(&LeafCase) -> Node<Self> }` trait for both `LeafCase`-derived ladders, one `seq!`-based dispatch macro parameterized by range (the production `at_height!` in erased.rs spans 0..=32 and cannot be reused directly because `AdapterHeight` is not implemented at H32 and `FailureHeight` not at Z or H32), and a `leaves_of::<H>(node, prefix)` rebuild. Replace `under_root_pair` with `colliding_leaves(2)` and index fields by name; have fan_occupancy's `frames` call `leaf_run`. Acceptance: each helper has one definition in the partition; malformed.rs contains no `.0/.1/.2` access on leaf fixtures; the expect string appears once; `grep -rn 'macro_rules! dispatch_height' src` is empty; the partition's line count drops with no test removed.

### remote-adapter-tests-20: properties.rs states each of its six laws twice, once for `Z` and once for `S<H>`
- Where: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:98-658 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:203-259, src/tree/mirror/streaming/remote/adapter/encode.rs:80-142, src/tree/mirror/streaming/remote/adapter.rs:22-26)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (side-by-side read of 103-133 vs 368-398, 135-175 vs 400-445, 177-251 vs 447-527, 253-313 vs 529-610, 315-333 vs 612-630, 335-354 vs 632-657)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (both impls from 00b29d32/07504b2e, no message bodies; the production split's rationale is stated at adapter.rs:22-26, the test-side duplication's is not; backend_errors.rs implements `FailureHeight` for `S<H>` only because leaf height has nothing to inject into, so it never faced this choice)
- Owner-gated: no

The differences between the `Z` impl (98-355) and the `S<H>` impl (357-658) are exactly: `decode_leaf_reply`/`encode_leaf_reply` vs `decode_reply`/`encode_reply`; `Scope::leaf(prefix)` vs `Scope::new(prefix, &nested)`; `leaf_case.nested.clear()` at 185 (leaf queries carry no listing); the literal `"height 0"` labels (`Z::HEIGHT` is 0, so `"height {}", Self::HEIGHT` serves both); and `Prefix::<S<Z>>` vs `Prefix::<S<S<H>>>`, both spellable as `Prefix::<S<Self>>`. Everything else is textually identical. A law stated twice can drift into two laws, and the reviewer must diff 560 lines to confirm the leaf case is the general case plus one exception.

Evidence:

    103	    fn supplied_leaf_is_lossless(
    104	        leaf: &LeafCase,
    105	        runtime: &tokio::runtime::Runtime,
    106	    ) -> TestCaseResult {
    107	        let (parent, radix) = Prefix::<Z>::containing(&leaf.path()).pop();
    108	        let scope = Scope::new(parent.erase(), &[]);
    109	        let frame = supplied_frame(leaf, Flow::End);
    110	        let mut frames = stream::iter([frame.clone()]);
    111	        let decoded = runtime
    112	            .block_on(decode_leaf_reply(

    368	    fn supplied_leaf_is_lossless(
    369	        leaf: &LeafCase,
    370	        runtime: &tokio::runtime::Runtime,
    371	    ) -> TestCaseResult {
    372	        let (parent, radix) = Prefix::<S<H>>::containing(&leaf.path()).pop();
    373	        let scope = Scope::new(parent.erase(), &[]);
    374	        let frame = supplied_frame(leaf, Flow::End);
    375	        let mut frames = stream::iter([frame.clone()]);
    376	        let decoded = runtime
    377	            .block_on(decode_reply::<Local, _>(

Resolution: move the varying operations onto `AdapterHeight`: `decode(..)` and `encode(..)` selecting the entry, `question(parent, radix, nested) -> Scope` (`Z`: `Scope::leaf`; `S<H>`: `Scope::new`), and `nested(case) -> &[(u8, Hash)]` (empty at `Z`). Implement those once per impl and write each of the six laws once as a free generic function over `H: AdapterHeight` (or default trait methods), labelling with `"height {}", H::HEIGHT` throughout. Acceptance: each law body appears once in properties.rs; the `Z` and `S<H>` impls contain only node construction and entry/question selection; the six proptests and their doc comments are unchanged; the file is roughly half its current length.

### remote-adapter-streams-4: The peer's decode premises (`version_bytes`, `ledger`, `codec`) travel as three loose parameters through six signatures and two structs
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:203-210 (related: src/tree/mirror/streaming/remote/adapter/decode.rs:68-75, src/tree/mirror/streaming/remote/adapter/decode.rs:231-238, src/tree/mirror/streaming/remote/adapter/decode.rs:261-269, src/tree/mirror/streaming/remote/proxy/work/pump.rs:184-190, src/tree/mirror/streaming/remote/proxy/work/pump.rs:198-205, src/tree/mirror/streaming/remote/proxy/work/pump.rs:227-234, src/tree/mirror/streaming/remote/proxy/work/pump.rs:290-297, src/tree/mirror/streaming/remote/proxy/work/pump.rs:384-391, src/tree/mirror/streaming/remote/proxy/work/pump.rs:417-430, src/tree/mirror/streaming/remote/proxy/work.rs:54-65, src/tree/mirror/streaming/remote/streams.rs:423-431)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read every pump.rs and work.rs site listed; the trio is read together at pump.rs:184-187, passed unchanged at four call sites, and re-stored in `Early` at 417-430)
- Seen by: structure ([7]), perfapi ([35]); refutation: confirmed ([7]), reframed ([35]: the stream-side half is only partly right, since sender and receiver contexts overlap on speaker/stats/observe only); history: no-rationale-found (each parameter was threaded on its own: 165b0dd3, 08f2899b, 4356e197; the one nearby arity argument, proxy/work/encode.rs:77-78, concerns per-edge dataflow, not shared constants)
- Owner-gated: no

`version_bytes: u64`, `ledger: SupplyLedger`, and `codec: PayloadCodec` are per-session constants derived from the peer's greeting. They are threaded together through `early_supplies`, `decode_reply`, `decode_leaf_reply`, `decode`, `read_reply`, and `read_early`, read together from `Work` (pump.rs:185-187), passed unchanged at four call sites, and stored together again in `Early` (pump.rs:417-430), each with its own copy of the same field doc. Three values always created, cloned, and consumed together are one thing without a name; a bare `u64` in the middle of a six-argument list is the types-first smell. The steelman at proxy/work/encode.rs:77-78 ("bundling edges into a struct would only rename the arity") argues against bundling channel edges; these are shared constants, so it does not apply. On the stream side, `read_frames` carries `#[allow(clippy::too_many_arguments)]` (streams.rs:423) for seven positional parameters of which five (speaker, budget, route, stats, observe) are session-scoped; a receiver-side context would drop the allow, but the sender shares only three of them, so one shared struct is less clean than the decode-side bundle.

Evidence:

       203	pub async fn decode_reply<B, F>(
       204	    backend: B,
       205	    version_bytes: u64,
       206	    ledger: SupplyLedger,
       207	    scope: Scope,
       208	    frames: &mut F,
       209	    codec: PayloadCodec,
       210	) -> Result<Decoded<B::Erased, Vec<Scope>>, DecodeError<B::Error>>

Resolution: Define a small `#[derive(Clone)]` struct in the adapter (the greeting's priced premises plus the peer's payload codec) with the three fields and their existing per-field docs; `Work` constructs it once; `Early` holds one field; `decode_reply(backend, premises: &Premises, scope, frames)`; `SupplyRuns::new(premises.version_bytes)`. Optionally a receiver-scoped context in streams.rs for `read_frames`. Acceptance: each adapter decode entry point takes four or fewer parameters; `Early` stores one premises field; the pump call sites shrink; adapter and proxy tests pass unchanged.

See also: remote-adapter-tests-2, remote-proxy-24, remote-proxy-4.

### remote-adapter-streams-9: `Prefix<Z>` to `[u8; 32]` spelled as a fallible `try_into().expect(..)` where an infallible `From` exists
- Where: src/tree/mirror/streaming/remote/adapter/decode.rs:520-526 (related: src/tree/typed/prefix.rs:18, src/tree/typed/prefix.rs:113-123, src/tree/mirror/streaming/remote/adapter/encode.rs:249-252, src/tree/mirror/streaming/remote/adapter/decode.rs:463)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (prefix.rs:18 declares `pub struct Prefix<H: Height = Z>`, so `impl From<Prefix> for [u8; 32]` at prefix.rs:119-123 is `From<Prefix<Z>>`; `previous_leaf` is `Option<Prefix<Z>>` at decode.rs:463; encode.rs:250 already uses `Path::from(current)` on the same type)
- Seen by: structure ([5]); refutation: confirmed; history: no-rationale-found (the `expect` dates to 00b29d32, when a two-step infallible route via `Path` already existed; the one-step `From` arrived in 9c73d7b4)
- Owner-gated: no

The wire-input rejection path converts a `Prefix<Z>` to `[u8; 32]` through a slice under an `expect`. The compiler already knows a `Prefix<Z>` is 32 bytes; re-deriving that at runtime trades an evidently correct conversion for one that needs a proof string. Types-first: the sanctioned use of `expect` is a one-line proof of programmer error, and here no proof is needed.

Evidence:

       520	            return Err(DecodeError::LeafOrder {
       521	                previous: previous
       522	                    .as_bytes()
       523	                    .try_into()
       524	                    .expect("a leaf prefix occupies a full content path"),
       525	                current: path.into(),
       526	            });

Resolution: `previous: previous.into(),`. Acceptance: no `try_into().expect` remains in decode.rs; `malformed.rs`'s `LeafOrder` pin passes unchanged.

### remote-adapter-streams-12: `render` repeats the flush-pending-then-yield block four times; two suffice
- Where: src/tree/mirror/streaming/remote/adapter/encode.rs:161-233 (related: src/tree/mirror/streaming/remote/adapter/encode.rs:105, src/tree/mirror/streaming/remote/adapter/encode.rs:139, src/tree/mirror/streaming/remote/adapter/encode.rs:183)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the block appears at 163-170, 173-180, 212-219, 224-231, and the Supply arm's `question` is asserted `None` at 183 because both `derive` closures return `Ok(None)` for `Supply` at 105 and 139)
- Seen by: structure ([6]); refutation: confirmed; history: no-rationale-found (the Supply-arm copies came with f94f2056, whose message states the flush rule but not the code shape)
- Owner-gated: no

The `Match`, `Query`, mid-run `Supply` flush, and end-of-`Supply` arms each contain the same eight lines: `if let Some((previous, question)) = pending.replace((<wire>, <question>)) { yield Encoded { frame: Frame::Reaction(previous, Flow::Continue), question } }`. Because the Supply arm's final `replace` uses `None` as its question and `question` is `None` for `Supply`, all three per-reaction replaces are one statement over a `wire: WireReaction` the match can produce; only the mid-run flush must stay inside the leaf loop. `yield` cannot be factored into a helper inside `try_stream!`, which is why the repetition accreted, but the match can yield the wire reaction and one trailing replace-and-yield can follow it. The one-frame lookahead is the whole idea of `render`; it reads more clearly stated once than four times, and each copy is a place for the Continue/End discipline to drift.

Evidence:

       162	                ProtocolReaction::Match => {
       163	                    if let Some((previous, question)) =
       164	                        pending.replace((WireReaction::Match, question))
       165	                    {
       166	                        yield Encoded {
       167	                            frame: Frame::Reaction(previous, Flow::Continue),
       168	                            question,
       169	                        };
       170	                    }
       171	                }

Resolution: `let wire = match reaction { Match => WireReaction::Match, Query(listing) => WireReaction::Query(listing), Supply(radix, node) => { <leaf loop with the mid-run flush yield>; assert!(!run.is_empty(), ..); WireReaction::Supply(run) } }; if let Some((previous, question)) = pending.replace((wire, question)) { yield Encoded { frame: Frame::Reaction(previous, Flow::Continue), question }; }`. Acceptance: two `yield Encoded { frame: Frame::Reaction(.., Flow::Continue), .. }` sites in `render`; `adapter/tests/{runs,properties}.rs` (frame sequences and Continue/End placement) pass unchanged; wire snapshots unchanged.

### remote-adapter-streams-20: `SendState` forces two `unreachable!`s that `Option`-shaped state removes
- Where: src/tree/mirror/streaming/remote/streams.rs:179-234 (related: src/tree/mirror/streaming/remote/streams.rs:131-134)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure ([4]); refutation: confirmed (failure semantics preserved: a failed end-control write drops the opened state, the contract's abort, exactly as `finish(mut self)` drops `self` today); history: no-rationale-found (at b3b877d9 only `write`'s `unreachable!` existed; 4be4b830, a WIP-labeled snapshot, added `Done` to the `Open` variant and introduced the second `unreachable!` plus the `mem::replace` without arguing the encoding)
- Owner-gated: no

`finish` and `write` each end with a `let SendState::Open(..) = ... else { unreachable!(..) }` whose proof is "the open state was just stored/written through". With `state: Option<Opened<Tx>>` (a two-field struct for the writer and its `Done`), `write` binds `None => state.insert(Opened { .. })` (`Option::insert` returns `&mut T`, reusing the same disjoint-field borrow the current `state @ SendState::Unopened` arm relies on) and `finish` becomes `let Some(mut opened) = self.state.take() else { return Ok(()) }; opened.frame(stream, Frame::End(End::Stream)).await?; opened.done.complete(opened.write.into_inner().into_inner())` once the frame write moves into `Opened::frame`. Both `unreachable!`s and the `mem::replace` disappear. Panics reachable only by programmer error are permitted, but a state encoding that needs two of them to say "I just set this" is less evidently correct than one where the type carries the fact.

Evidence:

       183	                self.write(Frame::End(End::Stream)).await?;
       184	                let SendState::Open(write, done) =
       185	                    std::mem::replace(&mut self.state, SendState::Unopened)
       186	                else {
       187	                    unreachable!("the open state was just written through");
       188	                };
        ...
       224	                let SendState::Open(write, _) = state else {
       225	                    unreachable!("the open state was just stored");
       226	                };

Resolution: Replace `enum SendState<Tx>` with `struct Opened<Tx> { write: FrameWrite<CountedWrite<Tx>>, done: Done<Tx> }` and `state: Option<Opened<Tx>>`; factor the frame write into `Opened::frame(&mut self, stream: Stream, frame: Frame)`; restructure `write`/`finish` as above. Acceptance: `git grep -n 'unreachable!' src/tree/mirror/streaming/remote/streams.rs` returns nothing from `StreamSender`; `unopened_sender_finishes_without_connecting`, `truncated_stream_is_reported_not_ended`, and `frames_flow_sender_to_claimed_receiver` pass unchanged.

See also: inventory-6.

### remote-adapter-streams-24: `StreamReceiver`'s two-`Option` state and `ReceiverStart` dissolve: the `stream!` generator is already lazy
- Where: src/tree/mirror/streaming/remote/streams.rs:304-331 (related: src/tree/mirror/streaming/remote/streams.rs:338-359, src/tree/mirror/streaming/remote/streams.rs:367-375, src/tree/mirror/streaming/remote/streams.rs:377-396, src/tree/mirror/streaming/remote/streams.rs:408-417, src/tree/mirror/streaming/remote/streams.rs:436-452)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read `read_frames`: every side effect, `claim.await` at 437 and the observer creation at 453-455 included, sits inside the `stream!` block, which runs only when first polled)
- Seen by: structure ([3]); refutation: confirmed (two costs added: `Box::pin` moves to `new` for every receiver including never-polled ones, one small allocation per stream per session, bounded by `STREAM_COUNT`; `StreamReceiver<Rx>`'s type parameter would become phantom); history: no-rationale-found (verbatim from b3b877d9; the deadlock design specifies when a receiver claims, not the struct shape that stages it)
- Owner-gated: no

`StreamReceiver` holds `start: Option<ReceiverStart<Rx>>` and `frames: Option<BoxStream>`, with `frames()` moving seven fields out of one into the other on first poll behind an `expect`. `read_frames` is an `async_stream::stream!` whose body runs only when first polled, so building the boxed stream in `new` preserves lazy claiming exactly. The only thing the two-`Option` shape buys is the "was it ever polled" bit that `finish` needs, which is one `bool` set in `poll_next`. An invariant ("exactly one of the two `Option`s is `Some`") held by convention plus an `expect`, and a seven-field struct that exists only to be destructured once, are incidental complexity when the generator already provides the laziness.

Evidence:

       304	pub struct StreamReceiver<Rx> {
       305	    /// The claim and identity, consumed to build `frames` on first poll.
       306	    start: Option<ReceiverStart<Rx>>,
       307	    /// `Some` exactly once the stream has been claimed: the first poll
       308	    /// builds it, and [`finish`](Self::finish) reads its absence as "this
       309	    /// level was never needed".
       310	    frames: Option<BoxStream<'static, Frame>>,
       311	}

Resolution: `StreamReceiver { frames: BoxStream<'static, Frame>, claimed: bool }`; `new` calls `Box::pin(read_frames(..))` directly; `poll_next` sets `claimed = true` before polling; `finish` checks `!self.claimed`. Delete `ReceiverStart` and `frames()`; move the per-field docs (stats and observe rationale) onto `read_frames`' parameters. Decide whether `Rx` stays as a phantom or the type loses the parameter (callers in state.rs name `StreamReceiver<A::Rx>`). Acceptance: no `expect` remains in `StreamReceiver`; `unpolled_receiver_finishes_vacuously`, `frames_flow_sender_to_claimed_receiver`, and `supply_failure_reaches_the_awaiting_receiver` pass unchanged.

### remote-adapter-tests-11: a positive round-trip law lives in malformed.rs and duplicates a weaker test in runs.rs
- Where: src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:337-396 (related: src/tree/mirror/streaming/remote/adapter/tests/runs.rs:257-296, src/tree/mirror/streaming/remote/adapter/tests/malformed.rs:1, src/tree/mirror/streaming/remote/adapter/tests/runs.rs:152-217)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read both bodies; `git log -S` places the malformed test at 00b29d32 and the runs.rs test at f94f2056, which also rewrote the malformed test's closing assertion to the batched form)
- Seen by: structure-prose, api-economics (proposing opposite deletions); refutation: confirmed (the malformed test is the stronger one: frame byte-equality at :395 implies runs.rs's `frames.len() == 1` at :264; both point tests are already implied by the runs.rs proptest apart from the default-budget instantiation); history: no rationale found (f94f2056 kept both without saying why)
- Owner-gated: no

`a_multi_leaf_run_is_one_supplied_subtree` decodes two unbatched frames to one node, rebuilds two leaves, and asserts the re-encoded frame list equals one batched run; runs.rs `a_batched_run_round_trips_the_reply` does the first two steps for four leaves and stops. Neither is a malformed-wire case, contradicting malformed.rs's module doc, and the same law is maintained in two files.

Evidence:

    1	//! Focused malformed-wire cases which are not naturally height-parametric.

    337	/// Consecutive leaves in one version-derived run assemble as one node and reexplode exactly.
    338	#[test]
    339	fn a_multi_leaf_run_is_one_supplied_subtree() {
    340	    let leaves = under_root_pair();

    257	/// The re-encoded reply of a multi-leaf reaction under the default budget
    258	/// is one batched frame whose decode reproduces the protocol reply
    259	/// exactly: the round trip is lossless through the batched form.
    260	#[test]
    261	fn a_batched_run_round_trips_the_reply() {

Resolution: keep one multi-leaf round-trip witness in runs.rs (the batching contract's module) carrying the stronger assertion (re-encoded frames equal one `leaf_run` of all leaves with `Flow::End`), delete the other, and retire `under_root_pair` in favor of the hoisted `colliding_leaves`. Either the version-bound and set-length tests' admitting halves move beside the laws they witness, or malformed.rs's module doc widens to "ingress validation: rejections and their admitting boundaries". Acceptance: one multi-leaf round-trip test in the partition, in runs.rs, asserting the canonical batched frame bytes; malformed.rs's module doc matches its roster.

### remote-adapter-tests-19: `ErasedUnit` and `ErasedU64` alias the same type; the per-payload distinction is gone
- Where: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:21-23 (related: src/tree/mirror/streaming/remote/adapter/tests/properties.rs:142, 186, 407, 454, 684, 763, 808, 827, 886, 920; src/tree/mirror/streaming/backend/local.rs:116)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git log -L21,23`: d8bef16b introduced `<Local as Backend<()>>::Erased` and `<Local as Backend<u64>>::Erased`, distinct under the then payload-generic `Backend<T>`; 48bc31df rewrote both right-hand sides to `<Local as Backend>::Erased` and kept the names and the "per payload" sentence)
- Seen by: structure-prose, api-economics; refutation: reframed (the structure-prose seed's premise that the distinction "has never existed in the type system" is wrong; it existed until the payload erasure); history: deliberate but expired
- Owner-gated: no

Both aliases expand to `typed::untyped::Node`. The names and the comment describe a distinction payload erasure removed, and the helper signatures below still choose one or the other (`Reply<ErasedUnit>` at 142, 407, 886, 920; `Reply<ErasedU64>` at 684, 763, 808, 827), suggesting a typing constraint that no longer exists. A reader must check the definitions to learn the two are interchangeable.

Evidence:

    21	/// The in-memory backend's erased node representations, per payload.
    22	type ErasedUnit = <Local as Backend>::Erased;
    23	type ErasedU64 = <Local as Backend>::Erased;

Resolution: collapse to one alias (`type Erased = <Local as Backend>::Erased;`) or use the path directly, and drop the "per payload" sentence. Acceptance: no `ErasedUnit`/`ErasedU64` identifiers remain in properties.rs.

### Nits (8)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| inventory-11 | `src/tree/mirror/streaming/remote/streams.rs:521-524` | Mutex poison is handled two ways in production code | Ride through poison in streams.rs as router.rs does, with its one-line rationale, or state why a poisoned deposit slot must abort; apply the rule to transport.rs |
| inventory-12 | `src/tree/mirror/streaming/remote/streams.rs:423-432` | A `too_many_arguments` allow on a seven-parameter function | Delete the attribute |
| remote-adapter-streams-10 | `src/tree/mirror/streaming/remote/adapter/decode.rs:530-539` | `SupplyOrder`'s "preceded" half is unreachable; `LeafOrder` fires first | Narrow the filter to `*previous == radix` and reword the variant to the reachable case, or comment why `<` cannot arrive |
| remote-adapter-streams-18 | `src/tree/mirror/streaming/remote/streams.rs:62-68` | Long qualified paths, function-local imports, an unimported `crate::Version`, an out-of-group import, and one implicit head width | Import `io`, `mem`, `Arc`, `Mutex`, `AsyncRead`, and `cbor` at file top; spell the label capacity with two `head_len` terms; fix the import groups in decode.rs, pump.rs, work.rs, and tests.rs |
| remote-adapter-streams-25 | `src/tree/mirror/streaming/remote/streams.rs:506` | The supply-failure deposit slot is spelled twice with two lock sites; a newtype names it once | `struct SupplyFailureSlot(Arc<Mutex<Option<io::Error>>>)` with `deposit(&self, io::Error)` (`get_or_insert`) and `take(&self) -> Option<io::Error>` |
| remote-adapter-streams-26 | `src/tree/mirror/streaming/remote/streams.rs:620-631` | `claims()` builds the slots through a `Vec` and a fallible `try_into` where two `array::from_fn` calls suffice | Build both fixed arrays with `std::array::from_fn`, filling one from the other's closure; no `Vec`, no `unreachable!` |
| remote-adapter-streams-27 | `src/tree/mirror/streaming/remote/streams.rs:736-741` | `ClaimSlots` lacks the `take` its mirror `Claims` has; the driver reaches into its field | `impl<Rx> ClaimSlots<Rx> { fn take(&mut self, stream: Stream) -> Option<oneshot::Sender<(Rx, Done<Rx>)>> }` |
| remote-adapter-tests-5 | `src/tree/mirror/streaming/remote/adapter/tests/backend_errors.rs:3-26` | import layout is codemod residue across the suite | One `use crate::{..}` tree per file in std / external / crate / super order; no item between imports; no `super::super::` or function-local `use` |

## Remote proxy

Files: src/tree/mirror/streaming/remote/proxy/ and its test suites. Entries: 22 (3 medium, 10 low, 9 nit).

### remote-proxy-4: complete_connect and accept duplicate the post-exchange tail, then thread it through four positional argument lists
- Where: src/tree/mirror/streaming/remote/proxy/start.rs:168-196 (related: src/tree/mirror/streaming/remote/proxy/start.rs:210-245, src/tree/mirror/streaming/remote/proxy/start.rs:338-350, src/tree/mirror/streaming/remote/proxy/start.rs:401-414, src/tree/mirror/streaming/remote/proxy/state.rs:41-44, src/tree/mirror/streaming/remote/proxy/state.rs:119-130, src/tree/mirror/streaming/remote/proxy/work.rs:101-112, src/tree/mirror/streaming/remote/proxy/work/encode.rs:77-79)
- Class / severity / confidence: simplification / medium / high
- Provenance: assessed (read)
- Seen by: structure, prose (duplicated depth-rationale comment), perfapi (arity and the `budget` copy); refutation: confirmed; history: no-rationale-found for the duplicated tail (each branch was edited in lockstep by aabdcea07, bdf74d45, 71de90c11); deliberate-and-holds for the arity ("one premise per argument", 48bc31df9, inline at start.rs:338-339 and work.rs:101-102)
- Owner-gated: no for the shared helper; the arity bundling contests an inline rationale and is listed under open questions

After the greetings cross, both trait impls stamp the codec limit, call `payload_depth_limits_match` under the same three-line comment, call `window.resolve` with the same five arguments, call `run_budget`, and pass nine positional arguments to `connected`, which passes eleven to `open`, which passes eight to `Work::new` and nine to `Connected::new`. The same session facts cross four argument lists under four `#[allow(clippy::too_many_arguments)]`, two of them (start.rs:401, state.rs:119) with no rationale and the other two with a comment that wraps from an attribute trailer onto a standalone line. `Session` additionally holds a `budget` copy whose only doc explains that the other copy lives in `Work`, which `Session` owns. Principle: one site per decision on a correctness-relevant path (the depth check and window resolution); duplicated logic edited in one branch and not the other is the concrete hazard.

Evidence:

    173	        // Payload depth limits must be equal — checked after both
    174	        // greetings are in hand and before the equal-versions resolution,
    175	        // so a mixed configuration is caught even on a converged session.
    176	        payload_depth_limits_match::<B::Error>(&self.codec, &self.versions.remote)?;
    177	        let window = self.window.resolve(
    178	            theirs.set_len,
    179	            self.versions.remote.set_len,
    180	            theirs.max_version_bytes,
    181	            self.versions.remote.max_version_bytes,
    182	            B::node_bytes,
    183	        );
    184	        let budget = run_budget(&theirs, &self.versions.remote);

    220	        // Payload depth limits must be equal — checked after both
    221	        // greetings are in hand and before the equal-versions resolution,
    222	        // so a mixed configuration is caught even on a converged session.
    223	        payload_depth_limits_match::<B::Error>(&self.codec, &remote)?;
    224	        let greeting = remote.clone();
    225	        let window = self.window.resolve(

    338	#[allow(clippy::too_many_arguments)] // The argument list is the handshake's
    339	// dataflow into the elected session, one premise per argument.
    401	#[allow(clippy::too_many_arguments)]

Resolution: add `fn connected(self, local: Greeting, remote: Greeting) -> Result<Connected<B, R, W, C, A>, Error<B::Error>>` on `impl<B, R, W, C, A, V> Handshaking<B, R, W, C, A, V>` (the free function never reads `versions`) holding the depth check, `window.resolve`, `run_budget`, and the hand-off; both impls reduce to their exchange plus `self.connected(ours, remote)`. Let the one remaining call-site comment shrink to the placement note ("before the equal-versions shortcut") and leave the full argument on `payload_depth_limits_match`. Replace `Connected::new`'s positional list with a struct literal. If remote-proxy-7 lands, `open` becomes the body of `initiator()`/`responder()` and the chain collapses further; whether the remaining premises get a named bundle is the open question below. Acceptance: one `window.resolve` and one `payload_depth_limits_match` call in start.rs; every surviving `too_many_arguments` allow carries a one-line rationale above the attribute; `start/tests.rs` and `proxy/tests.rs` pass unchanged.

See also: remote-proxy-7, remote-adapter-streams-4.

### remote-proxy-7: The proxy elects the initiator a second time and reconciles it with the driver's election by enum, unreachable!, and debug_assert
- Where: src/tree/mirror/streaming/remote/proxy/start.rs:356-376 (related: src/tree/mirror/streaming/remote/proxy/state.rs:93-110, src/tree/mirror/streaming/remote/proxy/state.rs:118-159, src/tree/mirror/streaming/remote/proxy/state.rs:237-252, src/tree/mirror/streaming/remote/proxy/state.rs:273, src/tree/mirror/streaming/remote/proxy/state.rs:310, src/tree/mirror/streaming/remote/proxy/state.rs:375-378, src/tree/mirror/streaming/remote/proxy/state.rs:406, src/tree/mirror/streaming/remote/proxy/state.rs:431, src/tree/mirror/streaming.rs:197-216, src/tree/mirror/streaming/driver.rs:161-171, src/tree/mirror/streaming/materialized.rs:342-356, src/observe.rs:83-92)
- Class / severity / confidence: simplification / medium / medium
- Provenance: verified (read both election sites and the dispatch: `descend` computes `message::initiates` once and calls `mirror_connected(local, remote)` or `mirror_connected(remote, local)`, whose `mirror!` block calls `initiator()` on the first argument and `responder()` on the second, so which method the driver calls on the proxy already encodes the election; grep shows `Connected::new`/`Connected::equal` have no callers outside start.rs)
- Seen by: structure; refutation: confirmed, with feasibility checked (the `elected` observer contract needs only "after the greetings, before any data stream opens", and streams open lazily inside `execute`); history: no-rationale-found (the enum and both `unreachable!`s arrived in 83edcd944, "WIP: swap over to streaming; DEADLOCK STILL PRESENT", with no message body; the four `debug_assert_eq!`s predate it)
- Owner-gated: no

`connected()` decides equality and the initiator role itself, allocates the whole session on the spot, and stores the result behind `ConnectedState::{Equal, Diverged}`. The driver makes the same two decisions from the same greetings and dispatches `complete_equal`/`initiator`/`responder`; the proxy then checks that the two agree with two `unreachable!`s and four `debug_assert_eq!`s on `session.remote`. The `Connected` doc even says the role is not yet known while the state already holds `remote: Speaker`. The materialized `Connected` is a plain struct that defers everything to the driver's call, which is the shape the protocol traits invite: `Accept::Next` and `CompleteConnect::Next` must implement all three of `CompleteEqual + Initiator + Responder`, so the participant need not know which will be called. Principle: circular justification is the tell; recompute-and-compare asserts on a deterministic function are not defense in depth once one election is the source of truth. The `debug_assert!(self.early.is_none())` at state.rs:375-378 is the same pattern one level down (every constructor below the first stage fixes `early` to `None`), and `Connected::new`/`equal` are `pub` for two callers in a private module.

Evidence:

    356	    if local.version == remote.version {
    357	        return Connected::equal(link.control_read, link.control_write);
    358	    }
    359	    // The role election of record: the smaller exchanged set initiates,
    360	    // canonical version bytes break ties (`message::initiates`).
    361	    let local = if initiates(
    362	        local.set_len,
    363	        &local.version,
    364	        remote.set_len,
    365	        &remote.version,
    366	    ) {

    state.rs:93	/// A proxy after the version exchange but before its elected role is known.
    state.rs:157	            ConnectedState::Equal(..) => unreachable!("descent opened for equal versions"),
    state.rs:249	            ConnectedState::Diverged(..) => unreachable!("equal completion for divergent versions"),
    state.rs:273	        debug_assert_eq!(session.remote, Speaker::Initiator);

    streaming.rs:208	    if message::initiates(local_len, &local_version, remote_len, &remote_version) {
    streaming.rs:209	        mirror_connected(local, remote).await
    driver.rs:161	    mirror! {
    driver.rs:162	        i.initiator;
    driver.rs:163	        r.responder;

Resolution: make `Connected` a struct holding the `Link`, backend, resolved `Window`/`RunBudget`, the remote greeting's `max_version_bytes`/`set_len`/`listing`, stats, codec, and observe handle. `complete_equal` destructures the link and returns the control halves. `Initiator::initiator` and `Responder::responder` fire `observe.elected(...)` and call `open` with `Speaker::Initiator`/`Speaker::Responder` respectively (the remote is the initiator exactly when the driver calls `initiator()` on the proxy), building `Session` there; equal sessions stay allocation-free because `open` runs only on the role paths. Delete `ConnectedState`, `Connected::new`, `Connected::equal`, `diverged()`, both `unreachable!`s, and the four `debug_assert_eq!`s. Acceptance: start.rs contains no call to `initiates` and no version comparison; state.rs contains no `unreachable!` and no `debug_assert_eq!` on `Speaker`; `just gate` clean; the proxy suites pass unchanged, including `equal_versions_return_both_roots` and `tests/observe.rs`'s election assertions.

See also: remote-proxy-4.

### remote-proxy-tests-5: The two-proxy topology is hand-rolled seven times; `harness::drive` already abstracts the axis they vary on
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:56-79 (related: tests.rs:83-111, 123-154, 158-206, 239-301; tests/harness.rs:579-618; tests/containment.rs:28-54; src/testing/transport.rs:109-120)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read all seven bodies; `grep -c 'PayloadCodec::new'`: tests.rs 10, containment.rs 2, harness.rs 2, work/tests.rs 1; `IoPlan::default()` is pass-through, `usize::MAX` chunks with no delays and no fault; the `T` parameter of `reconcile_symmetric_accepts` is instantiated only at `()` at tests.rs:352, 381, 438, 585 and of `_reordered` only at 542; crate-wide `PayloadCodec::new::<..>(PayloadDepthLimit::default())` occurs 67 times)
- Seen by: structure-prose, api-economics (two findings); refutation: confirmed; history: no rationale (each driver arrived by copy; the one constraint on a consolidation's shape is 6a2f271a, which rejected an erased test-link carrier, so a single driver must stay generic over concrete link types)
- Owner-gated: no

`Handshaking::start(Local, Root::<Local>::from(_)).window(..)` twice, `memory_with_capacity`, `RemoteHandshaking::start(Local, link, PayloadCodec::new::<T>(PayloadDepthLimit::default())).window(..)` twice, and `join!(mirror(..), mirror(..))` are written out in `reconcile`, `reconcile_symmetric_accepts`, `reconcile_symmetric_accepts_reordered`, `reconcile_after_preamble`, `reconcile_with_stacked_failures`, `harness::drive`, and `containment::reconcile_results`. They differ only in link wrapping, topology (asymmetric `mirror(a, remote_b) + mirror(remote_a, b)` versus the production `mirror(a, remote_b) + mirror(b, remote_a)`), backend (`Local` versus `Failing<Local>`), payload type, and window. `containment::reconcile_results` is behaviorally `harness::reconcile(a, b, TRANSPORT_CAPACITY, IoPlan::default(), IoPlan::default())` minus the report handles. The `PayloadCodec` argument's addition touched fourteen sites here; a harness exists so that the topology is constructed once, and a generic parameter with a single instantiation is a harness more general than its use. This finding couples with remote-proxy-tests-24: if `drive` gains the production arrangement, the consolidation should carry a topology axis rather than a second driver family.

Evidence:

        56	/// Drive two local starts, each paired directly with its remote protocol start.
        57	async fn reconcile(a: TreeRoot, b: TreeRoot) -> (TreeRoot, TreeRoot) {
        58	    let a = Handshaking::start(Local, Root::<Local>::from(a)).window(WindowConfig::FLOOR);
        59	    let b = Handshaking::start(Local, Root::<Local>::from(b)).window(WindowConfig::FLOOR);
        60	
        61	    let (a_link, b_link) = memory_with_capacity(TRANSPORT_CAPACITY);
        62	    let remote_b = RemoteHandshaking::start(
        63	        Local,
        64	        a_link,
        65	        PayloadCodec::new::<()>(PayloadDepthLimit::default()),
        66	    )
        67	    .window(WindowConfig::FLOOR);
        ...
        75	    let (a, b) = join!(Box::pin(mirror(a, remote_b)), Box::pin(mirror(remote_a, b)));

Resolution: Make `harness::drive` the single constructor, parameterized by backend pair (`Local` or `Failing<Local>`, converting the `Root<Failing<Local>>` result as remote-proxy-tests-7 needs), a two-variant topology, payload type, and window, staying generic over the concrete link types; express the five hub helpers as wrappers that only wrap links and choose topology; delete `containment::reconcile_results` in favor of the harness with the asymmetric topology; drop `T` from `reconcile_symmetric_accepts`/`_reordered` (keep it on `reconcile_after_preamble`, whose `u64` caller exists). The codec incantation then has one site in the partition; a `pub(crate)` convenience constructor on `PayloadCodec` is a crate-wide question outside this partition. Acceptance: `RemoteHandshaking::start` has one call site in the partition; `grep -c 'PayloadCodec::new' src/tree/mirror/streaming/remote/proxy/tests.rs src/tree/mirror/streaming/remote/proxy/tests/*.rs` drops to one or two; the suite passes unchanged.

### clippy-pedantic-2: `&mut` receivers and a `&mut Context` that never mutate
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:174-175 (related: pump.rs:276-277 and 371-372; src/tree/mirror/streaming/materialized/work/levels.rs:342-343, 540-541, 645-646; src/tree/mirror/streaming/remote/proxy/state.rs:75; src/tree/mirror/streaming/backend/local/adversarial.rs:101)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (lint hits in the run log; every body read; `backend()` is `fn backend(&self) -> B` at `remote/proxy/work.rs:128` and `materialized/work.rs:76`, so no read through `self` requires `&mut`)
- Verification: confirmed; history: no-rationale-found (`git log -L` on pump.rs:175 shows the receiver carried through cbfe1aff, 4d55d484, d27cb5aa, d8bef16b without a stated reason)
- Owner-gated: no

Seven methods take `&mut self` and one helper takes `cx: &mut Context<'_>`,
but every body builds its result from copies and clones (`self.progress`,
`self.backend()`, `self.peer_version_bytes`, `self.peer_supplies.clone()`,
`self.codec`, `self.stats.clone()`, `self.window.capacity(..)`,
`cx.waker().wake_by_ref()`). The signature is the reader's contract for what
a call may change; `&mut` here sends the maintainer looking for a mutation
that is not there.

Evidence:

    pump.rs
    174    fn decode_pump(
    175        &mut self,
    ...
    183        let progress = self.progress;
    184        let backend = self.backend();
    185        let version_bytes = self.peer_version_bytes;
    186        let ledger = self.peer_supplies.clone();
    187        let codec = self.codec;

    state.rs
    75    fn outgoing<H: Height>(&mut self) -> StreamSender<C> {

    adversarial.rs
    101 fn delay(delay: &mut Option<u8>, role: Role, cx: &mut Context<'_>) -> bool {

Resolution: `&self` at pump.rs:175, 277, 372; levels.rs:343, 541, 646;
state.rs:75. At adversarial.rs:101 `cx: &Context<'_>` compiles
(`Context::waker` takes `&self`), but `&mut Context<'_>` is the universal
poll-adjacent convention; an owner taste call, see Open questions. This lint
is nursery and has false positives around closure and async captures, so
apply site by site and compile; a site that fails as `&self` is a lint false
positive to leave in place. Acceptance: each site is `&self` or carries a
one-line note naming the capture that needs `&mut`; `just gate` clean.

### remote-proxy-12: execute fuses the biased select to the failure-attribution table
- Where: src/tree/mirror/streaming/remote/proxy/work.rs:197-268 (related: src/tree/mirror/streaming/remote/proxy/work.rs:159-196, src/tree/mirror/streaming/remote/proxy/work/tests.rs:41-77, src/tree/mirror/streaming/remote/proxy/work/tests.rs:123-330)
- Class / severity / confidence: modularity / low / high
- Provenance: assessed (read; work/tests.rs's four attribution tests all drive the table through `parked_session()`)
- Seen by: structure; refutation: confirmed (extraction is behavior-preserving; the flush poll must still precede attribution since `errors` is read after it); history: no-rationale-found (the table grew in place across 54420d7f6, 3903532f6, d2b2403c)
- Owner-gated: no

`execute` runs the select, does a conditional flush poll via a two-arm match whose first arm is empty, then applies a five-arm attribution over `(outcome, errors, remote)`. The table is a pure decision on values already in hand, but it can only be exercised through a parked-session harness because it is fused to the select, and its 38-line doc has to explain both the poll order and the attribution rule. Separating "which future won" from "what the session reports" gives each half a doc that fits its mechanism and makes the table directly testable as a function of its inputs. Principle: legibility of the trickiest code in the partition.

Evidence:

    221	            match &outcome {
    225	                Ok(_) | Err(Error::Accept(_)) => {}
    226	                Err(_) => {
    231	                    let _ = futures::poll!(accept.as_mut());
    232	                }
    233	            }

    257	            Err(error) => match errors.queued_supply_closed() {
    258	                Some(supply) => Err(Error::Stream(supply)),
    259	                None => match errors.take_supply_failure() {

Resolution: extract `fn attribute<E>(outcome: Result<O, Error<E>>, errors: &mut FirstStreamError, remote: Speaker) -> Result<O, Error<E>>` holding lines 236-267 and the attribution paragraphs of the doc; `execute` keeps the select, replaces the empty-arm match with `if !matches!(outcome, Ok(_) | Err(Error::Accept(_))) { let _ = futures::poll!(accept.as_mut()); }`, and calls `attribute`. Optionally add a table-driven unit test of `attribute` beside the session-level witnesses. Acceptance: `execute`'s body is the select, the flush, and one call; the four attribution tests in work/tests.rs pass unchanged.

### remote-proxy-23: The erase-and-box of a typed request stream is spelled ten times across the proxy and the walk, under two aliases for one type
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:98-99 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:147-148, src/tree/mirror/streaming/remote/proxy/work/pump.rs:255-256, src/tree/mirror/streaming/remote/proxy/work/pump.rs:316-317, src/tree/mirror/streaming/remote/proxy/work/pump.rs:347-348, src/tree/mirror/streaming/remote/proxy/work/encode.rs:38, src/tree/mirror/streaming/materialized/work/levels.rs:45, src/tree/mirror/streaming/materialized/work/levels.rs:193, src/tree/mirror/streaming/materialized/work/levels.rs:318, src/tree/mirror/streaming/materialized/work/levels.rs:532, src/tree/mirror/streaming/materialized/work/levels.rs:638, src/tree/mirror/streaming/materialized.rs:831)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn erase_reply src` excluding erased.rs lists exactly these ten call sites; both alias definitions read: `Pin<Box<dyn Stream<Item = Reply<E>> + Send>>` and `BoxStream<'static, Reply<E>>` are the same type)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (the two aliases were born the same day in parallel erasure commits d8bef16b9 and bf1a5b4bc)
- Owner-gated: no

`Box::pin(requests.map(erased::erase_reply::<B, H>))` appears five times here and five times in the walk, and the alias for its type is defined twice. The typed-to-erased boundary is the one place the module docs say erasure happens; naming that step once beside `erase_reply` states it where the vocabulary lives. The walk sites belong to another partition's reviewer, but the helper should be shared.

Evidence:

    98	        let requests: encode::Replies<B::Erased> =
    99	            Box::pin(requests.map(erased::erase_reply::<B, UnderRoot>));

    encode.rs:38	pub type Replies<E> = Pin<Box<dyn Stream<Item = Reply<E>> + Send>>;
    levels.rs:45	type Replies<E> = BoxStream<'static, Reply<E>>;

Resolution: add `pub(crate) fn erase_requests<B, H>(requests: impl Requests<B, H>) -> BoxStream<'static, Reply<B::Erased>>` to erased.rs next to `erase_reply`, with one `pub(crate)` alias there; replace the ten call sites and delete the two local aliases. Acceptance: `grep -rn 'map(erased::erase_reply' src` returns nothing outside erased.rs; one alias for the erased request stream.

### remote-proxy-24: Three decode pumps re-thread the same ingress context and repeat the same six-argument decode call four times
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:183-187 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:198-206, src/tree/mirror/streaming/remote/proxy/work/pump.rs:227-235, src/tree/mirror/streaming/remote/proxy/work/pump.rs:283-298, src/tree/mirror/streaming/remote/proxy/work/pump.rs:377-392, src/tree/mirror/streaming/remote/proxy/work/pump.rs:417-430, src/tree/mirror/streaming/remote/proxy/work/pump.rs:493-500, src/tree/mirror/streaming/remote/proxy/work/pump.rs:104-110, src/tree/mirror/streaming/remote/proxy/work/pump.rs:152-158, src/tree/mirror/streaming/remote/proxy/work/pump.rs:260-266, src/tree/mirror/streaming/remote/proxy/work/pump.rs:319-325, src/tree/mirror/streaming/remote/proxy/work/pump.rs:352-358, src/tree/mirror/streaming/remote/adapter/decode.rs:203-209, src/tree/mirror/streaming/remote/adapter/decode.rs:289)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the three identical five-line preludes and the four identical decode argument bundles; read decode.rs: `decode_reply`/`decode_leaf_reply` take `ledger: SupplyLedger` by value and immediately borrow it at line 289, while `early_supplies` returns a `'static` boxed stream and does need an owned ledger)
- Seen by: structure, perfapi (the ledger clone); refutation: confirmed both; history: no-rationale-found (pure accretion across 165b0dd3, 08f2899b, d8bef16b9, 4356e197)
- Owner-gated: no

`decode_pump`, `leaf_decode_pump`, and `terminal_decode_pump` each open with the identical capture of `progress`, `backend`, `version_bytes`, `ledger`, `codec`, then call the adapter with the identical bundle; `Early` holds the same fields again and re-threads them into `early_supplies`; the encode spawn sites repeat `self.backend(), self.budget, ..., self.progress` five times. The bundle (backend, declared version bound, supply ledger, payload codec) is one concept, the session's ingress premises, spelled as loose values at eleven sites. A minor cost rides along: each decode iteration clones `backend` and `ledger` (an `Arc` refcount round trip each) to hand owned values to adapter entries that only borrow the ledger; the adapter signature is another partition's, the pump follows.

Evidence:

    183	        let progress = self.progress;
    184	        let backend = self.backend();
    185	        let version_bytes = self.peer_version_bytes;
    186	        let ledger = self.peer_supplies.clone();
    187	        let codec = self.codec;

    290	                let Decoded { reply, questions } = decode_leaf_reply(
    291	                    backend.clone(),
    292	                    version_bytes,
    293	                    ledger.clone(),
    294	                    scope,
    295	                    &mut incoming,
    296	                    codec,
    297	                )

Resolution: introduce a `Clone` struct in `work/` (say `Ingress<B> { backend: B, version_bytes: u64, ledger: SupplyLedger, codec: PayloadCodec }`) with `async fn decode(&self, scope, &mut incoming)` and `decode_leaf` wrappers over the adapter calls; each pump captures one `Ingress` and the `Progress`; `Early` holds an `Ingress`. Whether the adapter's entries take the same struct, and whether they take `&SupplyLedger`, is the adapter reviewer's call. Acceptance: each decode pump's prelude is two lets; the six-argument decode call appears once per adapter entry; `just clippy` clean; proxy suites pass.

See also: remote-adapter-streams-4, remote-adapter-tests-2.

### remote-proxy-25: decode_pump's early-supply branch duplicates the decode call and the yield, exits through `continue`, and re-asserts what the adapter guarantees
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:192-240 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:207, src/tree/mirror/streaming/remote/adapter/decode.rs:220-225, src/tree/mirror/streaming/remote/adapter/scope.rs:33-37)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read; the refutation pass traced that for a request scope (empty listing, `is_request`) any `Query` reaction fails `read_reply` with `ScopeError::UnpositionedQuery` via `scope.next()` on an empty listing, so `asked` is provably empty on the `Ok` path)
- Seen by: structure; refutation: confirmed, adding that the `debug_assert!` at 207 recomputes an adapter guarantee; history: no-rationale-found (55d76d5cf's shape verbatim)
- Owner-gated: no

Inside the loop, the root-request branch and the general path both call `decode_reply` with the same arguments and both end in `yield_reply_scopes!`; the branch differs only in supplementing `reply.replies` with the exploded early node and forcing zero scopes. The `continue` and the second macro invocation (with `Vec::<Scope>::new()`) make the loop body fifty lines. The forced zero is already implied: a request scope's decode cannot return questions, which is also why `debug_assert!(asked.is_empty())` is a recompute of a deterministic guarantee rather than a guard. The invariant ("a root-level request's reply is the wire reply plus the early node's children") is easier to see as one decode followed by an optional supplement.

Evidence:

    207	                    debug_assert!(asked.is_empty(), "an empty request opens no lower scope");
    220	                    yield_reply_scopes!(
    221	                        progress, height, 0;
    222	                        yield Reply { replies };
    223	                        next_scopes => Vec::<Scope>::new();
    224	                    );
    225	                    continue;
    226	                }

Resolution: before decoding, compute `let early_key = (early.armed() && scope.is_request()).then(|| scope.parent().pop());`. Decode once. If `early_key` is `Some((root, radix))`, run `advance_to` and extend `reply.replies`. Then one `yield_reply_scopes!` with `questions.len()` and `next_scopes => questions` (empty in the root-request case, so behavior is unchanged); drop the `debug_assert!`. Acceptance: one `decode_reply` call and one `yield_reply_scopes!` in `decode_pump`; no `continue`; the early-supply fixtures in `proxy/tests/{greeting,declarations,malformed}.rs` pass unchanged.

### remote-proxy-28: leaf_decode_pump and terminal_decode_pump are one loop; the encode side already unifies the same pair with an Option
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:371-401 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:276-307, src/tree/mirror/streaming/remote/proxy/work/encode.rs:41-71, src/tree/mirror/streaming/remote/proxy.rs:33-35)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read both bodies; `yield_reply_scopes!` takes `$scopes` as a bare expression used as `&$scopes`, so the unification needs an `if let Some(..)` around the publish, which the encode side already shows)
- Seen by: structure; refutation: confirmed; history: no-rationale-found (both extracted by d8bef16b9; the encode side's `Option` shape is the original design from cbfe1aff3). The scope-A mutants note records that the terminal stage runs empty in every committed suite, so "proxy tests pass" is weak acceptance at height zero.
- Owner-gated: no

`terminal_decode_pump` is `leaf_decode_pump` with the derived questions rejected as `TerminalQuery` instead of published. `encode::terminal` handles exactly this fork for the outgoing direction with `questions: Option<Sender<Scope>>`, so the decode side carries a second function where the encode side carries one branch. Two spellings of one loop drift independently. If remote-proxy-29 deletes the `TerminalQuery` arm, the unification is a `None` sink and nothing else.

Evidence:

    393	                if !questions.is_empty() {
    394	                    Err(Error::TerminalQuery)?;
    395	                }
    396	                progress.decoded_reply(Z::HEIGHT, 0);
    397	                yield reply;

    encode.rs:64	        if let Some(questions) = &questions {
    encode.rs:65	            publish(questions, batch, progress, Z::HEIGHT).await;
    encode.rs:66	        } else if !batch.is_empty() {
    encode.rs:67	            return Err(Error::TerminalQuery);

Resolution: give `leaf_decode_pump` a `next_scopes: Option<Sender<Scope>>` parameter; when `None` and `questions` is nonempty, fail with `TerminalQuery` (or, after remote-proxy-29, nothing), else record and yield; `complete_responder` passes `None`, `leaf_replies` passes `Some(next_scopes)`. Delete `terminal_decode_pump`. Acceptance: one leaf-height decode pump in pump.rs; `instrumented_channels_cover_every_proxy_edge` and the `Trace` assertions still pass.

See also: remote-proxy-29.

### remote-proxy-29: The responder-terminal TerminalQuery check is unreachable from wire bytes
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:393-395 (related: src/tree/mirror/streaming/remote/codec/signal.rs:99, src/tree/mirror/streaming/remote/codec/signal.rs:338-340, src/tree/mirror/streaming/remote/proxy/error.rs:78-80, src/tree/mirror/streaming/remote/proxy/work/encode.rs:66-68, src/tree/mirror/streaming/remote/adapter/decode.rs:178-185, src/tree/mirror/streaming/remote/proxy/state.rs:431-433)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the chain: `complete_responder` binds `incoming::<Z>()` for `remote = Responder`; `Stream::at_height(Responder, 0)` is `MAX`; `class((Responder, MAX))` is `TerminalLeafReplies`; its placement grammar admits only `Signal::Supply(Flow::End) | Signal::End(_)`, so both query forms are rejected as `InvalidSignalPlacement` before the adapter sees a frame and `decode_leaf_reply`'s `questions` is always empty here; grep shows no test reaches the arm)
- Seen by: correctness; refutation: confirmed; history: no-rationale-found (born redundant: the grammar commit 3dac1b852 is an ancestor of the proxy landing cbfe1aff3)
- Owner-gated: no

The arm duplicates a check the codec makes one layer down, and `Error::TerminalQuery`'s doc ("The terminal responder attempted to ask another leaf question") describes a peer behavior the grammar already excludes; a peer that did it would receive `InvalidSignalPlacement`, not this. It is reachable only by in-process frame construction, the situation `read_early` documents at decode.rs:178-181. Doctrine: a guard is justified by naming a concrete, constructible failure the existing instruments miss.

Evidence:

    393	                if !questions.is_empty() {
    394	                    Err(Error::TerminalQuery)?;
    395	                }

    signal.rs:99	            (Speaker::Responder, Self::MAX) => StreamClass::TerminalLeafReplies,
    signal.rs:338	            StreamClass::TerminalLeafReplies => {
    signal.rs:339	                matches!(self.signal, Signal::Supply(Flow::End) | Signal::End(_))
    signal.rs:340	            }
    error.rs:78	    /// The terminal responder attempted to ask another leaf question.

Resolution: delete the arm and let the codec's placement rejection be the diagnosis; the variant stays for the local encode-side use at encode.rs:67, and its doc then names the local producer (remote-proxy-3). Alternatively keep it with a comment in the style of decode.rs:178-181 stating the grammar excludes the frame and the arm exists for in-process construction only, plus a `pump/tests.rs` test that constructs the frame. Acceptance: either the arm is gone and `Error::TerminalQuery`'s doc names only the local producer, or the arm carries the in-process-only rationale and a test fires it.

See also: remote-proxy-28.

### remote-proxy-30: Early encodes a four-state cursor as three Options and a bool
- Where: src/tree/mirror/streaming/remote/proxy/work/pump.rs:413-460 (related: src/tree/mirror/streaming/remote/proxy/work/pump.rs:464-522, .agent-notes/2026-08-21-cbor-wire-mutants-scopeA/03-proxy-adapter.md)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed; history: no-rationale-found for the representation (born in 55d76d5cf); the scope-A mutants note already designs a module split for `Early` with a pairing property test, not yet landed
- Owner-gated: no

`Early` has `receiver: Option`, `supplies: Option`, `lookahead: Option`, `exhausted: bool`: sixteen nominal combinations of which four are reachable (unarmed; armed but unclaimed; streaming with an optional lookahead; exhausted). `armed()` is a three-way disjunction, `advance_to` interleaves the state transitions with the radix pairing, and the `.expect("an unarmed cursor resolves no request")` exists to rule out a combination the representation admits (it is discharged today by the `armed()` guard at the single call site). Types-first: make the invalid states unrepresentable so the pairing logic reads without a mental table.

Evidence:

    423	    receiver: Option<StreamReceiver<Rx>>,
    424	    supplies:
    425	        Option<Pin<Box<dyn Stream<Item = Result<(u8, B::Erased), DecodeError<B::Error>>> + Send>>>,
    426	    lookahead: Option<(u8, B::Erased)>,
    427	    exhausted: bool,
    458	    fn armed(&self) -> bool {
    459	        self.receiver.is_some() || self.supplies.is_some() || self.lookahead.is_some()
    460	    }
    489	                    let receiver = self
    490	                        .receiver
    491	                        .take()
    492	                        .expect("an unarmed cursor resolves no request");

Resolution: `enum EarlyCursor<B, Rx> { Unarmed, Armed(StreamReceiver<Rx>), Streaming { supplies: BoxStream<..>, lookahead: Option<(u8, B::Erased)> }, Exhausted }`; `armed()` is `!matches!(self, Unarmed)`; `advance_to` matches on the state; `finish` is `Streaming { lookahead: Some(_) } => Err`, `Streaming { .. } => poll once`, else `Ok`. Land it together with the scope-A note's module split and pairing property (remote-proxy-31). Acceptance: `Early` has one state field; no `.expect` in its impl; the `UnaskedReply` path pinned in `proxy/tests/malformed.rs:180` still fires.

### remote-proxy-tests-4: Small helpers, a constant, and the n-versus-m fixture are each defined two or more times across sibling files
- Where: src/tree/mirror/streaming/remote/proxy/tests.rs:53-54 (related: tests/harness.rs:35-36, 521-534; tests/greeting.rs:84-105, 117-118, 189-194; tests/malformed.rs:135-139; tests/failures.rs:56-68, 103-113, 296-300; tests/transport.rs:10-20, 26-35; tests/declarations.rs:48-57, 71-80, 275-287; tests.rs:331-334, 347-350, 361-366, 593-596; work/tests.rs:238-249, 305-316; src/tree/mirror/streaming/tests/stats.rs:72-75)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read every cited body; `grep 'pub fn' src/tree/arb.rs` lists no pair builder; `tree/mirror/streaming/tests/stats.rs:72-75` computes the same election through `StreamingRoot<Local>::len()`)
- Seen by: structure-prose (three findings), blind-spots, api-economics; refutation: confirmed; history: no rationale (each copy arrived by copy; `left_initiates` landed at 3e5b95df with a doc arguing for one predicate and did not retarget greeting.rs)
- Owner-gated: no

`TRANSPORT_CAPACITY = 37` is defined with an identical doc in tests.rs and harness.rs (containment.rs imports the hub's copy, showing the intended shape). `greeting::order_by_election` re-derives `harness::left_initiates` including the hand-rolled `len` closure, and malformed.rs and failures.rs inline the same ordering on top of `left_initiates`; `stats.rs` computes the count through `StreamingRoot<Local>::len()`, so the closure is a reimplementation of an existing method. `transport::plan` and `failures::plan` are two undocumented `IoPlan` builders differing in parameter order. The parked `StreamReceiver` fixture is built twice in work/tests.rs. The fixture "n unit messages on `nth_party(0)` against m on `nth_party(1)`" is built by hand in `uneven_pair` (1 vs 4), `batched_uneven_pair` (1 vs FAN+1), `opening_bulk_pair` (4 vs 8), `failures::stacked_pair` (8 vs 8), inline in transport.rs (8 vs 8), four times in tests.rs (1 vs 1), and twice in greeting.rs; tests.rs:361-362 is the one site seeding parties with `before::Party::seed()`/`fork()` instead of `nth_party`. Tests that repeat setup a helper should own; every other helper in the partition carries a doc comment and the two `plan` functions do not.

Evidence:

    tests.rs:
        53	/// Bytes buffered by each per-stream pipe before backpressure applies.
        54	const TRANSPORT_CAPACITY: usize = 37;

    harness.rs:
        35	/// Bytes buffered by each per-stream pipe before backpressure applies.
        36	const TRANSPORT_CAPACITY: usize = 37;

    greeting.rs:
        93	    let len = |root: &crate::tree::Root| {
        94	        root.root
        95	            .as_ref()
        96	            .map(|node| node.len() as u64)
        97	            .unwrap_or_default()
        98	    };
        99	    if crate::tree::mirror::streaming::message::initiates(len(&a), &a.ceiling, len(&b), &b.ceiling)

Resolution: In harness.rs, single definitions: `TRANSPORT_CAPACITY` (imported by tests.rs), `order_by_election(a, b) -> (initiator, responder)` built on `left_initiates` (adopting `order_by_election`'s equal-ceiling assertion), `left_initiates` computing its count via `StreamingRoot::<Local>::from(root.clone()).len()`, one documented `plan(..)` builder, and `disjoint_pair(left: usize, right: usize) -> (TreeRoot, TreeRoot)` (or place the pair builder in `tree::arb` beside `nth_party`, since the sibling suites build the same shapes); rewrite the named fixtures as one-line wrappers keeping their docs; in work/tests.rs, a `parked_reporter(route, stream)` helper. Acceptance: `grep -rn 'fn hash_of\|fn union_hash\|fn plan(\|const TRANSPORT_CAPACITY\|fn order_by_election' src/tree/mirror/streaming/remote/proxy` returns one hit per name; `Action::Insert(Message::new(()))` appears in the partition only inside the builder and in fixtures needing a non-uniform shape.

### remote-proxy-tests-16: The endpoint-to-`MirrorError`-side projection is written seven times, with two polarities
- Where: src/tree/mirror/streaming/remote/proxy/tests/declarations.rs:109-125 (related: declarations.rs:100-104, 153-157, 188-198, 236-240, 245-261, 308-312, 317-333, 382-386; tests/failures.rs:128-149, 353-356, 380-383; tests/malformed.rs:42-59; tests.rs:469-483; tests/harness.rs:610-616)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read all sites; `endpoint_error`'s flag names the reporting side while `receiving_error`'s names the corrupt side)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale (both helpers were born in 77674c9c; the four inline copies were added by copy; the polarity split has no recorded reason)
- Owner-gated: no

Selecting the proxy error from a `(left, right)` result pair (left's proxy is `MirrorError::Server`, right's is `MirrorError::Client`, a consequence of `harness::drive`'s arrangement) appears inline in four declarations tests and in tests.rs, and as two differently named helpers, `failures::endpoint_error` and `malformed::receiving_error`, whose flags mean opposite things. The five `((left, right), hears) = if receiver_left {..}` arrangements in declarations.rs are the same shape again. The Server/Client-by-position rule is a fact about the harness topology and belongs beside `drive`, stated once; every copy is a place a topology change (remote-proxy-tests-24) breaks independently.

Evidence:

       109	        let receiver_error = if receiver_left {
       110	            match &left {
       111	                Err(MirrorError::Server(error)) => error,
       112	                other => panic!(
       113	                    "undetected target_message_size lie: the left proxy did not \
       114	                     report the violation: {other:?}"
       115	                ),
       116	            }
       117	        } else {
       118	            match &right {
       119	                Err(MirrorError::Client(error)) => error,
       120	                other => panic!(
       121	                    "undetected target_message_size lie: the right proxy did not \
       122	                     report the violation: {other:?}"
       123	                ),
       124	            }
       125	        };

    malformed.rs:
        42	/// Borrow the remote error detected opposite the corrupt writer.
        43	fn receiving_error<'a>(
        44	    corrupt_left: bool,

Resolution: Add to harness.rs a `proxy_error(side: IoSide, left: &Result<_, LeftError>, right: &Result<_, RightError>) -> &RemoteError<Infallible>` (panicking with the position and a caller-supplied context) plus a `Result<_, TestCaseError>` variant for proptest bodies, and an `arrange(receiver_left, receiver, sender, rewrite)` helper for the declaration tests; replace the seven sites, passing the reporting side everywhere so the flag means one thing. Acceptance: no `Err(MirrorError::Server(error)) => error` or `Err(MirrorError::Client(error)) => error` arm remains outside harness.rs.

### Nits (9)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| async-hazards-6 | `src/tree/mirror/streaming/remote/proxy/work.rs:132-135` | `Work::spawn` names a task-bag push in a crate that promises no spawning | Rename `Work::spawn` to `enqueue` or `add_task` and reword the encode.rs:8 comment to match |
| clippy-pedantic-12 | `src/tree/mirror/streaming/remote/proxy/work/pump.rs:486-493` | `get_or_insert` in the arm that already knows the slot is empty | `self.supplies.insert(Box::pin(early_supplies::<B, _>(..)))` |
| remote-proxy-5 | `src/tree/mirror/streaming/remote/proxy/start.rs:173-175` | Em-dashes in `//` comments at four production sites | replace with `--`, a colon, or a semicolon (start.rs:173 and 220 collapse to one site under remote-proxy-4) |
| remote-proxy-11 | `src/tree/mirror/streaming/remote/proxy/work.rs:8-9` | Idiom nits: a stray import group, qualified paths beside existing imports, a redundant `pin!`, and parameter rebinding | fold `PayloadCodec` into each file's `use crate::{...}` block |
| remote-proxy-14 | `src/tree/mirror/streaming/remote/proxy/work.rs:271-272` | `mod pump` and `fn pump` name two different things in one file | rename the module (`stages`) since the walk's relay shares the function name, and align the module docs' vocabulary |
| remote-proxy-21 | `src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:92-98` | Trace causality check mixes a named height with literal heights | import `Z` and `S` beside `UnderRoot` and write `Z::HEIGHT` / `<S<Z>>::HEIGHT` |
| remote-proxy-tests-3 | `src/tree/mirror/streaming/remote/proxy/tests.rs:3-8` | Import hygiene: a crate import ahead of the std group in four files, a split `tokio::io` group, and qualified paths where aliases exist | Move the `crate::message` import into each file's crate group |
| remote-proxy-tests-17 | `src/tree/mirror/streaming/remote/proxy/tests/failures.rs:194-197` | Unnamed capacities and counts: `17` four times, `64 * 1024`, `31` | Use `TRANSPORT_CAPACITY` in failures.rs (17 carries no intent), or name the value with a one-line doc |
| remote-proxy-tests-23 | `src/tree/mirror/streaming/remote/proxy/tests/harness.rs:456-456` | `capacity.max(1)` in `harness::reconcile` silences an assert for an input no caller passes | Pass `capacity` through unchanged |

## Test scaffolding

Files: src/testing.rs, src/testing/, src/tests.rs. Entries: 10 (2 medium, 5 low, 3 nit).

### testing-infra-2: `window_tradeoff_table` re-derives window internals in the facade: a second `DESIGN_SESSION_MESSAGES`, a hard-coded `KEY_DEPTH`, and a reimplemented `Window::widest`
- Where: src/testing.rs:207-235 (related: src/testing.rs:323-333; src/tree/mirror/streaming/window.rs:137, 243, 485-491; src/tree/mirror/streaming/window/tests.rs:15-24, 226, 257-264; tests/tradeoff_probe.rs:55-58; examples/window_tradeoff.rs:11)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -rn '62_500\|62,500' src tests examples benches` lists three definitions and two prose quotes; window.rs:137 `const KEY_DEPTH: usize = 32;` is private; window.rs:485-491 `widest` read)
- Seen by: structure-prose [5], [6]; blind-spots [22]; api-economics [36]; refutation: confirmed; history: deliberate-but-expired (`solve_window` landed hours before `Window::widest` in 487e17ea3 and was never retargeted; no placement rationale recorded for the renderer)
- Owner-gated: no

The design-session scale `62_500` is defined independently at testing.rs:218, window/tests.rs:24, and tests/tradeoff_probe.rs:58 and quoted in prose at window.rs:243, and nothing compares the copies, so the table this constant drives and the `SCOPE_ENVELOPE_BYTES` pin can describe different sessions without a test noticing. `solve_window` recomputes `Window::widest()` as `(0..=32).map(capacity).max()`, and `0..=32` hard-codes the private `KEY_DEPTH` here and again at line 332. The renderer's constants, pin, and committed output all live under the window module; the facade's responsibility is visibility. Principle 5: a number that matters lives in one mechanically enforced place that prose cites by name.

Evidence:

    215	    /// The design session's corpus scale per side: a round 62,500,
    216	    /// sized near the spec BDP in design-size records and stated
    217	    /// round, the scale `SCOPE_ENVELOPE_BYTES` is pinned at.
    218	    const DESIGN_SESSION_MESSAGES: u64 = 62_500;
    ...
    231	        (0..=32)
    232	            .map(|height| window.capacity(height) as u64)
    233	            .max()
    234	            .expect("thirty-three heights")

    24	const DESIGN_SESSION_MESSAGES: u64 = 62_500;          (window/tests.rs)
    58	const DIVERGENT: usize = 62_500;                       (tests/tradeoff_probe.rs)

Resolution: In window.rs, declare `pub(crate) const DESIGN_SESSION_MESSAGES: u64 = 62_500;` once beside `SCOPE_ENVELOPE_BYTES` under the same `cfg(any(test, feature = "test-internals"))`, make `KEY_DEPTH` `pub(crate)` (or add a `Window::capacities()` accessor), and move the renderer body to the window module as `pub(crate) fn tradeoff_table() -> String`, with `solve_window` becoming `Window::from_budget(...).widest()`. Leave `testing::window_tradeoff_table` as a one-line delegation for the example and the pin; expose the constant through `testing` for `tradeoff_probe`'s `DIVERGENT`; let window.rs:243 cite the constant by name. Acceptance: `grep -rn '62_500' src tests` returns one definition; `grep -n '0\.\.=32' src/testing.rs` is empty; `tradeoff_table_matches_the_derivation` still passes against today's `tradeoff.md` byte for byte.

### testing-infra-20: `Fuse`/`FusedConnector`/`fused_link` reimplement `wrap_link`'s byte-counted write fault
- Where: src/tests.rs:209-337 (related: src/tests.rs:345-360; src/testing/transport.rs:51-74, 186-216, 247-263, 360-375, 498-537, 587; src/peer/gossip.rs:450-455; src/link.rs:598-604; tests/lifecycle.rs:193-201; tests/common/fault.rs:1-27, 243-251)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (both mechanisms traced by reading: `Fuse::poll_write` admits `min(buf.len(), remaining)` then fails at zero (233-250); `AdversarialWrite::poll_write` clamps to `remaining_bytes(Write)` and `failure(Write)` fires once `write_bytes >= after` (360-375, 194-207); both share one budget across the control write and every connected stream (316-337 against 509-535); with a Write fault, `failure(Read | Accept | Connect)` returns `None` at 188, so reads and supply are unaffected in both. Retire classifies by `(intent, result)` alone (gossip.rs:450-455); `grep -rn 'ErrorKind::\|\.kind()' src` outside tests finds kind matches only on `UnexpectedEof` (party.rs:118-119, 154; codec decode; routed endpoint), so `BrokenPipe` versus `io::Error::other(InjectedIo)` is assertion-neutral)
- Seen by: structure-prose [1]; blind-spots [20]; api-economics [31]; refutation: confirmed ([31]'s latent `Done` claim tempered: `fused_link` takes `MemoryLink`, whose connector returns `Done::discard()`, link.rs:604); history: no rationale found (b3b877d9b introduced `wrap_link` and rewrote `Fuse` into the shared-budget shape in the same commit without saying why it stayed; 4be4b830 then re-plumbed `Fuse` for `Done`, the cascade this predicts)
- Owner-gated: no

About 130 lines implement a write-side byte fuse over a whole link that `testing::wrap_link(side, IoPlan { fault: Some(IoFault { operation: Write, after: budget, unit: Bytes }), ..IoPlan::default() }, link)` already provides, with an `IoReportHandle` besides; `tests/common/fault.rs` is a third copy. The public tier already severs sessions through `wrap_link` (tests/lifecycle.rs:193-201) while the in-crate tier hand-rolls it. Principle 3: the harness exists so suites do not each own a fault injector; three fuses is three places a change to `Link` or `Done` must land.

Evidence:

    211	/// An [`AsyncWrite`] wrapper that forwards writes until a byte budget is
    212	/// exhausted, then fails every write with [`BrokenPipe`]: a deterministic
    213	/// stand-in for a connection severed at a chosen point in the session.
    214	///
    215	/// The budget is shared across every fused writer of one link — the control
    216	/// half and each data stream — so the cut lands at a chosen point in the
    217	/// session's total outgoing byte count, wherever that byte travels. Reads
    ...
    316	/// Fuse one link's whole outgoing side to a shared byte budget.
    317	fn fused_link(

Resolution: Delete `Fuse`, `FusedConnector`, and `fused_link`; in `severed_retire` build `let plan = IoPlan { fault: Some(IoFault { operation: IoOperation::Write, after: budget, unit: IoFaultUnit::Bytes }), ..IoPlan::default() }; let (mut a_link, report) = wrap_link(IoSide::Left, plan, a_link);` and optionally assert `report.snapshot().injected.is_some()` to prove the cut fired. Drop the `AsyncWrite`, `Pin`, `Context`, `Poll`, `Connector`, `MemoryConnector`, `MemoryAcceptor`, `Link` imports that only the fuse needed. `tests/common/fault.rs` belongs to another partition; its header states the one capability `IoPlan` lacks (a severed direction also refuses connect/accept), which is the gap to close before it can follow. Acceptance: `src/tests.rs` contains no `impl AsyncWrite` and no `Connector` impl; the four `severed_*` tests pass with their assertions untouched; `grep -rn 'struct Fuse' src tests` returns at most the `tests/common/fault.rs` definition.

See also: tests-common-3.

### module-graph-7: `testing.rs` mixes a forwarding facade, a table renderer, and a deadlock detector, with imports mid-file
- Where: src/testing.rs:194-291 (related: src/testing.rs:1, :335-343, :345-394, :396-442; src/lib.rs:319-321)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (file read in full)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The `test-internals` facade holds three responsibilities: one-line forwarders to crate-internal meters (each naming the suite that consumes it), a 100-line markdown renderer that `examples/window_tradeoff.rs` prints and the window suite byte-compares, and the `run_to_quiescence` detector with its own two tests, with a `use std::{...}` block after 330 lines of items.

Evidence:

         1	//! Executor-agnostic test support shared across protocol and API suites.
    ...
       207	pub fn window_tradeoff_table() -> String {
    ...
       263	        "<!-- Generated by `just window-tradeoff`; do not edit. -->"
    ...
       335	use std::{
       336	    future::Future,
       337	    pin::pin,
    ...
       375	pub fn run_to_quiescence<F: Future>(future: F) -> Result<F::Output, Quiescence> {

Resolution: Split into `testing/quiescence.rs` (`Quiescence`, `WakeFlag`, `run_to_quiescence`, its two tests in a sibling `tests.rs`) and either `testing/tradeoff.rs` or `window_tradeoff_table` moved into `window.rs` under `#[cfg(any(test, feature = "test-internals"))]` beside `Window::from_budget`; keep `testing.rs` as the facade with `pub use` of both; hoist the imports. The module is `#[doc(hidden)]` and feature-gated, so no API moves. Acceptance: `testing.rs` contains only `mod`/`pub use` lines and forwarders; every `use` precedes every item.

### testing-infra-5: Inline `mod tests {}` blocks against the sibling-file convention
- Where: src/testing.rs:396-397 (related: src/testing/transport.rs:801-807, 827; AGENTS.md:72)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rln '^mod tests {' src/` returns five files, two of them in this partition)
- Seen by: structure-prose [8]; api-economics [34]; refutation: confirmed; history: both blocks predate the convention (dfd19c447, 2026-07-24), which stated no exemption
- Owner-gated: no

Both scaffolding files carry their unit tests inline; AGENTS.md's convention is `mod tests;` with a `tests.rs` sibling so the implementation reads without them. The transport tests also reach for `futures::{pin_mut, poll}` (803, 827) where the same file uses `std::pin::pin!` (694).

Evidence:

    396	#[cfg(test)]
    397	mod tests {

    801	#[cfg(test)]
    802	mod tests {
    803	    use futures::{pin_mut, poll};

Resolution: Move the blocks to `src/testing/tests.rs` and `src/testing/transport/tests.rs`; use `std::pin::pin!` in place of `pin_mut!`. Acceptance: `grep -rln '^mod tests {' src/testing.rs src/testing/transport.rs` is empty; the four moved tests still run.

See also: module-graph-6 (the entry of record for the six inline test modules; this entry is counted for the `pin_mut!` idiom at transport.rs:803 and 827).

### testing-infra-9: `State` spells the fault-arming rule three ways, with match arms no call site reaches
- Where: src/testing/transport.rs:186-216 (related: src/testing/transport.rs:220-231, 234-244, 247-263, 284-292, 304, 371)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (call sites checked: `failure(Operation::Read)` only at 288 under `injected.is_some()`, which `failure` returns on at 191-193; `remaining_bytes` called only with `Read` at 304 and `Write` at 371)
- Seen by: structure-prose [7]; blind-spots [25] (Flush arm); refutation: confirmed; history: deliberate-but-expired (the `(Read, _)` arms were live until b3b877d9b gated the read path; the `Flush` arm in `remaining_bytes` has been unreachable since birth)
- Owner-gated: no

`failure(Operation::Read)` is only ever called when `injected` is already set, and `failure` returns early in exactly that case, so the `(Operation::Read, _)` arms at 195-196 never execute; `read_fault_armed` and `inject_read` restate the arming test and the `InjectedIo` construction that `failure` already holds, and `inject_read` writes `get_or_insert` then `expect("just recorded")` on the same field (242-243). `remaining_bytes`'s `Operation::Flush` arm (257) is reached by no caller. Legibility: finished code should be obviously correct, and three spellings of "fire once the prefix is spent, then keep firing" hide the one deliberate special case (reads fire post-poll on payload).

Evidence:

    194	        let completed = match (operation, fault.unit) {
    195	            (Operation::Read, FaultUnit::Operations) => self.report.reads,
    196	            (Operation::Read, FaultUnit::Bytes) => self.report.read_bytes,

    242	        self.report.injected.get_or_insert(injected);
    243	        io::Error::other(self.report.injected.expect("just recorded"))

    257	            Operation::Flush => self.report.flushes,

Resolution: Split into `fn armed(&self, op) -> bool` and `fn inject(&mut self, op) -> io::Error` (idempotent via `let injected = *self.report.injected.get_or_insert(...)`). Write/flush/connect/accept: `if state.armed(op) { return Err(state.inject(op)) }`; read: `armed` before the unclamped read, `inject` on a payload-bearing result. `remaining_bytes` matches `(Read | Write, Bytes)` and returns `usize::MAX` otherwise. Acceptance: one arming predicate and one injector; every match arm in `State` is reachable from a call site; the two inline transport tests and the proxy `failures.rs` suite pass unchanged.

### testing-infra-11: `wrap_io` is exported with no consumer, `wrap_link` rebuilds its state instead of using it, and `AdversarialLink` cannot be named by callers
- Where: src/testing/transport.rs:451-475 (related: src/testing.rs:7-12; src/testing/transport.rs:489-537, 540-545, 628-632; tests/lifecycle.rs:70-71; tests/reuse.rs:140-141)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn wrap_io src tests benches examples`: the re-export at testing.rs:10 and this file's definition and two inline tests only; `grep -rn AdversarialLink src tests benches examples`: transport.rs:502 and 540 only)
- Seen by: structure-prose [15]; blind-spots [26]; api-economics [40]; refutation: confirmed (severity lowered: the module is `doc(hidden)`, so no rendered docs expose the unreachable name); history: deliberate-but-expired (`wrap_io`'s external callers were migrated to `wrap_link` by b3b877d9b; the export and the "extended over a whole link" framing survived)
- Owner-gated: no

`wrap_io` is re-exported but its only callers are the two unit tests at the bottom of this file; `wrap_link`'s doc frames itself as `wrap_io` "extended over a whole link" (494-497) but duplicates the `State` literal (510-517 against 458-465) instead of building on it, and the `AdversarialRead { inner, state, delay: None }` literal appears three times (467-471, 519-523, 628-632) while only `wrap_write` exists. `AdversarialLink` is `pub type` in the private module and absent from the facade's export list, so `wrap_link`'s return type cannot be written by tests/lifecycle.rs or tests/reuse.rs. Principle 3: a function whose only callers are its own tests is a candidate for dissolution or for becoming the primitive it is documented as.

Evidence:

    452	pub fn wrap_io<R, W>(
    453	    side: Side,
    454	    plan: IoPlan,
    455	    read: R,
    456	    write: W,
    457	) -> (AdversarialRead<R>, AdversarialWrite<W>, IoReportHandle) {
    458	    let state = Arc::new(Mutex::new(State {
    459	        side,
    460	        plan,
    461	        report: IoReport::default(),
    462	        read_step: 0,
    463	        write_step: 0,
    464	        flush_step: 0,
    465	    }));

Resolution: Add `State::new(side, plan)` and a `wrap_read` twin of `wrap_write`; have `wrap_link` and the acceptor use them. Either route `wrap_link`'s control halves through `wrap_io` (sharing the state `Arc` with the connector and acceptor wrappers) or narrow `wrap_io` to `pub(super)` and drop it from the facade, re-pointing its two tests at `wrap_link` over `link::memory()`. Add `AdversarialLink` to the `pub use transport::{...}` list. Acceptance: one `State` constructor; `wrap_io` has a non-test caller or is not exported; `rumors::testing::AdversarialLink` is importable.

### testing-infra-17: `party_of` and `with_messages` are duplicated between `src/tests.rs` and `src/peer/gossip/tests.rs`
- Where: src/tests.rs:30-45 (related: src/peer/gossip/tests.rs:225-245; src/peer.rs:145-169; src/lib.rs:319-321)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn party_of\|fn with_messages\|fn provider_with' src tests`; gossip/tests.rs:237-245 body read, identical to tests.rs:38-45 save the parameter name; `provider_with` at 226-234 is `send_all` over a seed)
- Seen by: structure-prose [3]; refutation: confirmed; history: no rationale found (the copy arrived with db99759db)
- Owner-gated: no

`party_of` has a byte-identical body in `src/peer/gossip/tests.rs`, and `provider_with` there is `with_messages(Peer::seed(), values)`. `Peer`'s fields are `pub(crate)` and `testing` is compiled for every in-crate test build, so one home is possible; the `dangerously_alias` escape hatch in particular should have one home with one caveat.

Evidence:

    37	/// Read a `Peer`'s party for assertions.
    38	fn party_of(k: &Peer<u64>) -> Party {
    39	    k.inner
    40	        .borrow()
    41	        .party
    42	        .as_ref()
    43	        .expect("a live Peer holds its party")
    44	        .dangerously_alias()
    45	}

Resolution: Host `pub(crate) fn party_of<T>(&Peer<T>) -> Party` and `with_messages` in a `cfg(test)` submodule of `crate::testing` (so the aliasing helper does not ship under `test-internals`); both suites import them and `provider_with` goes. Acceptance: `grep -rn 'fn party_of' src` returns one definition under `src/testing` (plus the unrelated label-based helper in `src/tree/tests.rs`); `provider_with` is gone.

### Nits (3)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| testing-infra-3 | `src/testing.rs:335-343` | Import placement and grouping | Hoist the std block to the top of testing.rs |
| testing-infra-10 | `src/testing/transport.rs:328-328` | A `debug_assert` that restates tokio's `ReadBuf` contract, and an assertion of unit against unit | Delete line 328 and the `before` binding at 309 |
| testing-infra-14 | `src/testing/transport.rs:689-690` | Em-dashes in `//` comments | Rewrite each with a colon, semicolon, or parentheses |

## Integration tests

Files: tests/common/ first, then the suites under tests/. Entries: 85 (7 medium, 41 low, 37 nit).

### tests-bookmark-3: Suite-local bootstrap and gossip drivers reimplement `common::wire` for bookmarked peers and lose its drained-control assertion
- Where: tests/bookmark_attach.rs:22-43 (related: tests/bookmark_causality.rs:735-806, 1156-1186; tests/bookmark_transmit_window.rs:148-189, 447-465; tests/bookmark_when.rs:189-227; tests/common/wire.rs:144-158, 244-264; tests/common/sim.rs:879-906; src/rumors.rs:27; src/snapshot.rs:77-79)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -c assert_control_drained` is 0 in all four suites; `a pristine seed attaches` occurs 1+4+4+2 times; `gossip_pair_async` and `bootstrap_fork_configured` take `&Rumors<T>` at wire.rs:144-147 and 244; `Rumors<T, B: BookmarkError = NoBookmark>` at src/rumors.rs:27; transmit's `boot_from` at L150-172 contains no `sync_window_floor`)
- Seen by: structure-prose, api-economics, blind-spots (the floor omission); refutation: confirmed; history: no rationale found (wire.rs took `&Rumors<T>` when the suites were born; 52376b52's drain sweep listed the suites it touched and no bookmark suite is among them)
- Owner-gated: no

The shared drivers exist only over `Rumors<T>` (that is, `NoBookmark`), so every bookmarked suite re-derives them: five bootstrap helpers (`bootstrap_unbookmarked`, `boot_from_async`, `boot_from`, `serve_bootstrap`, `bootstrap_fork_peer`), three clean gossip-pair helpers (`gossip`, `plain_gossip`, `clean_gossip`), two heal loops, and eleven copies of the seed-and-attach chain with the same expect string. The shared drivers call `assert_control_drained` after every successful session; none of the copies do, so the control-drain invariant every other suite enforces is unexercised across every bookmarked session. The copies also drift: transmit's `boot_from` skips `.sync_window_floor()` on the booted peer while attach (L42), causality (L1171), when (L208, L226), and common (`WindowChoice::Floor`, wire.rs:217) pin it, so B, C, and D in the transmit suite run at the default budget; and transmit's heal loop (L450, L456) fingerprints by `snapshot().hash()` alone where `sim::quiesce` and causality use `(hash, latest)`, although `Snapshot::hash`'s own doc says two replicas at different causal points can share a hash. A helper every suite reimplements is a missing harness method, and divergent copies are where coverage gaps hide.

Evidence:

        22	/// Bootstrap a fresh, still-unbookmarked peer from `server` over a clean
        23	/// in-memory link. Both sides run as spawned tasks so a finished one drops
        24	/// its end; the wires are reliable, so the bootstrap succeeds.
        25	async fn bootstrap_unbookmarked(server: &Rumors<String, FlakyInMemoryBookmark>) -> Peer<String> {

    tests/common/wire.rs:
       153	    let (a_result, b_result) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
       154	    let a_report = a_result.expect("wire gossip A");
       155	    let b_report = b_result.expect("wire gossip B");
       156	    assert_control_drained(a_link, b_link);

    tests/bookmark_transmit_window.rs:
       158	        let peer = Peer::<Msg>::bootstrap()
       159	            .join(&mut link)
       160	            .await
       161	            .expect("bootstrap ok")
       162	            .expect("the server is established");
       163	        peer.bookmark(bm).await.expect("in-memory persist")

Resolution: generalize `gossip_pair_async`, `wire_gossip_async`, `bootstrap_fork_configured`, and `sim::quiesce` over `B: Bookmark` (the `Rumors<T, B>` parameter exists and `Rumors<T, B>::gossip` is already generic); add `serve_bootstrap_to<T, B, B2: Bookmark>(server: &Rumors<T, B>, bookmark: B2) -> Rumors<T, B2>` for a newcomer that attaches a bookmark, and `seeded_with<T, B: Bookmark>(bookmark: B) -> Rumors<T, B>` for the pristine-seed attach. Delete the suite-local copies. Keep spawn-based drivers only where spawning is essential (causality's faulted sessions; transmit's gated and aborted sessions, pending finding 11), and route even those through the shared link and drain mechanics. Causality's `clean_gossip` becomes a thin wrapper that calls the driver and then `secure(a); secure(b)`. Acceptance: `grep -n 'async fn boot\|fn bootstrap_\|fn plain_gossip\|fn gossip(' tests/bookmark_*.rs` shows no clean-wire driver bodies; every successful bookmarked session passes through `assert_control_drained`; the pristine-seed expect string appears once, in `tests/common`; every booted peer in the four suites is at the floor.

See also: tests-lifecycle-3, tests-observation-32.

### tests-disruption-handshake-31: hop_trace's insertion fixture is byte-identical to gossip_pipelining's under a doc that says "reduced scale"; send_random has seven copies; bootstrap_fork and no-op budget calls are duplicated
- Where: tests/hop_trace.rs:437-446 (related: tests/gossip_pipelining.rs:23-28, 64-94; tests/hop_trace.rs:460-462, 475-493, 546-548, 637-639; benches/support/wire.rs:39-56; tests/window_census.rs:73; tests/window_corners.rs:63; tests/window_knee.rs:214; tests/window_operator.rs:82; benches/window_wallclock.rs:89; src/tree/mirror/streaming/window.rs:547-555)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (grep for `fn send_random` and the seed 0x9e37_79b9_7f4a_7c15; read window.rs:547-555 for `WindowConfig::default() == Budget(DEFAULT_SYNC_MEMORY_BUDGET)`; read benches/support/wire.rs:39-56)
- Seen by: structure-prose [3], api-economics [40]; refutation: confirmed (gossip_pipelining's `.sync_memory_budget` call is that test's subject and stays); history: "reduced scale" was false from birth relative to the pipelining test (7d8891b2's message compares to the bench fixture); the explicit budget calls are the pre-db2718d4 escape that commit removed from benches but not from this test
- Owner-gated: no

diverged_insertions builds the same pair as gossip_pipelining's diverged_pair (COMMON 2_048, DIVERGENT_PER_SIDE 512, seed 0x9e37_79b9_7f4a_7c15, default window), yet its doc says it mirrors the pipelining shape "at reduced scale": a doc claim the constants contradict. send_random has seven identical copies across the standalone measurement binaries and one bench; hop_trace's bootstrap_fork duplicates benches/support/wire.rs::bootstrap_fork except for a capacity and a `.sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)` call that is the default already. These binaries avoid `mod common;` (its `#![allow(dead_code)]` pulls in the whole sim engine) but all `#[path]`-include benches/support, which is the natural home.

Evidence:

       437	/// Two production-window V2 peers with a shared prefix and divergence on
       438	/// both sides, mirroring the pipelining test's shape at reduced scale.
       439	fn diverged_insertions() -> (Rumors<u64>, Rumors<u64>) {
       440	    const COMMON: usize = 2_048;
       441	    const DIVERGENT_PER_SIDE: usize = 512;
       442	
       443	    let left = Peer::seed()
       444	        .sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)
       445	        .into_rumors();
       446	    let mut rng = SmallRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);

    tests/gossip_pipelining.rs
        24	const COMMON: usize = 2_048;
        28	const DIVERGENT_PER_SIDE: usize = 512;
        69	    let mut rng = SmallRng::seed_from_u64(0x9e37_79b9_7f4a_7c15);

Resolution: Fix the doc ("the same shape as the pipelining test"). Move send_random, bootstrap_fork, and a `diverged_pair(common, per_side, seed)` fixture into a benches/support module and include it from gossip_pipelining, hop_trace, window_census, window_corners, window_knee, window_operator, tradeoff_probe, and benches/window_wallclock. Drop the no-op `.sync_memory_budget(DEFAULT_SYNC_MEMORY_BUDGET)` in hop_trace (443-445, 460-462, 490, 546-548, 637-639); keep it in gossip_pipelining, where the public setting is the test's point. Acceptance: `grep -rn 'fn send_random' tests benches` returns one definition; one copy of the 0x9e37_79b9_7f4a_7c15 fixture exists; hop_trace's fixture docs make no scale comparison the constants contradict.

Synthesis note: tests-resource-link-window-22 covers the same ten copies and proposes a different home (a `tests/common/measure.rs` fixture built on `bootstrap_fork_with_window_async`, gaining the stall detector and the drain check) where this entry proposes `benches/support`. One home is the point; the tests/common route also closes the liveness gap the other entry names, so prefer it unless the compile weight of `mod common;` is measured to matter (suite-economics-8, tests-common-8). tests-resource-link-window-22 is the entry of record for the fixture copies; this entry is counted for the "reduced scale" doc claim and the no-op budget calls.

See also: tests-resource-link-window-22, tests-disruption-handshake-28, suite-economics-8.

### tests-lifecycle-3: The bootstrap and retire session drivers are spelled out per suite instead of owned by `common::wire`
- Where: tests/bootstrap.rs:31-49 (related: tests/bootstrap.rs:165-180, tests/bootstrap.rs:209-224, tests/network.rs:87-99, tests/retire.rs:52-65, tests/retire.rs:78-87, tests/retire.rs:91-110, tests/retire_redaction.rs:39-46, tests/common/wire.rs:244-264, tests/common/wire.rs:232-240, tests/party_conservation.rs:103-120, tests/common/schedule/executor.rs:256-275)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read `wire_bootstrap` and `bootstrap_fork_configured` side by side: same link pair, same `tokio::join!` of `gossip` and `bootstrap().join`, same `expect`, same floor, same drain; the only differences are the 64 KiB buffer and an `Option` return that both callers `.expect` at lines 68 and 101; each other listed site read and confirmed to carry the same shape)
- Seen by: structure-prose, api-economics; refutation: reframed (the 64 KiB buffer is an unmotivated policy deviation, not a contradicted claim) and confirmed for the roster, adding party_conservation.rs:103-120; history: no-rationale-found (the per-suite drivers predate the shared helper's current shape and were never folded in)
- Owner-gated: no

`wire_bootstrap` is `bootstrap_fork_configured(provider, WindowChoice::Floor)` with a different buffer and an `Option` return no caller uses as an `Option`. The same provider-gossip/newcomer-join session is written inline again at bootstrap.rs:165-180 and network.rs:87-99 and as `wire_bookmarked_join` at 209-224; the retire-into-gossip session is written as three harnesses in retire.rs, inline in retire_redaction.rs, inline in party_conservation.rs, and inline in the executor. Each copy differs only in the builder configuration or the outcome type. A helper every suite reimplements is a missing helper, and the copies have already drifted: the bookmarked variant omits the drain check (tests-lifecycle-7). A future change to the session-boundary contract would need ten edits.

Evidence:

        31	fn wire_bootstrap<T>(provider: &Rumors<T>) -> Option<Rumors<T>>
        32	where
        33	    T: Serialize + DeserializeOwned + Eq + Send + Sync + 'static,
        34	{
        35	    block_on(async move {
        36	        let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
        37	
        38	        let (provider_out, bootstrap_out) = tokio::join!(
        39	            provider.gossip(&mut a_link),
        40	            Peer::<T>::bootstrap().join(&mut b_link),
        41	        );
        42	        provider_out.expect("provider gossip");
        43	        let joined = bootstrap_out
        44	            .expect("bootstrap handshake")
        45	            .map(|peer| peer.sync_window_floor().into_rumors());
        46	        assert_control_drained(a_link, b_link);
        47	        joined
        48	    })
        49	}

Resolution: In `tests/common/wire.rs` add a bootstrap driver that takes the configured `Bootstrap<T>` builder (so the zero-budget and bookmarked variants collapse into the caller's builder chain), a `Joined`-returning sibling for the bookmarked builder, and a `retire_into_async(retiree: Peer<T>, absorber: &Rumors<T>) -> Retire<T>`, each ending in `assert_control_drained`; express `bootstrap_fork_configured` through the first. Replace `wire_bootstrap(&provider).expect(..)` with `bootstrap_fork(&provider)`, and the inline sessions with the helpers. `bootstrap_fork_with_window_async` (wire.rs:232-240) is a pure pass-through of `bootstrap_fork_configured`; give the underlying function the public name. Acceptance: `grep -n 'bootstrap().*join(&mut' tests/*.rs` matches only wire.rs and tests whose subject is the builder itself; `grep -n '\.retire(&mut' tests/*.rs tests/common` matches only wire.rs and the snapshot capture drivers; no `fn wire_bootstrap` and no local `LINK_BUF` in bootstrap.rs; all affected suites pass.

See also: tests-bookmark-3, tests-observation-32, tests-lifecycle-6.

### tests-observation-2: Observer step/drain/live_map helpers are duplicated across causal.rs and listen.rs, and reimplement the public `try_next` and `oracle::readout`
- Where: tests/causal.rs:26-99 (related: tests/listen.rs:28-71, tests/common/sim.rs:586-592, src/rumors/unordered.rs:175-182, src/rumors/causal.rs:136-143, tests/common/oracle.rs:80-88)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read both helper blocks and compared line by line; read `try_next` in both observers and `readout` in oracle.rs; `git log -S` dates listen.rs to 040a0d042 on 2026-06-10, `try_next` to 586911cfd on 2026-06-17, `readout` to 8b1ade010 on 2026-05-27)
- Seen by: structure-prose [2], blind-spots [37], api-economics [41]; refutation: confirmed (one caveat: sim.rs's inline drain asserts per item, so a Vec-returning `drain` does not slot in there verbatim); history: no rationale found (causal.rs copied listen.rs's helpers 35 minutes after it landed)
- Owner-gated: no

causal.rs:26-58 and listen.rs:28-60 define the same `Step` enum, `step`, and `drain`, differing only in the observer type; causal.rs:79-87 and listen.rs:62-71 define the same `live_map` (already `readout(&rumors.snapshot())` in tests/common/oracle.rs); causal.rs:89-99 adds a `try_next`-based `drain_unordered`, so one file drains one observer face two ways; tests/common/sim.rs:586-592 inlines a fourth `while let Some(Some(..)) = obs.next().now_or_never()` loop. `step` is the public `try_next` (src/rumors/unordered.rs:175-182) with the `Arc` dereferenced. Since both observers' `Stream::Item` is the owned `(Version, Arc<T>)`, one generic definition serves every site. Duplicated harness code drifts: the two `live_map` docs already differ.

Evidence:

    38	/// Poll the observer exactly once without an executor.
    39	fn step(obs: &mut CausalMessages<u64>) -> Step {
    40	    match obs.next().now_or_never() {
    41	        None => Step::Quiet,
    42	        Some(None) => Step::Ended,
    43	        Some(Some((v, m))) => Step::Item((v, *m)),
    44	    }
    45	}

    (src/rumors/unordered.rs)
    175	    pub fn try_next(&mut self) -> TryNext<T> {
    176	        use futures::{FutureExt, StreamExt};
    177	        match self.next().now_or_never() {
    178	            None => TryNext::Quiet,
    179	            Some(None) => TryNext::Ended,
    180	            Some(Some(message)) => TryNext::Message(message),
    181	        }
    182	    }

Resolution: Add `tests/common/observer.rs` with `enum Step<T>`, `fn step<T: Copy, S: Stream<Item = (Version, Arc<T>)> + Unpin>(obs: &mut S) -> Step<T>` (or return `TryNext<T>` directly and match on it), and `fn drain<...>(obs: &mut S) -> (Vec<(Version, T)>, bool)`; import them in causal.rs and listen.rs; delete both `Step` enums, both `step`/`drain` pairs, `drain_unordered` (its sole caller at causal.rs:623 ignores termination), and both `live_map`s in favor of `readout(&r.snapshot())`. Leave sim.rs's per-item asserting loop, or give the shared `drain` a per-item callback. Acceptance: `grep -rn 'fn step\b\|fn drain\b\|fn live_map\|^enum Step' tests/` matches only under tests/common (bookmark_causality.rs's unrelated simulation `Step` aside).

### tests-observation-32: Six suites redefine a 64 KiB `LINK_BUF` for the retire path with a rationale the harness falsifies, listen.rs inlines the literal, and four suites reimplement the retire driver
- Where: tests/party_conservation.rs:48-51 (related: tests/party_conservation.rs:103-120, tests/listen.rs:252-265, tests/listen.rs:259, tests/retire.rs:31-33, tests/retire.rs:52-65, tests/bootstrap.rs:27, tests/gossip_when.rs:57, tests/reuse.rs:31, tests/bookmark_attach.rs:20, tests/common/wire.rs:61-63, tests/common/schedule/executor.rs:20, tests/common/schedule/executor.rs:256-275, tests/common/overlap.rs:57, tests/bookmark_causality.rs:80, tests/bookmark_causality.rs:673)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (`grep -rn '64 \* 1024\|const LINK_BUF' tests/` lists the six per-suite constants, the bare literal at listen.rs:259, and `wire::LINK_BUF = 8 * 1024`; read the four driver bodies; executor.rs imports `LINK_BUF` from `crate::common::wire` and retires over it at 258)
- Seen by: structure-prose [8], blind-spots [36], api-economics [42]; refutation: confirmed (42's severity down to low: harness duplication without correctness exposure; the constant confusion is the substantive part); history: deliberate-but-expired (retire.rs's "other wire tests' headroom" dates from when every wire test used a 64 KiB duplex; b3b877d9b created the shared 8 KiB constant and every private 64 KiB one in a single commit, so the premise expired in the commit that wrote it; party_conservation.rs then copied retire.rs's sentence)
- Owner-gated: no

`common::wire::LINK_BUF` is 8 KiB. Six suites define a private `LINK_BUF = 64 * 1024` for retire and bootstrap sessions, each justified by pointing at another file (here "keep `retire.rs`'s headroom"; retire.rs says "keep the other wire tests' headroom", which the shared constant contradicts), and listen.rs inlines `64 * 1024` with no name. Meanwhile tests/common/schedule/executor.rs runs every scheduled retirement over the shared 8 KiB constant, tests/bookmark_causality.rs retires over its own 8 KiB, and tests/common/overlap.rs runs whole sessions at 48 bytes; the link contract promises receiver-paced flow control at any positive capacity, so the 64 KiB "headroom" is not a requirement. Two constants named `LINK_BUF` with different values across one suite mislead a reader who knows the shared one. The retire driver itself (`try_into_peer`, `memory_with_capacity`, `join!(retire, gossip)`, expect the absorber, `assert_control_drained`, match `Retire::Retired`) is spelled four times, and they have already drifted: executor.rs:266-273 matches every `Retire` variant with a distinct panic while the other three use `matches!`/`assert!`.

Evidence:

    48	/// Capacity for each in-memory link stream on the retirement path: a
    49	/// divergent retiree's session moves content through its gossip round, so
    50	/// keep `retire.rs`'s headroom.
    51	const LINK_BUF: usize = 64 * 1024;

    (tests/retire.rs)
    31	/// Capacity for each in-memory link stream. A divergent retiree's session moves
    32	/// content through the gossip round, so keep the other wire tests' headroom.
    33	const LINK_BUF: usize = 64 * 1024;

    (tests/common/wire.rs)
    63	pub const LINK_BUF: usize = 8 * 1024;

    (tests/common/schedule/executor.rs)
    258	                let (mut link_r, mut link_a) = rumors::link::memory_with_capacity(LINK_BUF);

Resolution: Add `pub fn retire_into<T>(retiree: Rumors<T>, absorber: &Rumors<T>) -> Retire<T>` (with an `_async` core taking the `Peer`) to `tests/common/wire.rs`, using `wire::LINK_BUF` and the executor's per-variant panics as the diagnostic; migrate the four drivers and the executor arm; delete the six per-suite constants where the retire helper was their only use (bootstrap.rs, gossip_when.rs, reuse.rs, bookmark_attach.rs each need a look at their other uses). If a run shows some retire path needs more than 8 KiB, add one `RETIRE_LINK_BUF` to `common::wire` whose doc states the actual reason. Also sweep the three private `LINK_BUF = 8 * 1024` copies (bookmark_when.rs:69, bookmark_causality.rs:80, bookmark_transmit_window.rs:49) onto the shared constant. Acceptance: `grep -rn 'const LINK_BUF\|64 \* 1024' tests/*.rs` shows no per-suite `LINK_BUF` and no inline capacity literal at listen.rs:259; one retire driver under tests/common.

See also: tests-bookmark-2, tests-lifecycle-3.

### tests-resource-link-window-3: async_wire.rs restates pairwise.rs's union property under a qualifier whose referent was deleted
- Where: tests/async_wire.rs:1-59 (related: tests/pairwise.rs:32-37, tests/pairwise.rs:47-61, tests/pairwise.rs:219-237, tests/common/wire.rs:1, tests/async_wire.rs:61-81)
- Class / severity / confidence: simplification / medium / high
- Provenance: verified (read both files side by side; `dup` in pairwise.rs:32-37 is `bootstrap_fork`; `git log --all -- tests/sync_wire.rs` ends at 83edcd944, 2026-07-16, and the file is absent at HEAD; `git grep asynchronous -- tests` returns async_wire.rs:1 and common/wire.rs:1)
- Seen by: api-economics (structure-prose separately noted the two identical bodies); refutation: confirmed; history: deliberate-but-expired
- Owner-gated: no

`async_gossip_converges_on_the_union` is `pairwise::gossip_unions_content` plus `pairwise::gossip_converges`'s fingerprint check, built from the same generators and helpers. The binary exists because 691909e7 split wire tests into "async and sync binaries"; the sync binary was deleted on 2026-07-16, so "the *asynchronous* gossip path" names the only path there is, a ghost reference by contrast. What remains distinct is the `T = String` variant, one property. The duplicate costs a link unit and 256 cases of two bootstraps plus a session for no new claim. If the file is kept instead, its two bodies (47-58, 69-80) differ only in the type parameter and generator and belong in one generic helper; `assert_fingerprints_equal` also spells `rumors::Rumors<T>` while `rumors::Peer` is imported.

Evidence:

     1	//! Convergence of the *asynchronous* gossip path.

    47	        let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
    48	        let a = build_local(bootstrap_fork(&seed), &a_actions);
    49	        let b = build_local(bootstrap_fork(&seed), &b_actions);
    50	
    51	        let mut expected = readout(&a.snapshot());
    52	        expected.extend(readout(&b.snapshot()));
    53	
    54	        wire_gossip(&a, &b);

Resolution: Make `pairwise::gossip_unions_content` generic over the value strategy (or add one `_string` case) so the `String` payload rides the union oracle there, delete tests/async_wire.rs, and reword tests/common/wire.rs:1 to "Wire helpers for `Rumors::gossip` over in-memory links". Acceptance: tests/async_wire.rs is gone; pairwise.rs exercises a non-primitive payload through the union oracle; `git grep -n asynchronous tests/` describes no gossip path.

See also: tests-lifecycle-24.

### tests-resource-link-window-22: ten binaries hand-roll the budgeted bootstrap fork that tests/common owns, losing its stall detector and drain check; binding_capacity is duplicated with a magic slice
- Where: tests/window_census.rs:48-75 (related: tests/window_census.rs:158-170, tests/window_census.rs:191-203, tests/window_corners.rs:33-65, tests/window_knee.rs:98-113, tests/window_knee.rs:189-216, tests/window_operator.rs:57-84, tests/window_operator.rs:100-108, tests/tradeoff_probe.rs:63-91, tests/latency_link.rs:51-71, tests/gossip_pipelining.rs:65-94, tests/hop_trace.rs:475-497, benches/window_wallclock.rs:64-89, benches/support/wire.rs:39-56, tests/common/wire.rs:34-42, tests/common/wire.rs:232-264, tests/common/window.rs:34-43)
- Class / severity / confidence: modularity / medium / high
- Provenance: verified (`grep -rln '"bootstrap newcomer"' tests/ benches/` lists ten files; `grep -rn '^fn send_random'` lists seven definitions; `grep -rln 'support/latency.rs'` lists nine includes; `grep -rn '28\.\.=30' tests/` returns window_knee.rs:101 and window_operator.rs:103; `grep -c '^mod common;'` is 0 for every measurement suite except window_sweep; common/wire.rs:40-42 `block_on` is `run_to_quiescence` and `bootstrap_fork_configured` ends with `assert_control_drained` at 262; common/window.rs:39 maps `WindowChoice::Budget` to `sync_memory_budget`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (no commit mentions `mod common`, compile weight, or link units; the direct shared fixture arrived in 919132dc after the suites, but a workable path existed since cb69fc951 and target_message_size.rs used it the same week)
- Owner-gated: no

The same block (`memory_with_capacity`, `tokio::join!(gossip, bootstrap().join)`, two `expect`s, `.sync_memory_budget(b).into_rumors()`) appears in window_census (three times), window_corners, window_knee, window_operator, tradeoff_probe, latency_link, gossip_pipelining, hop_trace, and both bench harnesses; a three-line `send_random` in five of them; the `#[path]` latency include in nine. `common::wire::bootstrap_fork_with_window_async(parent, WindowChoice::Budget(b))` already performs this fork, and does two things the copies do not: it runs under `run_to_quiescence`, so a stall in fixture construction fails at its source rather than parking `pollster::block_on` until nextest's 180 s kill, and it asserts the control drain. The suites whose whole point is exact wire measurement build their fixtures on the one poller that cannot report a stall. `binding_capacity` (min over `capacities[28..=30]`) is duplicated between window_knee and window_operator, each with its own "depths two through four; heights 30 down to 28" paragraph and the collision-expectation rationale stated only in the knee; the range is a magic slice in both. Drift between copies is what produced the stale numbers in finding 29.

Evidence:

    53	    let right = pollster::block_on(async {
    54	        let (mut provider, mut newcomer) = rumors::link::memory_with_capacity(LINK_CAPACITY);
    55	        let (served, joined) = tokio::join!(
    56	            left.gossip(&mut provider),
    57	            Peer::<u64>::bootstrap().join(&mut newcomer),
    58	        );
    59	        served.expect("serve bootstrap");
    60	        joined
    61	            .expect("bootstrap newcomer")
    62	            .expect("provider is established")
    63	            .sync_memory_budget(budget)
    64	            .into_rumors()
    65	    });

    window_operator.rs:
    103	    capacities[28..=30]
    104	        .iter()
    105	        .copied()
    106	        .min()
    107	        .expect("three engaged heights")

Resolution: Add `mod common;` to the measurement suites and build one measurement fixture (tests/common/measure.rs) on `bootstrap_fork_with_window_async`: `diverged(budget, common, left_extra, right_extra, seed) -> (Rumors<u64>, Rumors<u64>)`, `send_random`, `const ENGAGED_HEIGHTS: RangeInclusive<usize> = 28..=30` with the collision-expectation rationale once, and `binding_capacity(session_len, budget)`. The one design point to settle first is link capacity: the copies use 8 MiB pipes and `common::wire` uses `LINK_BUF` (8 KiB); since the fork is corpus construction, not the measured session, a capacity parameter on the shared helper (or accepting 8 KiB) both work. Replace every copy, the bench harnesses included. Acceptance: `grep -rn '"bootstrap newcomer"' tests/ benches/` returns one site; `grep -rn '^fn send_random'` returns one; `grep -rn '28\.\.=30' tests/` returns one; every measurement fixture runs through `run_to_quiescence` and `assert_control_drained`.

See also: tests-disruption-handshake-31 (which shares the fixture copies; this entry is the entry of record for them), tests-disruption-handshake-28, suite-economics-8.

### tests-common-3: two byte-budget fault injectors coexist and neither doc names the other or what separates them
- Where: tests/common/fault.rs:1-26 (related: src/testing/transport.rs:186-215, src/testing/transport.rs:247-262, tests/common/sim.rs:412-419)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (read `State::failure` and `State::remaining_bytes` in src/testing/transport.rs: byte-unit faults never constrain `Connect`/`Accept`, and every injected failure is `io::Error::other(injected)`, kind `Other`, which `honest_io` at sim.rs:412-419 rejects)
- Seen by: structure-prose; refutation: reframed (fault.rs, 2026-06-10, predates `testing::wrap_link`, 2026-07-16, so `wrap_link` is the second injector); history: no rationale for coexistence
- Owner-gated: no for the missing sentence; dissolving either injector is the owner's option

`rumors::testing::wrap_link` and `fault.rs` both sever a link at a byte offset. Two properties separate them, and neither module states them: fault.rs refuses new streams once a direction's budget is spent (lines 15-18, 201-207, 227-233), where transport.rs's byte-unit faults never constrain the stream supply; and fault.rs raises typed `BrokenPipe`/`ConnectionReset`, which the disruption engine's classifier keys on, where transport.rs raises `io::Error::other(InjectedIo)`. A maintainer meeting both cannot tell which to reach for or whether one can go. Infrastructure is justified by naming what it serves that the existing tool does not.

Evidence:

         1	//! Wire-fault injection for the disruption simulations: deterministic,
         2	//! byte-budgeted severing of either direction of a gossip link.
    ...
        15	//! happens to travel. A severed direction also refuses new streams: once
        16	//! its budget is exhausted, [`FaultConnector::connect`] fails alongside the
        17	//! writers and [`FaultAcceptor::accept`] alongside the readers, because a
        18	//! dead connection cannot open or deliver streams any more than it can

    (src/testing/transport.rs:258-260)
                // Supply operations transfer no bytes, so a byte-counted fault
                // never constrains them.
                Operation::Connect | Operation::Accept => return usize::MAX,

Resolution: add one paragraph to fault.rs's module doc (or to transport.rs's, or both) naming `rumors::testing::wrap_link` and the two properties the disruption engine needs that it lacks: stream-supply refusal on exhaustion, and severed-transport error kinds the classifier recognizes. If the owner prefers one injector, extend `IoPlan`/`IoFault` with those two properties and dissolve fault.rs into `wrap_link`. Acceptance: each injector's module doc names the other and the property that distinguishes them, or one of them is gone.

See also: testing-infra-20.

### tests-common-4: `metered` repeats `faulty_link`'s wrapping body
- Where: tests/common/fault.rs:102-125 (related: tests/common/fault.rs:78-85, tests/common/fault.rs:128-155)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both bodies: the same `LinkParts { Cut, Fuse, FaultConnector, FaultAcceptor, session }.into_link()` construction, differing only in the two budget clones kept for the meter)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale (`metered` copied the body at 919132dc; the same-day fix round left it)
- Owner-gated: no

The `ByteMeter` doc claims "the meter and the cut draw on identical accounting" (lines 83-84). Today that is true because two copies of the wiring happen to agree; one private core taking the two budgets makes it true by construction.

Evidence:

       102	pub fn metered(link: MemoryLink) -> (FaultyLink, ByteMeter) {
       103	    let write = budget(None);
       104	    let read = budget(None);
    ...
       110	    let link = LinkParts {
       111	        control_read: Cut::new(parts.control_read, read.clone()),
       112	        control_write: Fuse::new(parts.control_write, write.clone()),

       141	    LinkParts {
       142	        control_read: Cut::new(parts.control_read, read_budget.clone()),
       143	        control_write: Fuse::new(parts.control_write, write_budget.clone()),

Resolution: add `fn wrap<CR, CW, C, A>(link: Link<CR, CW, C, A>, write: Budget, read: Budget) -> Link<Cut<CR>, Fuse<CW>, FaultConnector<C>, FaultAcceptor<A>>`; `faulty_link` calls it with `budget(plan.write_cut)`/`budget(plan.read_cut)`, `metered` with two `budget(None)` whose clones go into the meter. Acceptance: one `LinkParts { .. }.into_link()` in fault.rs.

### tests-common-8: forty-two binaries compile the harness from source, and the arrangement needs a blanket allow
- Where: tests/common/mod.rs:3-4 (related: tests/common/mod.rs:31-33, Cargo.toml:145)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified for the count (`grep -l '^mod common;' tests/*.rs | wc -l` is 42 of 58 binaries; tests/common totals 5189 lines); the compile-time cost is assessed, not measured
- Seen by: api-economics; refutation: severity down to low (unmeasured; the cheaper half is finding 10); history: no rationale for the layout beyond the allow's comment
- Owner-gated: yes: a workspace layout decision

Every binary that writes `mod common;` re-parses and re-typechecks the whole harness, and the blanket allow at 31-33 exists only so that arrangement lints clean. A path dev-dependency crate (the manifest already carries a self-referential dev-dependency at Cargo.toml:145, so the cycle is permitted) compiles the harness once and restores private-item and import linting; `pub` items in a library are not dead-code-linted either, so the allow's removal buys less than the whole. The compile saving is a hypothesis until measured.

Evidence:

         3	//! Each per-category test binary pulls this module in via `mod common;`
         4	//! and reaches its pieces through `crate::common::*`.

Resolution: measure first (`cargo build --tests --timings` at HEAD and on a branch with `crates/rumors-testkit`, on a quiet machine), then decide. Keep `tests/main.rs` as the seed anchor either way. Acceptance: the two numbers recorded in the decision; if adopted, no `mod common;` in tests/*.rs and no module-wide allow.

### tests-common-10: `unused_imports` is allowed module-wide without a reason that covers it, and the residue it hides
- Where: tests/common/mod.rs:31-33 (related: tests/common/wire.rs:14, tests/common/sim.rs:86, tests/common/fault.rs:86-97, tests/common/flaky.rs:199, tests/common/peer.rs:57-61, tests/common/sim.rs:687-689, tests/common/wire.rs:180-182, tests/sanity.rs:84, tests/sanity.rs:92)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -rn Protocol tests/common` matches only wire.rs:14; `grep -n readout_multiset tests/common/sim.rs` matches only the import; `ByteMeter.read` has no accessor; `git show 368da2a5 -- tests/common/wire.rs` removes every `Protocol::V2` use; `git show 0d48b153 -- tests/common/fault.rs` removes `pub fn read`; `git blame -L687,689 tests/common/sim.rs` dates the braces to 9eadfc68 and the line inside to 212c6914; `.observations()` is called only at tests/sanity.rs:84,92)
- Seen by: all three lenses; refutation: confirmed, and added the `readout_multiset` import; history: deliberate-but-expired for `Protocol` (V1 retirement), `ByteMeter.read` (accessor ruled dead), and the braces (the scoped batch guard is gone); no rationale for `let _ = self.label;` and `observations()`
- Owner-gated: no

The allow's stated reason ("Not every binary uses every module") justifies `dead_code`: an item one binary does not reach. It does not justify `unused_imports`: a `use` unused inside a module is unused in every binary that includes it, so that half of the allow only hides rot. It hides two dead imports today, and the `dead_code` half hides a field whose accessor was already ruled dead, a no-op statement, an accessor that clones a `pub` field, and a scope that scopes nothing. A `WindowChoice::Default.apply(..)` on the seed in `divergent_pair` is the identity by `apply`'s definition; beside sim.rs:682-686, which sweeps the seed's window, it suggests a lost parameter.

Evidence:

        31	//! Not every binary uses every module; suppress unused-code warnings here
        32	//! rather than peppering allows across modules.
        33	#![allow(dead_code, unused_imports)]

    (tests/common/wire.rs:14)
        14	use rumors::{Peer, Protocol, Rumors, testing::run_to_quiescence};

    (tests/common/sim.rs:86)
        86	use crate::common::oracle::{readout, readout_multiset, version_key};

    (tests/common/fault.rs:87-88)
        87	    write: Budget,
        88	    read: Budget,

    (tests/common/flaky.rs:199)
       199	        let _ = self.label;

    (tests/common/peer.rs:59-61)
        59	    pub fn observations(&self) -> Vec<(Version, T)> {
        60	        self.observations.clone()
        61	    }

    (tests/common/sim.rs:687-689)
       687	    {
       688	        seed.send_all(plan.seed_messages.iter().copied()).unwrap();
       689	    }

    (tests/common/wire.rs:180-182)
       180	    let seed = WindowChoice::Default
       181	        .apply(Peer::<u64>::seed())
       182	        .into_rumors();

Resolution: narrow the allow to `dead_code`; if rustc then warns on the `pub use` re-exports in schedule/mod.rs:19-26 or gossip_snapshot.rs:94, allow `unused_imports` on those items only. Delete `Protocol` and `readout_multiset` from the imports; drop `ByteMeter.read` (the field, and its clone into the meter at fault.rs:107) and make the doc at 78-80 singular; delete `let _ = self.label;`; delete `Peer::observations()` and read the field at tests/sanity.rs:84,92; unwrap the braces; write `Peer::<u64>::seed().into_rumors()` in `divergent_pair`, or make the seed's window a parameter. Acceptance: `#![allow(dead_code)]` alone stands at mod.rs:33 with `just clippy` clean; none of the seven items remains.

### tests-common-14: overlap.rs re-implements `schedule::arb`'s shadow and its `fork_tree`
- Where: tests/common/overlap.rs:350-356 (related: tests/common/overlap.rs:504-560, tests/common/schedule/arb.rs:161-165, tests/common/schedule/arb.rs:255-363)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn fork_tree' tests/` returns the two identical three-line bodies; read `Knowledge::merge_session` against `SimState::absorb` and traced the four known/live combinations per peer: `merge_session(&self.clone(), a, b)` and `absorb(a, b); absorb(b, a)` produce the same sets and the same per-peer observation order)
- Seen by: structure-prose (`fork_tree`), api-economics (`Knowledge`); refutation: confirmed the unification, and that an equality meta-test is not attainable (see finding 12); history: no rationale (`Knowledge` landed with the doc "as in `schedule::arb`'s shadow" and no shared code)
- Owner-gated: no

Both copies announce themselves as copies. `fork_tree` exists twice because arb.rs's is private; `Knowledge` re-encodes the deletion-honoring merge that `SimState::absorb` encodes, differing only in taking a fork-time snapshot as the source. Two hand-kept encodings of one semantics stay in agreement only by reading.

Evidence:

       350	/// Fold raw entropy into a valid fork tree (as
       351	/// [`schedule::arb`](super::schedule::arb) does).
       352	fn fork_tree(n_peers: usize, raw: &[usize]) -> Vec<usize> {
    ...
       504	/// Per-peer knowledge sets, as in `schedule::arb`'s shadow: everything
       505	/// the peer has ever held, the subset currently live, and the exact
       506	/// observation order.
       507	#[derive(Clone)]
       508	struct Knowledge {

Resolution: minimum: make `arb::fork_tree` `pub` and import it. Fuller: give `SimState` a snapshot-parameterized `merge_session(&mut self, snapshot: &SimState, a, b)`, express `gossip(a, b)` as `let frozen = self.clone(); self.merge_session(&frozen, a, b)`, build `build_overlap_schedule` on `SimState`, and delete `Knowledge`. Do not add an equality meta-test for the overlap shadow; state at the site that it is approximate by design (finding 12). Acceptance: one `fn fork_tree` and one shadow type in tests/common.

### tests-common-18: the fingerprint tuple appears twelve times and the quiesce loop three times, with the round bound declared twice
- Where: tests/common/peer.rs:140-181 (related: tests/common/sim.rs:106-107, tests/common/sim.rs:881-906, tests/common/sim.rs:926-929, tests/pairwise.rs:42-45, tests/retire.rs:379, tests/retire.rs:389, tests/retire.rs:421, tests/retire.rs:431, tests/bookmark_causality.rs:814, tests/bookmark_causality.rs:829, tests/multi_peer.rs:144, tests/session_overlap.rs:43-58)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'snapshot.hash(), snapshot.latest().clone()' tests/ src/`: 12 hits, all under tests/; `MAX_QUIESCE_ROUNDS_PER_PEER = 16` at peer.rs:181 and sim.rs:107)
- Seen by: structure-prose, api-economics; refutation: confirmed and enlarged (the api-economics citation of tests/lifecycle.rs:50-58 is a fixture builder, not a loop); history: the 0d48b153 dedup round unified peer.rs's two loops and left sim.rs's copy "the same criterion" without saying why
- Owner-gated: no

The convergence criterion `(snapshot.hash(), snapshot.latest().clone())` is the one fact every convergence assertion in the suite rests on, and it is spelled out in twelve places; the bounded full-mesh fixed-point loop around it exists in peer.rs (sync, over `Peer<T>`), sim.rs (async, over `Rumors<u64>`, its doc acknowledging the transcription), and tests/session_overlap.rs (over readouts, with its own `ROUNDS = 8`); the round bound is a hand-maintained duplicate constant.

Evidence:

       149	    let fingerprint = |peer: &Peer<T>| {
       150	        let snapshot = peer.local.snapshot();
       151	        (snapshot.hash(), snapshot.latest().clone())
       152	    };
    ...
       181	const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;

    (tests/common/sim.rs:106-107)
       106	/// Headroom on the heal loop, as in `peer::quiesce`.
       107	const MAX_QUIESCE_ROUNDS_PER_PEER: usize = 16;

Resolution: add `pub fn fingerprint<T>(snapshot: &Snapshot<T>) -> ([u8; MERKLE_HASH_LEN], Version)` beside `readout` in oracle.rs and use it at every site; write one async core `quiesce_handles<T>(peers: &[&Rumors<T>])` (fingerprint fixed point, one bound, one panic message) that `peer::quiesce_refs` wraps with `block_on` plus per-peer drains, `sim::quiesce` awaits, and session_overlap.rs calls in place of `converge`; make the bound `pub` in one place. Acceptance: one definition of the fingerprint and one of the round bound in tests/; pairwise.rs and session_overlap.rs import rather than redefine.

See also: tests-lifecycle-14.

### tests-common-19: socket-clamp dial and bind code duplicated between tcp.rs and routed_tcp.rs
- Where: tests/common/routed_tcp.rs:34-43 (related: tests/common/routed_tcp.rs:99-111, tests/common/tcp.rs:66-76, tests/common/tcp.rs:112-119)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the four blocks: the same send-buffer-clamped dial and the same recv-buffer-clamped loopback bind, differing only in the backlog constant and where the inherit-before-listen comment sits)
- Seen by: structure-prose; refutation: confirmed (the two files implement different traits, so only the socket-option blocks are shareable); history: no rationale (routed_tcp.rs landed twelve days after tcp.rs)
- Owner-gated: no

Two copies of socket policy invite divergence: one already documents the inherit-before-listen ordering inline and the other in a doc comment. One helper pair keeps the OS-clamp knowledge in one place.

Evidence:

        34	    async fn dial(&self, addr: &SocketAddr) -> io::Result<TcpStream> {
        35	        match self.send_buffer {
        36	            None => TcpStream::connect(*addr).await,
        37	            Some(send) => {
        38	                let socket = TcpSocket::new_v4()?;
        39	                socket.set_send_buffer_size(send)?;
        40	                socket.connect(*addr).await
        41	            }
        42	        }
        43	    }

    (tests/common/tcp.rs:112-119)
       112	        let stream = match self.0.send_buffer {
       113	            None => TcpStream::connect(self.0.peer).await?,
       114	            Some(send) => {
       115	                let socket = TcpSocket::new_v4()?;
       116	                socket.set_send_buffer_size(send)?;
       117	                socket.connect(self.0.peer).await?
       118	            }
       119	        };

Resolution: add `pub async fn dial_clamped(addr: SocketAddr, send_buffer: Option<u32>) -> io::Result<TcpStream>` and `pub async fn bind_loopback(recv_buffer: Option<u32>, backlog: u32) -> io::Result<TcpListener>` in tcp.rs and call them from both files. Acceptance: each socket-clamp `match` appears once in tests/common.

### tests-common-32: the sweep judges paths only; a committed seed whose shrink note predates the strategy awaits an owner ruling
- Where: tests/seed_liveness.rs:129-134 (related: tests/seed_liveness.rs:148-203, proptest-regressions/shadow_validity.txt:9-14, tests/common/schedule/arb.rs:141-150, tests/common/sim.rs:330-333)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the seed file; `git log -S` on the hash: added 80a3155f 2026-05-18, consolidated 2720a1ff 2026-08-13; the `fork_parents` draw entered the strategy at 1af0ab9a 2026-06-03; the in-file disposition note was added at ce3664dd 2026-09-01; arb.rs:141-150 draws `n_peers - 1` raw parents before the choices, so the same RNG seed now generates an unrelated schedule)
- Seen by: blind-spots; refutation: confirmed with date corrections; history: already-known (a pending owner ruling recorded at 390160a7, 2026-08-13, has persisted across three commits)
- Owner-gated: yes: the seed's disposition is the owner's ruling

The third `cc` line in proptest-regressions/shadow_validity.txt records a `Schedule` with no `fork_parents` and a bare observed-log tuple; the current strategy draws parents before the choices and yields `ShadowFinal`, so replaying that seed regenerates an unrelated case. The file annotates this and asks for a ruling. The doctrine says every contradiction resolves to a fix or a declared model, and a pending status that persists across commits is a process failure. The seed sweep cannot see the class: `collect_regressions` judges only that a `.txt` reverse-resolves to a live source, never whether its note still matches the strategy, so the next draw-order change (finding 25 adds one) will orphan seeds the same way, caught only by the comment convention at sim.rs:330-333.

Evidence:

       132	/// A `.txt` must reverse-resolve to a live source whose deepest anchor
       133	/// is one of `anchors`, and any other file is an orphan outright —
       134	/// proptest writes and reads only `<suffix>.txt` here.

    (proptest-regressions/shadow_validity.txt:9-14; the cc line is elided after its opening)
    # The next entry predates the current Schedule strategy shape (its shrink
    # note lacks fork_parents), so it no longer replays the failure it was
    # written for. It awaits owner disposition; do not strip it without a
    # ruling. Proptest reads only the hash before the first '#' on a cc line,
    # so this comment and the stale shrink note cost nothing at replay.
    cc 86723fa839f009875eafd473501eceafe68d5e6b00c28572965fb6fa4da8a381 # shrinks to (schedule, shadow_observed) = (Schedule { n_peers: 3, events: [...]

Resolution: owner rules on the entry: delete it (the failure it recorded is fixed, and its replay under the current strategy is a different case) or re-derive it by reintroducing the fixed defect on a branch and letting proptest write a fresh seed. Then extend seed_liveness.rs so a shrink note whose field skeleton disagrees with the strategy's current value type fails the sweep (per suite, a required-field list derived from one generated value's Debug rendering, or a per-suite regex), with a fixture seed demonstrating the verdict. Acceptance: no `cc` line in proptest-regressions/ carries a note lacking a field the current strategy always emits; the disposition comment is gone; seed_liveness fails a fixture seed whose note lacks a required field.

### tests-bookmark-2: `LINK_BUF` and the heal-round cap are re-declared per suite, one value contradicting a sibling's "matching" claim
- Where: tests/bookmark_attach.rs:19-20 (related: tests/bookmark_causality.rs:78-80, 82-85, 760; tests/bookmark_transmit_window.rs:37, 47-53, 461-463; tests/bookmark_when.rs:68-69; tests/common/wire.rs:61-63; tests/common/sim.rs:106-107; tests/bootstrap.rs:24-27)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'const LINK_BUF' tests/` shows ten definitions; `git log -S` places "matching the sibling bookmark" at 077b64db and "the mirror protocol alternates" at b7fb409f; `src/protocol.rs:15-19` holds only `V2`)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (the shared constant is younger than three of the suites; causality's 8K rationale was written for the pre-streaming alternating mirror and carried through the V2 port unchanged)
- Owner-gated: no

`common::wire::LINK_BUF` is already `pub const LINK_BUF: usize = 8 * 1024`. Three suites re-declare it at the same value; attach declares `64 * 1024` with no reason, which makes transmit's "matching the sibling bookmark suites" false for one sibling. Causality's rationale names an alternating mirror protocol the crate no longer has. The heal-round cap is spelled three times with two semantics: `MAX_HEAL_ROUNDS_PER_PEER` is multiplied by `n()` at L760, `MAX_HEAL_ROUNDS` is a total at L461-463, and `sim::MAX_QUIESCE_ROUNDS_PER_PEER` is private. Transmit also imports `before::Version` (L37) where its siblings import the re-exported `rumors::Version`. Named constants live once; a hand-maintained cross-reference has already rotted.

Evidence:

        19	/// Capacity for each in-memory link stream carrying a bootstrap session.
        20	const LINK_BUF: usize = 64 * 1024;

    tests/bookmark_transmit_window.rs:
        47	/// Capacity for every in-memory link stream, matching the sibling bookmark
        48	/// suites.
        49	const LINK_BUF: usize = 8 * 1024;

    tests/bookmark_causality.rs:
        78	/// Capacity for every in-memory link stream; the mirror protocol alternates
        79	/// within a session, so a modest buffer suffices and exercises backpressure.
        80	const LINK_BUF: usize = 8 * 1024;

Resolution: import `crate::common::wire::LINK_BUF` in all four suites. If attach needs 64K for a bootstrap payload, state why at the one declaration (tests/bootstrap.rs:24-27 carries such a rationale; retire.rs, gossip_when.rs, reuse.rs, and party_conservation.rs also use 64K, so the crate holds two conventions worth unifying by whoever reviews those partitions). Make `sim::MAX_QUIESCE_ROUNDS_PER_PEER` public and use it for both heal loops. Import `rumors::Version` in transmit. Acceptance: `grep -rn 'const LINK_BUF' tests/bookmark_*.rs` is empty; one heal cap is defined in `tests/common` and imported; no bookmark suite mentions an alternating mirror.

See also: tests-observation-32.

### tests-bookmark-14: Asserts implied by the checks before them: the vacuity floor in `assert_live_content_is_durable` and the model self-check in the schedule proptest
- Where: tests/bookmark_causality.rs:885-892 (related: tests/bookmark_causality.rs:160-163, 868-883; tests/bookmark_when.rs:276-278, 284-371, 734-739, 765-775)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read: the loop at L870-883 asserts `contains_exact` for every leaf, so `live_leaves > 0` entails a non-empty log; every `Model` transition that clears `pending` sets `loaded` in the same statement)
- Seen by: structure-prose (both), blind-spots (the causality site); refutation: confirmed both, and rejected structure-prose's alternative floor (every message can be legitimately redacted or crash-lost); history: no rationale found for the causality assert (implied from birth); the model assert's rationale is stated inline but is internal to the model
- Owner-gated: no

Two asserts cannot fail independently of the checks before them. In causality, if `live_leaves > 0` then at least one `contains_exact` returned true, so `emissions.len() >= 1`; the guarded assert is unreachable and is the only caller of `EmissionLog::len`. In when, `pending || loaded` holds by inspection of every `Model` transition (`pristine_seed` sets `pending`, `bootstrap_fork` sets `loaded`, `local_change` sets `pending`, `plain_gossip` and `absorb_retire` set `loaded` where they clear `pending`), so the assert checks test scaffolding against itself; the observable read-before-write property is already asserted on the log at L765-775. A guard is justified by naming a concrete failure existing checks miss (Principle 3).

Evidence:

       885	        if self.next_seq > 0 && live_leaves > 0 {
       886	            assert!(
       887	                self.emissions.len() > 0,
       888	                "{live_leaves} live message leaves survived after {} sends, but the durable \
       889	                 emission log is empty",
       890	                self.next_seq,
       891	            );
       892	        }

    tests/bookmark_when.rs:
       734	                // The load never lags the first write: a checkpoint is cleared
       735	                // only by a write, and a write is always preceded by the read.
       736	                assert!(
       737	                    world.model.pending || world.model.loaded,
       738	                    "model reached `!pending && !loaded`, which a write cannot produce",
       739	                );

Resolution: delete L885-892 and `EmissionLog::len`; delete when L734-739 and keep the invariant statement in `Model`'s doc at L276-278. Do not replace the causality floor with "`next_seq > 0` implies some node holds content": every sent message can be legitimately redacted or crash-lost. Acceptance: the per-leaf witness loop and the global log checks remain; neither deleted block has a caller.

### tests-bookmark-18: Paired near-duplicates in the causality harness
- Where: tests/bookmark_causality.rs:1188-1196 (related: tests/bookmark_causality.rs:278-305 vs 321-362, 496-535 vs 781-806, 809-818 vs 826-837, 993-1003 vs 1050-1059, 1026-1039 vs 1196-1213)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read each pair)
- Seen by: structure-prose, api-economics (the fingerprint-type item, folded in); refutation: confirmed; history: no rationale found (all birth-state)
- Owner-gated: no

`run_reliable_plan`'s own doc names it as `run_plan` plus one crash guard, and its body repeats the whole `match step` dispatch. Likewise `World::seed` and `single_network` both hand-build the `Node` literal; `clean_gossip` is `gossip` with `FaultPlan::NONE` and results expected; `arb_reliable_step` repeats `arb_step` with other weights; `assert_healed` re-derives the `(hash, latest)` fingerprint that `fingerprints` computes, spelling the tuple type by hand where `let mut reference = None;` would infer it. A 1350-line harness reads faster when each mechanism appears once, and parallel copies invite fix-one-forget-the-other drift.

Evidence:

      1188	/// Execute a reliable-recovery plan to its post-heal end state.
      1189	///
      1190	/// The fleet shares one network from the start, and the only departure from
      1191	/// [`run_plan`] is the crash guard: a crash that would extinguish the network —
      1192	/// leaving no live member for the victim to later reboot from — is skipped, so
      1193	/// that the precondition "every party which restarted eventually restores
      1194	/// itself" holds by construction. (Retirement never extinguishes the network:
      1195	/// the absorber stays live.)

Resolution: add `Node::live(peer, store, faults, label)`; have `gossip` return both outcomes so `clean_gossip` is a call plus two `expect`s; one `fn run(world, steps, guard_crashes: bool)`; `assert_healed` compares `self.fingerprints()` entries pairwise. If the two step strategies must keep distinct weights, parameterize one strategy by a small weights struct, preserving generation order so the committed seeds still replay. Acceptance: each of the five mechanisms has one body; both proptests' committed seeds replay unchanged.

### tests-bookmark-20: transmit_window repeats the A-seed prelude four times and the dominance assertion twice, bypassing `Scene`
- Where: tests/bookmark_transmit_window.rs:324-337 (related: tests/bookmark_transmit_window.rs:206-212, 217-270, 286-303, 373-390, 502-511, 583-592)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no rationale found (ccd88401 acknowledged the copy inline and explained why `Scene` could not be reused, not why staging was not factored)
- Owner-gated: no

The seed-A-with-gated-bookmark-then-send-M0 prelude appears verbatim at L220-228, L327-335, L502-510, and L583-591; the comment at L324-326 acknowledges the copy. The record-dominates-transmitted assertion block (party alias, fold of recorded clocks, `b.snapshot().latest()`, two projections, assert) appears at L286-303 and L373-390 differing only in the message. `Scene` exists to carry the staged world but the cancelled-persist test bypasses it because the gate must be armed between staging and the session. Two assertion copies can drift apart in what they project.

Evidence:

       324	        // The scene through the two bootstrap serves, exactly as
       325	        // `transmit_during_persist` stages it: token cleared by the
       326	        // donations, record persisted at M0's frontier.
       327	        let store_a = DurableStore::default();
       328	        let bm_a = GatedBookmark::new(store_a.clone());

Resolution: split staging from the gated session: `async fn stage() -> Scene` (seed A, M0, boot B and C) and `async fn gated_session(&Scene)`; the cancelled-persist test calls `stage()`, sends M1, arms, and aborts. Extract `fn assert_record_dominates_transmitted(a, b, store_a, context: &str)` for L286-303 and L373-390. The two donation tests share a `seed_a_with_b()` prelude. Acceptance: one prelude body and one dominance-assertion body in the file; both tests that need the gate call the same staging function.

### tests-bookmark-25: `Instrument::pristine_seed` duplicates `birth(Origin::Seed)`; `Birth`, `World`, and `Instrument` overlap; the read/write count is spelled out seven times
- Where: tests/bookmark_when.rs:138-160 (related: tests/bookmark_when.rs:168-175, 393-398, 410-426, 608-613, 707-716, 749-757; the filter expression at 172, 472, 528, 561, 566, 754, 768)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the inline at L750-757 is forced because `retire_subject(world.probe.subject, ...)` at L749 partially moves `world.probe`)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (birth-state bb6ce27e)
- Owner-gated: no

`Instrument::pristine_seed` is `birth(Origin::Seed)` with the same expect and assert messages. `Birth` is `Instrument` plus `model` and `helpers`; `World` is the same plus `next_msg`; the proptest rebuilds an `Instrument` from `Birth`'s parts. The expression `.iter().filter(|e| **e == Io::Read).count()` appears seven times, and L750-757 re-implements `counts_since` inline because the method receiver has been moved out: the tell that the count belongs on the log, not the instrument.

Evidence:

       146	            let subject = Peer::<u64>::seed()
       147	                .sync_window_floor()
       148	                .bookmark(probe)
       149	                .await
       150	                .expect("a pristine seed attaches without touching storage");
        ...
       411	            let subject = Peer::<u64>::seed()
       412	                .sync_window_floor()
       413	                .bookmark(probe)
       414	                .await
       415	                .expect("a pristine seed attaches without touching storage");
        ...
       750	                let (reads, writes) = {
       751	                    let log = log.lock().unwrap();
       752	                    let slice = &log[cursor..];
       753	                    (
       754	                        slice.iter().filter(|e| **e == Io::Read).count(),
       755	                        slice.iter().filter(|e| **e == Io::Write).count(),
       756	                    )
       757	                };

Resolution: make `birth` return `World` (or fold `Birth` into `World`) and define `Instrument::pristine_seed` as `birth(Origin::Seed)`; add `fn delta(log: &[Io]) -> Delta` and use it in `counts_since`, the anchor tests, the retire tail, and the global read-once check. Acceptance: one construction path per origin; `filter(|e| **e == Io::Read)` appears once in the file.

### tests-disruption-handshake-2: Hand-rolled environment-variable codec for the child-process script
- Where: tests/disruption.rs:484-506 (related: tests/disruption.rs:455-460, 777-785, 889-901; Cargo.toml:69, 144-172)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure-prose [13]; refutation: confirmed; history: no rationale found (original to 9eadfc68)
- Owner-gated: no

The parent serializes a ChildPlan into six environment variables through four hand-written encode and decode functions, with "-" as the None sentinel and ":" and "," as separators, and the child re-parses them. It is a second grammar for ChildPlan's shape that must track the struct by hand; the doctrine prefers a library over a hand-rolled format. serde_json is a workspace dependency (Cargo.toml:69) but not among rumors' dev-dependencies (144-172); ciborium is already a regular dependency.

Evidence:

       484	fn encode_cut(cut: Option<usize>) -> String {
       485	    cut.map_or_else(|| "-".to_owned(), |n| n.to_string())
       486	}
       487	
       488	fn decode_cut(s: &str) -> Option<usize> {
       489	    (s != "-").then(|| s.parse().expect("malformed cut budget"))
       490	}
       491	
       492	fn encode_fault(fault: &FaultPlan) -> String {
       493	    format!(
       494	        "{}:{}",
       495	        encode_cut(fault.write_cut),
       496	        encode_cut(fault.read_cut)
       497	    )
       498	}

Resolution: Derive Serialize and Deserialize on FaultPlan (tests/common/fault.rs) and ChildPlan, carry `{index, plan}` in one environment variable beside CHILD_ADDR through serde_json (add it to rumors' dev-dependencies) or ciborium, and delete encode_cut, decode_cut, encode_fault, decode_fault and the split-and-parse in child_main. Acceptance: two environment variables in the child protocol (address and plan); no string codec remains in tests/disruption.rs.

### tests-disruption-handshake-12: Local copies of common::wire helpers in gossip_when.rs and handshake_liveness.rs
- Where: tests/gossip_when.rs:61-65 (related: tests/gossip_when.rs:54, 57, 896; tests/reuse.rs:26, 31, 52-56; tests/handshake_liveness.rs:69-75; tests/common/wire.rs:144-158, 213-218)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read tests/reuse.rs:1-60 and tests/common/wire.rs in full; grep 'fn pair()' tests returns exactly the two definitions)
- Seen by: structure-prose [11], [12]; refutation: confirmed, plus a new nit (gossip_when.rs:896 bypasses the file's own `links()`); history: 11 no rationale found (both copies born a day apart while bootstrap_fork_async already existed), 12 deliberate but expired (gossip_over predates gossip_pair_async, added at 0d48b153)
- Owner-gated: no

gossip_when's `pair()`, `DEADLINE` (10 s), and `LINK_BUF` (64 KiB) are body-for-body the same as tests/reuse.rs:26, 31, 52-56, rationale comments included. handshake_liveness's `gossip_over` is `common::wire::gossip_pair_async` with the link capacity parameterized instead of fixed at LINK_BUF. Duplicated helpers drift independently, and the clean-drain invariant should have one enforcement site. Also, gossip_when.rs:896 builds its link with `rumors::link::memory_with_capacity(LINK_BUF)` directly while every other test in the file uses `links()`.

Evidence:

        61	async fn pair() -> (Rumors<u64>, Rumors<u64>) {
        62	    let a: Rumors<u64> = Peer::seed().sync_window_floor().into_rumors();
        63	    let b = bootstrap_fork_async(&a).await;
        64	    (a, b)
        65	}

    tests/handshake_liveness.rs
        69	async fn gossip_over(a: &Rumors<u64>, b: &Rumors<u64>, capacity: usize) {
        70	    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(capacity);
        71	    let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
        72	    a_out.expect("gossip completes on side A");
        73	    b_out.expect("gossip completes on side B");
        74	    assert_control_drained(a_link, b_link);
        75	}

Resolution: Add `seeded_pair_async<T>()` (floor-pinned seed plus fork) and a generous-deadline constant to tests/common/wire.rs and use them from gossip_when.rs and reuse.rs; give wire.rs a `gossip_pair_with_capacity_async(a, b, capacity)` that `gossip_pair_async` calls with LINK_BUF, and delete `gossip_over`. Use `links()` at gossip_when.rs:896. Acceptance: one definition of the seeded pair under tests/; handshake_liveness.rs defines no session-driving helper of its own.

See also: tests-lifecycle-6.

### tests-disruption-handshake-27: The shape/cell split, the "matrix" framing, and the "V2" qualifiers are residue of the retired protocol column
- Where: tests/handshake_liveness.rs:370-375 (related: tests/handshake_liveness.rs:1, 167-171, 173-178, 368-431)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (git show 368da2a5 -- tests/handshake_liveness.rs: every shape lost its `protocol` parameter and the section comment changed from "The `_v1`/`_v2` cells below instantiate each shape per protocol" to "The test cells below instantiate one shape each"; grep -c V2 is 9, one per cell doc)
- Seen by: structure-prose [9]; refutation: confirmed; history: deliberate but expired (b37c2d45 built shapes x protocols; 368da2a5 removed the column and kept the indirection)
- Owner-gated: no

Nine async shape functions each have a one-line `#[test]` wrapper with a second doc comment restating the shape's doc. Before 368da2a5 the wrappers were the `_v1`/`_v2` cells of a shapes-by-protocol grid, which the "matrix" word (line 1 and the header at 368) and the "V2" in every cell doc served. With one dialect the qualifier distinguishes nothing and the two-layer structure costs nine pairs of docs that must agree. Machinery outlives the constraint that justified it.

Evidence:

       370	/// A V2 session between converged seasoned replicas stays live and
       371	/// re-converges over a one-byte-window link.
       372	#[test]
       373	fn converged() {
       374	    block_on(converged_session());
       375	}

Resolution: Inline each shape body into its `#[test] fn` inside `block_on(async { ... })`, keep one merged doc comment per test, replace "matrix" with the list it is, and drop "V2" from the cell docs (the dialect's name belongs where wire bytes are spelled). Acceptance: one function and one doc comment per session shape; `grep -c V2 tests/handshake_liveness.rs` is 0.

### tests-disruption-handshake-28: The `#[path]` inclusion of benches/support/latency.rs generates dead_code allows at every include site and inside the module
- Where: tests/hop_trace.rs:26-29 (related: tests/gossip_pipelining.rs:11-15; tests/window_corners.rs:15; tests/window_knee.rs:20; tests/window_operator.rs:25; tests/tradeoff_probe.rs:31; tests/latency_link.rs:14; benches/support/latency.rs:398-400, 463-466, 511-514, 534-537)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep -rn 'benches/support/latency.rs' tests benches: seven includers, each under `#[allow(dead_code)]`; the four per-item allows in latency.rs each carry the comment "the module is `#[path]`-included by several targets")
- Seen by: structure-prose [14]; refutation: confirmed; history: no rationale found (`#[path]` is Cargo's conventional idiom for sharing bench support with tests, which is why this is owner-gated)
- Owner-gated: yes: restructuring the support code into a dev-only crate or a feature-gated module is a build-surface decision

Seven test binaries compile latency.rs by `#[path]`, each needing `#[allow(dead_code)]`, and latency.rs carries four per-item allows each justified only by the inclusion mechanism. Infrastructure is suspect when it generates its own maintenance cascade; each allow exists because the support code sits in the wrong compilation unit.

Evidence:

        26	// Only the pipe layer is reused; the wire driver here is trace-aware.
        27	#[allow(dead_code)]
        28	#[path = "../benches/support/latency.rs"]
        29	mod latency;

Resolution: Move benches/support into a dev-only path crate under crates/ (or a `test-internals`-gated module of rumors) so it compiles once and unused items are simply unused; delete the allows and their comments. Acceptance: `grep -rn 'path = "../benches/support' tests` returns nothing; latency.rs has no `#[allow(dead_code)]`.

See also: benches-envelope-22, suite-economics-8.

### tests-disruption-handshake-30: Trace::hops reimplements latency::hops_on_lattice, and first_write_hop divides without the lattice check
- Where: tests/hop_trace.rs:137-143 (related: tests/hop_trace.rs:125-128, 165; benches/support/latency.rs:538-553)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both; latency.rs's reader is nanosecond-strict and carries `#[allow(dead_code)]` for exactly this include)
- Seen by: structure-prose [10], api-economics [46]; refutation: confirmed; history: deliberate but expired (hop_trace's local reader predates hops_on_lattice, added at 814f07ad without retargeting hop_trace)
- Owner-gated: no

hop_trace `#[path]`-includes latency.rs, whose `hops_on_lattice(elapsed, delay)` performs this division with the exactness assert; Trace::hops re-derives it in milliseconds, and first_write_hop divides with no assert at all, contradicting the doc's "exact, not a rounding". One reading for one quantity; the two readers have already drifted in strictness.

Evidence:

       137	        let millis = last.as_millis() as u64;
       138	        assert_eq!(
       139	            millis % DELAY.as_millis() as u64,
       140	            0,
       141	            "every traced event lands on an exact delay multiple"
       142	        );
       143	        millis / DELAY.as_millis() as u64

       165	        first.as_millis() as u64 / DELAY.as_millis() as u64

Resolution: Return `latency::hops_on_lattice(last, DELAY)` from hops() and `latency::hops_on_lattice(first, DELAY)` from first_write_hop(); drop the local assert and adapt the return type (u32). Acceptance: no `as_millis() ... / DELAY.as_millis()` arithmetic remains in tests/hop_trace.rs outside report()'s display code.

### tests-lifecycle-1: A committed seed annotated for a property that no longer exists
- Where: proptest-regressions/retire.txt:8-8 (related: tests/retire.rs:352-443, tests/seed_liveness.rs:207-242)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 1d3a3df4d -- tests/retire.rs` removes `fn retire_refused_iff_snapshots_outstanding(n in 0usize..4)` at diff line 658; both live properties in retire.rs take `(a_actions, b_actions)`; `tests/seed_liveness.rs:207-242` read in full, it checks path resolution only)
- Seen by: blind-spots; refutation: confirmed; history: deliberate-but-expired (the seed was committed in 89e447841 for a property deleted the next day; it survived the port and the 2026-09-01 relocation because the never-strip rule kept it out of every diff)
- Owner-gated: yes: AGENTS.md's never-strip rule makes any edit to a seed file an owner call

The second `cc` line records a shrunk counterexample `n = 1` for a property `tests/retire.rs` no longer contains; the header at retire.rs:16-18 states that case is unrepresentable. The line still replays as an RNG seed for the two live properties (harmless), but its annotation refers to code that does not exist, and `seed_liveness.rs` cannot see it because it checks that seed files resolve to live source paths, not that each `cc` line's parameters match a live property. The never-strip rule exists to preserve regression coverage; this line preserves none.

Evidence:

         8	cc f63e981a4e7979519ac37ccb1ac2e1f16d85925cb8b50228079c1d673e708005 # shrinks to n = 1

Resolution: Remove the line in a commit whose message names the deleted property, or record in the commit that it is retained deliberately. Consider extending `seed_liveness.rs` to parse each `cc` comment's parameter names against the live `proptest!` signatures in the owning file, so a deleted property's seeds surface mechanically. Acceptance: retire.txt carries only `cc` lines whose parameter names match a live property in tests/retire.rs, or the retention is recorded; if the sweep is extended, a fixture with a mismatched `# shrinks to` fails it.

### tests-lifecycle-8: `seeded()` is copied into four binaries, one copy documenting the duplication
- Where: tests/bootstrap_snapshot.rs:33-43 (related: tests/retire_snapshot.rs:35-44, tests/network.rs:16-22, tests/gossip_snapshot.rs:31, tests/opening_supply.rs:25-27)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'fn seeded' tests/`; opening_supply.rs:27 omits `sync_window_floor`, so it is a different fixture; observe.rs:258 takes an observer and is distinct)
- Seen by: structure-prose, api-economics; refutation: reframed (four identical copies modulo the stream parameter; opening_supply's is a distinct fixture); history: no-rationale-found (the "Mirrors" sentence was added in a style pass that documented the duplication instead of removing it)
- Owner-gated: no

bootstrap_snapshot.rs, retire_snapshot.rs, network.rs (parameterized by stream), and gossip_snapshot.rs each define the fixed-RNG seed at the floor; the bootstrap_snapshot copy's doc says "Mirrors `gossip_snapshot::seeded`", naming the duplication rather than removing it. The copies diverge in trivia (`SeedableRng as _` versus `SeedableRng`; fully qualified serde bounds beside imports of the same names three lines up). One helper in `common` is one place to change when `seed_rng`'s signature or the floor policy moves.

Evidence:

        33	/// A provider seeded from a fixed RNG, so the [`rumors::Network`] id carried in
        34	/// the preamble — and the party region it forks off for the newcomer — are
        35	/// deterministic and these captures stay reproducible.
        36	///
        37	/// Mirrors `gossip_snapshot::seeded`.
        38	fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>()
        39	-> Rumors<T> {
        40	    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
        41	        .sync_window_floor()
        42	        .into_rumors()
        43	}

Resolution: Add `pub fn seeded_floor<T>(stream: u64) -> Rumors<T>` to `common::wire` (network.rs's shape subsumes the zero-stream callers); delete the four copies and the "Mirrors" sentence; leave opening_supply's default-window fixture alone or give the helper a `WindowChoice`. Acceptance: `grep -rn 'fn seeded' tests/*.rs` returns only helpers with a distinct signature; the `.snap` files are byte-identical (the fixture is unchanged).

See also: tests-wire-format-2.

### tests-lifecycle-14: The `(hash, latest)` fingerprint and the canonical identity-to-value map are re-implemented per suite instead of living in `common::oracle`
- Where: tests/pairwise.rs:39-45 (related: tests/multi_peer.rs:142-145, tests/retire.rs:126-127, tests/retire.rs:332-333, tests/retire.rs:378-379, tests/retire.rs:388-389, tests/retire.rs:420-421, tests/retire.rs:430-431, tests/common/peer.rs:149-152, tests/common/sim.rs:886-889, tests/membership.rs:53-58, tests/multi_peer.rs:86-91, tests/common/oracle.rs:63-100)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'hash(), .*latest().clone()' tests/` lists the sites; `diff` of membership.rs:53-58 against multi_peer.rs:86-91 is identical modulo the binding name; tests/common/oracle.rs read in full: it holds `readout`, `readout_multiset`, `version_key`, and no fingerprint or canonical-map lens)
- Seen by: structure-prose; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

The `(hash, latest)` fingerprint is a fn here, a closure in multi_peer.rs and in the harness's `quiesce_refs` and `sim::quiesce`, and an inline tuple at six sites in retire.rs; its type is spelled twice (pairwise.rs:42, common/peer.rs:156). The canonical map built from `resolved_versions` filtered by the oracle's redaction set appears verbatim in membership.rs and multi_peer.rs. `common::oracle` exists to hold these lenses; re-spelling them means a change to what a fingerprint is (say, adding the ceiling) touches ten sites.

Evidence:

        39	/// The `(hash, latest)` fingerprint of a peer: equal fingerprints mean the
        40	/// same live content *and* the same causal frontier — gossip between two
        41	/// peers with equal fingerprints is a guaranteed no-op.
        42	fn fingerprint<T>(k: &Rumors<T>) -> ([u8; rumors::MERKLE_HASH_LEN], Version) {
        43	    let snapshot = k.snapshot();
        44	    (snapshot.hash(), snapshot.latest().clone())
        45	}

Resolution: Add to `common::oracle` a `Fingerprint` type and `fn fingerprint<T>(&Snapshot<T>) -> Fingerprint`, and a `canonical_readout` on `ExecutionResult`/`MembershipExecutionResult` (or on `Oracle` taking `&resolved_versions`); replace the sites. Acceptance: `grep -rn 'hash(), .*latest().clone()' tests/` hits only common/oracle.rs; membership.rs and multi_peer.rs build the canonical map through one call.

See also: tests-common-18.

### tests-lifecycle-24: `async_known` is a relic: its qualifier contrasts with a removed surface, `send_all` expresses it, and its doc is false for five call sites
- Where: tests/retire.rs:37-42 (related: tests/retire.rs:112, tests/retire.rs:123, tests/retire.rs:201, tests/retire.rs:226, tests/retire.rs:304, tests/retire.rs:329, tests/retire.rs:193, tests/bootstrap.rs:6, tests/common/wire.rs:1, tests/async_wire.rs:1, tests/pairwise.rs:3-4)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 83edcd944^:tests/retire.rs` has `fn sync_known(peer: rumors::sync::Rumors<u64>, ..)` at line 47 beside `async_known`; `git log -S'pub mod sync' -- src/lib.rs` shows the module removed in 83edcd944 on 2026-07-16; `grep -n 'async_known(' tests/retire.rs` lists twelve call sites, five of which pass `seed` itself; retire.rs:193 already uses `send_all`; 212c6914 (HEAD's parent) added `send_all`)
- Seen by: structure-prose, api-economics; refutation: confirmed (adds the doc inaccuracy for the seed call sites); history: deliberate-but-expired (twice: the sync twin left on 2026-07-16, and `send_all` on 2026-09-01 removed the reason to route fixture inserts through `LocalAction`)
- Owner-gated: no

The helper's `async_` prefix distinguished it from a `sync_known` twin over `rumors::sync::Rumors`, a surface that no longer exists; the `known` half names a type renamed on 2026-06-11. Its body is `peer.send_all(vals.iter().copied()).unwrap(); peer`, which line 193 of the same file already writes. Its doc says it inserts into "a genuine bootstrap fork", but five of twelve call sites pass the seed itself. The section header at line 112 ("async behavioral tests") carries the same dead qualifier, as do common/wire.rs:1 and async_wire.rs:1 ("the *asynchronous* gossip path") and bootstrap.rs:6 ("Mirrors `async_wire.rs`'s setup"). pairwise.rs:3-4 ("there is no in-process `join`") carries the same negative space, more softly. Names and prose speak in the present tense; a qualifier that contrasts with removed code is a ghost reference in disguise.

Evidence:

        37	/// Build an async `Rumors<u64>` by inserting `vals` into a disjoint originator
        38	/// (a genuine bootstrap fork: its own party region, ready to originate).
        39	fn async_known(peer: Rumors<u64>, vals: &[u64]) -> Rumors<u64> {
        40	    let actions: Vec<LocalAction<u64>> = vals.iter().map(|&v| LocalAction::Insert(v)).collect();
        41	    build_local(peer, &actions)
        42	}

       112	// ---- async behavioral tests ---------------------------------------------

Resolution: Delete the helper; at each call site write `let a = bootstrap_fork(&seed); a.send_all([1, 2]).unwrap();` (or a two-line local `fn with(peer, vals)` if the expression form reads worse); drop the unused `LocalAction` import; rename the section header "behavioral tests"; drop "asynchronous" from wire.rs:1 and async_wire.rs:1 and the "Mirrors" clause from bootstrap.rs:6. Acceptance: `grep -rn 'async_known\|\*asynchronous\*\|async behavioral' tests/` returns nothing; retire.rs builds fixtures through `send_all`.

See also: tests-resource-link-window-3.

### tests-lifecycle-25: retire.rs keeps a point test its counting sibling strictly subsumes, an overlapping divergent pair, and a byte-identical oracle block
- Where: tests/retire.rs:114-141 (related: tests/retire.rs:322-348, tests/retire.rs:159-182, tests/retire.rs:296-320, tests/retire.rs:382-390, tests/retire.rs:424-432)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`diff` of retire.rs:121-127 against 327-333 is identical, and `gossip_absorbs_retiree_without_observations` adds `novel == 0` to the same three assertions; `diff` of 382-390 against 424-432 is identical; the divergent pair differ in fixture ([1]/[2] versus [1,2]/[3]) and in assertion (exact live list versus count))
- Seen by: structure-prose, api-economics; refutation: confirmed ("overlap", not subsumption, for the divergent pair); history: deliberate-but-expired (the `gossip_*` twins were distinct at birth because they counted the absorber's `on_message` callbacks, which the `retire_into_*` twins did not exercise; the callback removal and the shared-state port re-spelled that count as a drain and collapsed the distinction)
- Owner-gated: no

`retire_into_converged_peer_succeeds` asserts a strict subset of `gossip_absorbs_retiree_without_observations` over an identical setup. `divergent_retiree_reconciles_then_retires` and `gossip_learns_content_from_divergent_retiree` are two point instances of `unsynchronized_retire_matches_plain_gossip`, differing in fixture and in whether the exact live list or its count is asserted. The "Oracle" blocks of the two wire-equivalence properties are byte-identical. Duplicated oracle construction is where two copies drift (one gets a fix, the other keeps the bug); a subsumed point test adds run and reading time without coverage.

Evidence:

       120	fn retire_into_converged_peer_succeeds() {
       121	    let seed = Peer::<u64>::seed().sync_window_floor().into_rumors();
       122	    let a = async_known(bootstrap_fork(&seed), &[1, 2]);
       123	    let b = async_known(seed, &[3, 4]);
       124	
       125	    wire_gossip(&a, &b);
       126	    let pre = b.snapshot();
       127	    let (hash, version) = (pre.hash(), pre.latest().clone());

Resolution: Delete `retire_into_converged_peer_succeeds`; keep one divergent point test as the legible worked example beside the property, folding the exact-content assertion into it; extract `fn plain_gossip_oracle(a_actions, b_actions) -> Fingerprint` and call it from both properties. Acceptance: retire.rs has no two tests with identical setup lines, one plain-gossip oracle construction, and every assertion the deleted tests made present in a survivor.

### tests-lifecycle-27: reuse.rs takes a Tokio runtime and a wall-clock deadline where the harness's `block_on` witnesses a wedge deterministically
- Where: tests/reuse.rs:23-26 (related: tests/reuse.rs:61, tests/reuse.rs:97, tests/reuse.rs:136, tests/reuse.rs:193, tests/reuse.rs:238, tests/reuse.rs:70-74, tests/common/wire.rs:34-48, src/testing.rs:366-394, tests/lifecycle.rs:99, .config/nextest.toml:1-5)
- Class / severity / confidence: simplification / low / medium
- Provenance: verified (all five tests are `#[tokio::test(flavor = "current_thread")]` and the only Tokio facility used is `timeout`, at lines 70, 115, 145, 173, 199, 211, 242; `tokio::io::AsyncWriteExt::write_all` on the memory duplex needs no runtime; lifecycle.rs:99 already drives `wrap_link`ed links under `run_to_quiescence`; `run_to_quiescence` caps at `MAX_POLLS = 1_000_000` at testing.rs:376)
- Seen by: structure-prose, api-economics; refutation: confirmed with the `MAX_POLLS` caveat; history: deliberate-but-expired in part (the idiom predates `run_to_quiescence` by five weeks; ba3eacb1ad kept the repeated block "deliberately alone", but its stated reason concerns extraction, not the runtime)
- Owner-gated: no

`common::wire::block_on` returns `Quiescence::Stalled` the moment a closed-world future goes `Pending` without arranging a wake, which is exactly the wedge `DEADLINE` guards against, and the harness's own guidance reserves Tokio for tests that need "task spawning, timers, or networking" (wire.rs:44-48). A wall-clock threshold over a deterministic quantity relocates flakiness to the threshold: a stall here costs ten seconds and a timeout panic instead of an immediate `Stalled` at the failing poll. The recorded ruling in ba3eacb1ad declined to extract the repeated block; it did not rule on the runtime.

Evidence:

        23	/// Generous wall-clock bound: these sessions are in-memory and finish in
        24	/// microseconds, so hitting the deadline means lost bytes wedged a session,
        25	/// not a slow machine.
        26	const DEADLINE: Duration = Duration::from_secs(10);

Resolution: Make the tests plain `#[test]`s that call `block_on(async { tokio::join!(..) })` per round (per round, not once around the body: the 259-session epoch-wrap test should stay well inside `MAX_POLLS`); delete `DEADLINE`, the `timeout` calls, and the `Duration` / `tokio::time` imports. If the runtime flavor is deliberate (coverage under a real executor, which .config/nextest.toml's comment names as a stall class), say so in the module doc instead. Acceptance: reuse.rs has no `tokio::test`, `timeout`, or `DEADLINE`, or its module doc states why it runs under a real executor; a deliberately planted wedge fails with `Stalled` rather than after 10 s.

### tests-lifecycle-33: A hand-rolled Fisher-Yates over an inline LCG where `rand` and proptest already provide a shuffle
- Where: tests/single_peer.rs:109-124 (related: Cargo.toml:165, tests/bootstrap_snapshot.rs:25-26)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (Cargo.toml:165 `rand = { workspace = true, features = ["small_rng"] }` under `[dev-dependencies]`; the snapshot suites use `SmallRng`)
- Seen by: structure-prose; refutation: confirmed; history: deliberate-but-expired (no `rand` in the manifest when the LCG was written on 2026-05-18; `small_rng` arrived the next day; the "no extra dependency" comment was added two months later when the premise was already false)
- Owner-gated: no

The shuffle carries two unnamed 64-bit constants and a comment justifying it as avoiding a dependency, but `rand` with `small_rng` is already a dev-dependency, and proptest's `prop_shuffle` would make the permutation a first-class strategy input the shrinker can simplify. Prefer a dependency over hand-rolling; named constants over magic numbers; the stated premise is false for this crate.

Evidence:

       111	            // Fisher-Yates over an inline 64-bit LCG: deterministic
       112	            // from `seed`, no extra dependency; any step function whose
       113	            // high bits reduce to a uniform-enough draw over `0..=i`
       114	            // would do.
       115	            let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
       116	            for i in (1..v.len()).rev() {
       117	                state = state
       118	                    .wrapping_mul(6364136223846793005)
       119	                    .wrapping_add(1442695040888963407);

Resolution: Take `(values, shuffled) in vec(any::<u64>(), 0..=16).prop_flat_map(|v| (Just(v.clone()), Just(v).prop_shuffle()))`, or `values.shuffle(&mut SmallRng::seed_from_u64(seed))`; delete the LCG. Acceptance: no literal LCG multiplier or increment in single_peer.rs; the test passes under both committed proptest seeds.

### tests-observation-12: Helpers duplicated verbatim between a suite and its sibling or `tests/common`: `corpora` and `gossip_pair`
- Where: tests/observe.rs:291-302 (related: tests/wire_legibility.rs:104-116, tests/session_stats.rs:26-41, tests/common/wire.rs:144-158)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read all four bodies and compared; `git log -S'fn gossip_pair'` gives 7626543e (2026-07-24) for the session_stats copy and 0d48b153 (2026-08-13) for the shared one)
- Seen by: structure-prose [3] [4], api-economics [45]; refutation: confirmed; history: no rationale found for either copy (observe.rs copied wire_legibility.rs one hour after it landed, documenting the copy as "mirroring"; the shared `gossip_pair_async` was added beside the older private one rather than replacing it)
- Owner-gated: no

observe.rs's `corpora()` is byte-identical to tests/wire_legibility.rs:108-116 (payload `vec(any::<u8>(), 0..48)`, three sides `0..12`, same `#[allow]`), and its doc promises the two "mirror" each other: a hand-maintained invariant a shared definition makes structural. session_stats.rs's `gossip_pair` is line-for-line `common::wire::gossip_pair_async` (same capacity, same `tokio::join!`, same expects, same `assert_control_drained`); the `assert_control_drained` and serde imports at session_stats.rs:26-29 exist only to serve the copy.

Evidence:

    291	/// Arbitrary payload corpora, mirroring the wire-legibility suite's
    292	/// shape: enough variety to drive matches, queries, empty queries, and
    293	/// batched supply runs, small enough for many full sessions.
    294	#[allow(clippy::type_complexity)]
    295	fn corpora() -> impl Strategy<Value = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>)> {
    296	    let payload = vec(any::<u8>(), 0..48);

    (tests/session_stats.rs)
    32	async fn gossip_pair<T>(a: &Rumors<T>, b: &Rumors<T>) -> (Gossiped, Gossiped)
    ...
    36	    let (mut a_link, mut b_link) = rumors::link::memory_with_capacity(LINK_BUF);
    37	    let (a_out, b_out) = tokio::join!(a.gossip(&mut a_link), b.gossip(&mut b_link));
    38	    let pair = (a_out.expect("gossip A"), b_out.expect("gossip B"));
    39	    assert_control_drained(a_link, b_link);
    40	    pair
    41	}

Resolution: Move `corpora` into `tests/common/gossip_snapshot.rs` (the capture harness both suites already import) and import it in observe.rs and wire_legibility.rs, dropping the "mirroring" sentence. Delete `session_stats::gossip_pair` and its serde imports; replace its six call sites (58, 83, 85, 108, 128, 320) with `gossip_pair_async`; drop `assert_control_drained` from the line-26 import. Acceptance: `grep -rn 'fn corpora\|fn gossip_pair' tests/*.rs` is empty.

### tests-observation-16: Braced single-statement blocks left by the batch-closure to `send_all` rewrite
- Where: tests/listen.rs:79-81 (related: tests/listen.rs:629-632, tests/listen.rs:653-656, tests/session_stats.rs:262-268, tests/session_stats.rs:309-316, tests/stale_floor.rs:35-37)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 212c6914 -- tests/listen.rs` shows the block previously held a multi-line `.batch(|batch| { .. }).expect(..)` chain)
- Seen by: structure-prose [5], api-economics [48]; refutation: confirmed; history: deliberate-but-expired (at 040a0d042 the braces scoped a Drop-committing batch guard; ce27df86 turned `Batch` into a closure scope, deleting that purpose; 212c6914 carried the braces forward around one `send_all`)
- Owner-gated: no

Six sites brace a single `send_all(..).unwrap()` in an otherwise empty block; inside `proptest!` bodies, which rustfmt cannot reach, the rewrite also left the receiver on its own line. A scoped block signals a lifetime or borrow that matters; here none does, and the reader stops to look for one.

Evidence:

    79	    {
    80	        rumors.send_all(0..8u64).unwrap();
    81	    }

    (tests/session_stats.rs)
    309	            {
    310	                a
    311	                    .send_all(a_sends.iter().copied()).unwrap();
    312	            }

Resolution: Unwrap the braces and re-flow to one line at listen.rs:79-81, 629-632, 653-656; session_stats.rs:262-268, 309-316; stale_floor.rs:35-37. Acceptance: none of the six sites has a bare `{` preceding a lone `send_all`; `just fmt-check` still passes.

### tests-observation-35: The `encoded_bits` assertions are implied by the preceding `Party` equality and are the sole reason the `meter` dev-feature is enabled
- Where: tests/party_conservation.rs:326-335 (related: tests/party_conservation.rs:314, tests/party_conservation.rs:377-381, tests/party_conservation.rs:29-32, Cargo.toml:146-149, crates/before/src/party.rs:82-85, crates/before/src/party.rs:597-600)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`Party: PartialEq` is `codec::canonical_eq`, a raw-byte compare under marker padding that pins the live length inside the final byte; `encoded_bits` is `self.0.len()`; `grep -rn encoded_bits src tests benches examples Cargo.toml` finds only this file and the Cargo.toml comment; rumors' own `meter` feature routes to `before/limb-meter` and `before/scan-meter`, which imply `meter`, so src/tree/tests.rs's `before::meter` use does not depend on the dev-dependency's feature)
- Seen by: structure-prose [7]; refutation: confirmed; history: deliberate-and-holds (6fef7eb3 wrote the size clause as the fragmentation bound's own denomination; 05d87e1b records Finch's direct ruling routing `encoded_bits` behind `meter` and lighting it in rumors' dev-dependency "for the conservation suite, whose invariant is unchanged"; that ruling did not consider the redundancy)
- Owner-gated: yes: reopens the 05d87e1b routing with new evidence (the implication)

Under `Party`'s byte-level equality, `now == baseline` implies `now.encoded_bits() == baseline.encoded_bits()`, so the two `encoded_bits` assertions recompute a deterministic O(1) function of values just asserted identical; recompute-and-compare on a pure function is not defense in depth. They are also the only consumer of the dev-dependency's `meter` feature (Cargo.toml:146-149 says so). The counter-argument the history pass makes is fair: the size assertion states the bound independently of how `PartialEq` happens to be implemented, and `Party`'s doc commits to byte-level equality by design (party.rs:76-81). Whether that independence is worth a dependency feature is the owner's call.

Evidence:

    326	            prop_assert!(
    327	                now == baseline,
    328	                "cycle {cycle}: the provider's party must return to its \
    329	                 baseline, got {now:?} vs {baseline:?}"
    330	            );
    331	            prop_assert_eq!(
    332	                now.encoded_bits(), baseline_bits,
    333	                "cycle {}: the party's encoded size must not grow with the \
    334	                 cycle count", cycle
    335	            );

    (crates/before/src/party.rs)
    82	impl PartialEq for Party {
    83	    fn eq(&self, other: &Self) -> bool {
    84	        codec::canonical_eq(&self.0, &other.0)
    85	    }
    ...
    597	    pub fn encoded_bits(&self) -> u64 {
    598	        // The stored form's O(1) length: exact at every size memory holds.
    599	        self.0.len()
    600	    }

Resolution: If the owner agrees: delete the `encoded_bits` assertions and `baseline_bits` (314, 331-335, 377-381); reword module-doc item 4 (29-32) to state the size bound as a consequence of bit-for-bit return ("so the encoded size cannot grow with churn"); drop `"meter"` from the `before` dev-dependency and its three-line comment in Cargo.toml. If the owner keeps them: add one sentence at 331 stating that the size check is kept as an implementation-independent statement of the bound. Acceptance: either `grep -rn encoded_bits src tests benches examples Cargo.toml` is empty and `just gate` passes with `before = { workspace = true, features = ["serde"] }` under `[dev-dependencies]`, or the retained assertion carries its rationale at the site.

### tests-resource-link-window-5: the allocator meters duplicate their scaffolding, and decode_alloc's run_body wrapper is bypassed
- Where: tests/decode_alloc.rs:20-25 (related: tests/encode_alloc.rs:18-23, tests/decode_alloc.rs:62-69, tests/encode_alloc.rs:58-65, tests/decode_alloc.rs:92-94, tests/decode_alloc.rs:276)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both files; `grep -rn 'static METER_LOCK' tests/` returns the two sites; `grep -c 'rumors::testing::' tests/decode_alloc.rs` returns 17)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no-rationale-found (encode_alloc created today, 0521b207, copying the block)
- Owner-gated: no

The `#[global_allocator]` static, `METER_LOCK` with its doc, and `metered<T>` are identical between the two binaries. Separately, decode_alloc.rs defines `run_body(len)` as a one-line wrapper over `rumors::testing::lone_record_run` and then calls `lone_record_run` directly at line 276, so the wrapper is half-adopted; and the file fully qualifies `rumors::testing::` at seventeen call sites where one `use` line would do (imports over long qualified paths).

Evidence:

    20	#[global_allocator]
    21	static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;
    22	
    23	/// Serializes metered regions: the allocator counters are process-global,
    24	/// so concurrent tests would attribute each other's traffic.
    25	static METER_LOCK: Mutex<()> = Mutex::new(());

    92	fn run_body(len: usize) -> Vec<u8> {
    93	    rumors::testing::lone_record_run(len)
    94	}

    276	        body.extend_from_slice(&rumors::testing::lone_record_run(HONEST_LEN / 2));

Resolution: Either merge the two meters into one `alloc_meter.rs` binary (one allocator, one lock, one `metered`), or extract a `tests/support/meter.rs` holding `ALLOCATOR`, `METER_LOCK`, and `metered`, `#[path]`-included by both (the idiom the window suites use for latency.rs; a `#[global_allocator]` cannot live in tests/common because every binary including `common` would inherit it). In decode_alloc.rs, use `run_body` at line 276 or delete the wrapper, and import the `rumors::testing` functions once. Acceptance: `grep -rn 'static METER_LOCK' tests/` returns one site; decode_alloc.rs has no fully qualified `rumors::testing::` call and one spelling of the run-body constructor.

### tests-resource-link-window-11: opening_supply shadows the imported path_radix with an identical closure; over-generic seeded<T>; "These tests" for one test
- Where: tests/opening_supply.rs:103-106 (related: tests/opening_supply.rs:18, tests/opening_supply.rs:21, tests/opening_supply.rs:25-28, tests/opening_supply.rs:8, tests/opening_supply.rs:114, tests/common/shape.rs:17-24, tests/hop_trace.rs:535-544)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (shape.rs:22-24 defines `path_radix` as `leaf_path(version)[0]` with `leaf_path` the SHA3-256 digest; line 21 imports it and lines 85 and 94 use it; `seeded` has one call site at `T = u64`, line 80; the module has one `#[test]`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the closure predates the shared helper by two hours and survived the SHA3 sweep 4f18c347 as a duplicate hash site)
- Owner-gated: no

The fixture imports `common::shape::path_radix` for staging and then rebinds the same name to a closure computing the same formula for its self-checks, pulling in `use sha3::Digest;` for that alone; the comment frames it as an independent check but it is the same derivation, so a reader must verify two definitions agree for no gain. `seeded<T>` carries a five-bound generic signature for one `u64` call; the module doc says "These tests pin" over one test. hop_trace.rs:535-544 (another partition) carries the same closure.

Evidence:

    103	    // Fixture self-checks: one shared radix, disputed; the subtree holder
    104	    // is the smaller set and initiates. A leaf's path is the full-width
    105	    // SHA3-256 hash of its version's canonical bytes.
    106	    let path_radix = |version: &Version| sha3::Sha3_256::digest(version.as_bytes())[0];

Resolution: Delete the closure and the `sha3::Digest` import, using the imported `path_radix` in the self-checks (they remain valid checks that staging landed the shape); drop the `radix` rebinding at 114; make `seeded` return `Rumors<u64>`; write "This test pins" or split the module doc's two claims into two tests. Acceptance: one `path_radix` in scope, no `sha3` import, `seeded() -> Rumors<u64>`.

### tests-resource-link-window-13: routed_link repeats the establishment tail three times and the timeout wrapper five times; vestigial braces around send_all
- Where: tests/routed_link.rs:76-90 (related: tests/routed_link.rs:53-68, tests/routed_link.rs:94-101, tests/routed_link.rs:104-128, tests/routed_link.rs:169-192, tests/routed_link.rs:198-216, tests/routed_link.rs:238-243, tests/routed_link.rs:259-278, tests/hop_trace.rs:550-552)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read the three constructors and five `timeout(` sites; `git log -L238,243:tests/routed_link.rs` lists b2675dbe, ce27df86 "rework Batch into a closure scope", 6e4b6eea, 212c6914 "send_all")
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the braces scoped a `Batch` guard, then a closure, then nothing)
- Owner-gated: no

`tcp_pair`, `memory_pair`, and `pooled_tcp_pair` end in the same nine lines (`tokio::join!(b.link(addr), a_incoming.accept())`, two expects, the `dialer_first` swap); `pooled_endpoint` re-spells `tcp_endpoint` for a different dial type; `tcp_conformance` names the `timeout(SUITE_TIMEOUT, check(..)).await.expect(..)` wrapper and four later tests inline it anyway. The two bare blocks at 238-243 are husks of a removed `Batch` scope. Maintenance surface with no information content.

Evidence:

    82	    let (linked, arrival) = tokio::join!(b.link(a_addr), a_incoming.accept());
    83	    let dialed = linked.expect("establishment succeeds");
    84	    let (_info, accepted) = arrival.expect("the router delivers the link");
    85	    if dialer_first {
    86	        (dialed, accepted)
    87	    } else {
    88	        (accepted, dialed)
    89	    }

    238	        {
    239	            seed.send_all(0..48u64).unwrap();
    240	        }
    241	        {
    242	            newcomer.send_all(48..96u64).unwrap();
    243	        }

Resolution: Write one generic `endpoint<D: Dial>` and one `establish<D>(b, a_incoming, addr, dialer_first)`, and one `conformance_under(pair)` wrapper used by all eight conformance tests; drop the braces (and consider naming 48/96 so line 249's `96` is derived). hop_trace.rs:550-552 carries the same braces. Acceptance: one spelling of the establishment tail and one of the timeout wrapper; no single-statement bare blocks in the file.

### tests-resource-link-window-14: target_message_size: near-duplicate pair builders, a thrice-spelled corpus seed, an unenforced OFF_DEFAULT guard, and "budget" for "target"
- Where: tests/target_message_size.rs:26-47 (related: tests/target_message_size.rs:79-106, tests/target_message_size.rs:40, tests/target_message_size.rs:97, tests/target_message_size.rs:275, tests/target_message_size.rs:160, tests/target_message_size.rs:304-307, src/tree/mirror/streaming/window.rs:275)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (read both builders; `0x5eed_0f1e_a55e_d000` at lines 40, 97, 275 and `seed_from_u64(0)` at 85, 272; no inequality assertion on `OFF_DEFAULT_BUDGET`; window.rs:275 makes the "512 MiB" accurate today)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no-rationale-found
- Owner-gated: no

`diverged_pair` and `seeded_diverged_pair` differ only in `Peer::seed()` versus `seed_rng(..0)` and in interleaved versus sequential sends; the network identifier is not part of a version or path, so the two convergence tests lose nothing by using the seeded builder. The corpus seed is a magic number spelled three times. `OFF_DEFAULT_BUDGET`'s doc argues the pin cannot pass by coincidence, but nothing asserts `OFF_DEFAULT_BUDGET != DEFAULT_SYNC_MEMORY_BUDGET`, and "512 MiB" is a hand-maintained restatement. Line 160's "If the budget silently regressed" names the wrong setting in a test about the target.

Evidence:

    26	fn diverged_pair(left_target: usize, right_target: usize) -> (Rumors<u64>, Rumors<u64>) {
    27	    block_on(async {
    28	        let left = Peer::seed()
    29	            .sync_window_floor()
    30	            .target_message_size(left_target)
    31	            .into_rumors();

    304	/// A budget far from [`rumors::DEFAULT_SYNC_MEMORY_BUDGET`]'s 512 MiB, so
    305	/// the wire-equality pin below cannot pass by the two configurations
    306	/// coinciding.
    307	const OFF_DEFAULT_BUDGET: usize = 1024 * 1024;

Resolution: Delete `diverged_pair`; route `zero_target_still_converges` and `mixed_targets_interoperate` through `seeded_diverged_pair(l, r, (DIVERGENT_PER_SIDE, DIVERGENT_PER_SIDE))`; name `const CORPUS_SEED: u64` and `const IDENTITY_SEED: u64`; add `const _: () = assert!(OFF_DEFAULT_BUDGET != DEFAULT_SYNC_MEMORY_BUDGET);` and drop the literal from the doc; write "target" at line 160. Acceptance: one pair builder; each seed literal once; a compile-time guard on the off-default budget.

### tests-resource-link-window-15: three binaries each own a parser for the rendered capture's signal line
- Where: tests/target_message_size.rs:122-130 (related: tests/target_message_size.rs:108-120, tests/target_message_size.rs:132-152, tests/opening_supply.rs:42-61, tests/gossip_snapshot.rs:161-178, tests/gossip_snapshot.rs:215-229, src/tree/mirror/streaming/remote/codec/capture.rs:173-212)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn 'split_once(" / ")' tests/` returns exactly opening_supply.rs:53, gossip_snapshot.rs:170, target_message_size.rs:126; the renderer is one function)
- Seen by: structure-prose, api-economics; refutation: confirmed (the three differ subtly: `is_supply_signal` never strips the trailing ` /`, `signal_semantic` additionally requires an uppercase first character); history: no-rationale-found (today's 3327a92b6 re-documented two of them separately)
- Owner-gated: no

`is_supply_signal`, `frames_labeled`, and `signal_semantic` all parse the renderer's `<state code> / <Semantic> /` line with the same bare-digit test, each with its own paragraph explaining why the digits disambiguate, and each restating the oracle caveat about relabeling. A renderer-vocabulary change (a sanctioned re-accept class) must be chased into three parsers; the maintenance cost has already been paid once.

Evidence:

    125	fn is_supply_signal(line: &str) -> bool {
    126	    let Some((code, rest)) = line.trim_start().split_once(" / ") else {
    127	        return false;
    128	    };
    129	    !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()) && rest.starts_with("Supply")
    130	}

Resolution: Move `signal_semantic` (the most general form) into tests/common/gossip_snapshot.rs as `pub fn`, add `pub fn frames_labeled(capture, prefix) -> usize` and a per-direction variant on top of it, and express all three suites' counts through them; keep the oracle-assumption paragraph once at the shared definition. Acceptance: `grep -rn 'split_once(" / ")' tests/` returns one site under tests/common.

### tests-resource-link-window-21: magic numbers in the window suites' admittance constants and hop bounds, with one latent coupling
- Where: tests/window_census.rs:33-40 (related: tests/window_census.rs:51, tests/window_census.rs:119, tests/window_corners.rs:99, tests/window_corners.rs:113, tests/window_corners.rs:132-136, tests/window_corners.rs:242, tests/encode_alloc.rs:128, src/testing.rs:323-333, src/tree/mirror/streaming/window.rs:132, src/tree/mirror/streaming/window.rs:137)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (read the sites; `window_capacities` returns `(0..=32)` entries; `FAN` is `pub(crate)` and `KEY_DEPTH` private, neither exposed through `rumors::testing`; encode_alloc.rs:128 spells the fan as `usize::from(u8::MAX) + 1`)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: no-rationale-found (the history pass notes the accessor constraint)
- Owner-gated: no

window_census restates the fan width as `256`, the height count as `33`, and the common prefix as `1_024` at two sites that must agree (line 51 sends it, line 119 recomputes `session_len` from it): change one and the admittance is computed for the wrong session. window_corners uses bare `12`, `64`, `8 * 2 *`, and `64 * delay` as bounds where window_knee names the same kind of bound (`PIPELINED_HOPS`, `KNEE_MARGIN`) with rationale. Named constants over magic numbers; no hand-maintained counts.

Evidence:

    33	/// Handles one buffered scope can pin at most: a full fan of child
    34	/// references plus its own bookkeeping.
    35	const HANDLES_PER_SCOPE: usize = 256 + 2;
    36	
    37	/// Handles the assembly fan queues can hold beyond the window: one full
    38	/// fan per active level (their capacity is a correctness floor the window
    39	/// never scales; see the window module docs).
    40	const ASSEMBLY_FAN_HANDLES: usize = 33 * 256;

Resolution: In window_census: `const COMMON: usize = 1_024;` used at both sites; derive the height count from `capacities.len()` at the use site; derive the fan from `usize::from(u8::MAX) + 1` as encode_alloc does, or expose `FAN` through `rumors::testing`. In window_corners: name `LADDER_HOPS_BOUND` (12), `WAVE_FLOOR_HOPS` (64), and the linear-cost factor, carrying the rationale the inline comments at 93-97 already give. Acceptance: no bare `1_024`, `256`, `33`, or repeated `12`/`64` outside a named constant in the two files.

### tests-resource-link-window-26: zero_budget_serializes_but_completes builds and runs its slowest pair twice because session_hops discards the reconciled handles; tradeoff_probe copies the primitive with a rounding reader
- Where: tests/window_corners.rs:128-129 (related: tests/window_corners.rs:139-140, tests/window_corners.rs:31-41, benches/support/latency.rs:515-522, benches/support/latency.rs:538-553, tests/tradeoff_probe.rs:100-125, tests/tradeoff_probe.rs:124, tests/tradeoff_probe.rs:173)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (window_corners.rs:128-129 call `pair(0, 2_048, divergence, divergence)` twice; latency.rs:520 reads `let (_pair, elapsed) = wire.round_trip_virtual(a, b);`; tradeoff_probe.rs:124 reads `(elapsed.as_millis() / DELAY.as_millis()) as u64`; `pair`'s doc at 31-32 omits `common`, and every caller passes `common >= 1`, so `common.max(1)` at line 41 is dead)
- Seen by: structure-prose, blind-spots, api-economics; refutation: confirmed; history: deliberate-but-expired (the second build served dd2b1645d2's two-delay differencing, retired by 814f07ad1; the probe's copy is a parallel-branch artifact predating `hops_on_lattice`)
- Owner-gated: no

The test measures a 2,048 + 2,000 + 2,000 floor session through `hops(pair(...))`, whose `session_hops` throws away the pair `round_trip_virtual` returns, then builds the identical pair again and runs a second full serialized session purely to check the outcome; the convergence check thereby certifies a different session than the one measured. tradeoff_probe works around the same signature by copying `session_hops` (100-125), replacing `hops_on_lattice`'s exactness assertion ("drift fails loudly instead of rounding silently", latency.rs:530-533) with truncating integer division and returning `u64` where every sibling returns `u32`, which is what forces the `u32::try_from(transfer).expect("small")` at line 173.

Evidence:

    128	    let measured = hops(pair(0, 2_048, divergence, divergence));
    129	    let (left, right) = pair(0, 2_048, divergence, divergence);

    latency.rs:
    519	    let mut wire = DelayedWire::new(capacity, delay);
    520	    let (_pair, elapsed) = wire.round_trip_virtual(a, b);
    521	    hops_on_lattice(elapsed, delay)

    tradeoff_probe.rs:
    124	    (elapsed.as_millis() / DELAY.as_millis()) as u64

Resolution: Have `session_hops` return `((Rumors<T>, Rumors<T>), u32)` (or a sibling that does); window_corners builds its pair once and asserts on the returned handles; tradeoff_probe collapses `wire_hops` onto it (keeping its `hash()` convergence assertion). Also give `pair`'s doc its `common` parameter and delete the dead `.max(1)`. Acceptance: one `pair(...)` call in the test; no `as_millis` division in tradeoff_probe; every hop count flows through `hops_on_lattice`.

### tests-wire-format-2: Fixture helpers the harness should own are re-spelled per suite: seeded, version_for, the leaf-prefix collector
- Where: tests/gossip_snapshot.rs:28-47 (related: tests/gossip_snapshot.rs:109-116, 276-283, 562-571; tests/bootstrap_snapshot.rs:33-43; tests/retire_snapshot.rs:40-44; tests/opening_supply.rs:25-28, 88; tests/network.rs:18-22; tests/observe.rs:258-261; tests/hop_trace.rs:546; tests/target_message_size.rs:85, 272; tests/listen.rs:484; tests/retire.rs:197; tests/changes.rs:49; tests/common/shape.rs:17-24)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -rn -E 'fn seeded|seed_rng\(' tests/` lists ten test files; `grep -rn -E 'fn version_for|then_some\(v\.clone\(\)\)' tests/` lists four; `grep -rn -F '[path[0], path[1]]' tests/` lists exactly the three gossip_snapshot sites)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (bootstrap_snapshot.rs's copy has said "Mirrors `gossip_snapshot::seeded`" since 2026-06-19; `common::shape` was extracted on 2026-08-18 and left the three inline prefix collectors in place)
- Owner-gated: no

Three fixture helpers are duplicated across suites rather than living in `tests/common`: the fixed-RNG floor-window peer constructor (ten copies, two of which already differ: opening_supply.rs omits `sync_window_floor()`, network.rs parametrizes the seed), the read-path version lookup (one named function plus three inline `find_map` copies that drop its uniqueness precondition and named panic), and the two-byte leaf-prefix collector with its hand-written pair-shape self-check (three copies in this file, beside a `common::shape` module that owns `leaf_path`, `path_radix`, and `shaped_pair`). Duplicated helpers drift independently; `tests/common` exists to own exactly this.

Evidence:

    31	fn seeded<T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static>()
    32	-> Rumors<T> {
    33	    Peer::seed_rng(&mut SmallRng::seed_from_u64(0))
    34	        .sync_window_floor()
    35	        .into_rumors()
    36	}

    41	fn version_for(rumors: &Rumors<u64>, value: u64) -> Version {
    42	    rumors
    43	        .snapshot()
    44	        .iter()
    45	        .find_map(|(v, m)| (*m == value).then_some(v.clone()))
    46	        .unwrap_or_else(|| panic!("no live message holds {value}"))
    47	}

    109	    let prefixes: Vec<[u8; 2]> = a
    110	        .snapshot()
    111	        .iter()
    112	        .map(|(v, _)| {
    113	            let path = leaf_path(v);
    114	            [path[0], path[1]]
    115	        })
    116	        .collect();

Resolution: Add `common::wire::seeded<T>(stream: u64) -> Peer<T>` (returning the `Peer` so callers may add settings before `into_rumors()`), with the determinism rationale stated once; add `common::peer::version_of<T: PartialEq>(rumors, payload) -> Version` with the uniqueness precondition in its doc; add `common::shape::leaf_prefix` and an `assert_shaped` that re-verifies a landed pair. Replace every site. Acceptance: `grep -rn 'seed_rng(' tests/` and `grep -rn 'then_some(v.clone())' tests/` each hit only `tests/common`; `grep -c '\[path\[0\], path\[1\]\]' tests/gossip_snapshot.rs` is 0.

See also: tests-lifecycle-8.

### tests-wire-format-11: cbor_evolution runs sessions on a real tokio runtime and hand-rolls the cross-typed bootstrap that payload_depth also hand-rolls
- Where: tests/cbor_evolution.rs:65-87 (related: tests/cbor_evolution.rs:154-170, 178-214; tests/payload_depth.rs:140-156, 265-281, 331-343; tests/gossip_snapshot.rs:150-157; tests/wire_legibility.rs:161-172; tests/common/wire.rs:34-59, 244-264; .config/nextest.toml:25-26)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read `bootstrap_fork_configured` at wire.rs:244-264: private, single-typed, consumes the `Peer` via `into_rumors()`, and calls `assert_control_drained`, which none of the inline copies do; read wire.rs:44-48, whose doc reserves the tokio path for tests that "explicitly need Tokio facilities"; `grep -n -E 'tokio::test|tokio::spawn'` in the partition hits only cbor_evolution.rs)
- Seen by: structure-prose, api-economics; refutation: confirmed, severity lowered to low (the tests are correct today; the cost is diagnosis time and harness inconsistency); history: no rationale found (`bootstrap_fork_configured` landed same-typed on 2026-08-13; the later suites inlined their own joins)
- Owner-gated: no

Every other session-driving file in the partition runs under `common::wire::block_on` (`run_to_quiescence`), so a wire stall is reported at its source; payload_depth.rs:349-350 states that payoff for exactly the shape cbor_evolution uses. cbor_evolution instead uses `#[tokio::test]` with `tokio::spawn` and relies on `drop(near)` to unhang the server, so a stall here is reported only through nextest's 180-second kill. The cross-typed bootstrap (`a.gossip` joined with `Peer::<B>::bootstrap().join`) is spelled inline five times across cbor_evolution and payload_depth because `bootstrap_fork` is same-typed and has no builder hook, and any setting applied after a fork pays an async `try_into_peer` reclaim (payload_depth.rs:265-281 does three). The two failure tests also assert less than their comment: "its session then fails too" (166-167) is checked only as `let _ = server.await.expect(...)`, which discards the `Result`.

Evidence:

    73	    let (mut near, mut far) = rumors::link::memory();
    74	    let serve = sender.clone();
    75	    let server = tokio::spawn(async move { serve.gossip(&mut far).await.unwrap() });
    76	    let receiver = Peer::<B>::bootstrap()
    77	        .join(&mut near)
    78	        .await
    79	        .expect("the bootstrap session succeeds")
    80	        .expect("the sender is established")
    81	        .into_rumors();
    82	    server.await.expect("the serving task");

    165	    // Drop the failed side's link so the donor sees the transport close
    166	    // (a peer that errored out of a session hangs up); its session then
    167	    // fails too — but only ever as an error, never a panic.
    168	    drop(near);
    169	    let _ = server.await.expect("the serving task must not panic");

Resolution: Generalize `bootstrap_fork_configured` over two payload types with a builder hook (`bootstrap_fork_with<T, U>(parent: &Rumors<T>, configure: impl FnOnce(Bootstrap<U>) -> Bootstrap<U>) -> Peer<U>`, async core plus sync wrapper, draining the control stream as the existing form does), expose a variant returning the raw `Joined` so the failure tests can assert `Err`, and have `bootstrap_fork` delegate to it. Use it from cbor_evolution.rs (adding `mod common;`) and payload_depth.rs under `block_on` + `tokio::join!`; assert the server side's `Err` in the two failure tests. Acceptance: no `#[tokio::test]` or `tokio::spawn` in cbor_evolution.rs; `grep -rn '::bootstrap()' tests/cbor_evolution.rs tests/payload_depth.rs` returns nothing; the failure tests assert `server` completed with `Err`.

### tests-wire-format-21: FixtureTree hand-rolls a temp directory and is duplicated between the two provenance sweeps
- Where: tests/snapshot_liveness.rs:258-309 (related: tests/seed_liveness.rs:244-290; Cargo.toml:144-172; Cargo.lock)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (`grep -c 'name = "tempfile"' Cargo.lock` is 1, a transitive dependency; Cargo.toml's dev-dependencies do not list it; read both `FixtureTree` definitions, identical save `file`'s signature and the sweep body; neither sweep declares `mod common;`)
- Seen by: structure-prose, api-economics; refutation: confirmed; history: no rationale found (seed_liveness.rs's copy is the original from 2026-08-13; snapshot_liveness.rs copied it "modeled on the seed-liveness sweep"; `tempfile` has never been a rumors dependency and no note weighs it)
- Owner-gated: yes: adds a dev-dependency

`FixtureTree::new` builds a pid-tagged path under `std::env::temp_dir()`, pre-removes a stale one, and removes on drop; `tempfile::tempdir()` does exactly this with a unique name, and the crate is already in the lockfile, so adding it as a dev-dependency introduces no new crate to the supply chain. The stale-leftover dance exists only because the name is not unique. The struct is also defined twice, in two sweeps that deliberately do not compile the 4.4k-line harness, so the ordinary "move it to tests/common" fix is the wrong trade.

Evidence:

    264	    fn new(tag: &str) -> Self {
    265	        let dir =
    266	            std::env::temp_dir().join(format!("snapshot-liveness-{tag}-{}", std::process::id()));
    267	        // A stale run's leftovers must not leak into this one.
    268	        let _ = std::fs::remove_dir_all(&dir);

Resolution: Add `tempfile` to dev-dependencies and hold a `tempfile::TempDir` in `FixtureTree`, dropping the constructor's cleanup and the `Drop` impl; share the remaining `file`/`sweep` shell between the two sweeps through a small path-included module (`#[path = "support/fixture_tree.rs"] mod fixture_tree;`) so neither binary compiles `tests/common`. Acceptance: one definition of `FixtureTree` holding a `TempDir`; no `remove_dir_all` in either sweep.

### tests-wire-format-23: A third observer recorder where the harness already holds two
- Where: tests/payload_depth.rs:202-244 (related: tests/common/gossip_snapshot.rs:258-316; tests/observe.rs:39-105)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read the three: `SessionShape`/`ShapeObserver`/`ShapeSession` retains an elected flag and a data-stream count; `HookRecorder` retains the elected role and every sent stream by `StreamId`; `Recording` retains sessions, roles, every stream with its `StreamInfo`, and items)
- Seen by: structure-prose; refutation: confirmed; history: no rationale found (both existing recorders predate this one)
- Owner-gated: no

The payload_depth pin needs only "did an election happen" and "how many data streams opened", which both existing recorders already hold. Three `impl Observer` recorders in `tests/` is duplicated harness; `Recording` in observe.rs is the superset and belongs in `tests/common`, queried for the shape each suite asserts.

Evidence:

    209	#[derive(Default)]
    210	struct SessionShape {
    211	    elected: std::sync::Mutex<bool>,
    212	    streams: std::sync::Mutex<usize>,
    213	}
    214	
    215	struct ShapeObserver(std::sync::Arc<SessionShape>);

Resolution: Move `Recording` (or `HookRecorder`) into `tests/common`, give it `elected()` and `data_streams()` accessors, and delete `SessionShape`/`ShapeObserver`/`ShapeSession`. Acceptance: one `impl Observer` recorder type exists under `tests/`, in `tests/common`.

### tests-wire-format-28: Unused Protocol import in the shared wire harness, masked by a blanket unused_imports allow
- Where: tests/common/wire.rs:14 (related: tests/common/mod.rs:31-33)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`grep -n Protocol tests/common/wire.rs` returns only line 14; tests/common/mod.rs:33 is `#![allow(dead_code, unused_imports)]` with the rationale "Not every binary uses every module" at 31-32)
- Seen by: api-economics; refutation: confirmed; history: deliberate but expired (368da2a5 removed every use of `Protocol` in wire.rs and left the import; the allow dates to 2026-05-27)
- Owner-gated: no

`Protocol` is imported and never used; its last uses left with the V1 retirement. It survives because of `#![allow(dead_code, unused_imports)]`, whose stated rationale (per-binary variance) justifies `dead_code` only: an import referenced by any item, live or dead, is a used import, so `unused_imports` in the allow does no work for that rationale and only hides rot like this. This file sits in `tests/common`, the harness every partition file drives; if another partition owns `tests/common`, merge there.

Evidence:

    14	use rumors::{Peer, Protocol, Rumors, testing::run_to_quiescence};

    31	//! Not every binary uses every module; suppress unused-code warnings here
    32	//! rather than peppering allows across modules.
    33	#![allow(dead_code, unused_imports)]

Resolution: Remove `Protocol` from the import; drop `unused_imports` from the allow at mod.rs:33 and fix whatever else `just clippy` then reports. Acceptance: `just clippy` is clean with `#![allow(dead_code)]` alone on tests/common/mod.rs.

### suite-economics-8: sixty link units for 836 tests: tests/common compiled 42 times, latency.rs seven times
- Where: tests/common/mod.rs:31-33 (related: tests/gossip_pipelining.rs:13-15, tests/hop_trace.rs, tests/latency_link.rs, tests/tradeoff_probe.rs:30-32, tests/window_operator.rs, tests/window_corners.rs:14-16, tests/window_knee.rs; tests/partition.rs, tests/retire_redaction.rs, tests/stale_floor.rs, tests/opening_supply.rs, tests/gossip_pipelining.rs (single-test binaries); proptest-regressions/partition.txt)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grep: 42 of the 58 `tests/*.rs` files begin `mod common;`; seven include `benches/support/latency.rs` by `#[path]`; Cargo.toml declares no `[[test]]`, so each file is its own binary; the run log's per-binary table shows five single-test binaries in `tests/` and one two-test window binary; build.log: 32.74 s warm rebuild of the test targets under load)
- Verification: confirmed; history: deliberate-and-holds for the per-category layout (`tests/common/mod.rs:3` "Each per-category test binary pulls this module in via `mod common;`", and AGENTS.md's "category binaries in tests/"); the build cost itself is a recorded owner concern (`.agent-notes/2026-08-20-item-erasure/item-erasure.md:28-31`: "the 24 session-exercising test binaries each re-buying the subsystem"), which item erasure reduced per binary without reducing the binary count.
- Owner-gated: yes: AGENTS.md documents per-category binaries as the intended layout, so consolidation is a layout decision, not a defect.

Every `tests/*.rs` is auto-discovered as its own binary, so nextest built
60 units: 42 include `mod common;` (hence the blanket `allow(dead_code,
unused_imports)`), and seven re-include `benches/support/latency.rs` via
`#[path]`, each a separate compile of that module plus a full link against
the rumors rlib. Five binaries run a single test (gossip_pipelining,
opening_supply, partition, retire_redaction, stale_floor) and
`future_size` compiles to zero tests under the gate. nextest parallelism is
per test, so merging costs no runtime parallelism; the price is compile and
link time on every gate, ci, mutants and coverage build. One cost the sweep
did not state: committed proptest seeds are keyed by source path
(`tests/main.rs` and `tests/seed_liveness.rs` enforce
`proptest-regressions/<suite>.txt`), so folding `partition.rs` into
`multi_peer.rs` moves `proptest-regressions/partition.txt`'s entries into
`multi_peer.txt`, where every seed replays before every property in the
file.

Evidence:

    31	//! Not every binary uses every module; suppress unused-code warnings here
    32	//! rather than peppering allows across modules.
    33	#![allow(dead_code, unused_imports)]

    14	#[allow(dead_code)]
    15	#[path = "../benches/support/latency.rs"]
    16	mod latency;

Resolution: candidates that lose no clarity: fold the window family
(window_census, window_corners, window_knee, window_operator,
window_sweep, gossip_pipelining, tradeoff_probe) into one `tests/window.rs`
with submodules and a single `mod latency;`; fold the single-test files
into their category siblings (retire_redaction into retire, stale_floor and
opening_supply into the handshake or listen category, partition into
multi_peer), moving the committed seeds with them. Leave the schedule-engine
suites as they are: their per-category names are the map AGENTS.md points
at. Acceptance: fewer test binaries in `cargo nextest list -p rumors` with
every test still present by name; latency.rs compiled once for the window
family; `tests/seed_liveness.rs` still passes; `just gate` verdict
unchanged.

### Nits (37)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| tests-common-1 | `tests/common/action.rs:7-9` | serde imports appended as an orphan group with no blank line before the next item | Merge the serde pair into the external-crate import group and restore the blank line before the first item, at all seven sites |
| tests-common-5 | `tests/common/fault.rs:189-196` | hand-written `Clone` where `derive` is identical | `#[derive(Clone)]` on `FaultConnector` |
| tests-common-15 | `tests/common/peer.rs:22-23` | `common::peer::Peer` shadows `rumors::Peer`, and two unrelated `pub Session` types share a name | rename the test type (`Observed<T>` says what it is: a `Rumors<T>` plus an observation log) and `sim::Session` (`PlannedSession`) |
| tests-common-17 | `tests/common/peer.rs:115` | duplicate and implied trait bounds | drop the second `Eq` at the three peer.rs sites |
| tests-common-21 | `tests/common/schedule/events.rs:3-6` | `EventIdx` is a synonym for the same `usize` that indexes peers | Make `EventIdx` a `Copy` newtype over `usize` (and `PeerIdx` if wanted) with `Display` so counterexamples still read |
| tests-common-26 | `tests/common/sim.rs:692-696` | qualified paths where an import is the idiom | import at the top of each file and drop the two redundant annotations |
| tests-bookmark-10 | `tests/bookmark_causality.rs:147-149` | Em-dashes in `//` comments and in one assert message | a colon in the assert message |
| tests-bookmark-17 | `tests/bookmark_causality.rs:1174-1177` | Hand-rolled `.expect`, manual loops where the iterator idiom exists, and small idiom nits | `.expect("bookmark ok")` |
| tests-disruption-handshake-6 | `tests/disruption.rs:707-709` | Bare blocks around single send_all statements | Replace each block with the bare statement at the four sites |
| tests-disruption-handshake-11 | `tests/gossip_when.rs:49` | `tokio_block_on as block_on` shadows the quiescence poller's name | Import `tokio_block_on` under its own name at the four sites and call it as such |
| tests-disruption-handshake-13 | `tests/gossip_when.rs:178` | Em-dashes in line comments | Rewrite the twelve sites with colons, semicolons, or parentheses |
| tests-disruption-handshake-19 | `tests/gossip_when.rs:918-921` | The preamble width is a bare `30` beside a named constant in a sibling suite | Move `PREAMBLE_LEN` and its derivation comment to tests/common and import it in both suites |
| tests-disruption-handshake-29 | `tests/hop_trace.rs:49-54` | The hop_trace import block bleeds into the first item, and the tracer keys pipes by primitive sentinels | Fold the serde imports into the sorted block and add the blank line (here and at the two tests/common sites) |
| tests-lifecycle-6 | `tests/bootstrap.rs:90-94` | Twin proptest bodies and three local pair builders that a shared generic body or a `common` helper would own | One generic bootstrap proptest body called from both entries; `seeded_pair_async` in `common::wire`; rename lifecycle's `divergent_pair` |
| tests-lifecycle-12 | `tests/pairwise.rs:22-55` | Import idioms: qualified paths beside imports that exist, serde import lines with no separating blank, and a `futures` no-op waker beside std's | Import `Peer` and `RangeInclusive` where qualified; fold the serde lines into the import group; use `Waker::noop()`; drop the scare quotes |
| tests-lifecycle-13 | `tests/pairwise.rs:30-37` | Vestigial leftovers: a one-line alias of `bootstrap_fork`, bare brace blocks that scope nothing, and window configuration in a suite that never gossips | Replace `dup(..)` with `bootstrap_fork(..)` and delete the fn |
| tests-observation-6 | `tests/causal.rs:464-477` | `any::<usize>()` modulo a generated length where `prop::sample::Index` shrinks better | Draw `prop::sample::Index` and call `.index(len + 1)` at the three sites |
| tests-observation-10 | `tests/observe.rs:235-237` | `assert_election`'s `opened` predicate is asymmetric in A and B | Reduce to the two `Sent` disjuncts |
| tests-observation-13 | `tests/observe.rs:350-352` | The `Protocol::V2` assertion is vacuous with a one-variant enum | Drop line 351 and the words "and protocol" from the doc at 316 |
| tests-observation-21 | `tests/session_overlap.rs:20-30` | Import-path inconsistencies across the partition | Switch the two files to `use crate::common::` |
| tests-observation-29 | `tests/shadow_validity.rs:54` | `execute_with(..., \|_, _, _\| true)` where `execute` already spells that | `let result = execute(&schedule, &windows);` and import `execute` instead of `execute_with` |
| tests-observation-30 | `tests/shadow_validity.rs:55-72` | The two shadow-validity tests duplicate the version-to-event translation | Add `event_index_map` and `observed_events` helpers at file scope and use them in both tests |
| tests-observation-33 | `tests/party_conservation.rs:91-95` | Em-dashes in `//` comments and in one assert message | Replace `—` with ` -- ` or a colon at party_conservation.rs:93-94, causal.rs:144-145, 555-556, 655-656, listen.rs:497, 517, 634 |
| tests-observation-34 | `tests/party_conservation.rs:285-290` | `assert!(a == b, "... {a:?} vs {b:?}")` where the equality macros print both sides | Use `prop_assert_eq!`/`assert_eq!` (passing `&now, &baseline` where `Party` is moved) and drop the hand-formatted operand tails |
| tests-resource-link-window-17 | `tests/tradeoff_probe.rs:44-46` | tradeoff_probe: import spacing, undocumented helpers, a bare expect("small"), a hand-maintained 5,431 | Merge the imports into the main block and add the blank line |
| tests-resource-link-window-23 | `tests/window_census.rs:221-221` | em-dashes in seven non-doc comments | Replace with a colon, semicolon, or sentence break at each site |
| tests-wire-format-8 | `tests/gossip_snapshot.rs:505-511` | Braces left behind by the send_all conversion | Remove the braces |
| tests-wire-format-10 | `tests/cbor_evolution.rs:24-29` | A trailing serde import glued to the first item, at 22 sites across the tree | Fold each stray serde line into the existing serde import and restore the blank line, in one sweep over the 22 sites |
| tests-wire-format-13 | `tests/cbor_evolution.rs:165-167` | Em-dashes in a // comment and an assert message | Replace with a colon or semicolon at both sites |
| tests-wire-format-17 | `tests/wire_legibility.rs:91-102` | wire_legibility loads payloads one send at a time and folds seed and fork into one Option-flagged helper | Use `send_all` at the three sites, inline the seed and fork construction shapes, and give the bootstrap property a single-corpus strategy |
| tests-wire-format-24 | `tests/payload_depth.rs:209-243` | Long qualified paths where imports exist or belong, Mutex for a counter, and misnamed link ends | Import `block_on`, `Arc`, `AtomicBool`, `AtomicUsize`, `Error`, and the `observe` items at the top of payload_depth.rs |
| clippy-pedantic-6 | `tests/common/peer.rs:115` | `Eq` bound repeated in three where clauses | delete the second `Eq` at lines 115, 125, 142 |
| clippy-pedantic-7 | `tests/single_peer.rs:118-119` | Long integer literals without digit separators | `6_364_136_223_846_793_005` and `1_442_695_040_888_963_407` |
| clippy-pedantic-8 | `tests/common/overlap.rs:551-552` | `contains` then `insert` on a `BTreeSet` where `insert`'s return answers both | `if self.ever_known[p].insert(k) { .. }` in place of `contains` then `insert` |
| clippy-pedantic-13 | `tests/disruption.rs:810` | Or-pattern spelled at the outer level where the crate nests it | `Some(EXIT_BOOT_LOSS \| EXIT_UNCERTAIN) => ..`, the nested or-pattern the crate uses elsewhere |
| clippy-pedantic-15 | `tests/common/overlap.rs:517-518` | `Default::default()` and a spelled-out `std::collections::BTreeSet` where the type name would tell the reader what is built | `use std::collections::BTreeSet;` at the file head |
| suite-economics-10 | `tests/disruption.rs:798-807` | inter-process child reaping busy-polls with a 25 ms real-clock sleep | Enable tokio's `process` feature for dev; spawn via `tokio::process::Command` with `kill_on_drop(true)` and `timeout(CHILD_DEADLINE, child.wait())` |

## Benches and examples

Files: benches/, examples/. Entries: 20 (1 medium, 9 low, 10 nit).

### benches-envelope-29: The "landed" flat baseline, its "default 16 GiB" budget, and the L(N) derivation denominate against designs on no surviving branch
- Where: examples/envelope_sim.rs:181-196 (related: examples/envelope_sim.rs:10-13, examples/envelope_sim.rs:21-22, examples/envelope_sim.rs:70-76, examples/envelope_sim.rs:139-179, examples/envelope_sim.rs:728-787, examples/envelope_sim.rs:1171, examples/envelope_sim.rs:1174-1221, examples/envelope_sim.rs:1223-1251, examples/envelope_sim.rs:1522-1569, src/tree/mirror/streaming/window.rs:275, .agent-notes/2026-07-22-sync-budget/sync-budget.md:593-596, .agent-notes/2026-07-22-sync-budget/sync-budget.md:602-606, .agent-notes/2026-07-22-uniformity-envelope/README.md:16-19)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (`4_644` and `NODE_BYTES`/`k_flat` appear nowhere in src, tests, or benches; line 1174 prints "at the default 16 GiB budget" while the shipped default is 512 MiB; sync-budget.md:593-596 records that the `NODE_BYTES` token survives only in the design documents and this simulator, "which speak about the rejected shape"; sync-budget.md:602-606 lists per-stream budget division (`L(N)`) under "Deliberately out of scope"; the uniformity-envelope README:16-17 states "Everything the body calls "landed" lives on the campaign branch, not on any branch that survives"; the Python header has `DEFAULT_BUDGET = 16 * (1 << 30)  # 16 GiB`)
- Seen by: structure, prose; refutation: confirmed; history: already-known (both notes acknowledge the residue as scoped out of their campaigns; neither is a ruling to keep it, so the owner decision the finding asks for has not been recorded)
- Owner-gated: yes (retiring or re-labeling a design comparison is the owner's call)

`check_landed_replication` pins "the reference figures" of a flat per-node solve (`K_flat(2^40, 16 GiB, 340) = 4 644`) that exists on no branch in this repository; `window.rs` prices nodes through the backend's `node_bytes`, not a flat constant. The report titles a section "at the default 16 GiB budget" when the crate's default is 512 MiB and was never 16 GiB on a surviving branch. Section 4 derives `L(N)` for a per-stream budget division the design record scopes out. "Landed" is dated provenance vocabulary (AGENTS.md hard rule: no references to code that no longer exists), and a pin whose only referent is the thing pinning it is the circular-justification tell. The sections worth keeping are the dominance sweep and the Monte Carlo tiers.

Evidence:

   181	/// Pins the flat replication: the reference figures (default window
   182	/// 4 644 at N = 256⁵; NODE_BYTES = 340 the unique consistent price;
   183	/// the ~3× widening at small declarations) must reproduce exactly.
   184	fn check_landed_replication() {
   185	    let consistent: Vec<u128> = (0..2048)
   186	        .filter(|&nb| k_flat(DEFAULT_N, DEFAULT_BUDGET, nb) == 4_644)
   187	        .collect();
   188	    assert_eq!(consistent, vec![340], "NODE_BYTES back-out failed");

Resolution: Owner decision between (a) retire the flat baseline and the `L(N)` section: delete `NODE_BYTES`, `per_scope_flat`, `charged_scopes`, `k_flat`, `check_landed_replication`, the `K_flat` columns, the `flat,` manifest rows, `l_of_n`, `heavy_count`, `stage_saturation_bytes`, and their report blocks, and rename `DEFAULT_BUDGET`/`DEFAULT_N` to `SIM_BUDGET`/`SIM_N` (or import `rumors::DEFAULT_SYNC_MEMORY_BUDGET` if the tables should be at the shipped default); or (b) keep the flat solve as a named comparison and re-state it positively ("a flat per-scope solve, the alternative the sharpened envelope is compared against, at a 16 GiB budget"), dropping "landed", "reference", "default", and the self-referential pins, and move the `L(N)` derivation to `.agent-notes/` beside the transport receive-window plan that earmarks it. Module-doc items 1, 4, and 10-13 and the printlns at 1171 and 1174 follow either way. If benches-envelope-32 dissolves the example, this is subsumed. Acceptance: `grep -n 'landed\|4_644\|4644\|default 16 GiB' examples/envelope_sim.rs` returns nothing; every remaining constant is imported from the crate or documented as the simulator's own parameter.

### benches-envelope-2: The `V2` series label distinguishes one dialect, and two latency group names are never emitted
- Where: benches/gossip_fixed.rs:9-10 (related: benches/gossip_fixed.rs:107-114, benches/gossip_fixed.rs:154, benches/gossip_fixed.rs:168-171, benches/gossip_fixed.rs:190, src/protocol.rs:15-19)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (`git show 368da2a5 -- benches/gossip_fixed.rs` replaces `BenchmarkId::new(format!("{protocol:?}"), param)` over a `PROTOCOLS` array with the literal and adds the parenthetical in the same hunk; protocol.rs:15-19 has one variant; `sweeps` at 168-171 holds only the two `Bidir` scenarios; `grep -c gossip tools/benchjudge-expected.json` is 0)
- Seen by: structure, prose, perfapi; refutation: confirmed (owner-gated added); history: no-rationale-found for the label (planned at v1-retirement/README.md:240 without a reason); the two unreachable arms predate the retirement (818a8707 wrote the four-arm match and the two-element sweep together), so only the label is retirement residue
- Owner-gated: yes (Criterion series ids name any locally saved baselines; renaming breaks comparison against them)

With `Protocol` a single-variant enum, the `V2` segment in every series id partitions nothing, and the doc's parenthetical describes a comparison that cannot be made. Separately, `Scenario::latency_group_name` has arms for `gossip_latency_unilateral_insertions` and `gossip_latency_unilateral_redactions`, but `bench_gossip_latency` sweeps only the two bidirectional scenarios and the module doc lists only those two groups, so the arms produce names no group is registered under. Principle 3: machinery outlives the constraint that justified it; a match arm whose output no path produces is prose about a sweep that does not exist.

Evidence:

     9	//! The four Criterion groups measure the wire protocol on the same
    10	//! fixtures (each series is labeled `V2`, the dialect it measures):

   107	    fn latency_group_name(self) -> &'static str {
   108	        match self {
   109	            Scenario::BidirInsertions => "gossip_latency_bidir_insertions",
   110	            Scenario::BidirRedactions => "gossip_latency_bidir_redactions",
   111	            Scenario::UnilateralInsertions => "gossip_latency_unilateral_insertions",
   112	            Scenario::UnilateralRedactions => "gossip_latency_unilateral_redactions",
   113	        }
   114	    }

Resolution: Use `BenchmarkId::from_parameter(param)` at 154 and `BenchmarkId::new(format!("divergence={param}"), latency_ms)` at 190, drop the parenthetical at 10, and note the baseline discontinuity in the commit; if the slot is deliberately reserved for a future dialect, say so in one sentence instead. Make the latency sweep table carry its group name directly (`(Scenario, &str, &[usize])`) and delete `latency_group_name`, or sweep the unilateral scenarios under latency and list them in the module doc. Acceptance: `grep -n '"V2' benches/` returns nothing (or the doc states the reservation); no function in the file returns a group name that no group is registered under.

### benches-envelope-11: in_memory.rs re-declares grid::send_units, twins drain for two observer types, qualifies an imported name, and harvests versions for callers that discard them
- Where: benches/in_memory.rs:59-64 (related: benches/support/grid.rs:56-63, benches/in_memory.rs:54, benches/in_memory.rs:68-73, benches/in_memory.rs:77-84, benches/in_memory.rs:256-263, benches/in_memory.rs:159, benches/in_memory.rs:46, src/rumors/unordered.rs:191, src/rumors/causal.rs:151)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (both `send_units` bodies read; both drain bodies read; `impl<T: Send + Sync + 'static> Stream for` at unordered.rs:191 and causal.rs:151; line 46 imports `Version`, line 159 writes `rumors::Version`; `build` is called at 117, 144, 213, 277, 325 and its versions are read only at 144 and 325)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: no-rationale-found for the `send_units` copy (ac68e8121 dropped a `use grid::{…, units}` import for a local helper); the twin drains are deliberate-but-expired (both called the type-specific lending `borrow_next()` until cd7c09db33 made `next()` the one engine)
- Owner-gated: no

`send_units` (59-64) is byte-identical to `grid::send_units` while line 54 already imports from `grid`; the "one CBOR null byte" rationale is stated at in_memory.rs:6-8 and grid.rs:56-58 both. `drain` and `drain_causal` have identical bodies over two types that both implement `Stream`. `rumors::Version` at 159 is spelled out although `Version` is imported. `build` collects every live version for five callers, of which only `redact` and `get` read them (a one-million-element collect at the top size, untimed but pointless). Doctrine: one definition per helper; imports over qualified paths; no work no caller consumes.

Evidence:

    59	/// Commit `n` unit payloads to `rumors` as one batch.
    60	fn send_units(rumors: &Rumors<()>, n: usize) {
    61	    rumors
    62	        .send_all(iter::repeat_n((), n))
    63	        .expect("flat test payloads are within any depth limit");
    64	}

Resolution: `use grid::{SIZES, sample_size_for, send_units};` and delete the local copy and the `std::iter` import, keeping the CBOR-null rationale once at grid.rs:56-58; replace the two drains with one `fn drain<S: Stream + Unpin>(observer: &mut S) -> usize`; write `Version` at 159; split `build` into `build(n) -> Rumors<()>` and `versions_of(&Rumors<()>) -> Vec<Version>` so only `redact` and `get` pay the harvest. Acceptance: `grep -c 'fn send_units\|fn drain' benches/in_memory.rs` is 1; no `rumors::Version` remains in the file; `build` returns no value its caller drops unread.

### benches-envelope-16: grid.rs owns wire.rs, so one bench re-implements the bootstrap fork and another includes the grid under a stale comment
- Where: benches/support/grid.rs:34-35 (related: benches/gossip_fixed.rs:50-54, benches/gossip_fixed.rs:141, benches/gossip_fixed.rs:216, benches/gossip_fixed.rs:238, benches/gossip_fixed.rs:254, benches/gossip_fixed.rs:268, benches/window_wallclock.rs:63-86, benches/support/wire.rs:39-56)
- Class / severity / confidence: modularity / low / high
- Provenance: verified (grid.rs:34-35 is the only `#[path]` include of wire.rs; gossip_fixed.rs reaches `grid::wire::Wire::new()` at 141 and `grid::wire::bootstrap_fork` at 216, 238, 254, 268; window_wallclock.rs:69-81 is wire.rs:44-55's `tokio::join!` bootstrap shape plus `.sync_memory_budget(budget)`)
- Seen by: structure, prose (the comment half), perfapi (the duplicate fork); refutation: confirmed; history: no-rationale-found (the nesting arrived in the WIP commit 83edcd9441 with no layout discussion; the gossip_fixed comment was accurate at 5a60fb3e3a and expired in the same commit)
- Owner-gated: no

`wire.rs` is reachable only as `grid::wire`. `window_wallclock.rs` needs a budget-configured bootstrap fork and no grid, so `diverged` (63-86) re-implements `wire::bootstrap_fork` inline, differing only in the `.sync_memory_budget(budget)` call. `gossip_fixed.rs` includes the grid under a comment saying it "only needs its sample-size policy", then reaches the wire harness through it at five sites. Fixtures and harness are separate concerns nested by accident of `#[path]` inclusion, and the cost is duplicated logic plus a comment contradicted by the code beneath it.

Evidence:

    34	#[path = "wire.rs"]
    35	pub mod wire;

    50	// The shared grid module exposes a superset of helpers; this bench only needs
    51	// its sample-size policy so fixed-N runs line up with the existing benches.

Resolution: Add `benches/support/mod.rs` declaring `pub mod grid; pub mod wire; pub mod latency;`, have each bench write `#[allow(dead_code)] mod support;` (a bench file is a crate root, so no `#[path]` is needed), and let `grid.rs` use `super::wire`. Have `bootstrap_fork` return the joined `Peer<T>` (callers finish with `.into_rumors()` or `.sync_memory_budget(b).into_rumors()`) and replace `window_wallclock::diverged`'s inline block with it. Fix the gossip_fixed comment to the siblings' generic wording. The tests' `#[path = "../benches/support/latency.rs"]` keep working because latency.rs imports no sibling. Acceptance: no bench contains a `tokio::join!(… .gossip(…), Peer::<_>::bootstrap().join(…))` block outside `support/wire.rs`; `gossip_fixed.rs` has no comment claiming to need only the sample-size policy.

### benches-envelope-22: Two dead-code conventions for the shared bench modules, one of them redundant everywhere it appears
- Where: benches/support/latency.rs:398-400 (related: benches/support/latency.rs:463-466, benches/support/latency.rs:511-514, benches/support/latency.rs:534-537, benches/gossip_fixed.rs:56-57, benches/support/grid.rs:39, benches/window_wallclock.rs:12, tests/window_corners.rs:14, tests/tradeoff_probe.rs:30, tests/window_knee.rs:19, tests/hop_trace.rs:27, tests/gossip_pipelining.rs:13, tests/window_operator.rs:24, tests/latency_link.rs:13)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`grep -rn -B3 'path = ".*latency.rs"' benches tests`: nine includers, eight with `#[allow(dead_code)]` on the `mod`, gossip_fixed.rs:56-57 alone without; `grep -rn -B3 'path = ".*grid.rs"' benches`: all three includers carry the module-level allow, so grid.rs:39's `#[allow(unused)]` is redundant)
- Seen by: structure, prose; refutation: confirmed (the structure lens's "nine of ten" corrected to eight of nine); history: no-rationale-found (the item-level allows accreted at 1988de6d and 814f07ad, each explaining the `#[path]` mechanism but never why the exception sits at the item rather than at the one includer lacking the module-level allow)
- Owner-gated: no

Four items in latency.rs carry their own `#[allow(dead_code)]` each preceded by the same three-line comment, and each exists only because one includer omits the module-level allow the other eight use. The comments also hand-maintain caller lists ("Used only by `window_wallclock`"). `grid.rs:39` carries `#[allow(unused)]` on `SIZES` although every includer already allows at the module level. One statement of a mechanism beats four, and a caller list in a comment rots (Principle 5).

Evidence:

   398	    // Used only by `window_wallclock`; the module is `#[path]`-included by
   399	    // several targets, each seeing its own copy's usage.
   400	    #[allow(dead_code)]

Resolution: Add `#[allow(dead_code)]` to `gossip_fixed.rs:56`'s `mod latency;` (as that file already does for `mod grid;` at 52), delete the four item-level allows and their comments in latency.rs, and delete `#[allow(unused)]` at grid.rs:39. Acceptance: `grep -c 'allow(dead_code)\|allow(unused)' benches/support/*.rs` is 0 and clippy stays clean across all include sites.

See also: tests-disruption-handshake-28.

### benches-envelope-25: The harnesses restate serde bounds on round_trip that Rumors::gossip does not demand
- Where: benches/support/wire.rs:26-29 (related: benches/support/latency.rs:428-435, benches/support/latency.rs:467-474, benches/support/latency.rs:484-487, benches/support/latency.rs:515-518, benches/support/wire.rs:40-43, src/rumors.rs:92, src/rumors.rs:489-494, src/peer.rs:201-205, src/peer/bootstrap.rs:247-252)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`Rumors::gossip` at rumors.rs:494 bounds `T: Send + Sync + 'static` inside `impl<T, B: BookmarkError> Rumors<T, B>` at 92, which carries no serde bounds; peer.rs:201-205 states the design; `Bootstrap::join` at bootstrap.rs:252 requires `Serialize + DeserializeOwned + Eq + Send + Sync + 'static`, so `bootstrap_fork` legitimately keeps the full set)
- Seen by: perfapi; refutation: confirmed; history: deliberate-but-expired (the harness bounds mirrored `gossip`'s Borsh bounds at 818a8707; 48bc31df and ce27df86 localized serde at construction, and a5a16f43 swept `Eq` through the harness without narrowing the gossip-only sites)
- Owner-gated: no

`Wire::round_trip`, `DelayedWire::round_trip`, `round_trip_virtual`, `reconcile`, and `session_hops` require `T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static`, but the only thing they call is `Rumors::gossip`, whose bound is `T: Send + Sync + 'static`. `Peer::seed`'s doc states the design the library paid for: serde obligations live at construction "so neither the send paths nor the gossip entry points carry serde bounds of their own". The harness is the first generic caller and it shows future callers the wrong idiom; it also spells `serde::de::DeserializeOwned` inline at every site.

Evidence:

    26	    pub fn round_trip<T>(&mut self, a: Rumors<T>, b: Rumors<T>) -> (Rumors<T>, Rumors<T>)
    27	    where
    28	        T: serde::Serialize + serde::de::DeserializeOwned + Eq + Send + Sync + 'static,
    29	    {

Resolution: Narrow the five gossip-only sites to `T: Send + Sync + 'static`; keep the full set on `bootstrap_fork` only, with `use serde::Serialize; use serde::de::DeserializeOwned;` at the top of wire.rs. Acceptance: `cargo check --all-targets` passes with the narrowed bounds; the only `DeserializeOwned` in benches/ is on `bootstrap_fork`.

### benches-envelope-33: envelope_sim repeats itself: two copies of the sweep grid, the KL tail, both bisections, the per-parent min, the theta list, and the measured population
- Where: examples/envelope_sim.rs:691-705 (related: examples/envelope_sim.rs:1109-1123, examples/envelope_sim.rs:222-236, examples/envelope_sim.rs:336-349, examples/envelope_sim.rs:170-178, examples/envelope_sim.rs:523-531, examples/envelope_sim.rs:259-267, examples/envelope_sim.rs:354-361, examples/envelope_sim.rs:647-649, examples/envelope_sim.rs:669-671, examples/envelope_sim.rs:715, examples/envelope_sim.rs:1137, examples/envelope_sim.rs:1162, examples/envelope_sim.rs:1225, examples/envelope_sim.rs:1044-1052, examples/envelope_sim.rs:1483-1491, src/tree/mirror/streaming/window.rs:449-456)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (the two thirteen-element arrays are textually identical; the KL expression at 234 recurs at 347; the largest-fitting upper bisection at 170-178 recurs at 523-531 and at window.rs:449-456; the lower bisection at 259-267 recurs at 354-361; `c_q_int` at 647-649 is the `sub = FAN` case of the min spelled out at 669-671, 715, and 1137; the theta list appears at 1162 and 1225; the `s_meas` computation at 1044-1052 recurs at 1483-1491 as `p_up * p_up` against `.powi(2)`)
- Seen by: structure, perfapi; refutation: confirmed; history: no-rationale-found (port-introduced: the Python defines the sweep list once and has no `--manifest` mode; 3824c7754 added the mode and spelled the list a second time)
- Owner-gated: no

A 1,586-line numerical tool is reviewable only if each formula has one home; the second sweep array lets the manifest stop covering what the sweep certifies without a diff a reviewer would notice, and the doc at 687 hand-counts it ("13 corpus sizes"). Several of these vanish if benches-envelope-32 dissolves the integer copies; the rest are one-line hoists.

Evidence:

   691	    let ns: [u64; 13] = [
   692	        2,
   693	        10,
   694	        100,
   695	        10u64.pow(4),

  1109	    let sweep_ns: [u64; 13] = [
  1110	        2,
  1111	        10,
  1112	        100,
  1113	        10u64.pow(4),

Resolution: Hoist `SWEEP_N` and `THETAS` to `const`; make `binom_tail_log` take `(nf: f64, p: f64, af: f64)` and call it from `chernoff_quantile_huge`; extract `largest_fitting(lo, hi, fits)` and `smallest_certifying(lo, hi, certifies)`; generalize `c_q_int` to `q_int(n, j, sub)` and call it at all four sites; extract `measured_stage_population`. Acceptance: each listed expression has exactly one definition in the file; the "13 corpus sizes" count is gone from prose.

### swarm-example-13: `drain_versions` hand-rolls `UnorderedMessages::try_next` and clones an owned `Version`
- Where: examples/swarm.rs:618-622 (related: examples/swarm.rs:137, src/rumors/unordered.rs:167-182, src/rumors/unordered.rs:191-192, src/lib.rs:346)
- Class / severity / confidence: idiom / low / high
- Provenance: verified (`try_next` read at src/rumors/unordered.rs:175-182; `TryNext` re-exported at src/lib.rs:346; `Stream::Item = (Version, Arc<T>)` at :192; grep shows line 619 is the only use of `FutureExt`/`StreamExt` in the file)
- Seen by: correctness, perfapi (clone); refutation: confirmed; history: deliberate but expired (the `now_or_never` spelling predates `try_next`, cbfc1571; the clone was written against the lending `borrow_next`, which cd7c09db replaced with the owned `next()`)
- Owner-gated: no

The crate exposes `try_next` as its non-blocking step ("One `Stream` poll with a no-op waker, rendered as the trichotomy"), which is exactly what `observer.next().now_or_never()` spells here, and the example is the place users learn the API. The `.clone()` bumps and immediately drops a refcount on a value the pattern already owns, and makes a reader look for a borrow that is not there.

Evidence:

    618	fn drain_versions(observer: &mut UnorderedMessages<Payload>, pool: &mut Vec<Version>) {
    619	    while let Some(Some((version, _))) = observer.next().now_or_never() {
    620	        pool.push(version.clone());
    621	    }
    622	}

    src/rumors/unordered.rs
    175	    pub fn try_next(&mut self) -> TryNext<T> {
    176	        use futures::{FutureExt, StreamExt};
    177	        match self.next().now_or_never() {
    192	    type Item = (Version, Arc<T>);

Resolution: `while let TryNext::Message((version, _)) = observer.try_next() { pool.push(version); }`, importing `rumors::TryNext`; the `futures` import at 137 then goes. Acceptance: no `now_or_never` or `.clone()` in `drain_versions`; the loop reads the trichotomy by name; `just clippy` stays clean.

### swarm-example-14: `wind_down`'s post-claim straggler drain is unreachable by the function's own argument
- Where: examples/swarm.rs:654-657 (related: examples/swarm.rs:624-631, examples/swarm.rs:640-650, examples/swarm.rs:693-718, examples/swarm.rs:554, examples/swarm.rs:642, examples/swarm.rs:749)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read; the protocol trace is in the claim)
- Seen by: structure, prose, correctness; refutation: confirmed (three candidates, one finding); history: no rationale found (the loop and "for good measure" are original to e8442e6a, under the same claim-before-send protocol)
- Owner-gated: no

An initiator pushes to a responder's inbox only after winning the compare-and-swap on that responder's `engaged` flag, and never clears the peer's flag after a successful send (749 clears only its own); the flag stays set until the responder serves and clears it (554, 642). A successful CAS in `wind_down` therefore reads a flag no initiator holds, so the inbox is empty and, the flag now permanently set, stays empty. The loop cannot execute, its comment concedes it ("nothing new can arrive"), and "for good measure" names no failure it catches: the circular-justification tell. Dead handling code also weakens a reader's trust in the proof above it.

Evidence:

    654	    // Locked: nothing new can arrive. Drain any straggler for good measure.
    655	    while let Ok(end) = inbox.try_recv() {
    656	        serve_sync(runtime, net, &rumors, end);
    657	    }
    628	/// new session with us. Because an initiator sets a peer's flag *before*
    629	/// delivering the session, a successful claim here proves nothing is owed; if

Resolution: Delete the loop and its comment (the doc at 626-631 carries the argument); or, if a check is wanted for a future path that pushes to an inbox without claiming, `debug_assert!(inbox.try_recv().is_err(), "an initiator delivered a session without claiming this party")`, which names the one mistake it would catch. Acceptance: no serve path exists after the successful CAS in `wind_down`, or the check that replaces it names what it catches.

### swarm-example-26: `compute` hides a consuming side effect inside an otherwise pure function
- Where: examples/swarm.rs:1464-1469 (related: examples/swarm.rs:1398-1411, examples/swarm.rs:1546-1547, examples/swarm.rs:463-479)
- Class / severity / confidence: simplification / low / medium
- Provenance: assessed (read)
- Seen by: structure; refutation: confirmed, with a correction (parties start before the UI, so the zero-width startup call at 1547 can consume a real best; it is harmless because its result is only the display until the first sample, not because nothing has run); history: no rationale found
- Owner-gated: no

`compute(net, prev, now)` has the signature of a pure function of two snapshots and the live gauges, but it also swaps the `sync_nanos_best` sentinel, so its call count matters: a second call in one window loses the window's best. `Snapshot::take` is already the "read the counters now" operation; a read that re-arms a windowed minimum belongs there, leaving `compute` pure and the startup call obviously safe. Finished code should be evidently correct, and a function named `compute` whose call count matters is not.

Evidence:

    1464	    // Consume the window's fastest session and re-arm the sentinel for the
    1465	    // next window.
    1466	    let latency_best_nanos = net
    1467	        .metrics
    1468	        .sync_nanos_best
    1469	        .swap(u64::MAX, Ordering::Relaxed);
    1547	    let mut stats = compute(net, &prev, &Snapshot::take(net));

Resolution: Move the swap into `Snapshot::take` as a `sync_nanos_best` field with a doc line stating that taking a snapshot consumes the window's best; `compute` reads `now.sync_nanos_best`. Acceptance: `compute` performs no atomic stores or swaps; `Snapshot` documents the consuming read.

### Nits (10)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| benches-envelope-5 | `benches/gossip_fixed.rs:282-297` | seeded_with_messages is seeded_with_versions minus the harvest | Keep one seeding helper: `seeded_with_versions` with callers taking `.0`, or `seeded_with_messages` plus a `versions_of` helper |
| benches-envelope-20 | `benches/support/latency.rs:1-8` | latency.rs bundles the pipe, the link adapter, and the measurement harness; its includers already name the seam | If taken up, split latency.rs into `delayed_pipe.rs` and `delayed_wire.rs` along the boundary the includers' comments already name |
| benches-envelope-23 | `benches/support/latency.rs:405` | DelayedWire selects its cost model by a bool | Introduce `enum Clock { Paused, Running }`, store it, and match on it in `round_trip` and `round_trip_virtual`'s assert |
| swarm-example-9 | `examples/swarm.rs:424-426` | Current-thread runtime construction is spelled out four times | `fn current_thread_runtime() -> Runtime` with the flavor rationale in its doc |
| swarm-example-12 | `examples/swarm.rs:512-518` | Constants restated by hand: "five seconds" beside `from_secs(5)`; the test copies the clap default literal | Drop the numeral from the deadline doc; one `DEFAULT_DUPLEX_CAPACITY` constant used by `Args` and the test |
| swarm-example-15 | `examples/swarm.rs:684-718` | The engaged-flag release is hand-written on every exit of `try_initiate` | Add `Claim` with `try_acquire(&AtomicBool) -> Option<Claim>`, `Drop` storing `false`, and `transfer(self)` forgetting it |
| swarm-example-19 | `examples/swarm.rs:812-820` | `steady_state_op` clamps a probability the types already bound, and its docs omit the zero-target case | Test `target == 0` on the integer, compute the ratio in `f64`, drop the `clamp`; qualify the two "always adds" sentences |
| swarm-example-24 | `examples/swarm.rs:1353-1353` | Bool-typed parameters in `Field::adjust` and `bump` make call sites unreadable without the signature | A two-variant `Direction { Down, Up }` on `adjust` and `bump`, with `Left => adjust(Direction::Down, coarse)` |
| swarm-example-25 | `examples/swarm.rs:1388-1388` | The local `Snapshot` collides with `rumors::Snapshot`, and several items are spelled by long path at every use | Rename the counters struct (`Counters` or `Sample`), import `rumors::Snapshot`, and import the other qualified items |
| swarm-example-28 | `examples/swarm.rs:1781-1786` | `mod tests` is declared between two formatting helpers, and the type aliases follow their first use | Move the six-line `mod tests` block to the end of the file (or beside the `use` block) |

## Manifest, lints, and verification recipes

Files: Cargo.toml, tools/, the justfile. Entries: 6 (1 medium, 3 low, 2 nit).

### deps-1: static_assertions is a normal dependency for one test module, and one redundant test in it is the sole reason forbid(unsafe_code) is conditional
- Where: Cargo.toml:128-128 (related: src/lib.rs:295-296, src/tree/typed/height.rs:166, src/tree/typed/height.rs:175-176, src/tree/typed/height/tests.rs:6-28, crates/before/Cargo.toml:28)
- Class / severity / confidence: vestigial / medium / high
- Provenance: verified (grep census of `static_assertions` over src/, tests/, benches/, examples/ and every manifest; read the three macro bodies in the registry crate; grep of `unsafe` over src/, tests/, benches/, examples/)
- Verification: confirmed and sharpened; history: deliberate-but-expired (fceb55f98, 2026-06-05, "remove all unsafe code", introduced the conditional with the same comment; today its trigger is one redundant test)
- Owner-gated: no

`static_assertions` is declared under `[dependencies]`, so every user of rumors compiles it, while its only use sites are eight macro calls in the `#[cfg(test)]` module `src/tree/typed/height/tests.rs`. Of the three macros used, only `assert_eq_size_val!` (through `assert_eq_size_ptr!`) expands to `#[allow(... unsafe_code ...)] let _ = || unsafe { ... }`; `assert_eq_size!` and `assert_eq_align!` expand to `const _: fn() = || { ... }` with no `unsafe`, so the single test `zero_size_val` is what keeps `#![forbid(unsafe_code)]` from being unconditional, and that test asserts nothing `zero_size` does not already assert (a `Sized` value's size is its type's size). `height.rs:166` already states a type-level fact with the std idiom `const _: () = assert!(...)`, so the replacement pattern is in the same file.

Evidence:

    Cargo.toml
       128	static_assertions = { workspace = true }

    src/lib.rs
       295	// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
       296	#![cfg_attr(not(test), forbid(unsafe_code))]

    src/tree/typed/height.rs
       166	const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);
       175	#[cfg(test)]
       176	mod tests;

    src/tree/typed/height/tests.rs
        13	/// Height *values* (not just the types) are zero-sized, so constructing
        14	/// one is free.
        15	#[test]
        16	fn zero_size_val() {
        17	    static_assertions::assert_eq_size_val!(Z, ());
        18	    static_assertions::assert_eq_size_val!(S::<Z>::default(), ());
        19	}

    ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/static_assertions-1.1.0/src/assert_eq_size.rs
        34	macro_rules! assert_eq_size {
        36	        const _: fn() = || {
        37	            $(let _ = $crate::_core::mem::transmute::<$x, $xs>;)+
        76	macro_rules! assert_eq_size_ptr {
        78	        #[allow(unknown_lints, unsafe_code, forget_copy, useless_transmute)]
        79	        let _ = || unsafe {
       119	macro_rules! assert_eq_size_val {

Resolution: Delete `zero_size_val` (its testdoc claims a value/type distinction that does not exist for `Sized` types, and `zero_size` covers both of its checks). Replace the remaining six assertions with `const _: () = assert!(size_of::<Z>() == 0 && align_of::<Z>() == 1);` and likewise for `S<Z>` and `Root`, either in `height.rs` beside line 166 (they are compile-time facts and need no `#[test]` wrapper to fire) or in `tests.rs` with the two testdocs kept. Remove `static_assertions` from rumors' `[dependencies]` (the workspace table entry stays; `crates/before` inherits it at its line 28). Make the crate attribute an unconditional `#![forbid(unsafe_code)]` and delete the comment at lib.rs:295. Acceptance: rumors' `[dependencies]` has no `static_assertions` entry; lib.rs carries `#![forbid(unsafe_code)]` with no `cfg_attr`; the three layout facts are `const _` assertions that fail compilation if a height type gains size or alignment; `just gate` is clean.

See also: api-core-9 (counted for the crate-attribute comments), inventory-2 (a cross-reference here).

### deps-4: futures-util is fully shadowed by futures at all six sites
- Where: Cargo.toml:139-140 (related: Cargo.toml:52-53, src/bookmark.rs:17, src/tree/mirror/handshake.rs:329, src/tree/mirror/streaming/remote/proxy/start.rs:219, src/peer/gossip.rs:13, src/peer/gossip.rs:946, src/peer/gossip.rs:1313)
- Class / severity / confidence: simplification / low / high
- Provenance: verified (grep census of `futures_util` over src/, tests/, benches/, examples/; grep of `futures-util` over every manifest in the workspace; read futures-0.3.32/src/lib.rs re-export lines and its `[features]` table; Cargo.lock resolves futures and futures-util both to 0.3.32)
- Verification: confirmed; history: no-rationale-found (60a3b056 added it; 2c73d032 moved it into the table unchanged)
- Owner-gated: no

Every `futures_util` path the crate uses (`FutureExt`, `StreamExt`, `future::try_join`, `stream::unfold`) is re-exported at the root of futures 0.3.32 (`pub use futures_util::future::{FutureExt, TryFutureExt};` at lib.rs:101, `pub use futures_util::stream::{StreamExt, TryStreamExt};` at 106, `pub use futures_util::{future, never, sink, stream, task};` at 131), and futures' `std` feature enables `futures-util/std`, so the second declaration adds a second spelling for the same items and one more table row. No other manifest inherits `futures-util`.

Evidence:

    Cargo.toml
        52	futures = { version = "0.3", default-features = false, features = ["std", "async-await"] }
        53	futures-util = { version = "0.3", default-features = false, features = ["std"] }
       139	futures = { workspace = true }
       140	futures-util = { workspace = true }

    src/peer/gossip.rs
        13	use futures_util::StreamExt;
    src/tree/mirror/handshake.rs
       329	    futures_util::future::try_join(write, read).await?;
    src/bookmark.rs
        17	use futures_util::FutureExt;

Resolution: Replace `futures_util::` with `futures::` at the six sites; remove `futures-util` from rumors' `[dependencies]` and from `[workspace.dependencies]`. Acceptance: `grep -rn futures_util src/` is empty; the workspace table has no `futures-util` row; `cargo tree -p rumors -e normal --depth 1` lists futures and not futures-util.

### inventory-2: `static_assertions` is a runtime dependency used only under `cfg(test)`

See deps-1, the entry of record for `static_assertions` sitting in `[dependencies]` for one `cfg(test)` module. This entry read the lib.rs:295 comment as accurate; deps-1 shows its premise does not hold (only `assert_eq_size_val!` expands to `unsafe`, and its one test is redundant), so `forbid(unsafe_code)` can become unconditional and the move to `[dev-dependencies]` is a step inside deps-1's resolution. The sweep's full entry is in `evidence/sweeps/inventory.md`.

### verification-infra-11: tools/digestshare keeps a V1 side-by-side skip for a render form that no longer exists
- Where: tools/digestshare:20-23 (related: tools/digestshare:43-49, tools/digestshare:76-78, justfile:211-220)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (read the tool; `grep -l '│' tests/snapshots/*.snap` matches 0 of 22 files; the only `V1` token in src is the unrelated `Protocol` example at src/lib.rs:272; the tool's git history has two commits, neither the V1 retirement (368da2a5) nor the prose re-denomination (c13c21b4))
- Verification: confirmed; history: no-rationale-found (the V1 retirement's re-denomination commit did not touch tools/)
- Owner-gated: no

The tool documents and implements skipping snapshots that contain the `│`
column separator as "V1 side-by-side timelines". No committed snapshot
contains that character and the protocol has no V1; the branch is dead and
its prose is a ghost reference to the retired dialect.

Evidence:

    tools/digestshare
        20	V1 side-by-side timelines (renders containing the `│` column separator)
        21	are skipped and reported as such: their transcripts show each byte twice
        22	(sent and received), so totaling them would double-count; the V1 digest
        23	atom is the same `Hash` the V2 corpus measures.
        48	    if "│" in body:
        49	        return None
        76	            if measured is None:
        77	                print(f"{path.name}: skipped (V1 side-by-side timeline)")
        78	                continue

Resolution: delete the `│` skip, the `measure` docstring's "or None" clause,
and the header paragraph; keep the liveness failure (a corpus with captures
but zero parsed bytes) as the tool's only non-threshold exit. Acceptance:
`grep -n 'V1\|│' tools/digestshare` is empty and `just digestshare` passes.

See also: session-bookmark-12.

### Nits (2)

The full record of each nit, in the finalizers' template, is in its evidence file (`evidence/partitions/<key>.md` or `evidence/sweeps/<key>.md`).

| id | where | claim | resolution |
|---|---|---|---|
| deps-8 | `Cargo.toml:126-126` | bytes' `serde` feature is enabled but no serde path touches a `Bytes` | `bytes = { workspace = true }` |
| deps-9 | `Cargo.toml:153-153` | the tokio dev-dependency's `default-features = true` is vacuous | Delete `default-features = true` from the tokio dev-dependency entry |

## Positives

- The type-state builders and the closed `Batch` scope do the work prose would otherwise do: `Bootstrap` and `BookmarkedBootstrap` give each `join` only its own outcomes (bootstrap.rs:262-273), and `Batch` is closed by the borrow checker with two `compile_fail` doctests pinning both escape routes (rumors.rs:320-336).
- Manual impls that exist for a reason say so in place: `Bootstrap`'s `Clone` (bootstrap.rs:81-83) and `Debug` (286-287), `ErrorRoute<E, S>`'s `Clone` in driver.rs, `Oracle<T>`'s hand-written `Default` (tests/common/oracle.rs:22-29), and the `PhantomData<fn() -> H>` argument at height.rs:6-21.
- Small mechanisms compose without a second concept: `Done<Half>` lets single-use and recycling transports share one completion hook and rides through the erasure layer as a `Bundle<H>` (link.rs:171-199, erased.rs:28-87); `Registration`'s `Drop` is the whole revocation protocol (router.rs:90-94); `SessionState` is sealed with `Copy`, private fields, public getters, and `pub(crate)` mutators (link.rs:345-407).
- The erasure boundary is thin and stated: walk bodies take `Replies<E>` and instantiate once per backend, `Work::respond` is the single re-tag point, and the prefix's byte length is the one runtime height witness (erased.rs, levels.rs:143-146 and 257-260, pump.rs module doc).
- Ordering rules are enforced by API shape rather than discipline: `Encoded::write_with` releases a question only after the frame's write resolves `Ok` (encode.rs:26-36), and `yield_resolve_query!`/`yield_reply_scopes!` keep wire, resolution, and dependent work in one expansion with the trace hooks inside it (materialized.rs:127-171, proxy.rs:18-38).
- `queues.rs` gives every channel edge a named constructor carrying its own capacity argument and separates the correctness floor (`assembly_level_returns` at `FAN`) from the amortization; `tasks.rs` is five items each used by both participants with a one-line invariant apiece.
- The wire codec keeps one definition per rule: `ListingBuilder` is the single ingress gate for both listing surfaces (frame.rs:421-486), `admits(body, record)` is `covers(body + record)` (budget.rs:147-162), `Heads<const N>` derives its capacity from the grammar's maxima (encode.rs:46-78), and `label()` is the one spelling of the stream label that the capture harness parses through the same head grammar (streams.rs:57-68, capture.rs:117-122).
- Layout facts the pricing rests on are compiler-checked: `REFERENCE_SLOT_BYTES` and `FAN_SLOT_BYTES` derive from `size_of` (window.rs:139-176), `local.rs:110` pins the `Local` handle at pointer size, and `height.rs:166` ties the numbered aliases to their heights with `const _: () = assert!(..)`, which is the idiom several findings here ask the neighbouring literals to adopt.
- `Progress` is a `Copy` type that is a ZST outside `cfg(test)`, passed by value, with the trace machinery behind one `#[cfg(test)] mod trace` (proxy/work/progress.rs): instrumentation with no `cfg` in production signatures.
- `remote/error.rs` is a nineteen-line flat alias layer with one renaming convention (`Codec*`, `Reply*`) that `src/error.rs:44-50` consumes by name; `streaming.rs:1-40` is an orientation map naming each child module with the one reason it is separate; every `.rs` file under `src/` is declared exactly once.
- The crate's clippy posture is clean where it matters: `await_holding_lock` produced zero hits, `future_not_send` does not fire on the public `conformance::link::check`, every narrowing `as` in non-test library code sits behind a range check or a `const` bound, match arms name the enum explicitly (154 sites to 13 `Self::`), and `let ... else` is the idiom at 107 sites.
- Inline tuple types under `#[allow(clippy::type_complexity)]` are preferred to coined synonyms where a name would carry no meaning (envelope_sim.rs:1335 and 1390, session_stats.rs:215-227, wire_legibility.rs's `corpora`), which is the doctrine applied.
- The manifest is disciplined: the workspace dependency table is the single version of record and `tools/manifestlint` states why cargo cannot check it; tokio's normal features (`io-util`, `macros`, `sync`) match exactly what the shipped library uses; `conformance` adds no tokio features; the `ciborium` pin and the `[lib] bench = false` comment each state the invariant they hold.
- The V1 retirement's prose pass held almost completely: across the crate the surviving references to deleted identifiers are the sites in tree-typed-19/prose-hygiene-1 and the "select" vocabulary, no `V1`, `LEGACY_MAGIC`, `Alternating`, or `blake` token remains, "mint" is purged, and there is no `TODO`, `FIXME`, or `HACK` in scope.

## Open questions for Finch

Decisions only the owner can make, deduplicated across partitions. Each carries the reviewers' recommendation.

1. **`pub` versus `pub(crate)` under the private `tree` module** (inventory-10, module-graph-12, tree-typed-1, streaming-backend-window-2, materialized-6, mirror-common-20). Enable `#![warn(unreachable_pub)]` and follow every diagnostic, or record the pub-in-private convention in AGENTS.md and reconcile typed.rs:19-22 with it? The sweep corrected an underestimate: the lint fires on every `pub` item unreachable from the crate root, not only the ten `pub mod` sites, so adoption is a campaign. Recommendation: enable the lint; it makes the API surface legible from the keyword and closes the class mechanically, and the pub-for-rustdoc-links rationale holds under `pub(crate)`.
2. **Inline `mod tests {}` blocks** (module-graph-6, suite-economics-11, mirror-common-22, streaming-backend-window-14, testing-infra-5, tree-core-25). Move all six to sibling files, or amend AGENTS.md to exempt test-only scaffolding modules (four of the six)? Recommendation: move all six; each is a pure move, `driver.rs` and `arb.rs` are not scaffolding, and a rule without exceptions is the one a checker can enforce. The inline production modules of module-graph-14 are separate and also pure moves.
3. **Em-dashes in `//` comments** (eight entries, roughly 159 sites). Sweep once and add a `tools/` check, or record a tolerance where the rule lives? Recommendation: the check plus one sweep; the rule already applies to assert messages, and nothing enforces it today.
4. **Import grouping** (tests-wire-format-10 and the import-hygiene pattern). Adopt `group_imports = "StdExternalCrate"` in a `rustfmt.toml`? The option is unstable, so `fmt-check` would move to the pinned nightly. Recommendation: adopt it if the nightly leg is acceptable for formatting; otherwise one hand sweep now, since three mechanical commits produced the residue and a fourth will produce more.
5. **A `[lints]` table** (clippy-pedantic open question; api-audit-12; api-core-34). Adopt `elidable_lifetime_names`, `redundant_closure_for_method_calls`, `manual_let_else`, and `match_wildcard_for_single_variants` at `warn` (zero false positives in the run), plus `missing_debug_implementations` and `unnameable_types`? Recommendation: adopt all six; each rides the existing `-D warnings` gate and closes a class this document lists by hand.
6. **Which layer of `type_complexity` allow to keep** (inventory-3, materialized-8, mirror-common-29). Recommendation: keep the module-wide allow at streaming.rs:43 and delete the item-level ones; the item-level allows document where the complexity lives, but a reader cannot tell which layer is active today.
7. **`seed_rng` and `warm_caches`** (api-core-12, inventory-4). Gate them under `test-internals` (with a private inner function for `seed`), or un-hide and document `seed_rng` as the deterministic-seeding entry downstream test suites want? The inventory sweep recommends un-hiding; the api-core finalizer recommends gating. Recommendation: gate `warm_caches` (calibration only) and decide `seed_rng` on whether deterministic `Network` seeding is a capability applications should have; if it is, document the shared-`Network` hazard beside it.
8. **`Snapshot::iter` and the unnameable `Iter`** (api-core-34). Delete the `IntoIterator` impl and the `pub use` so opacity is complete, or re-export `Iter` at the root and return it concretely, reversing 8dc0596ed? Recommendation: complete the opacity, since history already chose it; the tree-core finalizer recommends the reverse (re-export `Iter`, in the std convention), so this is a fork for the owner to settle.
9. **Two public `PreambleDefect` variants no peer can produce** (mirror-common-10). Reopens ruling R2 on one new fact: `Staged::buf` is now `[u8; V2_PREAMBLE_LEN]`, so the typed `decode` signature that dissolves `NetworkTruncated` and `TrailingBytes` is free. Recommendation: take it; the variants' only test artifact is a comment explaining why they cannot be tested.
10. **`ReplyFrameError` and `RemoteError::ReplyFrame`** (remote-adapter-streams-19). Delete the runtime `TryFrom` and the public variant in favor of infallible `ReplyFrame` constructors yielded by `render`? Recommendation: yes, pre-release; the variant has no producer.
11. **Typed head and listing defects in `DecodeErrorKind`** (remote-codec-9). Add `Head { part, source: HeadError }` and `InvalidListing(ListingIssue)`, dissolving `listing_issue` and `head_detail` and the atlas exemption? Recommendation: yes; it matches the greeting's typed surface and the re-accept is deliberate.
12. **`Error::widen` and the flat generic error** (api-core-7). Give the wire layer a non-generic session error wrapped beside `Bookmark`, or record the flat-shape decision in `widen`'s doc? Recommendation: record the decision unless a second generic arm ever appears; the flat shape's payoff (one-level matching and the remedy table) stands.
13. **`LinkParts`** (link-9). Publish `Link`'s fields and dissolve the twin, or keep it and state the representation-freedom reason in `Link`'s doc? Recommendation: dissolve, pre-release, unless a representation change is planned; 63 decorate-and-rebuild sites move mechanically.
14. **`#[non_exhaustive]` on public enums** (link-17). Rule crate-wide which taxonomies are open; the codec and adapter partitions ask the same question for their error enums. Recommendation: rule once, pre-release, and apply mechanically; the criterion recorded in 1e458d69 ("grows as enforcement grows") already exists.
15. **`NonZeroUsize` for `Config`'s counts and `memory_with_capacity`** (link-20). Recommendation: leave as is, consistent with the `usize`-plus-check precedent and the spirit of R79.
16. **The eightfold conformance signatures** (conformance-7) and **`ReversingAcceptor` versus `testing::ReorderingAcceptor`** (conformance-18), both reopening f5039abb. Recommendation: the non-breaking loosening of the focused checks' bounds plus one comment stating why the shape is otherwise kept; unify on `reorder_accepts` if `reordered > 0` and the five `Stalled` verdicts hold under the patient wait, otherwise have transport.rs's comment state the behavioral reason for two fixtures.
17. **`DEFAULT_TARGET_MESSAGE_SIZE`** (materialized-10, module-graph-2). Relocate beside `Greeting` and retype to `u64`, or keep it in `codec::budget` (its derivation is codec vocabulary) and thread the target through the walk's start? Recommendation: keep it in the codec and thread it, per module-graph-2; retyping is a public change to weigh separately.
18. **Ownership of the stream count** (link-3, remote-codec-3). Does `link::STREAM_COUNT` own the literal and the codec cite it, or does the codec derive `Stream::COUNT` from its stride constants and `link` keep the pin? Recommendation: the compile-time derivation either way; then one literal and one citation.
19. **`SCOPE_FIXED_BYTES` and `LEAF_REQUEST_BYTES`** (streaming-backend-window-26). Which two containers does the fixed cost price? Deriving from `size_of` may move the pinned `SCOPE_ENVELOPE_BYTES` (5_431) and the figure quoted at peer.rs:431. Recommendation: name `Query` and `Resolution`, derive both constants, and re-pin in one commit naming the layout attribution if the value moves.
20. **`examples/envelope_sim.rs`'s flat baseline and `L(N)` section** (benches-envelope-29). Retire the flat solve and the per-stream derivation, or keep them as a named comparison stated positively? Recommendation: retire; both agent notes record the rejected shape, and the tables describe a wire that no longer ships.
21. **Criterion series ids** (benches-envelope-2). Renaming the `V2` series breaks comparison against saved local baselines. Recommendation: rename now, before a release makes baselines worth keeping.
22. **Test-binary layout and a test-support crate** (suite-economics-8, tests-common-8, tests-disruption-handshake-28). Fold the window family into one binary and the single-test files into their categories; make `tests/common` a path dev-dependency crate; move `benches/support` into a dev-only crate? Recommendation: measure `cargo build --tests --timings` first; fold the window family and the single-test binaries regardless, since they lose no clarity; decide the crate on the number.
23. **The `encoded_bits` assertions and the `meter` dev-feature** (tests-observation-35). Reopens 05d87e1b on new evidence: `Party`'s byte-level equality implies the size equality. Recommendation: drop both, since a dependency feature whose only consumer is an implied assertion is the circular-justification tell.
24. **Two committed seeds annotated for properties that no longer exist** (tests-lifecycle-1, tests-common-32). AGENTS.md's never-strip rule makes any edit an owner call. Recommendation: remove each in a commit naming the deleted property, and extend `seed_liveness.rs` to match shrink-note parameter names against live `proptest!` signatures so the class cannot recur.
25. **`streaming` as a path segment** (module-graph-9). Fold its children into `tree::mirror`, or keep the level and re-word mirror.rs:3 so the name describes mechanism? Recommendation: keep the level and re-word; the segment still names the mechanism, and a fold touches every import and link under it.
26. **`tempfile` as a dev-dependency** (tests-wire-format-21). Recommendation: yes; it is already in the lockfile transitively.
27. **Design questions the findings raise without settling.** `Inner.party: Option<Party>` exists only while `Peer::retire` holds the party in flight and forces two unreachable arms with different shapes; holding the in-flight party in the retire future would delete both (api-core open question; recommendation: schedule as a design item). `define_peer!` spells a 16-deep and a 15-deep bound chain that height-indexed chain traits could derive (mirror-common open question; recommendation: one afternoon's experiment on a branch, abandoned only on compile-time or diagnostic evidence). `Progress` (a ZST passed by value) versus the walk's `#[cfg(test)]` parameters and `trace_id` field are two instrumentation shapes for one purpose (remote-proxy and materialized open questions; recommendation: keep the proxy's shape and move the walk toward it). `Backend` is a general trait with one implementor, unreachable from outside the crate, whose contract asserts guard backends that do not exist (inventory open question; recommendation: add the one-sentence "crate-internal today" status to backend.rs now and take the trait-shape pass at the publication commit). A named `Negotiated`/`Ingress` bundle for the greeting-derived premises contests the inline "one premise per argument" rationale at start.rs:338-339 (remote-proxy-4, remote-adapter-streams-4, remote-adapter-tests-2; recommendation: bundle after remote-proxy-7 collapses the hand-off, so the premise reading survives as the struct's field list). `FAN` and `MAX_QUERY_CHILDREN` name one radix fan as two constants (remote-codec-3; recommendation: define one from the other and say why). `bootstrap_fork` exists in three cargo-isolated copies (swarm-example open question; recommendation: accept for now and note it where the copies live, or resolve it with item 22).

## Counts

Recorded against commit 9e5784fb4dce977cfbdfd1619886d1482b5ce764; the counts describe this document's entries after reconciliation reduced five cross-lens duplicates (inventory-2, mirror-common-22, materialized-10, inventory-18, suite-economics-11) to cross-references that are not counted; `index/counts.tsv`, built from the partition and sweep reports, still counts them.

| severity | entries |
|---|---|
| high | 0 |
| medium | 37 |
| low | 154 |
| nit | 167 |
| total | 358 |

| class | entries |
|---|---|
| simplification | 128 |
| modularity | 30 |
| vestigial | 53 |
| idiom | 147 |

| module section | medium | low | nit | total |
|---|---|---|---|---|
| Crate root and public surface | 3 | 8 | 7 | 18 |
| Session and bookmark | 1 | 9 | 12 | 22 |
| Link | 1 | 5 | 7 | 13 |
| Conformance | 1 | 6 | 6 | 13 |
| Tree core | 1 | 4 | 9 | 14 |
| Tree typed | 2 | 8 | 14 | 24 |
| Mirror common | 3 | 12 | 16 | 31 |
| Streaming backend and window | 1 | 8 | 7 | 16 |
| Materialized | 2 | 10 | 7 | 19 |
| Remote codec | 2 | 6 | 2 | 10 |
| Remote capture and codec tests | 0 | 4 | 11 | 15 |
| Remote adapter and streams | 6 | 7 | 8 | 21 |
| Remote proxy | 3 | 10 | 9 | 22 |
| Test scaffolding | 2 | 5 | 3 | 10 |
| Integration tests | 7 | 41 | 37 | 85 |
| Benches and examples | 1 | 9 | 10 | 20 |
| Manifest, lints, and verification recipes | 1 | 2 | 2 | 5 |
