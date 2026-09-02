# Sweep inventory: Allow attributes, panic sites, and visibility inventory

## Method and coverage

The sweep ran three mechanical inventories over the 113 production source
files under `src/` (every `tests.rs` sibling and `tests/` directory
excluded; `cfg(test)`, `test-internals`, and `conformance` regions noted per
site), with scripts and raw output under
`scratchpad/sweeps/inventory/` (`inventory.sh` for allow/expect attributes,
panic macros, asserts, `as` casts, division, and indexing; `visibility.py`
for module reachability, pub-in-private items, and `pub(crate)` referencer
counts). Its counts: 39 allow sites and no `#[expect]`; 315 panic-macro,
`unwrap`, and `expect` lines and 136 assert lines, of which 88 production
panic sites remain after removing doc examples, non-panic `Reader::expect`
calls, and test regions; 92 `as` casts; 10 division sites; 98 indexing
sites; 46 crate-root re-exports over 9 publicly reachable modules.

This pass disputed every finding against the tree at
9e5784fb4dce977cfbdfd1619886d1482b5ce764. For each one I opened the cited
lines with line numbers, grepped for use sites across `src/`, `tests/`,
`benches/`, `examples/`, and `crates/`, ran `git blame` at the five sites
where history could carry a rationale (snapshot.rs:7, streaming.rs:43,
levels.rs:67, protocol.rs:10, bookmark.rs:302), and grepped `.agent-notes/`
and `design/` for rulings on every topic (pub-in-private, `unreachable_pub`,
`tree::Iter`, `warm_caches`, `seed_rng`, `static_assertions`,
`type_complexity`, `missing_const_for_thread_local`, `KEY_DEPTH`, poison
handling, `ensure_loaded`, `LEAF_TAG`, the greeting roster). The two review
packets (`2026-08-20-cbor-wire-review`, `2026-07-23-review-link-transport`)
verify the greeting key order and the bulk-assemble equivalence tests but
rule on neither the parse shape nor the `from_sorted_leaves` asserts; the
`2026-07-18-node-hash-preimage` note states the leaf preimage as
`LEAF_TAG ‖ prefix_len ‖ prefix`, corroborating inventory-9. I counted
column-0 `pub` items under `src/tree/` and `src/message.rs` independently
(218 in non-`tests.rs` files before subtracting re-exported names and
`cfg(test)`-only files) to check inventory-10's order of magnitude, and
counted parameters at all seven `too_many_arguments` sites for inventory-12.

I used neither of the two permitted test invocations: no finding is a
correctness claim a test run would settle. What this pass could not see:
clippy and rustdoc were not run, so lint-scoping and threshold claims
(inventory-3, inventory-12) rest on the documented semantics plus one
in-tree corroboration (a seven-parameter function with no allow passes the
`-D warnings` gate), and the illumos clippy misfire behind inventory-13's
rationale is unverified either way.

Twenty findings survive; two are reframed (inventory-4, inventory-6) and
one component is dropped (see Dropped).

## Findings

### inventory-1: `tree::Iter` reaches the public API through `IntoIterator` but has no nameable path
- Where: src/snapshot.rs:172-179 (related: src/snapshot.rs:4-7, src/snapshot.rs:105-112, src/lib.rs:317, src/lib.rs:347, src/tree.rs:176)
- Class / severity / confidence: api-surprise / medium / high
- Provenance: verified (grep for `Iter` across lib.rs, snapshot.rs, tree.rs; lib.rs:317 declares `mod snapshot;` private and lib.rs:347 re-exports only `Snapshot`; blame at snapshot.rs:7)
- Verification: confirmed; history: no-rationale-found (snapshot.rs:7 dates from ddc57045, 2026-06-10, and no note or ruling mentions it)
- Owner-gated: yes: the fix either adds a public name or changes `Snapshot::iter`'s return type

`impl IntoIterator for &Snapshot<T>` names `Iter<'a, T>` as its `IntoIter`, but the only re-export of `Iter` is `pub use crate::tree::Iter` inside the private `snapshot` module, and `Snapshot::iter` returns an opaque `impl DoubleEndedIterator + ExactSizeIterator` rather than the same type. A user can iterate `&snapshot` but can name the iterator only as `<&Snapshot<T> as IntoIterator>::IntoIter`, and the doc at snapshot.rs:4 describes a re-export no reachable path provides.

Evidence:

    4	/// The iterator of [`Snapshot::iter`], re-exported from the tree internals:
    5	/// every live message as `(&Version, Arc<T>)`, unspecified order,
    6	/// exact-size and double-ended.
    7	pub use crate::tree::Iter;

    105	    pub fn iter(
    106	        &self,
    107	    ) -> impl DoubleEndedIterator<Item = (&Version, Arc<T>)> + ExactSizeIterator + Send + Sync

    172	impl<'a, T: Send + Sync + 'static> IntoIterator for &'a Snapshot<T> {
    173	    type Item = (&'a Version, Arc<T>);
    174	    type IntoIter = Iter<'a, T>;

    317	mod snapshot;
    ...
    347	pub use snapshot::Snapshot;

Resolution: Decide which face is the contract. Either re-export `Iter` from the crate root and have `Snapshot::iter` return `Iter<'_, T>` so both faces agree and the doc at snapshot.rs:4-7 names the reachable path, or keep the opaque `iter` and make the `IntoIterator` impl's `IntoIter` a type users can reach. rustc's allow-by-default `unnameable_types` lint mechanizes this class; enabling it in lib.rs catches recurrences. Acceptance: `rumors::Iter` (or another reachable path) compiles from an external crate and is the type `Snapshot::iter` returns, or `unnameable_types` is enabled and clean.

### inventory-2: `static_assertions` is a runtime dependency used only under `cfg(test)`
- Where: Cargo.toml:124-128 (related: src/lib.rs:295-296, src/tree/typed/height/tests.rs:8-27)
- Class / severity / confidence: vestigial / low / high
- Provenance: verified (grep for `static_assertions`, `assert_eq_size`, `assert_eq_align`, `assert_impl` across src, benches, examples, tests: hits only in src/tree/typed/height/tests.rs)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`static_assertions` sits in `[dependencies]`, but its only consumer is the height module's test file. Every downstream build compiles a crate the library never uses. The lib.rs:295 comment motivating `#![cfg_attr(not(test), forbid(unsafe_code))]` by the macro's `#[allow(unsafe_code)]` is accurate and stays.

Evidence:

    124	[dependencies]
    125	before = { workspace = true, features = ["serde"] }
    126	bytes = { workspace = true, features = ["serde"] }
    127	sha3 = { workspace = true }
    128	static_assertions = { workspace = true }

    295	// Static assertions uses #[allow(unsafe_code)], so we allow it only in tests
    296	#![cfg_attr(not(test), forbid(unsafe_code))]

Resolution: Move `static_assertions = { workspace = true }` to `[dev-dependencies]`. Acceptance: `cargo tree -e normal -p rumors` lists no `static_assertions`; `just gate` stays clean.

### inventory-3: Twelve per-item `type_complexity` allows under `streaming/` are shadowed by the module-wide allow
- Where: src/tree/mirror/streaming.rs:42-43 (related: src/tree/mirror/streaming/protocol.rs:10; materialized/work/levels.rs:67, 174, 296, 341, 539, 644; materialized/work/answer.rs:31, 113; materialized/work/resolver.rs:61; materialized.rs:388; remote/adapter/encode.rs:57; and in test files materialized/tests.rs:48, materialized/work/tests/violations.rs:100, remote/adapter/tests/malformed.rs:620)
- Class / severity / confidence: vestigial / low / high
- Provenance: assessed (read; lint attributes on a module apply to every nested item, out-of-line files included, so the claim rests on documented semantics; clippy was not run)
- Verification: confirmed; history: no-rationale-found (the module-wide allow is da4234ba, 2026-07-10; protocol.rs's inner allow predates it, bdfdf252, 2026-07-01; the levels.rs per-item allows postdate it, bf1a5b4b, 2026-08-19)
- Owner-gated: no

`#![allow(clippy::type_complexity)]` at the top of `mod streaming` already covers every nested module. The twelve per-item allows and protocol.rs's second inner allow suppress nothing, and a reader cannot tell which layer is load-bearing.

Evidence:

    42	// Where we're going, we need to write some Complex Types.
    43	#![allow(clippy::type_complexity)]

    10	#![allow(clippy::type_complexity)]

Resolution: Keep one layer. Either delete the twelve production per-item allows (and the `clippy::type_complexity` half of levels.rs:341) plus the three test-file allows under `streaming/`, or drop the module-wide allow and keep per-item allows where each complex type lives. Acceptance: `just clippy` and `just clippy-default` stay clean with a single layer of `type_complexity` allow under `streaming/`.

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

### inventory-5: `parse_greeting` and `greeting_map` route a fixed-order roster through six `Option`s and eight panic sites
- Where: src/tree/mirror/streaming/remote/codec/greeting.rs:128-211 (related: greeting.rs:37-44, greeting.rs:60-88, greeting/tests.rs:56)
- Class / severity / confidence: simplification / low / high
- Provenance: assessed (read; the cbor-wire review packet, REVIEW.md:985-987, verified the roster order and the exact-roster parse but did not consider the loop shape)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The parser walks `KEYS` in wire order and returns `GreetingError::Shape` on any deviation (lines 136-149), so every field is present by the time the loop ends. The code nevertheless accumulates six `Option`s, dispatches on the key string with an `unreachable!` arm, and finishes with six `expect`s; the encoder mirrors the string dispatch with its own `unreachable!`. A straight-line parse in `KEYS` order reads as the grammar it implements and needs none of the eight panic sites. The wire bytes are unchanged, so the snapshot pins hold, and `greeting_key_roster_is_exact` (greeting/tests.rs:56) keeps guarding the roster.

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

### inventory-8: `ensure_loaded`'s doc promises a return value the signature lacks
- Where: src/bookmark.rs:302-308 (related: src/bookmark.rs:372, src/bookmark.rs:419, src/bookmark.rs:437)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (read lines 301-313 and the three `expect("loaded before mutation")` sites; blame at 302 is b7fb409f, 2026-06-15)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The rustdoc says the function reads the record "returning it for mutation"; the signature returns `Result<(), BookmarkIo<B::Error>>`, and `slice`, `record`, and `reclaim` each re-fetch the record with an `expect`. Returning `&mut` from `ensure_loaded` would fight the `&mut self` methods that follow it, so the sentence, not the signature, is the thing to fix.

Evidence:

    302	    /// Read the stored record on first use, returning it for mutation. A no-op
    303	    /// once loaded; the mutex serializes access, and no mutation precedes a
    304	    /// load, so the read is the record's first content.
    ...
    308	    pub(crate) async fn ensure_loaded(&mut self) -> Result<(), BookmarkIo<B::Error>> {

    372	        let inner = self.inner.as_mut().expect("loaded before mutation");

Resolution: Drop "returning it for mutation" and state what the function does: loads the record into `inner` on first use so the mutators that follow find it present. Acceptance: the doc sentence and the signature agree.

### inventory-9: `LEAF_TAG`'s doc lists the version's encoding as a preimage component
- Where: src/tree/typed/hash.rs:58-65 (related: src/tree/typed/hash.rs:88-99, src/tree/typed/hash.rs:110-118)
- Class / severity / confidence: documentation / low / high
- Provenance: verified (compared lines 60-64 against the body at 110-118 and the formula at 88-89; the `2026-07-18-node-hash-preimage` note states the same formula)
- Verification: confirmed and sharpened: a leaf's compressed suffix is only the path bytes below its branch point, so a leaf preimage alone binds neither the full path nor the version; the root digest does, through the branch preimages; history: deliberate-and-holds for the formula (the note), no-rationale-found for the sentence
- Owner-gated: no

The `LEAF_TAG` comment says a leaf's preimage commits "its compressed suffix — path bytes — and its version's canonical encoding". `Hash::leaf` hashes `LEAF_TAG ‖ suffix_len ‖ suffix` and nothing else; its own doc at 88-89 says exactly that, and 96-99 explain that the version is committed transitively because the path is the version's hash. A reader auditing the digest algebra against the `LEAF_TAG` sentence would look for version bytes that are not there.

Evidence:

    60	/// Leaves are version-addressed (the path is the full-width hash of the
    61	/// leaf's version; see [`Path::for_leaf`](super::Path::for_leaf)), so a
    62	/// leaf's preimage commits its compressed suffix — path bytes — and its
    63	/// version's canonical encoding, never its message bytes: every compared
    64	/// digest in the tree is a pure function of the version set.
    65	const LEAF_TAG: u8 = 0;

    88	    /// The hash of a leaf observed from the top of its compressed `suffix`:
    89	    /// `sha3_256(LEAF_TAG ‖ suffix_len ‖ suffix)`.

Resolution: Re-state: the leaf preimage carries the compressed suffix alone; the version enters only through the path being its full-width hash, and message bytes enter nowhere. Or cut the sentence and point at `Hash::leaf`, whose doc is exact. Acceptance: the `LEAF_TAG` comment matches the bytes `Hash::leaf` hashes.

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

### inventory-11: Mutex poison is handled two ways in production code
- Where: src/tree/mirror/streaming/remote/streams.rs:521-524 (related: streams.rs:552-557, src/link/routed/router.rs:56-69, src/observe.rs:332-343; under `test-internals`: src/testing/memnet.rs:84-91 rides through, src/testing/transport.rs:157, 285, 317, 361, 386, 606, 614, 785 expect; under `conformance`: src/conformance/backend.rs:480)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `.lock()` across src excluding test files; lib.rs:319 gates `testing` on `test-internals`)
- Verification: confirmed with the site list completed: the sweep counted memnet.rs (test-internals) but not transport.rs (same module, opposite position); history: deliberate-and-holds for the ride-through sites (each carries its rationale), no-rationale-found for the expects
- Owner-gated: no

router.rs and observe.rs lock with `unwrap_or_else(PoisonError::into_inner)` and each states why riding through is safe. The supply-failure slot in streams.rs has the same shape (its critical sections are `get_or_insert` and `take`) but calls `.expect("supply failure lock")` twice, with a message naming the lock rather than proving the poison cannot occur. The `test-internals` transport module takes the expect position a further eight times.

Evidence:

    521	    fn supply_failed(&self, source: std::io::Error) {
    522	        let mut slot = self.supply_failure.lock().expect("supply failure lock");
    523	        slot.get_or_insert(source);
    524	    }

    56	/// Lock the table, riding through a poisoning panic.
    57	///
    58	/// Every critical section is a single map operation, so a panic
    59	/// elsewhere cannot leave the map torn; continuing lets the surviving
    ...
    66	    table
    67	        .lock()
    68	        .unwrap_or_else(|poisoned| poisoned.into_inner())

Resolution: Ride through poison in streams.rs as router.rs does, with its one-line rationale, or state at the site why a poisoned deposit slot must abort the session; apply the same rule to transport.rs. Acceptance: every production and test-internals lock site either rides through poison with a rationale or documents why it aborts.

### inventory-12: A `too_many_arguments` allow on a seven-parameter function
- Where: src/tree/mirror/streaming/remote/streams.rs:423-432 (related: src/tree/mirror/streaming/remote/proxy/work/encode.rs:41-49)
- Class / severity / confidence: vestigial / nit / high
- Provenance: verified (counted parameters at all seven allow sites: streams.rs:424-431 has 7; levels.rs:342 has 8 including `self`; state.rs:120 has 9; start.rs:340 has 9; start.rs:402 has 11; work.rs:103 has 8; encode.rs:80 has 8; `encode::terminal` at encode.rs:41-49 has 7 with no allow and passes the `-D warnings` gate; no clippy.toml and no `[lints]` table exist)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Clippy's `too_many_arguments` fires above its threshold (default 7). `read_frames` has exactly seven parameters, so the attribute suppresses nothing and misreports the function's shape; every other allow in the crate sits on an eight-plus-parameter function.

Evidence:

    423	#[allow(clippy::too_many_arguments)]
    424	fn read_frames<Rx>(
    425	    claim: oneshot::Receiver<(Rx, Done<Rx>)>,
    426	    speaker: Speaker,
    427	    stream: Stream,
    428	    budget: RunBudget,
    429	    route: ErrorRoute,
    430	    stats: Recorder,
    431	    observe: SessionHandle,
    432	) -> impl futures::Stream<Item = Frame> + Send

Resolution: Delete the attribute. Acceptance: `just clippy` stays clean.

### inventory-13: Thirteen identical `missing_const_for_thread_local` allows with the rationale copied into seven files
- Where: src/tree.rs:632-639 (related: src/tree.rs:672-679, src/tree/mirror/streaming/backend/local/adversarial.rs:22-29, src/tree/mirror/streaming/channel/instrumented.rs:212-223, src/tree/mirror/streaming/materialized/transcript.rs:59-66, src/tree/mirror/streaming/materialized/progress.rs:391-400, src/tree/mirror/streaming/remote/adapter/decode.rs:565-574, src/tree/mirror/streaming/remote/proxy/work/progress/trace.rs:146-155)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep: 8 `thread_local!` invocations, 13 statics, 7 files, each preceded by the same four-line comment; every enclosing module is `#[cfg(test)]`: tree.rs:628, 668; local.rs:27; channel.rs:89; materialized.rs:176, 180; decode.rs:561; progress.rs:58)
- Verification: confirmed; history: deliberate-and-holds for the allow itself (the comment states its reason), no-rationale-found for the per-site placement
- Owner-gated: no

Every `thread_local!` in the crate is in test code and carries the same allow and the same comment. One crate-level allow with the rationale stated once replaces thirteen attributes and seven copies of the comment. Since the comment says the lint misfires on illumos even for `const { }` initializers, a crate-level allow forfeits nothing the gate could otherwise enforce.

Evidence:

    632	    // clippy's `missing_const_for_thread_local` misreads `thread_local!`'s
    633	    // fallback-TLS lowering (illumos among the gate's targets) and denies
    634	    // initializers that already sit in `const` blocks; the allow keeps
    635	    // `-D warnings` honest on every platform the gate runs.
    636	    thread_local! {
    637	        #[allow(clippy::missing_const_for_thread_local)]
    638	        static ROOT_HASH_READS: Cell<u64> = const { Cell::new(0) };
    639	    }

Resolution: Add `#![allow(clippy::missing_const_for_thread_local)]` to lib.rs (or `[lints.clippy]` in Cargo.toml) with the rationale once, and delete the per-static allows and comments. Acceptance: one allow, one rationale, `just clippy` clean on every gate target.

### inventory-14: Six `pub(crate)` items are referenced only from their own file
- Where: src/message.rs:91-93 (related: src/message.rs:175, src/message.rs:181, src/message.rs:368, src/rumors/changes.rs:76, src/bookmark/format.rs:342)
- Class / severity / confidence: modularity / nit / high
- Provenance: verified (`grep -rln` per name across src, tests, benches, examples: `recursion_limit`, `PayloadSerializer`, `PayloadDeserializer`, `try_from_arc` appear only in message.rs — the tests/future_size.rs hits are rustc's `#![recursion_limit]` attribute; `next_inner` only in changes.rs; `unframe` in format.rs and its child format/tests.rs)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The wider visibility advertises a crate-wide contract none of these items has; a child `tests.rs` sees private items, so `unframe` needs no modifier either.

Evidence:

    91	    pub(crate) fn recursion_limit(self) -> usize {
    92	        usize::try_from(self.0).unwrap_or(usize::MAX)
    93	    }

Resolution: Drop `pub(crate)` on message.rs:91 `recursion_limit`, 175 `PayloadSerializer`, 181 `PayloadDeserializer`, 368 `try_from_arc`; rumors/changes.rs:76 `next_inner`; bookmark/format.rs:342 `unframe`. Acceptance: the six items compile as private with `just test-all` green.

### inventory-15: The crate root aliases a private struct with `pub(crate) use peer::Inner`
- Where: src/lib.rs:339-339 (related: src/batch.rs:8, src/rumors/causal.rs:67, 88, src/rumors/changes.rs:68, src/rumors/unordered.rs:64, 71, 84, 97, src/bookmark.rs:223, src/peer/gossip.rs:41)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `Inner` outside peer.rs and tests; gossip.rs already imports it as `super::Inner`)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

Amid the root's public re-exports sits a crate-private alias for the replica's shared-state struct. Four modules import it as `crate::Inner` and bookmark.rs links to it; `crate::peer::Inner` is one segment longer and says where the type lives, which the bare name does not.

Evidence:

    339	pub(crate) use peer::Inner;

Resolution: Remove the root alias and import `crate::peer::Inner` at the use sites (batch.rs:8, causal.rs:67 and 88, changes.rs:68, unordered.rs:64, 71, 84, 97, and the doc link at bookmark.rs:223). Acceptance: lib.rs's re-export block contains only `pub use` lines.

### inventory-16: The 32-byte path width is a repeated literal with no shared name
- Where: src/tree/typed/prefix.rs:44-49 (related: src/tree/typed/path.rs:50, 76, 88; src/tree/typed/prefix.rs:59, 175; src/tree/typed/untyped.rs:310-312; src/tree/typed/untyped/iter.rs:387, 473-474; src/tree/typed/node.rs:304-305; src/tree/mirror/streaming/window.rs:137; src/tree/typed/height.rs:166; src/tree/typed/hash.rs:12)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (grep for `\b32\b` in typed/ and window.rs; `MERKLE_HASH_LEN` is 24 at hash.rs:12, so the path width is a distinct quantity from the Merkle digest width)
- Verification: confirmed and sharpened: `MERKLE_HASH_LEN = 24` means the crate already names one width and leaves the other as a literal; history: no-rationale-found
- Owner-gated: no

The path (full-width digest) length appears as `32` in arithmetic across path.rs, prefix.rs, untyped.rs, iter.rs, and node.rs, while window.rs privately defines `KEY_DEPTH: usize = 32` for the same quantity and height.rs:166 pins `H32::HEIGHT == 32`. The `[u8; 32]` array types are fine as types; the arithmetic sites are the ones that would drift.

Evidence:

    45	        debug_assert_eq!(
    46	            self.hash.len(),
    47	            32 - H::HEIGHT,
    48	            "an erased prefix re-tags at the height it was erased at",
    49	        );

    137	const KEY_DEPTH: usize = 32;

    12	pub const MERKLE_HASH_LEN: usize = 24;

Resolution: Define one `pub(crate) const PATH_LEN: usize = 32;` in typed/ (or use `Root::HEIGHT`), replace the arithmetic literals at path.rs:50, 76, 88, prefix.rs:47, 59, 175, untyped.rs:310, iter.rs:387, 473, and have window.rs's `KEY_DEPTH` alias it. Acceptance: no bare `32` path-width arithmetic remains in typed/, iter.rs, or window.rs.

### inventory-17: Head width computed as `1 + (n - 1)`
- Where: src/tree/mirror/cbor.rs:191-194
- Class / severity / confidence: simplification / nit / high
- Provenance: verified (read 166-197; `rest` is a suffix of `input` after at least the initial byte, so the inner subtraction cannot underflow)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`read_head` computes the consumed width as `1 + (input.len() - rest.len() - 1)`, which is `input.len() - rest.len()`. The shortest-form check is the codec's central invariant and should read as the plain difference it is.

Evidence:

    191	    let width = 1 + (input.len() - rest.len() - 1);
    192	    if width != head_len(value) {
    193	        return Err(HeadError::NotShortest);
    194	    }

Resolution: `let width = input.len() - rest.len();`. Acceptance: the line reads as a plain difference; the cbor tests pass unchanged.

### inventory-18: A truncating `as u8` on the advertised-name length is guarded only by a `debug_assert`
- Where: src/link/routed/header.rs:247-257 (related: src/link/routed/endpoint.rs:204-207, src/link/routed/endpoint.rs:259, src/link/routed/header.rs:85)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read both sites; `MAX_ADDR_LEN` is `u8::MAX as usize`; `link_header` has one caller, endpoint.rs:259, passing the construction-checked `encoded`)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The bound is established at construction, so the cast is safe today, but a violated invariant would truncate in release rather than fail. This is the only truncating `as` cast in the crate whose bound is not enforced by an adjacent release-mode check or match arm.

Evidence:

    247	pub(super) fn link_header(token: &Token, addr: &[u8]) -> Vec<u8> {
    248	    debug_assert!((1..=MAX_ADDR_LEN).contains(&addr.len()));
    ...
    254	    bytes.push(addr.len() as u8);

Resolution: Replace the `debug_assert` plus `as u8` with `u8::try_from(addr.len()).expect("the endpoint validated the advertised name's length at construction")`, or carry the encoded name as a newtype whose constructor is the only length check. Acceptance: no `as u8` remains in header.rs whose operand bound is not visible at the cast.

### inventory-19: `pub(crate)` fields on a module-private struct
- Where: src/peer/gossip.rs:1379-1382
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read 1375-1404; grep for `PartyGuard` finds no use outside gossip.rs)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

`PartyGuard` has no `pub`, so `pub(crate)` on its fields grants nothing beyond the struct's own visibility and reads as if the fields were crate-shared state.

Evidence:

    1379	struct PartyGuard<T> {
    1380	    pub(crate) party: Option<Party>,
    1381	    pub(crate) recover: watch::Sender<Inner<T>>,
    1382	}

Resolution: Drop the `pub(crate)` on both fields. Acceptance: the fields are private; the file compiles unchanged.

### inventory-20: Expect messages that restate the call instead of proving it
- Where: src/tree/typed/node.rs:361-366 (related: src/peer/gossip.rs:765, 773, 781)
- Class / severity / confidence: idiom / nit / high
- Provenance: verified (read the three sites and their enclosing control flow)
- Verification: confirmed; history: no-rationale-found
- Owner-gated: no

The crate's own standard elsewhere (`expect("a leaf prefix is a full 32-byte path")`, `expect("clamped to usize range")`) is a one-line proof; these three name what failed rather than why it cannot. For `Node<Z>::message` the proof is that a `Node<Z>` is only re-tagged from a node erased at leaf height; for gossip.rs the proof is the `else if guarded.party.is_some()` branch at 765.

Evidence:

    362	    pub fn message(&self) -> &Message {
    363	        self.inner
    364	            .as_leaf()
    365	            .expect("typed leaf failed to be a leaf")
    366	    }

    765	        } else if guarded.party.is_some() {
    ...
    773	            let donated = guarded.party.as_ref().expect("is_some");
    ...
    781	            let donated = guarded.party.take().expect("is_some");

Resolution: node.rs:365: `expect("a Node<Z> is only re-tagged from a node erased at leaf height")`. gossip.rs: bind the first use with `else if let Some(donated) = guarded.party.as_ref()` (the borrow ends before `take()` under NLL) and give the `take()` at 781 a message naming the branch. Acceptance: each message states the reason the panic is unreachable.

## Positives

- Zero caller-, environment-, or wire-reachable panic sites across the 88 production panic-macro sites and 8 release asserts the sweep inventoried. I re-verified the wire-facing parsers named for this sweep: `parse_greeting` returns `GreetingError` on every deviation (greeting.rs:122-203), `read_head` returns `HeadError` for truncation, reserved, indefinite, and non-shortest heads (cbor.rs:166-197), and the leaf decoder returns `OversizedVersion`, `LeafOutsideScope`, `LeafOrder`, and `SupplyOrder` before any run reaches `from_sorted_leaves` (decode.rs:497-540).
- Compile-time pins where they matter, both verified: `const _: () = assert!(std::mem::size_of::<typed::Node<Z>>() == std::mem::size_of::<*const ()>());` at local.rs:110 with its reason stated ("the window's per-reference price rests on it"), and `const _: () = assert!(H0::HEIGHT == 0 && H32::HEIGHT == 32);` at height.rs:166.
- Mutex poison handled deliberately with a stated rationale at router.rs:56-69 and observe.rs:332-343 (production) and memnet.rs:84-91 (test-internals): each says why a single-operation critical section cannot be torn.
- Allows that carry their reasons, verified: `too_many_arguments` at proxy/work.rs:101-102 and proxy/start.rs:338-339 each state the trade ("one premise per argument"), and every other `too_many_arguments` allow sits on a function that clippy would flag.
- The `PartyGuard` drop-recovery comment at gossip.rs:1389-1392 explains why the join runs unconditionally rather than inside a `debug_assert!`: a maintainer-altitude comment stating what the code cannot show.
- Sweep-reported, not re-verified by this pass: recursion discipline throughout (iterative leaf walks, typed traversals bottoming out at `Z`, erased `unknown` bounded by remaining height, the capture renderer's `MAX_DEPTH`); saturating or checked arithmetic on peer-declared quantities (window.rs:432-435, frame.rs `record_len`/`push`, `SupplyLedger::charge`, `PayloadDepthLimit::recursion_limit`); and all 17 `Stream::at_height` pairings the proxy states request resolving against the stream-height table.

## Open questions for Finch

1. `seed_rng` (peer.rs:212-213) is hidden but is also the natural deterministic-seeding entry a downstream test suite would want. Gate it behind `test-internals` (with a private inner function for `seed`), or un-hide and document it? Recommendation: un-hide it; deterministic seeding is a legitimate testing need for users, and the `#[doc(hidden)]` is the only thing making it look internal.
2. The observers' `channel: Option<Channel<T>>` (unordered.rs:40, causal.rs, changes.rs): should `None` mean "ended" (a fused stream that returns `Poll::Ready(None)` forever after close), which dissolves the four `expect("channel state present")` and three `unreachable!("matched Ready above")` sites in one design move? Today a closed observer restores `Ready(rx)` and re-polls open a fresh pass. Recommendation: yes if the fused semantics are acceptable; otherwise the receiver-clone transition removes the `Option` without changing behavior.
3. The `Backend` trait's generality (erase/assume, `node_bytes` pricing, `leaves`/`assemble` overrides, the conformance backend suite) and the release asserts enforcing its contract in adapter/encode.rs:223-262 and adapter/decode.rs:427-439 serve backends that do not yet exist; `Local` is the sole implementor and the trait is unreachable from outside the crate. Designed-ahead, or machinery awaiting a constraint? A design-lens question the inventory cannot settle.
4. The thirteen `missing_const_for_thread_local` allows rest on a claim that clippy misfires on illumos's fallback-TLS lowering. Neither the sweep nor this pass ran clippy on illumos; does the pinned toolchain still exhibit it? If not, the allows and their rationale can go entirely rather than being consolidated.
5. `Stream::at_height` returns `Option` with exactly one production caller expecting `Some` (state.rs:90). A type-level height-to-stream mapping would make the schedule/stride agreement a compile-time fact; the snapshot tests pin it today. Design proposal, not a defect.
6. The debug-only `debug_assert!(false, ..)` guards at batch.rs:137 and gossip.rs:1395 degrade to a no-op (a dropped batch, a leaked fork) in release if the Peer/Rumors exclusivity or party linearity were violated; both are type-enforced, so this is the most benign behavior available, but you may prefer an explicit error path for the batch case.

## Dropped

- Sweep [4]'s `StreamReceiver` component (streams.rs:305-311, 378-396): the `start`/`frames` pair of `Option`s is the consume-once idiom absent a placeholder value for `ReceiverStart` (a `oneshot::Receiver` has none); no panic-free spelling exists without a dependency, and `frames`'s absence carries meaning for `finish`. Not a finding.
- Sweep [4]'s claim that a take-and-restore `match` removes the observers' `expect("channel state present")`: moving the payload out of an `Option` still leaves a `None` arm, so the panic relocates unless `None` is given a meaning. Reframed into inventory-6's design option and open question 2.
- Sweep [5]'s resolution as written for `seed_rng`: `Peer::seed` calls it (peer.rs:207), so the gate cannot be applied directly. Reframed in inventory-4.
- Sweep [6]'s class `verification-gap`: nothing is unverified; the finding is about assert discipline and debug cost. Reclassified as `idiom` in inventory-7.
