# Partition materialized: The materialized (fixed-memory) mirror: progress, transcript, unknown pruning, work queues, levels, resolver, assembly

## Partition summary

The `materialized` module is the in-process participant of the streaming mirror. `materialized.rs` holds the type-level phase schedule (`Handshaking<B, Start | Connecting | Connected>` into `Descending<B, H>` into `Completing`), the `SupplyLedger`, the `Query`/`Resolution`/`Resolve` item vocabulary, the `yield_resolve_query!` macro that fixes the publication order the deadlock-freedom argument rests on, and the initiator's terminal `absorb` loop. `work.rs` accumulates every independently runnable pump as a boxed task and drives them through `tasks::complete`; `work/levels.rs` holds the five walk bodies (initiator opening, responder opening, internal, leaf-parent, leaf), `work/answer.rs` the three merge-joins that answer one query, `work/resolver.rs` the per-reply reaction loop that classifies counterparty faults into `Violation`s, `work/assembly.rs` the positional upward reassembly, and `work/queues.rs` one named constructor per channel edge with its capacity argument. `unknown.rs` is the deletion-honoring prune, the streaming twin of `traverse::unknown`. `progress.rs` and `transcript.rs` are `#[cfg(test)]` thread-local recorders of publication order and payload-erased wire order that the cross-peer suites validate. `error.rs` defines `Error` and `Violation`, the only two items here that reach the crate's users (re-exported as `MaterializedError` and `MaterializedViolation` through `crate::error`).

I read all 18 files, 4511 lines. Test code: `progress.rs` (a `#[cfg(test)]` module, 535 lines), `progress/tests.rs` (198), `tests.rs` (166), `transcript.rs` (`#[cfg(test)]`, 112), `unknown/tests.rs` (84), `work/tests.rs` (270), and `work/tests/violations.rs` (352), 1717 lines in all; the remaining 2794 lines are production.

The production code is in good shape. I found no in-model correctness bug: every panic is guarded by structure the counterparty cannot influence, the stage heights line up, the stream-termination chain has no cycle, and every peer-controlled datum reaches the walk through a merge-join or a peekable fan. The deadlock-freedom argument in the module doc is precise and is enforced structurally by the macro and re-checked by the test-only trace; the channel constructors each carry their own capacity argument; the height erasure is done with a thin typed boundary. The dominant issues are on the verification side and in prose. The one high finding is that the leaf-height deletion verdict in `unknown` has no committed test that fails when it is inverted: the handoff note from 2026-08-21 records the surviving mutant, no follow-up has landed, and by reading I can say why every fixture, including the module's own oracle proptest, misses the arm. Below that sit a medium simplification (the initiator's terminal state hand-rolls `tasks::complete`), a medium legibility fix in `internal_walk`'s opening hand-off, two medium prose findings (roster IDs and adjudication narrative in the test-only recorders, which the hard rules forbid; public `Violation` docs that name reaction kinds the user cannot reach), and a long tail of low and nit items: duplicated handshake derivations, a vestigial version copy, redundant `Sync` bounds and dead clippy attributes, one ghost reference, one vocabulary collision, and a handful of default-dialect tells.

## Findings

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

### materialized-2: Window saturation is unobservable to the user tuning the budget
- Where: src/tree/mirror/streaming/materialized.rs:147-156 (related: src/tree/mirror/streaming/materialized/work/levels.rs:422, 503; src/tree/mirror/streaming/stats.rs:133-150; src/tree/mirror/streaming/window.rs:55-60)
- Class / severity / confidence: feature-gap / low / medium
- Provenance: verified (read `SessionStats` in full: `window_granted` is the only window field, stats.rs:133-150; the struct is `#[non_exhaustive]`, stats.rs:40; window.rs:57 documents serialization as the degradation)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (487e17ea added `window_granted` as "the window solve's widest granted stage"; 7626543e pinned every counter against a computable dispute oracle, and a stall count is scheduler-dependent, so it could only be pinned as zero versus nonzero)
- Owner-gated: yes: adds a public `SessionStats` field

The window doc says that when a stage's population exceeds its capacity "that stage serializes", and `SessionStats::window_granted` reports only the granted ceiling. Nothing reports whether any send in the macro ever waited on a full edge, so a user who set `Peer::sync_memory_budget` cannot tell whether the budget bound the session or was slack; `SessionStats` is exactly where the crate already hands such readouts back.

Evidence:

    147	        if $resolutions.send(resolution).await.is_err() {
    148	            return;
    149	        }
    150	        for query in $next_queries {
    151	            #[cfg(test)]
    152	            progress::dependent($work, &query);
    153	            if $queries.send(query).await.is_err() {
    154	                return;
    155	            }
    156	        }

Resolution: Owner decision. If wanted: before each window-edge `send().await` in the macro and at the `upper.send` sites in `levels.rs`, spot-check `sender.capacity() == 0` (O(1) on tokio's bounded `Sender`) and count through a new `Recorder::stalled()` into a `SessionStats::window_stalls: u64` field documented like its siblings. Acceptance: a test in `streaming/tests/stats.rs` runs a maximally disputed fixture at `WindowConfig::FLOOR` and asserts `window_stalls > 0`, and the same fixture at a wide window asserts `window_stalls == 0`; `tests/session_stats.rs` re-checks the field publicly.

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

### materialized-4: `SupplyLedger::charge` reports its overdraw as a bare `u64`
- Where: src/tree/mirror/streaming/materialized.rs:228-241 (related: src/tree/mirror/streaming/materialized.rs:248; src/tree/mirror/streaming/remote/adapter/decode.rs:165-167, 372-374)
- Class / severity / confidence: idiom / nit / medium
- Provenance: verified (read all three callers: the walk discards the payload at 248 with `Err(_)`; both decoder sites render it as `DecodeError::OverdrawnSupply { declared }`)
- Seen by: perfapi; refutation: confirmed; history: deliberate and holds for the vocabulary-neutral error (08f2899b; the doc at 230-233 states why the ledger does not return a `Violation`), which does not exclude a named payload
- Owner-gated: no

An unnamed integer in error position leaves the reader to consult the doc comment to learn what the number is; a newtype makes the three call sites self-describing and lets the decoder's rendering name the field at the source.

Evidence:

    234	    pub(crate) fn charge(&self, leaves: u64) -> Result<(), u64> {
    235	        let prior = self.absorbed.fetch_add(leaves, Ordering::Relaxed);
    236	        match prior.checked_add(leaves) {
    237	            Some(total) if total <= self.declared => Ok(()),
    238	            // A wrapped counter is past any declarable length too.
    239	            _ => Err(self.declared),
    240	        }
    241	    }

Resolution: Introduce `pub(crate) struct Overdrawn { pub declared: u64 }` as the `Err` type. Acceptance: no `Result<(), u64>` in the crate; the decoder's overdraw error carries the same declared length (its snapshot pins unchanged).

### materialized-5: `Query`'s summary sentence describes the queue, not the item
- Where: src/tree/mirror/streaming/materialized.rs:253-255 (related: src/tree/mirror/streaming/materialized.rs:375)
- Class / severity / confidence: documentation / nit / high
- Provenance: assessed (read)
- Seen by: prose; refutation: confirmed; history: no rationale found (verbatim from 7126e489)
- Owner-gated: no

A `Query` is one item of `Receiver<Query<B::Erased>>` (line 375), but its first sentence, which is what the module index shows, calls it "the pairing queue between consecutive same-side stages".

Evidence:

    253	/// A pending query, which we will resolve by a remote reply: the pairing
    254	/// queue between consecutive same-side stages, and the in-process twin of
    255	/// the wire's expected scopes.

Resolution: "A pending question, resolved by one remote reply: one item of the pairing queue between consecutive same-side stages, and the in-process twin of a wire scope." Acceptance: the summary sentence names the item.

### materialized-6: Field visibility on `Query`, `Resolution`, and `Resolve` is inconsistent inside a tree that is private at the crate root
- Where: src/tree/mirror/streaming/materialized.rs:265-288 (related: src/lib.rs:322; src/error.rs:41-43; src/tree/mirror/streaming/window.rs:123)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: `mod tree;` is private at lib.rs:322; the only re-exports from this module are `Error` and `Violation` at src/error.rs:41-43; `Resolve` is used outside the module at window.rs:123, so `pub(crate)` is its true reach)
- Seen by: structure, perfapi; refutation: confirmed; history: no rationale found (the mix predates the erasure and traces to 7126e489; `pub` items inside private modules is the crate-wide idiom, e.g. `protocol.rs`'s traits)
- Owner-gated: no

`Query` has `pub` fields, `Resolution` has `pub(crate)` fields, and both are unreachable from outside the crate; a reader seeing `pub` beside `pub(crate)` on sibling types looks for the API boundary the distinction implies and finds none. The crate-wide `pub`-in-private-module idiom means the ask here is local consistency, not a visibility sweep.

Evidence:

    265	pub struct Query<E> {
    266	    /// The prefix at which the resolved node will sit.
    267	    pub prefix: ErasedPrefix,
    268	    /// Our children of the node (empty if we don't have it at all).
    269	    pub ours: Vec<(u8, E)>,
    270	}
    ...
    274	pub struct Resolution<E> {
    275	    /// The prefix at which the resolved node will sit.
    276	    pub(crate) prefix: ErasedPrefix,
    277	    /// The possibly-resolved children of the node.
    278	    pub(crate) resolved: Vec<(u8, Resolve<E>)>,

Resolution: Pick one spelling for the walk's three item types. Whether to enable `unreachable_pub` crate-wide is a separate owner question (see below). Acceptance: the three item types use the same field visibility.

### materialized-7: `Descending`'s doc links the erased `Reply` under the typed signature `Reply<B, H>`
- Where: src/tree/mirror/streaming/materialized.rs:358-359 (related: src/tree/mirror/streaming/materialized.rs:110; src/tree/mirror/streaming/erased.rs:63; src/tree/mirror/streaming/message.rs:159)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (the import at materialized.rs:110 is `erased::{self, Reaction, Reply}`; `erased::Reply<E>` has one parameter at erased.rs:63; `message::Reply<B, H>` at message.rs:159 is the type the stage erases on entry)
- Seen by: structure; refutation: confirmed; history: deliberate but expired (written against `message::Reply<B, T, H>` at 7126e489; bf1a5b4b switched the import to the erased type and 48bc31df dropped `T` mechanically, leaving the two-parameter spelling)
- Owner-gated: no

The link text names a type with the wrong arity for the type it resolves to, misdirecting the reader about which vocabulary the stage speaks; `work.rs:9-14` takes care to say the walks run erased.

Evidence:

    358	/// A mirror stage inside the descent, consuming [`Reply<B, H>`](Reply)
    359	/// against a [`Query`] queue at the same height.

Resolution: "consuming the stage's [`Reply`]s (erased on entry by [`erased::erase_reply`]) against a [`Query`] queue at the same height." Acceptance: the link text matches the linked type's arity.

### materialized-8: Attributes that guard nothing: twelve per-item `type_complexity` allows under a module-wide allow, and `#[cfg(test)]` inside a `#[cfg(test)]` module
- Where: src/tree/mirror/streaming/materialized.rs:388 (related: src/tree/mirror/streaming.rs:42-43; src/tree/mirror/streaming/materialized/tests.rs:48; work/answer.rs:31, 113; work/resolver.rs:61; work/levels.rs:67, 174, 296, 341, 539, 644; work/tests/violations.rs:100; src/tree/mirror/streaming/materialized/progress.rs:109, 213; src/tree/mirror/streaming/materialized.rs:176-177)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (grep `type_complexity` across the partition lists the twelve sites; streaming.rs:43 is `#![allow(clippy::type_complexity)]` for the whole streaming tree; `progress` is declared `#[cfg(test)] pub(super) mod progress;` at materialized.rs:176-177)
- Seen by: refutation pass (allows), structure (cfg); refutation: raised as new; history: no rationale found (the inner `cfg(test)` attributes were already redundant when 77674c9c and 4407590b wrote them)
- Owner-gated: no

The module-level allow means the lint is off for every item beneath it, so each per-item `#[allow(clippy::type_complexity)]` is dead, and a reader takes them as the lint having noticed something. Likewise `assert_valid_without_wire_contiguity` and `assert_parent_early` carry `#[cfg(test)]` inside a module that only exists under `cfg(test)`, and the doc's "test-only entry point" distinguishes nothing. Either the module-level allow or the per-item ones should go.

Evidence:

    388	    #[allow(clippy::type_complexity)]
    389	    early_supplies: Option<oneshot::Receiver<Vec<(u8, Vec<(u8, B::Erased)>)>>>,

    42	// Where we're going, we need to write some Complex Types.
    43	#![allow(clippy::type_complexity)]

    109	    #[cfg(test)]
    110	    fn assert_valid_without_wire_contiguity(&self) {

Resolution: Delete the twelve per-item allows (or the module-level one, if the owner prefers per-site justification), delete the two inner `#[cfg(test)]`, and reword progress.rs:106 to "This entry point keeps the sibling-contiguity check independently falsifiable". Acceptance: `just clippy` clean with one `type_complexity` policy in the streaming tree; no `#[cfg(test)]` inside `progress.rs` other than `mod tests;`.

### materialized-9: Qualified paths where imports exist, and clone helpers that half the call sites bypass
- Where: src/tree/mirror/streaming/materialized.rs:400 (related: src/tree/mirror/streaming/materialized.rs:111, 124, 648, 700, 751, 865; src/tree/mirror/streaming/materialized/work.rs:16, 76-83, 109; work/assembly.rs:41, 54; work/levels.rs:88, 195, 361, 556, 656)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `PhantomData` in the partition: four `std::marker::PhantomData` spellings; `impl futures::Stream` at 865 with `futures::StreamExt` imported at 124; `std::pin::pin!` at work.rs:109 with `std::pin::Pin` imported at 16; `Work` imported by its full crate path at 111; grep `.stats()` shows one caller at 827 while levels.rs clones `self.stats` directly at five sites; `self.backend.clone()` at assembly.rs:41, 54 bypasses `Work::backend()`)
- Seen by: structure; refutation: confirmed; history: no rationale found (accretion across bf1a5b4b, 7126e489, 487e17ea)
- Owner-gated: no

Imports over long qualified paths except where the qualification informs; a clone helper that half the call sites bypass is not pulling its weight either way. `Work::stats()` disappears with materialized-13.

Evidence:

    400	    height: std::marker::PhantomData<fn() -> H>,

    111	        materialized::work::Work,

    865	    requests: impl futures::Stream<Item = Reply<B::Erased>> + Send,

Resolution: Import `PhantomData`, `Stream`, and `pin`; `use work::Work;` beside `use channel::...`; use `self.backend()` in `assembly.rs` or delete the helper. Acceptance: no `std::marker::`, `std::pin::pin!`, or `futures::Stream` at use sites in the partition; helper usage uniform.

### materialized-10: The walk imports `DEFAULT_TARGET_MESSAGE_SIZE` from `remote` and casts it to the greeting's type
- Where: src/tree/mirror/streaming/materialized.rs:438 (related: src/tree/mirror/streaming/materialized.rs:114; src/tree/mirror/streaming/remote/codec/budget.rs:71; src/tree/mirror/streaming/message.rs:103; src/message.rs:59; src/peer.rs:18)
- Class / severity / confidence: modularity / low / medium
- Provenance: verified (grep `DEFAULT_TARGET_MESSAGE_SIZE` across src/: defined as `usize` at budget.rs:71, imported at materialized.rs:114, cast at 438; `Greeting.target_message_size` is `u64` at message.rs:103; this is the only `remote::` import in materialized.rs; the public name is re-exported through peer.rs:18 and lib.rs:342)
- Seen by: structure; refutation: confirmed; history: no rationale found (bdf74d45 introduced the cast when it made the greeting the single source of the target; the sibling default `DEFAULT_PAYLOAD_DEPTH_LIMIT` lives in `crate::message` typed as its field)
- Owner-gated: yes: retyping the constant to `u64` changes a public item; relocating it alone does not

A default both participants advertise in the greeting is typed for the codec's `RunBudget` and lives in the proxy's module, so the in-process walk depends on the wire proxy for one constant and pays a cast. The module-doc reading order at streaming.rs:10-24 lists `remote` after `materialized`; it does not assert a strict layering, so this is an inference from the import direction, not a documented breach.

Evidence:

    438	            target_message_size: DEFAULT_TARGET_MESSAGE_SIZE as u64,

Resolution: Define the constant beside `Greeting` in `message.rs` (keeping the `FAN * FULL_FAN_QUERY_FRAME_LEN` derivation), have `remote::codec::budget` import it and convert once at `RunBudget::from_bytes`, drop the cast here. The type change is the owner's call. Acceptance: `materialized.rs` imports nothing from `remote`; the value pinned at `budget/tests.rs:9` is unchanged.

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

### materialized-14: The opening and terminal legs classify and check counterparty faults differently from the `Resolver`, contradicting the `Violation` docs
- Where: src/tree/mirror/streaming/materialized.rs:884-896 (related: src/tree/mirror/streaming/materialized/work/levels.rs:204-248; src/tree/mirror/streaming/materialized/error.rs:25-33; work/resolver.rs:66-109; work/tests/violations.rs:246-297; src/tree/mirror/streaming/tests/faults.rs:41-46, 66-67; src/tree/mirror/streaming/testing/faulting.rs:213-241; src/tree/mirror/streaming/materialized/tests.rs:49-86)
- Class / severity / confidence: correctness / low / high
- Provenance: assessed (read; the classification arms and the `Resolver` arms were compared by reading, and grep confirms no test drives `absorb` or `responder_level` with the malformed shapes)
- Seen by: correctness, perfapi (terminal), correctness (opening); refutation: confirmed, with corroboration that the connected suite's `0..=15` step range cannot reach the terminal reply phase on the depth-32 comb and that `Faulting` can already script all eight reply-shaped violations while `arb_connected_violation` samples two; history: no rationale found (the `absorb` arms are verbatim from 7126e489 under a comment stating the acceptance condition, not the taxonomy; the opening loop is from 55d76d5c with `UnexpectedQuery` as its catch-all). One recorded constraint: 50c8b0a3 records "a pre-existing liveness gap at the position", a violation raised in the responder's early-supply loop before the stage's one reply yields stalls a wire session instead of aborting typed, and the probing test was held back; every new opening-leg violation proposed here fires at that position.
- Owner-gated: no

Three related gaps. (a) `absorb` maps `[Match]`, `[Query(_)]`, and every reply with two or more reactions to `UnfinishedReply`, whose public doc is "The reply ended before reacting to every listed child"; a leaf request lists no children and these replies have too many reactions, not too few. The `Resolver` maps the same shapes to `UnexpectedMatch`, `UnexpectedQuery`, and `InvalidSupply`. (b) `responder_level` reports a missing opening `Query` and any non-`Supply` trailing reaction as `UnexpectedQuery` ("A positional `Query` after every held child has been answered"), which is false for a leading or trailing `Match`. (c) The responder's early supplies get the containment and ledger checks but not the structural checks every solicited supply gets (strictly ascending radices, radix not already held; resolver.rs:74-83): an out-of-order, duplicated, or locally-held early radix is charged to the ledger, exploded, and parked in `supplied` where no root-level request will claim it, so its `messages_gained` credit never lands; and the opening reads exactly one reply (levels.rs:207) without the trailing `requests.next().await.is_some()` check every descending walk performs, so a second opening reply is never `UnaskedReply`. All of this is off-model for security (an honest initiator emits none of these shapes) and in-model for the taxonomy: `error.rs:10-12` promises "exactly the semantic faults", `MaterializedViolation` is public, and the exact-variant proptest never drives either leg. On the wire path the adapter already rejects non-ascending supplied paths (adapter.rs:73; `LeafOrder`), so (c) is narrower over a wire than in process.

Evidence:

    884	        let supply = match replies.as_slice() {
    885	            [] => None,
    886	            [Reaction::Supply(radix, leaf)] if *radix == expected => {
    887	                if !contained(leaf.span().hi(), &their_version) {
    888	                    return violation(Violation::UncontainedSupply);
    889	                }
    890	                ledger.absorb(leaf.len() as u64)?;
    891	                stats.gained(1);
    892	                Some(leaf.clone())
    893	            }
    894	            [Reaction::Supply(_, _)] => return violation(Violation::InvalidSupply),
    895	            _ => return violation(Violation::UnfinishedReply),
    896	        };

    211	            let Some(Reaction::Query(theirs)) = reactions.next() else {
    212	                return violation(Violation::UnexpectedQuery)?;
    213	            };
    214	            let mut early = Vec::new();
    215	            for reaction in reactions {
    216	                let Reaction::Supply(radix, node) = reaction else {
    217	                    return violation(Violation::UnexpectedQuery)?;
    218	                };

    25	    /// The reply ended before reacting to every listed child.
    26	    #[error("reply failed to cover every listed radix")]
    27	    UnfinishedReply,

Resolution: In `absorb`, classify by the first offending reaction (`Match` -> `UnexpectedMatch`, `Query` -> `UnexpectedQuery`, a second reaction after a supply -> `UnexpectedSupply` or `InvalidSupply` by radix), and match over the owned `Vec` rather than `as_slice()` so the accepted leaf moves instead of cloning (`Some(leaf)`, removing the one `Arc` clone per requested leaf at 892). In `responder_level`, report a missing opening query as `UnfinishedReply` (or a documented new variant) and a non-`Supply` trailing reaction as `UnexpectedMatch`; check early radices strictly ascending and absent from `fan`, reporting `InvalidSupply`/`UnexpectedSupply`; add the trailing `requests.next()` check. Alternatively route both legs through `Resolver::react` so one classifier serves every height. Before landing the opening-leg changes, resolve the 50c8b0a3 liveness gap (a violation before the opening's one yield must abort typed over a wire). Then add `Completing`- and opening-leg injections to `violations.rs`, and widen `arb_connected_violation` to every variant `Faulting` can script. Acceptance: each malformed terminal and opening reply shape maps to the `Violation` whose doc describes it, pinned by committed injections that fail on the current arms; no `.clone()` in `absorb`; the four `terminal_absorb_*` tests still pass.

Construction: Reuse `absorb_scripted` in `materialized/tests.rs`, replacing the single `Reaction::Supply(0, ..)` with each of `[Match]`, `[Query(vec![])]`, `[Supply(0, leaf), Supply(0, leaf)]`, and `[Supply(1, leaf)]` against expected radix 0; on HEAD the first three report `UnfinishedReply`. For the opening, build `Work::new(Local, Window::FLOOR, Recorder::default()).responder_level(..)` with `fan = [(3, node)]` and an opening reply of `[Query([(3, h)]), Supply(3, other)]` (held radix) or `[Query([]), Supply(9, n), Supply(4, n)]` (descending): on HEAD both are accepted and the supplies sit unclaimed in `supplied`. A leading `[Match]` opening reply reports `UnexpectedQuery` on HEAD.

### materialized-15: Leaf counts cross the `usize`/`u64` boundary at every ingestion site, and `absorb` charges `len()` where it credits `1`
- Where: src/tree/mirror/streaming/materialized.rs:890-891 (related: src/tree/mirror/streaming/backend.rs:257-265, 302-303, 389-394; work/resolver.rs:92-95; work/levels.rs:227, 407-412; src/tree/mirror/streaming/materialized/unknown.rs:104, 150)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep `as u64` on `len()` in the partition: seven sites; `Node::len` and `ErasedNode::len` return `usize` at backend.rs:265 and 303, with `Root::len` casting once more at 389-394; the resolver uses `node.len() as u64` for both the ledger and the counter at 92-95)
- Seen by: structure, correctness; refutation: confirmed (correcting the structure lens: the trait is `Node`, backend.rs:207, not `Leaf`); history: no rationale found (487e17ea wrote `stats.gained(1)`; 50c8b0a3 added `ledger.absorb(leaf.len() as u64)` beside it without reconciling them)
- Owner-gated: no

The ledger and the counter are two views of the same absorbed leaf count, and `SessionStats::messages_gained` promises the exact live-leaf count; the pair agrees today because a leaf's `len()` is one by contract (backend.rs:259), but nothing at the site says so, and every `as u64` is a reader's question. The traits are crate-private (`mod tree;` at lib.rs:322), so retyping `len` is not a public API change.

Evidence:

    890	                ledger.absorb(leaf.len() as u64)?;
    891	                stats.gained(1);

Resolution: At minimum `let leaves = leaf.len() as u64; ledger.absorb(leaves)?; stats.gained(leaves);` to match the resolver; consider `fn len(&self) -> u64` on `Node`/`ErasedNode` in `backend.rs`, removing every cast. Acceptance: ledger and stats read one binding at the terminal leg; or no `as u64` on `len()` in the partition.

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

### materialized-17: `MaterializedError` derives only `Debug`, is exhaustive while its sibling is not, and the enclosing `mirror::Error`'s `Clone` derive is unsatisfiable for the production alias
- Where: src/tree/mirror/streaming/materialized/error.rs:1-8 (related: src/tree/mirror.rs:42-43; src/tree/mirror/streaming/remote/proxy/error.rs:16-18; src/error.rs:53)
- Class / severity / confidence: api-surprise / nit / medium
- Provenance: verified (read the three derives: `#[derive(Debug, thiserror::Error)]` here with no `#[non_exhaustive]`; `mirror::Error<C, S>` derives `Clone` at mirror.rs:42; `RemoteError<E>` derives only `Debug` and is `#[non_exhaustive]` at proxy/error.rs:16-18; `MirrorError = mirror::Error<MaterializedError<Infallible>, RemoteError<Infallible>>` at src/error.rs:53)
- Seen by: perfapi; refutation: reframed (the dead `Clone` hinges on both inner types, not this one alone); history: no rationale found (the outer `Clone` predates this type, 2fd590d5)
- Owner-gated: yes: public derives and exhaustiveness

`Clone`/`PartialEq`/`Eq` are free here (`Violation` is `Copy + Eq`; the production `E` is `Infallible`), and a `Clone` derive on the outer `mirror::Error` that neither inner type satisfies is a dead derive. The two inner error enums also disagree on `#[non_exhaustive]`; the two-variant partition ("a backend error or a counterparty Violation") is a defensible reason to stay exhaustive, but the decision should be explicit and made for both types together.

Evidence:

    1	/// A session-fatal failure: a backend error or a counterparty [`Violation`].
    2	#[derive(Debug, thiserror::Error)]
    3	pub enum Error<E> {
    4	    #[error(transparent)]
    5	    Backend(#[from] E),
    6	    #[error(transparent)]
    7	    Violation(Violation),
    8	}

Resolution: Owner decision. Add `Clone, PartialEq, Eq` to both `MaterializedError` and `RemoteError` (or drop `Clone` from `mirror::Error`), and decide `#[non_exhaustive]` for both, recording the reason in each doc comment. Acceptance: `MirrorError: Clone + PartialEq` compiles, or `mirror::Error` no longer claims `Clone`; the exhaustiveness choice is stated at both enums.

### materialized-18: The public `Violation`/`Error` rustdoc names reaction kinds the user cannot reach and leaves `Error`'s variants undocumented
- Where: src/tree/mirror/streaming/materialized/error.rs:10-39 (related: src/tree/mirror/streaming/materialized/error.rs:3-7; src/error.rs:41-43; src/tree/mirror/streaming.rs:49, 51)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (src/error.rs:41-43 re-exports both enums; `Reaction` lives in `pub(crate) mod message` and private `mod erased`, streaming.rs:49/51; grep of lib.rs and Cargo.toml finds no `missing_docs` lint; `Error::Backend` and `Error::Violation` carry no doc comment)
- Seen by: prose; refutation: confirmed; history: no rationale found (the variants have been undocumented since 7126e489; the style pass dfd19c44 rewrote `Violation`'s summary but left the `Match`/`Query`/`Supply` vocabulary)
- Owner-gated: no

This file is user-facing: a user arriving from `Error::Mirror` to file a bug report (error.rs's own module doc promises the taxonomy is for matching and bug reports) meets `Match`, `Query`, `Supply`, and "positional", the vocabulary of a `pub(crate)` type defined nowhere in the public docs, and an enum that speaks as "we"/"our". Documentation altitude for public rustdoc: state the contract, name nothing the API does not reach.

Evidence:

    3	pub enum Error<E> {
    4	    #[error(transparent)]
    5	    Backend(#[from] E),
    6	    #[error(transparent)]
    7	    Violation(Violation),

    10	/// The ways a counterparty can misbehave: exactly the semantic faults
    11	/// only this side can detect, because they depend on what we hold (our
    12	/// questions, our tree, and the greeting the peer declared to us).

    28	    /// A positional `Match` after every held child has been answered.
    29	    #[error("reply attempted to match unknown child")]
    30	    UnexpectedMatch,

Resolution: Give `Error::Backend` and `Error::Violation` one-line docs (noting that in `MirrorError` the backend error is `Infallible`). In `Violation`'s enum doc, define the three answer kinds once in plain language (a reply answers each child the question listed, in order, with an agreement, a sub-question, or, for a child the asker lacks, a supplied subtree) and rewrite the variants against those words; replace "we"/"our" with "the receiving replica". Acceptance: `cargo doc` for `rumors::error::MaterializedViolation` reads without a backtick identifier that does not resolve to a public item; every variant of both enums has a doc comment.

### materialized-19: `progress.rs` prose: a hand-maintained count, a one-off name for a named invariant, two glosses for `d6`, and a check cited by quoting another file's comment
- Where: src/tree/mirror/streaming/materialized/progress.rs:58 (related: src/tree/mirror/streaming/materialized/progress.rs:94, 117, 122-124; src/tree/mirror/streaming/materialized/progress/tests.rs:23; src/tree/mirror/streaming/materialized/work/levels.rs:498-499, 609-610; src/tree/mirror/streaming/materialized.rs:40-41)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (counted the seven bullets at 60-76 against the header; grep `write-before-publish` across src/ returns only progress/tests.rs:23; the quoted comment appears at levels.rs:498-499 and 609-610)
- Seen by: prose; refutation: confirmed; history: no rationale found (4407590b wrote "Seven checks:" and both `d6` glosses; "write-before-publish" predates the module doc's wording)
- Owner-gated: no

Four small prose defects in one file. "Seven checks:" is correct today and rots on the next added or merged check (no hand-maintained counts). progress/tests.rs:23 names the wire-before-internal-publication invariant "write-before-publish", a phrase used nowhere else, so it reads as a third invariant. `d6` is glossed "(parent-last)" at 94 and "(epilogue-placement)" at 117. And `assert_parent_last`'s doc anchors itself to a `//` comment in `levels.rs` by quoting its wording and naming the file; rewording that comment (it appears twice) silently orphans the citation.

Evidence:

    58	    /// Seven checks:

    94	    /// `d6` (parent-last) ordering axiom of the formal model: the local

    117	    /// This is the `d6` (epilogue-placement) ordering axiom of the

    122	    /// the discipline the encoder actually follows (the scope epilogue's
    123	    /// "Launch every `Pending` slot's work before publishing its
    124	    /// enclosing parent resolution" placement in levels.rs), and the

    23	/// Internal readiness before the corresponding wire action violates write-before-publish.

Resolution: "The checks:"; "violates wire-before-internal-publication"; one gloss for `d6` at both sites; and at 122-124 state the placement inline ("the discipline the walk follows: a scope's parent resolution is published only after its reaction loop ends and every child's queries are sent"). Acceptance: no numeral precedes the bullet list; `grep -rn write-before-publish src` is empty; `d6` carries one gloss; progress.rs contains no quoted comment text and no `.rs` file name.

### materialized-20: Agent-note roster IDs and process history cited from code: "finding #6", "finding #7", "adjudicated", "the mux adjudication"
- Where: src/tree/mirror/streaming/materialized/progress.rs:82-98 (related: src/tree/mirror/streaming/materialized/progress.rs:93, 114, 208; progress/tests.rs:62, 80, 102, 121; src/tree/mirror/streaming/materialized/transcript.rs:10-11; outside the partition: src/tree/mirror/streaming/tests/capacity.rs:244, 280; tests/local_eq.rs:142, 259; tests/wedge.rs:9)
- Class / severity / confidence: documentation / medium / high
- Provenance: verified (grep `finding #|adjudicat` across src/ and tests/ lists every site; the roster resolves only to `formal/MODEL.md:35-36`, `formal/PROGRESS.md`, and `.agent-notes/2026-07-18-parent-placement/`, none of which code may cite; `B5` is a named axiom, `formal/lean/StreamingMirror/Mux/Causal.lean:20`)
- Seen by: structure, prose, correctness, perfapi; refutation: confirmed (correcting the correctness lens: `B5` does resolve, as a Lean axiom, and may be cited by name); history: contradicts the hard rule (the IDs entered with 77674c9c and 4407590b; the 2026-07-23 citation sweeps 07f33b0c and ccdd4c5d removed `MODEL.md` path citations but left the IDs and the adjudication narrative)
- Owner-gated: no

The recorder's docs, its tests, and the transcript module tag their invariants with numbered findings that index the formal campaign's roster, and narrate how a decision was reached ("adjudicated", "promoted to a proptest bridge by the mux adjudication", "Retained as the design-space record"). AGENTS.md's hard rule forbids citing `MODEL.md`/`PROGRESS.md` from code, and an ID that resolves only there is such a citation by proxy; Principle 5 forbids opaque roster IDs and dated rationale at the declaration site. The text beside each tag already names the invariant in plain words (wire contiguity, parent placement, payload independence) and the Lean theorem names beside them (`Control.jam_not_deadlockFree`, `Sched.deadlock_free`, `Sched.deadlock_free_d5`, axiom `B5`) are the sanctioned anchors.

Evidence:

    81	    /// checks and deadlock. Wire contiguity is its wire-stream twin
    82	    /// (finding #6): without it, a wire stream that runs ahead of an

    93	    /// reorder within a channel. Parent placement (finding #7) is the

    208	    /// the adjudicated design decision, with the capacity-universal

    10	//! payloads — promoted to a proptest bridge by the mux adjudication
    11	//! (bridge B5): the announced dispute skeleton must be reconstructible from

Resolution: Delete every "(finding #N)" parenthetical. Rewrite progress.rs:206-212 to state the decision positively ("the walk publishes a scope's parent resolution last, trading any-capacity deadlock freedom for pipelining under the assembler's fan floor; `Sched.deadlock_free_d5` is the theorem for the other placement") without "adjudicated" or "Retained as the design-space record". In transcript.rs:7-13 keep the premise and the axiom name `B5`, drop "promoted to a proptest bridge by the mux adjudication". Sweep the sites outside the partition in the same pass. Acceptance: `grep -rn 'finding #\|adjudicat' src tests` returns nothing; the rewritten paragraphs read as present-tense statements of the discipline and the theorem that backs it.

### materialized-21: "The encoder" in the progress recorder means the walk, colliding with the crate's use of the word for the wire codec; "the weave" is model vocabulary never defined in the crate
- Where: src/tree/mirror/streaming/materialized/progress.rs:97 (related: src/tree/mirror/streaming/materialized/progress.rs:122, 127, 196-197, 203, 211-212; progress/tests.rs:96-109; src/tree/mirror/streaming/materialized.rs:450; src/tree/mirror/streaming/window.rs:82, 105, 112; src/tree/mirror/streaming/remote.rs:37; src/tree/mirror/streaming.rs:5-6)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (grep `encoder` across `streaming/`: the partition's ten sites all denote the traced walk; window.rs, remote.rs, backend.rs:163, message.rs:99, and materialized.rs:450 all denote the wire codec's encoder; streaming.rs:5-6 defines "the walk" as the in-process participant; `weave` is defined only in `formal/MODEL.md:37` and `EventDag.lean`)
- Seen by: prose, structure; refutation: confirmed; history: no rationale found (the term is the formal campaign's name for the walk, adopted by 77674c9c/4407590b; nobody weighed it against the codec's "encoder")
- Owner-gated: no

One term, one meaning per crate: a maintainer reading "the encoder's traces" one file away from "the wire encoder enforces each by panic" (backend.rs:163) must resolve the word per sentence. The crate already has a name for the traced participant. "The weave" costs the reader a lookup in a different artifact.

Evidence:

    97	    /// so the encoder's traces pin exactly the discipline the proof

    196	    /// weave's placement) — deliberately NOT wired into `assert_valid`:
    197	    /// the encoder does not and should not satisfy it.

    450	    /// session's encoders on both ends run at the minimum of the two

Resolution: Replace "the encoder" with "the walk" throughout progress.rs and progress/tests.rs (rename the test to `walk_order_violates_parent_early_discipline` and update the citation at progress.rs:212). Replace "the weave's parent-early discipline" with "the parent-early discipline, `d5` in the formal model" (or cite the Lean definition `weaveScope` by name if the owner wants the model term). Acceptance: `grep -n encoder` in the two progress files returns nothing; `weave` is defined at first use or absent.

### materialized-22: `assert_parent_early` is fed only a hand-written literal, so the pin its testdoc promises cannot fire
- Where: src/tree/mirror/streaming/materialized/progress/tests.rs:96-117 (related: src/tree/mirror/streaming/materialized/progress.rs:195-263; src/tree/mirror/streaming/tests.rs:83-95; formal/lean/StreamingMirror/Statement.lean:21)
- Class / severity / confidence: test-quality / low / high
- Provenance: verified (grep `assert_parent_early` across src/ and tests/: the definition at progress.rs:214 and one call at progress/tests.rs:116, fed the four-event literal at 110-115)
- Seen by: structure; refutation: confirmed; history: deliberate and holds for the retention (4407590b: "The parent-early (d5) probe stays as the design-space record ... still deliberately unwired", restated at progress.rs:195-212, and the Lean artifact keeps `Sched.deadlock_free_d5`); what no source supports is the testdoc's claim that the test moves when the walk's order changes
- Owner-gated: yes: disposition of a deliberately retained design-space record

The testdoc says the trace "is the encoder's own order" and "if this test starts failing because the panic disappears, the encoder's order changed corners". The input is a literal, not a captured trace, so a change to the walk cannot move this test; only editing the literal can. An inaccurate testdoc is a bug in the test. The checker (about 50 lines plus a 20-line doc) has no other caller and its stated purpose is to document a rejected alternative, which the formal model already records.

Evidence:

    98	/// This trace is the encoder's own order (the same trace
    99	/// `accepts_wire_resolution_work_parent_order` accepts): the sole disputed
    100	/// child's dependent work departs after the final resolution and before the
    101	/// parent summary, exactly what the weave's d5 placement forbids. Pinned as
    102	/// the design-space record (finding #7, adjudicated: the encoder keeps the
    103	/// epilogue placement and the `d6`/`assert_parent_last` check instead): if
    104	/// this test starts failing because the panic disappears, the encoder's
    105	/// order changed corners — re-audit the parent-placement trade before
    106	/// accepting.

Resolution: Either make the pin real (capture a session with `with_trace` as `streaming/tests.rs:83-95` does and run it through `assert_parent_early` under `#[should_panic]`), or dissolve `assert_parent_early` and this test, leaving the d5/d6 record to the model, and reword `assert_parent_last`'s doc (progress.rs:127-128) to stop pointing at the removed checker. Keeping a checker purely as a record is the owner's call; at minimum the testdoc must stop claiming a sensitivity it lacks. Acceptance: either a real-session trace reaches `assert_parent_early`, or the function and test are gone and no prose references them.

### materialized-23: Default-dialect tells: unanchored metaphors, moralized code, a jokey banner, a next-line narration, a compressed cross-reference
- Where: src/tree/mirror/streaming/materialized/tests.rs:1-8 (related: src/tree/mirror/streaming/materialized/tests.rs:91, 117, 152; work/answer.rs:24, 27, 107; work/queues.rs:80, 308; progress.rs:46; progress/tests.rs:124; work/tests.rs:227; src/tree/mirror/streaming/materialized.rs:290-292, 898; src/tree/mirror/streaming/materialized/unknown.rs:22-23)
- Class / severity / confidence: documentation / nit / medium
- Provenance: verified (grep of each quoted phrase across the partition; two of the lens's sites are withdrawn: "honest" at tests.rs:156 is the model's term of art ("authenticated-honest-peer", AGENTS.md; "An honest replica", materialized.rs:201), and "seam" is anchored at streaming.rs:19 ("the height-erased seam"))
- Seen by: prose, perfapi; refutation: confirmed; history: no rationale found (the banner and "Then we send" are verbatim from 7126e489; "chokepoint" is commit vocabulary from 0116a081, undefined in code)
- Owner-gated: no

Each is cheap to fix: "chokepoint" for the single site (tests.rs:7, 117; answer.rs:24, 107); moralizers ("keeps the rejections below honest", tests.rs:91; "genuine dispute", answer.rs:27; "genuinely stall", queues.rs:80; "real coverage", progress/tests.rs:124; "all-real group", work/tests.rs:227; "input goodwill", unknown.rs:22-23); "knob" (queues.rs:308); "positive session" for a successful one (progress.rs:46); a banner comment carrying no information (materialized.rs:290-292); a comment narrating its next line (materialized.rs:898); and "the connected greeting-lie family" (tests.rs:152), a compressed reference to `faults.rs`'s `GreetingLie` suite.

Evidence:

    7	//! so its containment check is a chokepoint of its own and gets its own

    91	/// The happy path that keeps the rejections below honest: the scripted

    290	// --------------------------------------------------------------------------------
    291	// PROTOCOL IMPLEMENTATION TIME
    292	// --------------------------------------------------------------------------------

    898	        // Then we send that (optional) leaf upwards.

    308	/// scopes the memory model already charges, so no knob applies. (Contrast

Resolution: "chokepoint" -> "the one site where"; drop "honest"/"genuine(ly)"/"real" (answer.rs:27 "a dispute exactly when"; queues.rs:80 "can stall a session"; tests.rs:91 "the happy path the rejections below are measured against"); queues.rs:308 "so no window capacity applies"; progress.rs:46 "A completed successful session's ordering trace."; delete the banner and the next-line narration; tests.rs:152 link `GreetingLie` through the test module path. Acceptance: none of the quoted phrases remain in the partition.

### materialized-24: "the materialized filter/prune/oracle" names the in-memory oracle with the word this module uses for itself
- Where: src/tree/mirror/streaming/materialized/unknown.rs:11-12 (related: src/tree/mirror/streaming/materialized/unknown/tests.rs:1-3, 67-70; src/tree/mirror/streaming.rs:13; src/tree/traverse/unknown.rs:9)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (grep `materialized` outside this module: every use denotes the streaming walk or a materialized node; traverse/unknown.rs:9 calls the oracle "the in-memory join")
- Seen by: prose; refutation: confirmed, severity down to low (two files, local context disambiguates on a careful read); history: deliberate but expired (bbe89b94 wrote "Unlike the materialized filter" when no `materialized` module existed; 61ddf3a5 created `streaming/materialized.rs`, inverting the word's referent in this file)
- Owner-gated: no

Inside `streaming::materialized::unknown`, "the materialized filter" means `traverse::unknown`, the in-memory traversal; everywhere else "materialized" is this walk. A term of art must mean one thing in one crate, and the crate already has the unambiguous name.

Evidence:

    11	//! Unlike the materialized filter, which walks one owned subtree, this version
    12	//! is generic over any [`Backend`].

    1	//! The streaming [`unknown`] prune must agree, node for node,
    2	//! with the materialized [`Unknown`](crate::tree::traverse::unknown::Unknown)
    3	//! oracle it mirrors.

Resolution: "Unlike the in-memory filter ([`traverse::unknown`](crate::tree::traverse::unknown)) ..." at unknown.rs:11; "the in-memory [`Unknown`] oracle" at tests.rs:2; "the in-memory prune" at tests.rs:67; rename `agrees_with_materialized_oracle` to `agrees_with_in_memory_oracle`. Acceptance: `grep -n materialized` in the two files returns no line using the word for the `traverse` oracle.

### materialized-25: Ghost reference to the removed height-typed recursion: "exactly as the typed tower did"
- Where: src/tree/mirror/streaming/materialized/unknown.rs:19-23 (related: src/tree/mirror/streaming/erased.rs:30; src/tree/mirror/streaming/window.rs:137; src/tree/typed/height.rs:166)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (`git log -S"typed tower" -- src/` returns only bf1a5b4b, "streaming: erase the materialized walk's workers", whose message reads "boxed per step exactly as the typed tower was"; grep finds no other occurrence in src/)
- Seen by: structure, prose, perfapi; refutation: confirmed; history: contradicts the hard rule (a ghost at birth: written by the commit that deleted the tower, after the 44724ad0 prose sweep, so no sweep has caught it)
- Owner-gated: no

The module doc justifies boxing each recursive future by comparison with an implementation that no longer exists, in the past tense. The same sentence hand-writes the depth bound ("at most 32") where a named constant exists (`KEY_DEPTH` in window.rs:137; `Root::HEIGHT` pinned at height.rs:166) and closes with a moralizer. The sibling "exactly as before" at erased.rs:30 is the same tell from the same commit, outside this partition.

Evidence:

    19	//! would instantiate one per level. Each recursive call boxes its future
    20	//! ([`BoxFuture`]) exactly as the typed tower did — the type stays flat —
    21	//! and the depth is bounded by the prefix's remaining height, at most 32,
    22	//! so the recursion is stack-safe by construction rather than by input
    23	//! goodwill.

Resolution: "Each recursive call boxes its future ([`BoxFuture`]) so the future type stays finite, and the depth is bounded by the prefix's remaining height (the key depth), so the recursion is stack-safe by construction." Fix erased.rs:30 in the same pass. Acceptance: `git grep 'typed tower\|as before' src/tree/mirror/streaming/` returns nothing; the paragraph reads as a description of what is.

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

### materialized-27: The leaf-height deletion verdict in `unknown` has no committed test that fails when it is inverted
- Where: src/tree/mirror/streaming/materialized/unknown.rs:88-97 (related: src/tree/mirror/streaming/materialized/unknown.rs:99-108, 156-167; src/tree/mirror/streaming/materialized/unknown/tests.rs:29-48, 66-83; work/answer.rs:136-144, 182; src/tree/arb.rs:509-513, 625-670; src/tree/mirror/streaming/tests/fixtures.rs:29-32, 494-502; .agent-notes/2026-08-21-unknown-pruning-survivor/README.md)
- Class / severity / confidence: verification-gap / high / medium
- Provenance: assessed (read; the survival itself is agent-reported in the handoff note and cannot be re-run without modifying the tree; the reachability argument and the fixture-role analysis are by reading: `git log -- .agent-notes/2026-08-21-unknown-pruning-survivor` shows one commit, 1a7afe1e; `grep unknown .cargo/mutants.toml` finds no exclusion; `leaf_sibling_path` is used only by the two `leaf_parent_*_pair` fixtures, arb.rs:575-654)
- Seen by: correctness; refutation: confirmed and reframed (the module's own differential oracle builds its tree from `Path::for_leaf` hashed paths, so it never reaches the arm either); history: already known (the note asks for exactly this construction; no follow-up has landed; the v1-retirement note's decision 3 deferred the scoped mutants re-check)
- Owner-gated: no

The height-0 arm is the only place the streaming prune judges an individual leaf, and a known-bad mechanism (the inverted verdict) passes the whole suite. Reading the reachability explains why: a single leaf reached from any height above is a path-compressed spine whose `span()` is a point, so `knowledge()` at 99-108 answers `Before` or `After` and the recursion never descends to height 0. The arm runs only when a real height-1 branch (two or more leaves sharing 31 path bytes) classifies `Between` and the holder's counterparty lacks the parent (the `answer::internal` Left arm, `unknown_providing`, or the initiator's early supplies). Every committed fixture avoids that shape: hashed-path generators cannot reach height 1 (`unknown/tests.rs:39` uses `Path::for_leaf`), the divergence fixtures keep extras concurrent ("nothing is deletion-pruned when provided across", fixtures.rs:29-32), and `leaf_parent_redaction_pair` has both sides hold the parent, so its verdict lands in `answer::leaf_parent`'s `known` filter (answer.rs:138), not here. Under the inversion the receiver's `Resolver` absorbs the re-supplied redacted leaf (its containment check passes because the sender holds it), so this is a behavioral hole in the redaction contract, not a quantitative one. Doctrine: every criterion needs a committed demonstration that a known-bad mechanism fails it, and "redaction leaves no tombstones" rests on exactly this verdict.

Evidence:

    88	        if prefix.height() == 0 {
    89	            // A leaf is known iff its ceiling is causally at or before
    90	            // `known`; a concurrent ceiling is beyond the known-at range,
    91	            // so those survive.
    92	            let verdict = Some(node).filter(|node| !self::known(node, known));
    93	            if verdict.is_none() {
    94	                stats.shed(1);
    95	            }
    96	            return Ok(verdict);
    97	        }

Resolution: Two committed tests. (1) In `unknown/tests.rs`, a `leaf_sibling_path` variant of `tree_and_known` (k leaves under one shared 31-byte prefix with per-leaf known flags) driven through the existing differential oracle, the cheapest kill. (2) A cross-peer fixture beside `leaf_parent_redaction_pair` in `src/tree/arb.rs`: side `a` holds leaves at `leaf_sibling_path(0x00)` (party 0) and `leaf_sibling_path(0x01)` (party 1); side `b` has forgotten 0x00 and never held 0x01, so it holds nothing under that parent, with ceiling `v00 | tick(party 2)`; drive it through `streaming_mirror_sides` in both orientations, assert both endpoints equal `join_oracle`, and assert the holder sheds exactly one and the other gains exactly one. Then generalize (2) into a proptest with `Tree::join` as oracle. Verify the kill by applying the inversion as a reversible string swap, confirming the new tests fail, restoring, and checking `git diff` is empty. Record the disposition in the handoff note (which may then be retired). Acceptance: a committed test fails with `!self::known` replaced by `self::known` and passes on HEAD.

Construction: `a = act(None, [(leaf_sibling_path(0x00), v00 on party 0, Insert), (leaf_sibling_path(0x01), v01 on party 1, Insert)])`; `b` = empty root with ceiling `v00 | forget_tick(party 2)`. Whichever side initiates, a's root child at radix 0x00 is exclusive: as initiator it flows through the early-supply `unknown` call (levels.rs:111); as responder through `answer::internal`'s Left arm (answer.rs:80). The compressed spine's span is `[meet(v00, v01), v00 | v01]`, `Between` against b's ceiling, so the recursion descends to the height-1 branch and judges each leaf: 0x00 known (shed), 0x01 unknown (travels). Expected: both sides hold {0x01}. Mutated (`!` removed): b receives 0x00, the redacted leaf, and not 0x01; equality with `join_oracle` fails.

### materialized-28: The whole-subtree `shed` arms are never pinned at a count above one
- Where: src/tree/mirror/streaming/materialized/unknown.rs:103-106 (related: src/tree/mirror/streaming/materialized/unknown.rs:149-152; src/tree/mirror/streaming/tests/stats.rs:185-186, 208-210, 258-259; tests/session_stats.rs:64-66, 109-111, 302-335; src/tree/mirror/streaming/stats.rs:94-97)
- Class / severity / confidence: verification-gap / low / medium
- Provenance: verified for the gap (grep `messages_shed` across tests/ and streaming/tests/: every assertion is 0, 1, or the conservation identity; the conservation proptest issues only `send_all`, so nothing is redacted); assessed for the kill (by reading)
- Seen by: correctness; refutation: confirmed; history: no rationale found (7626543e pinned "Redaction honored reads as one shed"; nothing above one was recorded or deferred)
- Owner-gated: no

`SessionStats::messages_shed` promises "a pruned subtree adds its exact live-leaf count, so the number always reads in messages", but replacing `stats.shed(node.len() as u64)` with `stats.shed(1)` at either site survives the suite. Any quantity computable two ways gets a committed test comparing them; `messages_gained`'s multi-leaf arm is covered by the conservation proptest, so the gap is on the shed side only.

Evidence:

    103	            Dominance::After => {
    104	                stats.shed(node.len() as u64);
    105	                return Ok(None);
    106	            }

Resolution: Add a redaction schedule to `sessions_conserve_the_live_count` (a `Forget` over the shared base, as `arb_divergent_pair` already does in-crate) so `after = before + gained - shed` exercises shed > 0; and add a walk-tier pin where one side holds k >= 2 leaves under one root radix the other has seen and forgotten, asserting `messages_shed == k`. Acceptance: a committed test fails when either `stats.shed(node.len() as u64)` is replaced by `stats.shed(1)`.

Construction: a holds leaves p0, p1 under root radix R (both party 0, v1 < v2); b has forgotten both (ceiling >= v2, nothing under R, ballast elsewhere). Gossip: a's root child R is exclusive, `unknown` at height 31 classifies `After` and sheds `node.len() == 2`; mutated it sheds 1 and `live(after) == before + gained - shed` fails on a's side.

### materialized-29: Four production files lack module docs; `work.rs` names two of its five children; `Resolver`'s methods are undocumented
- Where: src/tree/mirror/streaming/materialized/work.rs:3-5 (related: src/tree/mirror/streaming/materialized/work.rs:20-24; work/answer.rs:1; work/resolver.rs:1, 45, 111, 115, 119; src/tree/mirror/streaming/materialized/common.rs:1; src/tree/mirror/streaming/materialized/error.rs:1)
- Class / severity / confidence: documentation / low / medium
- Provenance: verified (read each file's opening: answer.rs, resolver.rs, and common.rs open with `use`; error.rs opens with an item doc; work.rs:3-5 links `levels` and `assembly` while declaring five children at 20-24; `Resolver::new`, `ready`, `pending`, and `finish` carry no doc comment)
- Seen by: prose; refutation: confirmed; history: no rationale found (unchanged since 7126e489; the work.rs map dates from the cbfe1aff split when `answer` and `resolver` already existed)
- Owner-gated: no

A module doc's first sentence is what a maintainer sees in the listing; `work.rs`, the natural entry point, does not map `answer`, `queues`, or `resolver`. `finish` is where `UnfinishedReply` is raised and says nothing.

Evidence:

    3	//! [`Work`] owns every independently runnable pump while the type-level walk
    4	//! advances. [`levels`] contains the phase-specific walks, while [`assembly`]
    5	//! reconstructs their resolved scopes upward.

    1	use itertools::{EitherOrBoth, Itertools};

    119	    pub fn finish(mut self) -> Result<Resolution<B::Erased>, Error<B::Error>> {

Resolution: Add one-sentence `//!` docs to answer.rs ("The per-question answers: merge-joins of both listings at internal, leaf-parent, and leaf heights, where disputes and shed leaves are counted"), resolver.rs, and error.rs (common.rs dissolves under materialized-16; if it stays, document it and the cfg split's reason: the test receiver is itself a `Stream`, channel.rs:3-6). Extend work.rs:3-7 to name all five children. Doc `Resolver::new`, `ready`, `pending`, and `finish` (with the `UnfinishedReply` condition). Acceptance: every non-test `.rs` in the partition opens with a `//!` block; work.rs links all five submodules; every `pub fn` in resolver.rs has a doc comment.

### materialized-30: Fan-bounded per-scope vectors grow from empty, and no meter prices the walk's allocations
- Where: src/tree/mirror/streaming/materialized/work/answer.rs:50-52 (related: work/answer.rs:121-123; work/resolver.rs:54; src/tree/mirror/streaming/materialized/common.rs:29; work/levels.rs:97, 107-108; tests/encode_alloc.rs:1-10; tests/decode_alloc.rs)
- Class / severity / confidence: performance / nit / medium
- Provenance: assessed (read; `tests/encode_alloc.rs` and `tests/decode_alloc.rs` meter the codec only per their module docs)
- Seen by: perfapi; refutation: confirmed, with one caveat (reserving `ours.len() + theirs.len()` over-reserves by up to 2x on all-`Both` scopes, so the reservation is a small memory trade rather than a pure deletion); history: no rationale found
- Owner-gated: no

Per answered scope, `internal` and `leaf_parent` build three vectors whose final lengths are bounds of the inputs, and `Resolver` starts `resolved` at zero capacity. The cost is small; what is missing is the instrument: no committed number holds the walk's per-scope allocation count, and instruments come before cures.

Evidence:

    50	    let mut reactions = Vec::new();
    51	    let mut asked = Vec::new();
    52	    let mut resolved = Vec::new();

Resolution: First land a `stats_alloc` meter (same shape as `tests/encode_alloc.rs`) over an in-process `streaming::mirror` session of a fixed disputed shape (e.g. `full_depth_comb_pair`), committing the current allocation-event count as the ceiling with a liveness floor. Then reserve (`with_capacity(ours.len())` in `Resolver::new`, `with_capacity(theirs.len())` for `asked`, a bound for `reactions`/`resolved`) and tighten the ceiling in the same commit. Acceptance: a committed meter whose ceiling the reservation commit lowers, before and after numbers in the commit message.

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

### materialized-32: The full-fan and one-slot capacity arguments are each stated in more than one place
- Where: src/tree/mirror/streaming/materialized/work/assembly.rs:29-31 (related: src/tree/mirror/streaming/materialized.rs:79-87; work/queues.rs:3-5, 41-45, 60-83; work.rs:130-133)
- Class / severity / confidence: documentation / low / medium
- Provenance: assessed (read all five sites)
- Seen by: prose; refutation: confirmed; history: no rationale found (accretion across 7126e489, c9b8f38b, 4d55d484, cbfe1aff)
- Owner-gated: no

The full-fan argument appears in the module doc, the constructor doc, and `Work::assemble`'s doc; the one-buffered-response argument appears at `pump` and at `outgoing_responses`. `queues.rs:3-5` declares the constructors the home of capacity reasoning. Three copies of one argument drift independently. The module-doc copy is arguably deliberate (the deadlock argument's home, and the fan queue is its one exception), so it stays as a summary with a link.

Evidence:

    29	    /// A full fan lets every lower scope enqueue before the parent resolution
    30	    /// containing its [`Resolve::Pending`] slots is published, without relying
    31	    /// on blocked sender futures remaining independently runnable.

    3	//! Each function names one edge in the protocol dataflow. Keeping capacity
    4	//! choices here makes them reviewable alongside the exact item type and keeps
    5	//! queue arithmetic out of the walk itself.

Resolution: Keep the full arguments at `assembly_level_returns` and `outgoing_responses`; reduce assembly.rs:29-31 and work.rs:130-133 to one sentence each ending in a link to the constructor; end the module-doc paragraph at materialized.rs:79-87 with the same link. Acceptance: each mechanism ("full fan", "one buffered response") is spelled out at exactly one constructor; the other sites link there.

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

Resolution: Introduce `enum Opening<E> { Survivors(oneshot::Receiver<Vec<(u8, Option<E>)>>), Supplies(oneshot::Receiver<Vec<(u8, Vec<(u8, E)>)>>), None }` with the two field docs as variant docs; `Descending.opening: Opening<B::Erased>`; `initiator()`/`responder()` construct their arm; `reply()` passes `Opening::None`; the `debug_assert!` becomes `matches!(self.opening, Opening::None)`. `internal_level`/`internal_walk` take one `Opening` parameter and, at the top of the `try_stream!` body, match it once into `survivors: BTreeMap<..>` and `supplied: BTreeMap<..>` with a one-line comment ("filled before the opening yields; our first query follows that yield, so this await never crosses wire progress; a dropped sender means the opening never sent a query"). Replace the clause at 393 and block at 396-400 with `&& let Some(children) = supplied.remove(&radix)`; replace 443-449 with `if let Some(survivor) = survivors.remove(&radix)`. Acceptance: `internal_walk` has no `Option<BTreeMap>` and no `.take()` on the hand-off; no `Option<oneshot::Receiver<..>>` field on `Descending`; violations.rs passes `Opening::None` where it passed `None, None`; `Trace::assert_valid` on the real-session traces (streaming/tests.rs:95, 114), the capacity suite, and the early-supply tests under `remote/adapter/tests/opening.rs` stay green.

### materialized-35: `Resolver` owns its `Recorder` and is constructed per resolved scope, so the walks clone the `Arc` once per query
- Where: src/tree/mirror/streaming/materialized/work/resolver.rs:35-38 (related: work/resolver.rs:27-34, 49; work/levels.rs:430, 571, 668; src/tree/mirror/streaming/stats.rs:162-165)
- Class / severity / confidence: performance / nit / high
- Provenance: verified (read the struct: `their_version` and `ledger` are `&'v`, `stats` is owned; the three `Resolver::new` calls pass `stats.clone()` inside the per-query loop; `Recorder` is `Arc<Counters>`, stats.rs:162-165)
- Seen by: perfapi; refutation: confirmed; history: no rationale found (487e17ea added the owned recorder when the struct's only borrowed input was `their_version`; 50c8b0a3 then added `ledger` as a borrow)
- Owner-gated: no

Two atomic RMWs (clone and drop) per disputed scope that a `&'v Recorder` field removes; the struct already spells its per-session inputs with one lifetime and the owned recorder is the odd one out. Fixed sign.

Evidence:

    35	    /// The session's stats recorder: each absorbed supply credits its
    36	    /// exact live-leaf count as
    37	    /// [`messages_gained`](crate::SessionStats::messages_gained).
    38	    stats: Recorder,

Resolution: `stats: &'v Recorder`; constructor parameter `&'v Recorder`; pass `&stats` at the three sites. Acceptance: `grep -n "stats.clone()" levels.rs` shows only the per-stage clones at the top of each walk body, none inside a `while let Some(query)` loop.

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

### materialized-37: The `Resolver`'s skip-past supply arm has no injection
- Where: src/tree/mirror/streaming/materialized/work/resolver.rs:81-83 (related: work/resolver.rs:74-79; work/tests/violations.rs:160-183; src/tree/mirror/streaming/testing/faulting.rs:222-226)
- Class / severity / confidence: verification-gap / low / high
- Provenance: assessed (read; the `InvalidSupply` script at violations.rs:169-183 sends two supplies at one radix against an empty fan, which trips `radix <= *last` at 74; the `Faulting` harness pushes `Supply(0)` twice, faulting.rs:224-225, also line 74; nothing reaches 81-83)
- Seen by: correctness; refutation: confirmed, with one correction (`>` to `>=` at 81 is an equivalent mutant because the `==` case is consumed at 78, so arm deletion is the catchable mutation); history: no rationale found
- Owner-gated: no

A supply whose radix skips past a held-but-unmatched child is a third structural arm no test reaches; deleting it survives the suite (the supply is absorbed and `finish()` reports `UnfinishedReply` instead). Adequacy: keep known-bad artifacts committed and failing.

Evidence:

    81	                    Some((next, _)) if radix > *next => {
    82	                        return violation(Violation::InvalidSupply);
    83	                    }

Resolution: Add an `Injection::InvalidSupplySkipsHeld` script: `ours = {r}` (any held radix with `r < 255`), reply `[Supply(r + 1, supplied)]`, expected `InvalidSupply`; run it through the same 32-height dispatch. Acceptance: a committed injection fails when the arm at 81-83 is removed.

Construction: `ours = [(5, node)]`, reply `[Supply(7, node)]`: `resolved.last()` is `None`, `fan.peek()` is `Some(5)`, `7 != 5`, `7 > 5` -> `InvalidSupply` on HEAD; with the arm deleted the supply is absorbed and `finish()` reports `UnfinishedReply`.

### materialized-38: Em-dashes in `//` comments
- Where: src/tree/mirror/streaming/materialized/work/resolver.rs:86 (related: src/tree/mirror/streaming/materialized/unknown.rs:112; work/tests/violations.rs:185-186)
- Class / severity / confidence: documentation / nit / high
- Provenance: verified (grep for the em-dash character across the partition filtered to non-doc comments: four sites; the same grep across src/ finds 116 sites in `//` comments)
- Seen by: prose; refutation: reframed (the crate's prevailing practice in `//` comments is the em-dash, 116 sites, so the four partition sites are a sample, not a local inconsistency); history: contradicts the owner's global doctrine (CLAUDE.md: colons or spaced double-hyphens in comments; no gate leg checks it)
- Owner-gated: no

The owner's convention places em-dashes in rendered prose only. Fixing four sites would leave the convention inconsistent crate-wide; this is a sweep decision.

Evidence:

    86	                    // is one read per supplied subtree — at worst the

    112	        // group — `None` entries are the children that pruned away. A group

Resolution: Sweep `//` comments crate-wide (116 sites) to ` -- ` or a colon, or record that the crate tolerates em-dashes in comments. Acceptance: `grep -rn '—' src | grep -v '///\|//!' | grep '//'` returns nothing, or the tolerance is recorded.

### materialized-39: The exhaustive violation suite enters at internal walk entries without stating the decision, while the public-wiring injector covers two variants and cannot reach the terminal phase
- Where: src/tree/mirror/streaming/materialized/work/tests/violations.rs:246-297 (related: work/tests/violations.rs:1; src/tree/mirror/streaming/tests/faults.rs:41-46, 64-101; src/tree/mirror/streaming/testing/faulting.rs:193-241)
- Class / severity / confidence: test-quality / low / medium
- Provenance: verified for the entries and strategies (read: the three `InjectHeight` impls call `Work::leaf_level`, `leaf_parent_level`, `internal_level`; the module doc at line 1 records no decision; `arb_connected_violation` samples `UnexpectedQuery` and `UncontainedSupply` while `Faulting` scripts all eight reply-shaped violations; `server_steps`/`client_steps` draw from `0..=15`); assessed for the phase-count arithmetic (a depth-32 comb session has 17 reply phases per side, so step 15 never reaches the terminal reply `absorb` consumes)
- Seen by: correctness; refutation: confirmed, with the structural corroboration; history: contradicts doctrine, not an AGENTS.md hard rule (83db6b26 built the suite at `Work::*_level` because a per-height, per-variant matrix needs a scripted counterparty, a defensible reason that is not stated)
- Owner-gated: no

Doctrine: exhaustive suites exercise the public surface; internal-entry checks only as deliberate, documented decisions at the check site, because a suite locked to an internal entry lets coverage drift when the public wiring (`Descending::reply`, `complete_responder`, `complete_initiator`) changes. The reason here is good and unstated. Separately, the connected suite that does cross the public wiring carries two variants and a step range that structurally misses the opening and terminal legs, which is the mechanical reason materialized-14 is unpinned.

Evidence:

    1	//! Semantic-violation injection across every materialized walk height.

    286	        let mut work = Work::new(Local, Window::FLOOR, Recorder::default());
    287	        let (responses, _asked, _upper, _lower) = work.internal_level::<H>(

    41	fn arb_connected_violation() -> impl Strategy<Value = Violation> {
    42	    prop_oneof![
    43	        Just(Violation::UnexpectedQuery),
    44	        Just(Violation::UncontainedSupply),
    45	    ]
    46	}

Resolution: State the decision in the violations.rs module doc (the per-height, per-variant matrix needs a scripted counterparty the connected driver cannot express cheaply); widen `arb_connected_violation` to every variant `Faulting` can script, and widen the step range (or add a terminal-phase case) so the public wiring carries the taxonomy at least once per variant and per leg. Acceptance: the module doc names why it enters at `Work::*_level`; `connected_violation_aborts_without_mutating_root` covers every `Violation` variant the harness can construct, including at the opening and terminal phases.

### materialized-40: `violations.rs` hard-codes the height count and mirrors `Violation` with an identity enum
- Where: src/tree/mirror/streaming/materialized/work/tests/violations.rs:345 (related: work/tests/violations.rs:35-58, 320-321; src/tree/typed/height.rs:166; src/tree/mirror/streaming/window.rs:137)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the loop bound and the comment at 320-321; `Root::HEIGHT == 32` is asserted at height.rs:166; `Injection` at 35-58 maps 1:1 onto `Violation` variants through `expected()`)
- Seen by: structure; refutation: confirmed; history: no rationale found for the literal; the enum's subset role has one (50c8b0a3: `OverdrawnSupply` is scripted at its own terminal test against a spent ledger)
- Owner-gated: no

Named constants over magic numbers (the number exists under a name one module away; the 32 underscores in `inject_at_height!` are structural to the token-count recursion and can stay); a 1:1 identity enum must be edited in step with `Violation` whenever a scriptable fault is added.

Evidence:

    345	        for height in 0..32 {

    46	impl Injection {
    47	    fn expected(self) -> Violation {

Resolution: `for height in 0..Root::HEIGHT`; either generate the strategy over `Violation` values directly with an explicit `OverdrawnSupply => unreachable!("scripted against a spent ledger in materialized/tests.rs")` arm in `violation_script`, or keep `Injection` and derive the mapping as `From<Injection> for Violation` with a doc stating why the subset exists. Acceptance: no bare 32 as a loop bound; one fewer hand-maintained mapping, or the mapping justified at its site.

## Positives

- The deadlock-freedom argument (materialized.rs:26-87) is a model maintainer document: it states the two ordering invariants, derives why one slot suffices, names the independence premise and exactly where it is supplied (the link contract) and refuted (the mux fixture), and isolates the one exception (the assembly fan queue) with its reason. `yield_resolve_query!` then makes the order a structural property of every call site by keeping wire, resolution, and dependent work in one expansion with the `#[cfg(test)]` trace hooks inside it, so the test-only instrument cannot drift from the production order; the remote proxy's `yield_reply_scopes!` mirrors the shape.
- `queues.rs` gives every channel constructor its own cardinality or flow argument, distinguishes the correctness floor (`assembly_level_returns` at `FAN`) from the amortization (`terminal_leaf_resolutions`), and ties the floor to a demonstrating test that exists and is exercised from both sides (`underbuffered_mirror_stalls`, capacity.rs:26; a stall at 253 and completion at 254).
- The erasure boundary is disciplined and cheap: walk bodies take `Replies<E>` and instantiate once per backend, the typed re-tags are `Work::respond`'s exit and the two fixed-height root re-tags (levels.rs:143-146, 257-260), each commented, and the prefix length is the single runtime height witness so coordinate and height cannot drift.
- `Resolver` isolates the counterparty-fault taxonomy in one 130-line type, and `injected_fault_reports_exact_violation` drives every scriptable fault through all 32 walk heights under arbitrary channel schedules asserting the exact public `Violation`.
- `SupplyLedger::charge` handles the wrapped-counter case explicitly, uses one `Relaxed` `fetch_add` with a post-check that is race-correct for two ingestion sites, and returns a vocabulary-neutral error so the walk and the wire decoder each render the overdraw in their own terms; the doc says exactly why two instruments exist.
- Every peer-controlled datum reaches the walk through a merge-join or a peekable fan: no indexing, no arithmetic on wire values except the ledger's checked add. Every panic I traced is guarded by structure the counterparty cannot influence.
- `unknown.rs` states the recursion's depth bound in the module doc (the prefix's remaining height), satisfying the input-controlled-depth rule with an argument rather than a guard, and the span classification prunes whole subtrees without descending; the streaming prune is differential-tested against `traverse::unknown`.
- `transcript.rs` captures the wire transcript at the single funnel every response stream passes through (`pump`) and states the causal-consistency property of the capture point.
- Every test in the partition has a doc comment stating its invariant, and every one I traced against its body (all of progress/tests.rs, materialized/tests.rs, work/tests.rs, violations.rs, unknown/tests.rs) is accurate, with one exception recorded as materialized-22. Several state the deadlock the check prevents, not merely the panic expected (progress/tests.rs:44-47, 60-66, 78-83).
- Branch comments carry the why rather than the next line: levels.rs:94-96, 118-119, 219-223, 232-233, 339-340, 386-390; assembly.rs:81-83; materialized.rs:838-840; answer.rs:149-151.

## Open questions for Finch

- `assert_parent_early` (materialized-22): keep it as a design-space record fed a real captured trace, or dissolve it and let `formal/lean` (`Sched.deadlock_free_d5`) carry the record? My recommendation: dissolve; the model already records the rejected corner, and test code performing a design-doc function is the circular-justification tell.
- `MaterializedError` derives and exhaustiveness (materialized-17): derive `Clone`/`PartialEq`/`Eq` on both inner error enums and decide `#[non_exhaustive]` for both together, or drop the dead `Clone` from `mirror::Error`? My recommendation: derive on both and keep `MaterializedError` exhaustive with the two-variant partition stated in its doc.
- `DEFAULT_TARGET_MESSAGE_SIZE` (materialized-10): relocate to `message.rs` keeping `usize`, or also retype to `u64` to match the greeting field (a public API change)? My recommendation: relocate and retype pre-release, since `DEFAULT_PAYLOAD_DEPTH_LIMIT` already sets the precedent of a default typed as its field.
- Window-stall observability (materialized-2): is a `SessionStats::window_stalls` counter wanted, given that stalls are off-model under uniform hashing and the counter would be a diagnostic for misconfigured budgets and off-model key distributions? My recommendation: yes, as zero-versus-nonzero; it is the one readout that tells a user which way to move `sync_memory_budget`.
- `unreachable_pub` (materialized-6): the partition's `pub` items inside the private `tree` module follow the crate-wide idiom; enabling the lint is a crate-wide decision. My recommendation: leave the idiom and only make the three item types' fields consistent.
- Em-dashes in `//` comments (materialized-38): sweep the 116 crate-wide sites or record a tolerance? My recommendation: sweep once, mechanically, in a dedicated commit.
- A walk-side allocation meter (materialized-30): wanted at all? Without it the vector-capacity reservations should stay recorded candidates rather than land.
- The `#[cfg(test)] progress::` instrumentation appears at about fifteen production call sites plus `trace_id` plumbing. A `#[cfg(not(test))]` no-op stub module would remove the attributes from the call sites at the cost of a zero-sized `trace_id` in release builds. The current form guarantees zero release cost; the trade is taste, and worth a ruling before anyone touches it.

## Dropped

- The prune recursion boxes one future per visited node (perfapi candidate 54): deliberate and documented; bf1a5b4b chose one prefix-guided recursion for one instantiation per backend, measured at -40.5% cumulative llvm-lines, and unknown.rs:17-23 states the rationale; the lens itself proposed no change until a meter exists.
- Greeting construction duplicated in `connect` and `accept` (prose 37, perfapi 57): duplicates of materialized-11, which composes `accept` from the two existing transitions rather than hoisting a helper.
- A third body of the listing derivation the doc calls single (perfapi 56): duplicate of materialized-31; its claim that `fan_listing`'s doc is weakened is refuted (the doc is scoped to the two positionally-paired listings and is accurate).
- Hand-rolled left-only merge (perfapi 55): duplicate of materialized-33.
- `known` and `contained` (correctness 44): duplicate of materialized-26.
- Opaque roster IDs (structure 13, correctness 45, perfapi 61): duplicates of materialized-20; the correctness lens's claim that `B5` resolves to nothing is refuted (it is a named Lean axiom, citable by name).
- Typed tower ghost (prose 25, perfapi 50): duplicates of materialized-25; the hand-maintained-count charge in 25 is folded in.
- The terminal leg denominates one absorbed leaf two ways (correctness 46): duplicate of materialized-15.
- Inconsistent field visibility (structure 19): duplicate of materialized-6.
- Terminal absorb reports extra reactions as `UnfinishedReply` (correctness 40) and opening early supplies skip structural checks (correctness 42): merged into materialized-14 as one pattern (the two legs outside the `Resolver`).
- Terminal absorb clones the supplied leaf (perfapi 53): folded into materialized-14's resolution (matching over the owned `Vec` removes the clone).
- `unwrap_or_default()` undocumented (prose 31): folded into materialized-34; the refutation's reframe (the comment is a benign-default argument, not a strandable path) is adopted there.
- `Descending` encodes a three-state hand-off as two `Option`s (structure 2): merged into materialized-34.
- `#[cfg(test)]` inside a `#[cfg(test)]` module (structure 15): merged into materialized-8 with the dead per-item `type_complexity` allows.
- `Resolver::react`'s return contract undocumented (prose 27): merged into materialized-36 (the named struct carries the doc); the resolver's other undocumented methods and missing module doc moved to materialized-29.
- One invariant two names / `d6` glossed two ways (prose 33), "Seven checks:" (prose 34), `assert_parent_last` cites by quotation (prose 35): merged into materialized-19.
- `initiator()`/`responder()` prelude (structure 4) and `our_version` duplicating `root.ceiling` (structure 5): merged into materialized-11.
- "seam" as a default-dialect tell (prose 36, perfapi 61): withdrawn; the word is anchored in the crate at streaming.rs:19 ("the height-erased seam"). "honest" at tests.rs:156: withdrawn; it is the model's term of art ("authenticated-honest-peer"). Both removed from materialized-23's site list.
- Refutation's new item "connected fault suite cannot reach the terminal phase": folded into materialized-39 as corroboration.
- Refutation's new item "the module's own oracle uses hashed paths": folded into materialized-27.
- Refutation's new item "erased.rs duplicates the receiver-as-stream adapter": folded into materialized-16.
- Five-tuple returns of `initiator_level`/`responder_level` (structure open question): not raised as a finding; the comment at levels.rs:339-340 argues the arity is the dataflow, and a `Stage` struct is taste without a named cost.
